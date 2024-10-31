use crate::{
    chess_move::Move,
    edge::Edge,
    hashtable::HashTable,
    historized_board::HistorizedBoard,
    node::{GameState, Node},
    search_type::SearchType,
    uci::PRETTY_PRINT,
    value::SCALE,
};
use std::{
    f32::consts::SQRT_2,
    fmt::Debug,
    mem::size_of,
    num::NonZeroU32,
    ops::{Index, IndexMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const CPUCT: f32 = SQRT_2;

pub struct Arena {
    node_list: Box<[Node]>,
    hash_table: HashTable,
    depth: u64,
    nodes: u64,
    previous_board: Option<HistorizedBoard>,
    root: ArenaIndex,

    root_visits: i32,
    root_total_score: f32,

    lru_head: ArenaIndex,
    lru_tail: ArenaIndex,
}

impl Arena {
    pub fn new(mb: f32, report: bool) -> Self {
        let cap = (mb * 15. / 16. * 1024. * 1024. / size_of::<Node>() as f32) as usize;
        assert!(
            (0..u32::MAX as usize).contains(&cap),
            "Indexing scheme does not support tree capacities >= u32::MAX nodes, and tree must have at least one node"
        );
        let arena = vec![Node::default(); cap];

        let hash_table = HashTable::new(mb / 16.);
        if report {
            println!(
                "{mb} MB arena created with {} entries and hash table with {} entries.",
                arena.len(),
                hash_table.len()
            );
        }
        let mut arena = Self {
            node_list: arena.into_boxed_slice(),
            hash_table,
            root: ArenaIndex::NONE,
            root_visits: 0,
            root_total_score: 0.,
            depth: 0,
            nodes: 0,
            lru_head: ArenaIndex::NONE,
            lru_tail: ArenaIndex::NONE,
            previous_board: None,
        };
        arena.create_linked_list();
        arena
    }

    /// Bro we GOTTA call this function before returning an arena or it's gonna be BUSTED
    fn create_linked_list(&mut self) {
        let cap = self.node_list.len();
        for i in 1..cap - 1 {
            self.node_list[i].set_next(Some((i + 1).into()));
            self.node_list[i].set_prev(Some((i - 1).into()));
        }
        self.node_list[0].set_next(Some(1.into()));
        self.lru_head = 0.into();
        self.node_list[cap - 1].set_prev(Some((cap - 2).into()));
        self.lru_tail = (cap - 1).into();
    }

    pub fn reset(&mut self) {
        self.node_list.iter_mut().for_each(|n| *n = Node::default());
        self.create_linked_list();
        self.root = ArenaIndex::NONE;
        self.hash_table.clear();
        self.root_visits = 0;
        self.root_total_score = 0.;
        self.depth = 0;
        self.nodes = 0;
    }

    pub fn insert(&mut self, board: &HistorizedBoard, parent: Option<ArenaIndex>, edge_idx: usize) -> ArenaIndex {
        let idx = self.remove_lru_node();
        self[idx] = Node::new(board.game_state(), parent, edge_idx);

        self.insert_at_head(idx);

        idx
    }

    fn parent_edge(&self, idx: ArenaIndex) -> Option<&Edge> {
        Some(&self[self[idx].parent()?].edges()[self[idx].parent_edge_idx()])
    }

    fn parent_edge_mut(&mut self, idx: ArenaIndex) -> Option<&mut Edge> {
        let parent = self[idx].parent()?;
        let child_idx = self[idx].parent_edge_idx();
        Some(&mut self[parent].edges_mut()[child_idx])
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    pub const fn capacity(&self) -> usize {
        self.node_list.len()
    }

    fn remove_lru_node(&mut self) -> ArenaIndex {
        let tail = self.lru_tail;
        assert!(tail != self.root);
        let prev = self[tail].prev().expect("What");
        self[prev].set_next(None);
        self.lru_tail = prev;
        // Some nodes need to tell their parents they don't exist anymore, but only nodes that
        // have actually been initialized
        if let Some(parent) = self.parent_edge_mut(tail) {
            parent.set_child(None);
        }
        tail
    }

    fn remove_arbitrary_node(&mut self, idx: ArenaIndex) {
        let prev = self[idx].prev();
        let next = self[idx].next();

        if let Some(next) = next {
            self[next].set_prev(prev);
        } else {
            self.lru_tail = prev.expect("Didn't have a previous node to make the new tail");
        }

        if let Some(prev) = prev {
            self[prev].set_next(next);
        } else {
            self.lru_head = next.expect("Didn't have a next node to make the new head");
        }

        self[idx].set_prev(None);
        self[idx].set_next(None);
    }

    fn insert_at_head(&mut self, idx: ArenaIndex) {
        let old_head = self.lru_head;
        self[idx].set_next(Some(old_head));
        self[old_head].set_prev(Some(idx));

        self[idx].set_prev(None);

        self.lru_head = idx;
    }

    fn move_to_front(&mut self, idx: ArenaIndex) {
        self.remove_arbitrary_node(idx);
        self.insert_at_head(idx);
    }

    pub fn empty_slots(&self) -> usize {
        // Not sure if having a parent is the best way to denote this but its only for uci
        // output anyway so whatevs
        self.node_list.len() - self.node_list.iter().filter_map(Node::parent).count()
    }

    fn expand(&mut self, ptr: ArenaIndex, board: &HistorizedBoard) {
        assert!(
            self[ptr].edges().is_empty() && !self[ptr].is_terminal(),
            "{:?}",
            self[ptr]
        );
        self[ptr].set_edges(
            board
                .legal_moves()
                .into_iter()
                .map(|m| Edge::new(m, None))
                .collect::<Box<[_]>>(),
        );
    }

    fn evaluate(&self, ptr: ArenaIndex, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.wdl())
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    // Thanks lightvector! :)
    fn playout(
        &mut self,
        ptr: ArenaIndex,
        board: &mut HistorizedBoard,
        parent_visits: i32,
        parent_total_score: f32,
    ) -> f32 {
        self.move_to_front(ptr);
        // Simulate
        let u = if self[ptr].is_terminal() || parent_visits == 0 {
            self.evaluate(ptr, board)
        } else {
            self.depth += 1;
            if self[ptr].should_expand() {
                // Expand
                self.expand(ptr, board);
            }

            // Select
            let edge_idx = self.select_action(ptr, parent_visits, parent_total_score);

            board.make_move(self[ptr].edges()[edge_idx].m());

            let child_ptr = self[ptr].edges()[edge_idx].child().unwrap_or_else(|| {
                let child_ptr = self.insert(board, Some(ptr), edge_idx);
                self[ptr].edges_mut()[edge_idx].set_child(Some(child_ptr));
                child_ptr
            });

            let u = self.playout(
                child_ptr,
                board,
                self[ptr].edges()[edge_idx].visits(),
                self[ptr].edges()[edge_idx].total_score(),
            );

            // Backpropagation
            self[ptr].edges_mut()[edge_idx].update_stats(u);

            u
        };
        self.move_to_front(ptr);

        assert!((0.0..=1.0).contains(&u));
        1. - u
    }

    // Section 3.4 https://project.dke.maastrichtuniversity.nl/games/files/phd/Chaslot_thesis.pdf
    fn final_move_selection(&self, ptr: ArenaIndex) -> Option<&Edge> {
        let f = |edge: &Edge| {
            if edge.visits() == 0 {
                f32::NEG_INFINITY
            } else {
                edge.q()
            }
        };
        self[ptr]
            .edges()
            .iter()
            .max_by(|&e1, &e2| f(e1).partial_cmp(&f(e2)).unwrap())
    }

    fn display_stats(&self) {
        for edge in self[self.root].edges() {
            println!("{} - n: {:8}  -  Q: {}", edge.m(), edge.visits(), edge.q());
        }
    }

    fn reuse_tree(&self, board: &HistorizedBoard) -> Option<ArenaIndex> {
        let previous_board = self.previous_board.as_ref()?;
        if self.root == ArenaIndex::NONE {
            return None;
        }

        for first_edge in self[self.root].edges().iter().filter(|e| e.child().is_some()) {
            assert!(first_edge.child().is_some());
            for second_edge in self[first_edge.child().unwrap()]
                .edges()
                .iter()
                .filter(|e| e.child().is_some())
            {
                let mut temp_board = previous_board.clone();

                temp_board.make_move(first_edge.m());
                temp_board.make_move(second_edge.m());

                if temp_board == *board {
                    assert!(second_edge.child().is_some());
                    return second_edge.child();
                }
            }
        }
        None
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    /// Returns a usize indexing into the edge that should be selected next
    fn select_action(&self, ptr: ArenaIndex, parent_edge_visits: i32, parent_total_score: f32) -> usize {
        assert!(!self[ptr].edges().is_empty());

        self[ptr]
            .edges()
            .iter()
            .map(|child| {
                let q = if child.visits() == 0 {
                    1. - (parent_total_score / parent_edge_visits as f32)
                } else {
                    child.q()
                };
                // Try to assume an even probability since we don't have a policy yet. No
                // clue if this is a sound idea or not.
                let policy = 1. / self[ptr].edges().len() as f32;

                q + CPUCT * policy * (parent_edge_visits as f32).sqrt() / (1 + child.visits()) as f32
            })
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(index, _)| index)
            .unwrap()
    }

    pub fn print_uci(&self, nodes: u64, search_start: Instant, max_depth: u64, avg_depth: u64) {
        let q = self.final_move_selection(self.root).unwrap().q();
        print!(
            "info time {} depth {} seldepth {} score cp {} nodes {} nps {} hashfull {:.0} pv ",
            search_start.elapsed().as_millis(),
            avg_depth,
            max_depth,
            (-SCALE * ((1. - q) / q).ln()) as i32,
            nodes,
            (nodes as f64 / search_start.elapsed().as_secs_f64()) as i64,
            (self.capacity() as f64 - self.empty_slots() as f64) / self.capacity() as f64 * 1000.,
        );

        let mut ptr = Some(self.root);
        while let Some(p) = ptr {
            if let Some(edge) = self.final_move_selection(p) {
                print!("{} ", edge.m());
                ptr = edge.child();
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

        if let Some(new_root) = self.reuse_tree(board) {
            if self[new_root].edges().is_empty() {
                self.reset();
                self.root = self.insert(board, None, usize::MAX);
            } else if new_root != self.root {
                self.root_visits = self.parent_edge(new_root).map_or(0, Edge::visits);
                self.root_total_score = self.parent_edge(new_root).map_or(0.0, Edge::total_score);
                self[new_root].make_root();
                self.root = new_root;
            }
        } else {
            self.reset();
            self.root = self.insert(board, None, usize::MAX);
        }
        let root = self.root;
        self[root].set_game_state(GameState::Ongoing);

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;

        loop {
            self.depth = 0;

            let u = self.playout(self.root, &mut board.clone(), self.root_visits, self.root_total_score);
            self.root_visits += 1;
            self.root_total_score += u;

            self.nodes += 1;
            max_depth = self.depth.max(max_depth);

            total_depth += self.depth;

            if total_depth / self.nodes > running_avg_depth && report {
                running_avg_depth = total_depth / self.nodes;
                self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
            }

            if halt.load(Ordering::Relaxed)
                || search_type.should_stop(self.nodes, &search_start, total_depth / self.nodes)
            {
                break;
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

        self.final_move_selection(self.root).unwrap().m()
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str += format!("Head: {:?}\n", self.lru_head).as_str();
        str += format!("Tail: {:?}\n", self.lru_tail).as_str();
        str += "Nodes: \n";
        for node in &self.node_list {
            str += format!("{node:?}\n").as_str();
        }
        write!(f, "{str}")
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(32., true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ArenaIndex(NonZeroU32);

impl ArenaIndex {
    const NONE: ArenaIndex = unsafe { Self(NonZeroU32::new_unchecked((u32::MAX - 1) ^ u32::MAX)) };
}

impl Index<ArenaIndex> for Arena {
    type Output = Node;

    fn index(&self, index: ArenaIndex) -> &Self::Output {
        &self.node_list[usize::from(index)]
    }
}

impl IndexMut<ArenaIndex> for Arena {
    fn index_mut(&mut self, index: ArenaIndex) -> &mut Self::Output {
        &mut self.node_list[usize::from(index)]
    }
}

impl From<usize> for ArenaIndex {
    fn from(value: usize) -> Self {
        assert!((0..(u32::MAX as usize - 1)).contains(&value));
        NonZeroU32::new(value as u32 ^ u32::MAX).map(Self).unwrap()
    }
}

impl From<ArenaIndex> for usize {
    fn from(value: ArenaIndex) -> Self {
        assert!(value != ArenaIndex::NONE);
        (value.0.get() ^ u32::MAX) as Self
    }
}
