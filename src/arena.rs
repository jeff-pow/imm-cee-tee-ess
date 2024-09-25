use crate::{
    chess_move::Move, edge::Edge, hashtable::HashTable, historized_board::HistorizedBoard,
    node::Node, search_type::SearchType, value::SCALE,
};
use std::{
    f32::consts::SQRT_2,
    fmt::Debug,
    mem::size_of,
    ops::{Index, IndexMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const CPUCT: f32 = SQRT_2;
pub const FPU: f32 = 0.5;

pub struct Arena {
    node_list: Box<[Node]>,
    // TODO: Don't even need a free_list. The nodes that were under the deleted node will make
    // their way to the end of the linked list naturally and be removed eventually.
    //
    // In fact, I probably don't need an empty pointer. If the entire linked list is initialized
    // with nodes that point to the next node, it's going to start filling the node_list from the
    // back, which will take as many empty nodes as possible. All kinds of things to do.
    free_list: Vec<usize>,
    hash_table: HashTable,
    depth: u64,
    root: Option<usize>,
    nodes: u64,
    empty: usize,

    root_visits: i32,
    root_total_score: f32,

    lru_head: Option<usize>,
    lru_tail: Option<usize>,
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
        Self {
            node_list: arena.into_boxed_slice(),
            hash_table,
            free_list: Vec::new(),
            root_visits: 0,
            root_total_score: 0.,
            root: None,
            empty: 0,
            depth: 0,
            nodes: 0,
            lru_head: None,
            lru_tail: None,
        }
    }

    pub fn reset(&mut self) {
        self.node_list.iter_mut().for_each(|n| *n = Node::default());
        self.hash_table.clear();
        self.free_list.clear();
        self.root_visits = 0;
        self.root = None;
        self.root_total_score = 0.;
        self.depth = 0;
        self.empty = 0;
        self.nodes = 0;
        self.lru_head = None;
        self.lru_tail = None;
    }

    pub fn insert(
        &mut self,
        board: &HistorizedBoard,
        parent: Option<usize>,
        edge_idx: usize,
    ) -> usize {
        let idx = if self.empty < self.capacity() {
            let idx = self.empty;
            // Lord help me I'm not sure why I can't inline this variable.
            self[idx] = Node::new(board.game_state(), board.hash(), parent, edge_idx);

            self.empty += 1;

            self.empty - 1
        } else if let Some(idx) = self.free_list.pop() {
            self[idx] = Node::new(board.game_state(), board.hash(), parent, edge_idx);
            idx
        } else {
            self.remove_lru_node();
            return self.insert(board, parent, edge_idx);
        };

        assert_eq!(None, self[idx].next());
        assert_eq!(None, self[idx].prev());

        // Don't put the root in the LRU, it should never be able to be removed.
        if self.root.is_some() {
            self.insert_at_head(idx);
        }

        idx
    }

    fn recursively_delete_node(&mut self, idx: usize) {
        assert_ne!(self.root.unwrap(), idx);
        let mut stack = vec![idx];

        while let Some(current_idx) = stack.pop() {
            stack.extend(self[current_idx].edges().iter().filter_map(Edge::child));

            self.remove_arbitrary_node(current_idx);

            self[current_idx] = Node::default();

            self.free_list.push(current_idx);
        }
    }

    #[expect(unused)]
    fn parent_edge(&self, idx: usize) -> &Edge {
        &self[self[idx].parent().unwrap()].edges()[self[idx].parent_edge_idx()]
    }

    fn parent_edge_mut(&mut self, idx: usize) -> &mut Edge {
        let parent = self[idx].parent().unwrap();
        let idx = self[idx].parent_edge_idx();
        &mut self[parent].edges_mut()[idx]
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    pub fn capacity(&self) -> usize {
        self.node_list.len()
    }

    fn remove_lru_node(&mut self) {
        if self.lru_tail.unwrap() == self.root.unwrap() {
            self.print_links();
        }
        self.lru_tail.map_or_else(
            || panic!("Tried to remove a node while there was no tail node in LRU"),
            |tail| {
                assert!(
                    self[tail].prev().is_some(),
                    "Why are we removing one element in the arena? {:?}",
                    self
                );
                match self[tail].prev() {
                    Some(prev) => self[prev].set_next(None),
                    None => self.lru_head = None,
                };
                self.lru_tail = self[tail].prev();
                self.parent_edge_mut(tail).set_child(None);
                self.recursively_delete_node(tail);
            },
        );
    }

    fn remove_arbitrary_node(&mut self, idx: usize) {
        assert_ne!(idx, self.root.unwrap());

        let prev = self[idx].prev();
        let next = self[idx].next();

        if let Some(next) = next {
            self[next].set_prev(prev);
        } else {
            self.lru_tail = prev;
        }

        if let Some(prev) = prev {
            self[prev].set_next(next);
        } else {
            self.lru_head = next;
        }

        self[idx].set_prev(None);
        self[idx].set_next(None);
    }

    fn insert_at_head(&mut self, idx: usize) {
        assert_ne!(idx, self.root.unwrap());
        assert!(self[idx].next().is_none() && self[idx].prev().is_none());

        let old_head = self.lru_head;
        self[idx].set_next(old_head);

        assert!(self[idx]
            .prev()
            .is_none_or(|prev| self[prev].next() != Some(idx)));

        self[idx].set_prev(None);
        match self.lru_head {
            Some(head) => self[head].set_prev(Some(idx)),
            None => self.lru_tail = Some(idx),
        };
        self.lru_head = Some(idx);
    }

    fn move_to_front(&mut self, idx: usize) {
        if self.root.unwrap() == idx {
            return;
        }
        self.remove_arbitrary_node(idx);
        self.insert_at_head(idx);
    }

    pub fn empty_slots(&self) -> usize {
        self.node_list.len() - self.empty + self.free_list.len()
    }

    fn expand(&mut self, ptr: usize, board: &HistorizedBoard) {
        assert!(self[ptr].edges().is_empty() && !self[ptr].is_terminal());
        let legal_moves = board.legal_moves();
        let mut edges = Vec::with_capacity(legal_moves.len());
        for m in legal_moves {
            edges.push(Edge::new(m, None));
        }
        self[ptr].set_edges(edges.into_boxed_slice());
    }

    fn evaluate(&mut self, ptr: usize, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.scaled_eval())
    }

    fn print_links(&self) {
        let Some(mut node) = self.lru_head else {
            return;
        };
        loop {
            println!(
                "Node: {}, prev: {:?}, next: {:?}",
                node,
                self[node].prev(),
                self[node].next()
            );
            if let Some(next) = self[node].next() {
                node = next;
            } else {
                break;
            }
        }
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    // Thanks lightvector! :)
    fn playout(&mut self, ptr: usize, board: &mut HistorizedBoard, parent_visits: i32) -> f32 {
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
            let edge_idx = self.select_action(ptr, parent_visits);

            board.make_move(self[ptr].edges()[edge_idx].m());

            let len = self[ptr].edges().len();
            let child_ptr = self[ptr].edges()[edge_idx].child().unwrap_or_else(|| {
                let child_ptr = self.insert(board, Some(ptr), edge_idx);
                assert_ne!(ptr, child_ptr);
                assert_eq!(self[ptr].edges().len(), len);
                assert!(!self.free_list.contains(&ptr));
                assert!(!self.free_list.contains(&child_ptr));
                self[ptr].edges_mut()[edge_idx].set_child(Some(child_ptr));
                child_ptr
            });

            let u = self.playout(child_ptr, board, self[ptr].edges()[edge_idx].visits());

            // Backpropagation
            self[ptr].edges_mut()[edge_idx].update_stats(u);

            u
        };
        self.move_to_front(ptr);

        assert!((0.0..=1.0).contains(&u));
        1. - u
    }

    // Section 3.4 https://project.dke.maastrichtuniversity.nl/games/files/phd/Chaslot_thesis.pdf
    fn final_move_selection(&self, ptr: usize) -> Option<&Edge> {
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

    #[allow(dead_code)]
    fn display_stats(&self, root: usize) {
        for edge in self[root].edges() {
            println!("{} - n: {:5} - Q: {}", edge.m(), edge.visits(), edge.q());
        }
    }

    // https://github.com/lightvector/KataGo/blob/master/docs/GraphSearch.md#doing-monte-carlo-graph-search-correctly
    /// Returns a usize indexing into the edge that should be selected next
    fn select_action(&self, ptr: usize, parent_edge_visits: i32) -> usize {
        assert!(!self[ptr].edges().is_empty());

        self[ptr]
            .edges()
            .iter()
            .map(|child| {
                let q = if child.visits() == 0 { FPU } else { child.q() };
                // Try to assume an even probability since we don't have a policy yet. No
                // clue if this is a sound idea or not.
                let policy = 1. / self[ptr].edges().len() as f32;

                q + CPUCT * policy * (parent_edge_visits as f32).sqrt()
                    / (1 + child.visits()) as f32
            })
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(index, _)| index)
            .unwrap()
    }

    pub fn print_uci(
        &self,
        nodes: u64,
        search_start: Instant,
        max_depth: u64,
        avg_depth: u64,
        root: usize,
    ) {
        let q = self.final_move_selection(root).unwrap().q();
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

        let mut ptr = Some(root);
        while let Some(p) = ptr {
            if let Some(edge) = self.final_move_selection(p) {
                print!("{} ", edge.m());
                ptr = edge.child();
            } else {
                break;
            }
        }

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
        let root = self.insert(board, None, u32::MAX as usize);
        self.root = Some(root);
        self.root_visits = 0;
        self.root_total_score = 0.;

        let search_start = Instant::now();

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;

        loop {
            self.move_to_front(root);

            self.depth = 0;

            let mut b = board.clone();

            let u = self.playout(root, &mut b, self.root_visits);
            self.root_total_score += u;
            self.root_visits += 1;

            self.nodes += 1;
            max_depth = self.depth.max(max_depth);

            total_depth += self.depth;

            if self.nodes % 256 == 0
                && (halt.load(Ordering::Relaxed) || search_type.hard_stop(&search_start))
            {
                break;
            }

            if total_depth / self.nodes > running_avg_depth && report {
                running_avg_depth = total_depth / self.nodes;
                self.print_uci(
                    self.nodes,
                    search_start,
                    max_depth,
                    total_depth / self.nodes,
                    root,
                );
            }

            if self.nodes % 4096 == 0
                && search_type.should_stop(self.nodes, &search_start, total_depth / self.nodes)
            {
                break;
            }
        }

        if report {
            self.print_uci(
                self.nodes,
                search_start,
                max_depth,
                total_depth / self.nodes,
                root,
            );
        }

        self.final_move_selection(root).unwrap().m()
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str += format!("Head: {:?}\n", self.lru_head).as_str();
        str += format!("Tail: {:?}\n", self.lru_tail).as_str();
        str += format!("{:?}\n", self.free_list).as_str();
        str += "Nodes: \n";
        for node in &self.node_list {
            str += format!("{:?}\n", node).as_str();
        }
        write!(f, "{str}")
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new(2., true)
    }
}
impl Index<usize> for Arena {
    type Output = Node;

    fn index(&self, index: usize) -> &Self::Output {
        &self.node_list[index]
    }
}

impl IndexMut<usize> for Arena {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.node_list[index]
    }
}

#[cfg(test)]
mod arena_tests {
    use super::*;
    use crate::historized_board::HistorizedBoard;

    #[test]
    fn insert_into_empty_arena() {
        let mut arena = Arena::new(32., false);
        let board = HistorizedBoard::default();

        let idx = arena.insert(&board, None, 0);

        assert_eq!(idx, 0);
    }

    #[test]
    fn insert_incrementing_empty_pointer() {
        let mut arena = Arena::new(32., false);
        let board = HistorizedBoard::default();

        let idx1 = arena.insert(&board, None, 0);
        let idx2 = arena.insert(&board, None, 0);

        assert_eq!(idx2, idx1 + 1);
    }

    #[test]
    fn insert_multiple_times() {
        let mut arena = Arena::new(32., false);
        let board = HistorizedBoard::default();

        for i in 0..10 {
            let idx = arena.insert(&board, None, 0);
            assert_eq!(idx, i);
        }
    }

    #[test]
    fn empty_slots() {
        let mut arena = Arena::new(1., false);
        assert_eq!(arena.empty_slots(), arena.node_list.len());

        arena.insert(&HistorizedBoard::default(), None, 0);
        assert_eq!(arena.empty_slots(), arena.node_list.len() - 1);
    }
}
