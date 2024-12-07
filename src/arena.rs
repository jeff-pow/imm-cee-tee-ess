use crate::{
    chess_move::Move,
    hashtable::HashTable,
    historized_board::HistorizedBoard,
    node::{GameState, Node},
    search_type::SearchType,
    uci::PRETTY_PRINT,
    value::SCALE,
};
use arrayvec::ArrayVec;
use core::f32;
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
    ptr: ArenaIndex,
    hash: u64,
    m: Move,
}

impl PathEntry {
    const fn new(ptr: ArenaIndex, hash: u64, m: Move) -> Self {
        Self { ptr, hash, m }
    }
}

pub struct Arena {
    node_list: Box<[Node]>,
    hash_table: HashTable,
    depth: u64,
    nodes: u64,
    previous_board: Option<HistorizedBoard>,
    root: ArenaIndex,

    lru_head: ArenaIndex,
    lru_tail: ArenaIndex,
}

impl Arena {
    pub fn new(mb: f32) -> Self {
        let cap = (mb * 15. / 16. * 1024. * 1024. / size_of::<Node>() as f32) as usize;
        assert!(
            (0..u32::MAX as usize).contains(&cap),
            "Indexing scheme does not support tree capacities >= u32::MAX nodes, and tree must have at least one node"
        );
        let arena = vec![Node::default(); cap];

        let hash_table = HashTable::new(mb / 16.);
        let mut arena = Self {
            node_list: arena.into_boxed_slice(),
            hash_table,
            root: ArenaIndex::NONE,
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
            self.node_list[i].set_next(Some((i - 1).into()));
            self.node_list[i].set_prev(Some((i + 1).into()));
        }
        self.node_list[0].set_prev(Some(1.into()));
        self.node_list[0].set_next(None);
        self.lru_tail = 0.into();
        self.node_list[cap - 1].set_next(Some((cap - 2).into()));
        self.lru_head = (cap - 1).into();
        self.node_list[cap - 1].set_prev(None);
    }

    pub fn reset(&mut self) {
        self.node_list.iter_mut().for_each(|n| *n = Node::default());
        self.create_linked_list();
        self.root = ArenaIndex::NONE;
        self.hash_table.clear();
        self.depth = 0;
        self.nodes = 0;
    }

    pub fn insert(&mut self, board: &HistorizedBoard, parent: Option<ArenaIndex>, m: Move, policy: f32) -> ArenaIndex {
        let idx = self.remove_lru_node();
        self[idx] = Node::new(board.game_state(), parent, m, policy);

        self.insert_at_head(idx);

        idx
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    pub const fn capacity(&self) -> usize {
        self.node_list.len()
    }

    fn remove_children(&mut self, ptr: ArenaIndex) {
        for child in self[ptr].children() {
            self.remove_children(child);
            self[child].set_parent(None);
        }
        self[ptr].remove_children();
    }

    /// `ArenaIndex` returned is the start of the contiguous chunk, and it is guaranteed to be at
    /// least as long as the `required_size` parameter
    fn get_contiguous_chunk(&mut self, required_size: usize) -> Option<ArenaIndex> {
        assert!(required_size > 0);
        let mut tail = self.lru_tail;
        while tail != self.root {
            // If that fails, try counting forwards to get the right number of spaces from consecutively
            // unused nodes.
            if !self[tail].has_children()
                && usize::from(tail) + required_size <= self.node_list.len()
                && (0..required_size).all(|i| self[tail + i].parent().is_none())
            {
                for i in 0..required_size {
                    assert!(!self[tail + i].has_children());
                }
                return Some(tail);
            }

            // First see if we can steal it from another node that already had the memory allocated but
            // hasn't used it in a while
            if self[tail].num_children() >= required_size {
                let child_start = self[tail].first_child().unwrap();
                self.remove_children(tail);
                self.move_to_front(tail);
                return Some(child_start);
            }

            tail = self[tail].prev().unwrap();
        }
        None
    }

    fn unallocate_node(&mut self, ptr: ArenaIndex) {
        if let Some(parent) = self[ptr].parent() {
            self.remove_children(parent);
        }
        self.remove_children(ptr);
    }

    fn remove_lru_node(&mut self) -> ArenaIndex {
        let tail = self.lru_tail;
        assert_ne!(tail, self.root);
        let prev = self[tail].prev().expect("What");
        self[prev].set_next(None);
        self.lru_tail = prev;
        // Some nodes need to tell their parents they don't exist anymore, but only nodes that
        // have actually been initialized
        if let Some(parent) = self[tail].parent() {
            self.remove_children(parent);
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

    fn find_next(&self, mut after: ArenaIndex, finding_allocated: bool) -> Option<ArenaIndex> {
        while usize::from(after) < self.capacity() {
            if finding_allocated && self[after].is_allocated() || !finding_allocated && !self[after].is_allocated() {
                return Some(after);
            }
            after = after + 1;
        }
        None
    }

    fn left_align_arena(&mut self, path: &mut [PathEntry]) {
        let mut next_free = self.find_next(0.into(), false).unwrap();
        let mut next_allocated = self.find_next(next_free, true).unwrap();

        while usize::from(next_free) < usize::from(next_allocated) {
            self[next_free] = self[next_allocated];

            if let Some(parent) = self[next_free].parent() {
                if self[parent].first_child() == Some(next_allocated) {
                    self[parent].set_first_child(next_free);
                }
            }

            if self.root == next_allocated {
                self.root = next_free;
            }

            assert!(!path.iter().any(|entry| entry.ptr == next_free));
            path.iter_mut()
                .filter(|entry| entry.ptr == next_allocated)
                .for_each(|entry| entry.ptr = next_free);

            for child in self[next_free].children() {
                self[child].set_parent(Some(next_free));
            }

            self[next_allocated].zero_out();

            next_free = self.find_next(next_free + 1, false).unwrap_or(ArenaIndex::NONE);
            next_allocated = self.find_next(next_allocated + 1, true).unwrap_or(ArenaIndex::NONE);

            if next_free == ArenaIndex::NONE || next_allocated == ArenaIndex::NONE {
                break;
            }
        }
        self.create_linked_list();
        for ptr in (0..self.node_list.len()).map(ArenaIndex::from) {
            if self[ptr].is_allocated() {
                self.move_to_front(ptr);
            } else {
                break;
            }
        }
    }

    fn expand(&mut self, ptr: ArenaIndex, board: &HistorizedBoard, path: &mut [PathEntry]) {
        assert!(!self[ptr].has_children() && !self[ptr].is_terminal(), "{:?}", self[ptr]);

        let policies = board.policies();
        let start = self.get_contiguous_chunk(policies.len()).unwrap_or_else(|| {
            let mut tail = self.lru_tail;
            for _ in 0..self.node_list.len() / 10 {
                self.unallocate_node(tail);
                tail = self[tail].prev().unwrap();
            }
            self.left_align_arena(path);
            self.get_contiguous_chunk(policies.len()).unwrap()
        });
        self[ptr].expand(start, policies.len() as u8);
        for i in 0..policies.len() {
            let (m, pol) = policies[i];
            let mut new_board = board.clone();
            new_board.make_move(m);
            self[start + i].overwrite(new_board.game_state(), Some(ptr), m, pol);
            self.move_to_front(start + i);
        }
    }

    fn evaluate(&self, ptr: ArenaIndex, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.wdl())
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    // Thanks lightvector! :)
    fn playout(&mut self, board: &HistorizedBoard) {
        let mut board = board.clone();
        let mut path = ArrayVec::<PathEntry, 256>::new();
        let mut ptr = self.root;
        path.push(PathEntry::new(ptr, board.hash(), Move::NULL));

        let mut u = loop {
            self.move_to_front(ptr);
            if self[ptr].is_terminal() || self[ptr].visits() == 0 || path.is_full() {
                break self
                    .hash_table
                    .probe(board.hash())
                    .unwrap_or_else(|| self.evaluate(ptr, &board));
            }
            self.depth += 1;
            if self[ptr].should_expand() {
                self.expand(ptr, &board, &mut path);
            }

            // Select
            ptr = self.select_action(ptr);

            board.make_move(self[ptr].m());

            path.push(PathEntry::new(ptr, board.hash(), self[ptr].m()));
        };

        for PathEntry { ptr, hash, m } in path.into_iter().rev() {
            assert_eq!(m, self[ptr].m());
            self.move_to_front(ptr);
            self.hash_table.insert(hash, u);
            u = 1.0 - u;
            self[ptr].update_stats(u);
            assert!((0.0..=1.0).contains(&u));
        }
    }

    // Section 3.4 https://project.dke.maastrichtuniversity.nl/games/files/phd/Chaslot_thesis.pdf
    fn final_move_selection(&self, ptr: ArenaIndex) -> Option<ArenaIndex> {
        let f = |child: ArenaIndex| {
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
        for child in self[self.root].children() {
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

    fn reuse_tree(&self, board: &HistorizedBoard) -> Option<ArenaIndex> {
        let previous_board = self.previous_board.as_ref()?;
        if self.root == ArenaIndex::NONE {
            return None;
        }

        for first_child in self[self.root].children().filter(|&child| self[child].visits() > 0) {
            for second_child in self[first_child].children().filter(|&child| self[child].visits() > 0) {
                let mut temp_board = previous_board.clone();

                temp_board.make_move(self[first_child].m());
                temp_board.make_move(self[second_child].m());

                if temp_board == *board {
                    assert!(self[second_child].visits() > 0 && self[second_child].has_children());
                    return Some(second_child);
                }
            }
        }
        None
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    /// Returns a usize indexing into the edge that should be selected next
    fn select_action(&self, ptr: ArenaIndex) -> ArenaIndex {
        assert!(self[ptr].has_children());
        let parent_total_score = self[ptr].total_score();
        let parent_visits = self[ptr].visits();

        let x = self[ptr]
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
            .map(|(ptr, _)| ptr);
        if x.is_none() {
            self[ptr].children().for_each(|c| {
                dbg!(self[c]);
            });
        }
        x.unwrap()
    }

    pub fn print_uci(&self, nodes: u64, search_start: Instant, max_depth: u64, avg_depth: u64) {
        let q = self[self.final_move_selection(self.root).unwrap()].q();
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

        if let Some(new_root) = self.reuse_tree(board) {
            if !self[new_root].has_children() {
                self.reset();
                self.root = self.insert(board, None, Move::NULL, 1.0);
            } else if new_root != self.root {
                self[new_root].make_root();
                self.root = new_root;
            }
        } else {
            self.reset();
            self.root = self.insert(board, None, Move::NULL, 1.0);
        }
        let root = self.root;
        self[root].set_game_state(GameState::Ongoing);

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;

        loop {
            self.depth = 0;

            self.playout(board);

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

        self[self.final_move_selection(self.root).unwrap()].m()
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
        Self::new(32.)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaIndex(NonZeroU32);

impl ArenaIndex {
    pub(crate) const NONE: Self = unsafe { Self(NonZeroU32::new_unchecked((u32::MAX - 1) ^ u32::MAX)) };
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
        assert_ne!(value, ArenaIndex::NONE);
        (value.0.get() ^ u32::MAX) as Self
    }
}

impl Add<usize> for ArenaIndex {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::from(usize::from(self) + rhs)
    }
}

impl Debug for ArenaIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", usize::from(*self))
    }
}

#[cfg(test)]
mod arena_test {
    use super::*;

    #[test]
    fn test_left_align_arena_larger() {
        let mut arena = Arena::new(1.0);
        arena.node_list = vec![Node::default(); 12].into();
        arena.create_linked_list();

        // Create a tree-like structure with some empty spaces
        // Nodes will be at indices 1, 4, 6, 11
        let root: ArenaIndex = 1.into();

        // Set up root node
        arena[root].overwrite(GameState::default(), None, Move::NULL, 1.0);

        arena[root].expand(3.into(), 3);
        arena[ArenaIndex::from(3)].overwrite(GameState::default(), Some(1.into()), Move::NULL, 1.0);
        arena[ArenaIndex::from(4)].overwrite(GameState::default(), Some(1.into()), Move::NULL, 1.0);
        arena[ArenaIndex::from(5)].overwrite(GameState::default(), Some(1.into()), Move::NULL, 1.0);

        arena[ArenaIndex::from(3)].expand(9.into(), 2);
        arena[ArenaIndex::from(9)].overwrite(GameState::default(), Some(3.into()), Move::NULL, 1.0);
        arena[ArenaIndex::from(10)].overwrite(GameState::default(), Some(3.into()), Move::NULL, 1.0);

        for n in &arena.node_list {
            dbg!(n.first_child(), n.num_children(), n.parent());
            println!();
        }

        println!("\n\n\nAligning\n\n\n");
        arena.left_align_arena(&mut []);

        for n in arena.node_list {
            dbg!(n.first_child(), n.num_children(), n.parent());
            println!();
        }
    }
}
