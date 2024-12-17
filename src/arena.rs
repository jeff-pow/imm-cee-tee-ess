use crate::{
    chess_move::Move,
    hashtable::HashTable,
    historized_board::HistorizedBoard,
    node::{GameState, Node},
    node_buffer::NodeBuffer,
    search_type::SearchType,
    uci::PRETTY_PRINT,
    value::SCALE,
};
use arrayvec::ArrayVec;
use std::{
    f32::consts::SQRT_2,
    fmt::Debug,
    mem::size_of,
    num::NonZeroU32,
    ops::{Add, Index, IndexMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const CPUCT: f32 = SQRT_2;

struct PathEntry {
    ptr: NodeIndex,
    hash: u64,
}

impl PathEntry {
    const fn new(ptr: NodeIndex, hash: u64) -> Self {
        Self { ptr, hash }
    }
}

pub struct Arena {
    node_buffers: [NodeBuffer; 2],
    current_half: usize,
    hash_table: HashTable,
    nodes: u64,
    previous_board: Option<HistorizedBoard>,
}

impl Arena {
    pub fn new(mb: f32) -> Self {
        let cap = (mb * 15. / 16. * 1024. * 1024. / size_of::<Node>() as f32) as usize;
        assert!(
            (0..u32::MAX as usize).contains(&cap),
            "Indexing scheme does not support tree capacities >= u32::MAX nodes, and tree must have at least one node"
        );

        let hash_table = HashTable::new(mb / 16.);
        Self {
            node_buffers: [NodeBuffer::new(cap / 2, 0), NodeBuffer::new(cap / 2, 1)],
            current_half: 0,
            hash_table,
            nodes: 0,
            previous_board: None,
        }
    }

    pub fn reset_completely(&mut self) {
        self.node_buffers.iter_mut().for_each(NodeBuffer::reset);
        self.current_half = 0;
        self.hash_table.clear();
        self.nodes = 0;
        self.previous_board = None;
    }

    pub fn reset_tree(&mut self) {
        self.node_buffers.iter_mut().for_each(NodeBuffer::reset);
        self.current_half = 0;
    }

    pub fn contiguous_chunk(&mut self, required_size: usize) -> Option<NodeIndex> {
        self.node_buffers[self.current_half].get_contiguous(required_size)
    }

    pub fn flip_node(&mut self, from: NodeIndex, to: NodeIndex) {
        self[to] = self[from];
    }

    #[must_use]
    pub fn ensure_children(&mut self, ptr: NodeIndex) -> Option<()> {
        if self[ptr].first_child().unwrap().half() == ptr.half() {
            return Some(());
        }
        let start = self.contiguous_chunk(self[ptr].num_children())?;
        for (i, child) in self[ptr].children().enumerate() {
            self.flip_node(child, start + i);
        }
        self[ptr].set_first_child(start);
        Some(())
    }

    pub fn flip_halves(&mut self) {
        let old_root = self.root();
        self.current_half ^= 1;
        self.node_buffers[self.current_half].reset();

        let new_root = self.contiguous_chunk(1).unwrap();
        assert_eq!(0, new_root.idx());
        self.flip_node(old_root, new_root);

        self.node_buffers[self.current_half ^ 1].clear_references();
    }

    pub fn root(&self) -> NodeIndex {
        NodeIndex::new(self.current_half, 0)
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    #[must_use]
    fn expand(&mut self, ptr: NodeIndex, board: &HistorizedBoard) -> Option<()> {
        assert!(!self[ptr].has_children() && !self[ptr].is_terminal(), "{:?}", self[ptr]);

        let policies = board.policies();
        let start = self.contiguous_chunk(policies.len())?;

        self[ptr].expand(start, policies.len());
        assert!(self[ptr].has_children());
        for i in 0..policies.len() {
            let (m, pol) = policies[i];
            let mut new_board = board.clone();
            new_board.make_move(m);
            self[start + i] = Node::new(new_board.game_state(), m, pol);
        }

        Some(())
    }

    fn evaluate(&self, ptr: NodeIndex, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.wdl())
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    // Thanks lightvector! :)
    #[must_use]
    fn playout(&mut self, board: &HistorizedBoard, depth: &mut u64) -> Option<()> {
        let mut board = board.clone();
        let mut path = ArrayVec::<PathEntry, 256>::new();
        let mut ptr = self.root();
        path.push(PathEntry::new(ptr, board.hash()));

        let mut u = loop {
            if self[ptr].is_terminal() || self[ptr].visits() == 0 || path.is_full() {
                break self
                    .hash_table
                    .probe(board.hash())
                    .unwrap_or_else(|| self.evaluate(ptr, &board));
            }
            *depth += 1;
            if self[ptr].should_expand() {
                self.expand(ptr, &board)?;
                assert!(self[ptr].has_children(), "{}", board.board());
            }

            self.ensure_children(ptr)?;

            // Select
            ptr = self.select_action(ptr);

            board.make_move(self[ptr].m());

            path.push(PathEntry::new(ptr, board.hash()));
        };

        for PathEntry { ptr, hash } in path.into_iter().rev() {
            self.hash_table.insert(hash, u);
            u = 1.0 - u;
            self[ptr].update_stats(u);

            assert!((0.0..=1.0).contains(&u));
        }

        Some(())
    }

    // Section 3.4 https://project.dke.maastrichtuniversity.nl/games/files/phd/Chaslot_thesis.pdf
    fn final_move_selection(&self, ptr: NodeIndex) -> Option<NodeIndex> {
        let f = |child: NodeIndex| {
            if self[child].visits() == 0 {
                f32::NEG_INFINITY
            } else {
                self[child].q()
            }
        };
        self[ptr]
            .children()
            .max_by(|&e1, &e2| f(e1).partial_cmp(&f(e2)).unwrap())
    }

    fn display_stats(&self) {
        for child in self[self.root()].children() {
            if self[child].visits() > 0 {
                println!(
                    "{} - n: {:8}  -  Q: {}",
                    self[child].m(),
                    self[child].visits(),
                    self[child].q()
                );
            } else {
                println!("{} - unvisited", self[child].m());
            }
        }
    }

    fn reuse_tree(&self, board: &HistorizedBoard) -> Option<NodeIndex> {
        if self.node_buffers.iter().all(NodeBuffer::empty) {
            return None;
        }

        let previous_board = self.previous_board.as_ref()?;

        for first_child in self[self.root()].children().filter(|&child| self[child].visits() > 0) {
            for second_child in self[first_child].children().filter(|&child| self[child].visits() > 0) {
                let mut temp_board = previous_board.clone();

                temp_board.make_move(self[first_child].m());
                temp_board.make_move(self[second_child].m());

                if temp_board == *board {
                    return Some(second_child);
                }
            }
        }
        None
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    fn select_action(&self, ptr: NodeIndex) -> NodeIndex {
        assert!(self[ptr].has_children());
        let parent_total_score = self[ptr].total_score();
        let parent_visits = self[ptr].visits();

        self[ptr]
            .children()
            .map(|child| {
                let q = if self[child].visits() == 0 {
                    1. - (parent_total_score / parent_visits as f32)
                } else {
                    self[child].q()
                };

                let child_visits = self[child].visits();
                (
                    child,
                    q + CPUCT * self[child].policy() * (parent_visits as f32).sqrt() / (1 + child_visits) as f32,
                )
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(ptr, _)| ptr)
            .unwrap()
    }

    pub fn print_uci(&self, nodes: u64, search_start: Instant, max_depth: u64, avg_depth: u64) {
        let q = self[self.final_move_selection(self.root()).unwrap()].q();
        print!(
            "info time {} depth {} seldepth {} score cp {} nodes {} nps {} pv ",
            search_start.elapsed().as_millis(),
            avg_depth,
            max_depth,
            (-SCALE * ((1. - q) / q).ln()) as i32,
            nodes,
            (nodes as f64 / search_start.elapsed().as_secs_f64()) as i64,
        );

        let mut ptr = Some(self.root());
        while let Some(p) = ptr {
            if let Some(child) = self.final_move_selection(p) {
                print!("{} ", self[child].m());
                ptr = Some(child);
            } else {
                break;
            }
        }
        println!();
    }

    pub fn start_search(
        &mut self,
        board: &HistorizedBoard,
        halt: &AtomicBool,
        search_type: SearchType,
        report: bool,
    ) -> Move {
        let search_start = Instant::now();
        self.nodes = 0;

        if let Some(new_root) = self.reuse_tree(board) {
            if !self[new_root].has_children() {
                self.reset_tree();
                let root = self.contiguous_chunk(1).unwrap();
                self[root] = Node::new(GameState::Ongoing, Move::NULL, 1.0);
            } else if new_root != self.root() {
                println!("Reused!");
                self[new_root].make_root();
                let old_root = self.root();
                self[old_root].clear();
                self.flip_node(new_root, self.root());
            }
        } else {
            self.reset_tree();
            let root = self.contiguous_chunk(1).unwrap();
            self[root] = Node::new(GameState::Ongoing, Move::NULL, 1.0);
        }

        let root = self.root();
        self[root].set_game_state(GameState::Ongoing);

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;
        let mut timer = Instant::now();

        loop {
            let mut depth = 0;

            if self.playout(board, &mut depth).is_none() && !halt.load(Ordering::Relaxed) {
                self.flip_halves();
                continue;
            }

            self.nodes += 1;
            max_depth = depth.max(max_depth);

            total_depth += depth;

            if total_depth / self.nodes > running_avg_depth && report {
                running_avg_depth = total_depth / self.nodes;
                self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
            }

            if halt.load(Ordering::Relaxed)
                || search_type.should_stop(self.nodes, &search_start, total_depth / self.nodes)
            {
                break;
            }

            if timer.elapsed().as_secs() > 2 {
                self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
                timer = Instant::now();
            }
        }

        if report {
            self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
        }
        // TODO: Display stats if not in UCI mode, and add output if bestmove changes or every few nodes idk
        //       Also do tree reuse
        if report && PRETTY_PRINT.load(Ordering::Relaxed) {
            self.display_stats();
        }

        self.previous_board = Some(board.clone());

        self[self.final_move_selection(self.root()).unwrap()].m()
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str += "Nodes: \n";
        for node in &self.node_buffers {
            str += format!("{node:?}\n").as_str();
        }
        write!(f, "{str}")
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(32.)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeIndex(NonZeroU32);

impl NodeIndex {
    pub fn new(half: usize, idx: usize) -> Self {
        (half << 31 | idx).into()
    }

    pub fn half(self) -> usize {
        usize::from(self) >> 31
    }

    pub fn idx(self) -> usize {
        usize::from(self) & 0x7FFF_FFFF
    }
}

impl NodeIndex {
    const NONE: Self = unsafe { Self(NonZeroU32::new_unchecked((u32::MAX - 1) ^ u32::MAX)) };
}

impl Index<NodeIndex> for Arena {
    type Output = Node;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.node_buffers[index.half()][index]
    }
}

impl IndexMut<NodeIndex> for Arena {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.node_buffers[index.half()][index]
    }
}

impl From<usize> for NodeIndex {
    fn from(value: usize) -> Self {
        assert!((0..(u32::MAX as usize - 1)).contains(&value));
        NonZeroU32::new(value as u32 ^ u32::MAX).map(Self).unwrap()
    }
}

impl From<NodeIndex> for usize {
    fn from(value: NodeIndex) -> Self {
        assert!(value != NodeIndex::NONE);
        (value.0.get() ^ u32::MAX) as Self
    }
}

impl Add<usize> for NodeIndex {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::from(usize::from(self) + rhs)
    }
}
