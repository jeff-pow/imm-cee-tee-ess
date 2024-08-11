use crate::{
    chess_move::Move, edge::Edge, hashtable::HashTable, historized_board::HistorizedBoard, node::Node,
    search::SearchType, value::SCALE,
};
use std::{
    f32::consts::SQRT_2,
    mem::size_of,
    num::NonZeroU32,
    ops::{Index, IndexMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const CPUCT: f32 = SQRT_2;
pub const FPU: f32 = 0.5;

pub struct Arena {
    node_list: Box<[Node]>,
    #[allow(dead_code)]
    hash_table: HashTable,
    depth: u64,
    /// Pointer to the next empty slot in the 'arena'
    empty: ArenaIndex,
    nodes: u64,

    root: Option<ArenaIndex>,
    root_visits: i32,
    root_total_score: f32,
}

impl Arena {
    pub fn new(mb: usize, report: bool) -> Self {
        let cap = mb * 15 / 16 * 1024 * 1024 / size_of::<Node>();
        let arena = vec![Node::default(); cap];
        let hash_table = HashTable::new(mb / 16);
        if report {
            println!(
                "{mb} MB arena created with {} entries and hash table with {} entries.",
                arena.len(),
                hash_table.len()
            );
        }
        Self {
            node_list: arena.into_boxed_slice(),
            hash_table,
            root: None,
            root_visits: 0,
            root_total_score: 0.,
            depth: 0,
            empty: 0.into(),
            nodes: 0,
        }
    }

    pub fn insert(&mut self, board: &HistorizedBoard) -> ArenaIndex {
        let idx = self.empty;
        // Lord help me I'm not sure why I can't inline this variable.
        self[idx] = Node::new(board.game_state(), board.hash());

        self.empty += 1;

        self.empty - 1
    }

    fn expand(&mut self, ptr: ArenaIndex, board: &HistorizedBoard) {
        assert!(self[ptr].edges().is_empty() && !self[ptr].is_terminal());
        let legal_moves = board.legal_moves();
        let mut edges = Vec::with_capacity(legal_moves.len());
        for m in legal_moves {
            edges.push(Edge::new(m, None));
        }
        self[ptr].set_edges(edges.into_boxed_slice());
    }

    fn evaluate(&self, ptr: ArenaIndex, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.scaled_eval())
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    // Thanks lightvector! :)
    fn playout(&mut self, ptr: ArenaIndex, board: &mut HistorizedBoard, parent_visits: i32) -> f32 {
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
            let edge_idx = self.select_action(ptr, parent_visits);

            board.make_move(self[ptr].edges()[edge_idx].m());

            let child_ptr = self[ptr].edges()[edge_idx].child().unwrap_or_else(|| {
                // let child_ptr = edge.child().unwrap_or_else(|| {
                // TODO: Hash table lookup for graph transposition shenanigans
                // let child_ptr;
                // if let Some(hash_table_ptr) = self.hash_table.probe(board.hash()) {
                //     child_ptr = hash_table_ptr;
                // } else {
                //     child_ptr = self.insert(board);
                //     self.hash_table.insert(board.hash(), child_ptr);
                // }
                let child_ptr = self.insert(board);
                self[ptr].edges_mut()[edge_idx].set_child(child_ptr);
                child_ptr
            });

            let u = self.playout(child_ptr, board, self[ptr].edges()[edge_idx].visits());

            // Backpropogate
            self[ptr].edges_mut()[edge_idx].update_stats(u);

            u
        };

        assert!((0.0..=1.0).contains(&u));
        1. - u
    }

    // Section 3.4 https://project.dke.maastrichtuniversity.nl/games/files/phd/Chaslot_thesis.pdf
    fn final_move_selection(&self, ptr: ArenaIndex) -> Option<&Edge> {
        let f = |edge: &Edge| if edge.visits() == 0 { f32::NEG_INFINITY } else { edge.q() };
        self[ptr].edges().iter().max_by(|&e1, &e2| f(e1).partial_cmp(&f(e2)).unwrap())
    }

    #[allow(dead_code)]
    fn display_stats(&self) {
        for edge in self[self.root.unwrap()].edges() {
            println!("{} - n: {:5} - Q: {}", edge.m(), edge.visits(), edge.q());
        }
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    /// Returns a usize indexing into the edge that should be selected next
    fn select_action(&self, ptr: ArenaIndex, parent_edge_visits: i32) -> usize {
        assert!(!self[ptr].edges().is_empty());

        self[ptr]
            .edges()
            .iter()
            .map(|child| {
                let q = if child.visits() == 0 { FPU } else { child.q() };
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
        let q = self.final_move_selection(self.root.unwrap()).unwrap().q();
        print!(
            "info time {} depth {} seldepth {} score cp {} nodes {} nps {} hashfull {} pv ",
            search_start.elapsed().as_millis(),
            avg_depth,
            max_depth,
            (-SCALE * ((1. - q) / q).ln()) as i32,
            nodes,
            (nodes as f64 / search_start.elapsed().as_secs_f64()) as i64,
            (self.node_list.len() - usize::from(self.empty)) / self.node_list.len() * 1000,
        );

        let mut ptr = self.root;
        while let Some(edge) = self.final_move_selection(ptr.unwrap()) {
            print!("{} ", edge.m());
            ptr = edge.child();
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
        *self = Self::default();

        let node = self.insert(board);
        self.root = Some(node);
        self.root_visits = 0;
        self.root_total_score = 0.;

        let search_start = Instant::now();

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;

        loop {
            // Hacky solution to deal with running out of memory. Just quit the search :/
            if usize::from(self.empty) as f32 >= 0.97 * self.node_list.len() as f32 {
                break;
            }
            self.depth = 0;

            let mut b = board.clone();

            let u = self.playout(self.root.unwrap(), &mut b, self.root_visits);
            self.root_total_score += u;
            self.root_visits += 1;

            self.nodes += 1;
            max_depth = self.depth.max(max_depth);

            total_depth += self.depth;

            if self.nodes % 256 == 0 && (halt.load(Ordering::Relaxed) || search_type.hard_stop(&search_start)) {
                break;
            }

            if total_depth / self.nodes > running_avg_depth && report {
                running_avg_depth = total_depth / self.nodes;
                self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
            }

            if self.nodes % 4096 == 0 && search_type.should_stop(self.nodes, &search_start, total_depth / self.nodes) {
                break;
            }
        }

        if report {
            self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes);
        }

        self.final_move_selection(self.root.unwrap()).unwrap().m()
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(32, false)
    }
}

const _: () = assert!(size_of::<ArenaIndex>() == size_of::<Option<ArenaIndex>>());

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ArenaIndex(NonZeroU32);

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
        // Value can't be equal to u32 max because we're offsetting all indexes by one to account
        // for the fact that zero is our None value for a NonZeroU32
        assert_ne!(value, u32::MAX as usize);
        Self(NonZeroU32::new(u32::try_from(value).unwrap() + 1).unwrap())
    }
}

impl From<ArenaIndex> for usize {
    fn from(value: ArenaIndex) -> Self {
        (value.0.get() - 1) as usize
    }
}

macro_rules! non_assign_ops {
    ($($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait<ArenaIndex> for ArenaIndex {
            type Output = Self;

            fn $fn(self, rhs: Self) -> Self::Output {
                Self::from(std::ops::$trait::$fn(usize::from(self), usize::from(rhs)))
            }
        })*

        $(impl std::ops::$trait<usize> for ArenaIndex {
            type Output = Self;

            fn $fn(self, rhs: usize) -> Self::Output {
                Self::from(std::ops::$trait::$fn(usize::from(self), usize::from(rhs)))
            }
        })*
    };
}
non_assign_ops!(Add::add, Sub::sub);

macro_rules! assign_ops {
    ($($trait:ident::$fn:ident),*) => {
        $(impl std::ops::$trait<ArenaIndex> for ArenaIndex {

            fn $fn(&mut self, rhs: Self) {
                let mut x = usize::from(*self);
                std::ops::$trait::$fn(&mut x, usize::from(rhs));
                *self = ArenaIndex::from(x);
            }
        })*

        $(impl std::ops::$trait<usize> for ArenaIndex {

            fn $fn(&mut self, rhs: usize) {
                let mut x = self.0.get() as usize;
                std::ops::$trait::$fn(&mut x, rhs);
                *self = Self(NonZeroU32::new(x as u32).unwrap());
            }
        })*
    };
}
assign_ops!(AddAssign::add_assign, SubAssign::sub_assign);

#[cfg(test)]
mod bruh {
    use super::ArenaIndex;

    #[test]
    fn add_assign() {
        let (mut x, y, z) = (ArenaIndex::from(3), ArenaIndex::from(5), ArenaIndex::from(8));
        x += y;
        assert_eq!(x, z);
    }
}
