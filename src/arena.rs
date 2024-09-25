use crate::{
    chess_move::Move, edge::Edge, hashtable::HashTable, historized_board::HistorizedBoard, node::Node,
    search_type::SearchType, value::SCALE,
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
    hash_table: HashTable,
    depth: u64,
    nodes: u64,

    root_visits: i32,
    root_total_score: f32,

    lru_head: usize,
    lru_tail: usize,
}

/// Root node will always live at the first slot and never be part of the LRU removal cache, so it can never get removed.
const ROOT_NODE_IDX: usize = 0;

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
            root_visits: 0,
            root_total_score: 0.,
            depth: 0,
            nodes: 0,
            lru_head: usize::MAX,
            lru_tail: usize::MAX,
        };
        arena.create_linked_list();
        arena
    }

    /// Bro we GOTTA call this function before returning an arena or it's gonna be BUSTED
    fn create_linked_list(&mut self) {
        let cap = self.node_list.len();
        for i in 2..cap - 1 {
            self.node_list[i].set_next(Some(i + 1));
            self.node_list[i].set_prev(Some(i - 1));
        }
        self.node_list[1].set_next(Some(2));
        self.lru_head = 1;
        self.node_list[cap - 1].set_prev(Some(cap - 2));
        self.lru_tail = cap - 1;
    }

    pub fn reset(&mut self) {
        self.node_list.iter_mut().for_each(|n| *n = Node::default());
        self.create_linked_list();
        self.hash_table.clear();
        self.root_visits = 0;
        self.root_total_score = 0.;
        self.depth = 0;
        self.nodes = 0;
    }

    pub fn insert(&mut self, board: &HistorizedBoard, parent: Option<usize>, edge_idx: usize) -> usize {
        let idx = self.remove_lru_node();
        self[idx] = Node::new(board.game_state(), board.hash(), parent, edge_idx);

        self.insert_at_head(idx);

        idx
    }

    #[expect(unused)]
    fn parent_edge(&self, idx: usize) -> &Edge {
        &self[self[idx].parent().unwrap()].edges()[self[idx].parent_edge_idx()]
    }

    #[expect(unused)]
    fn parent_edge_mut(&mut self, idx: usize) -> &mut Edge {
        let parent = self[idx].parent().unwrap();
        let child_idx = self[idx].parent_edge_idx();
        &mut self[parent].edges_mut()[child_idx]
    }

    fn try_parent_edge_mut(&mut self, idx: usize) -> Option<&mut Edge> {
        let parent = self[idx].parent()?;
        let idx = self[idx].parent_edge_idx();
        self[parent].edges_mut().get_mut(idx)
    }

    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    pub fn capacity(&self) -> usize {
        self.node_list.len()
    }

    fn remove_lru_node(&mut self) -> usize {
        let tail = self.lru_tail;
        assert!(tail != ROOT_NODE_IDX);
        let prev = self[tail].prev().expect("What");
        self[prev].set_next(None);
        self.lru_tail = prev;
        // Some nodes need to tell their parents they don't exist anymore, but only nodes that
        // have actually been initialized
        if let Some(parent) = self.try_parent_edge_mut(tail) {
            parent.set_child(None);
        }
        tail
    }

    fn remove_arbitrary_node(&mut self, idx: usize) {
        assert!(idx != ROOT_NODE_IDX);
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

    fn insert_at_head(&mut self, idx: usize) {
        assert!(idx != ROOT_NODE_IDX);

        let old_head = self.lru_head;
        self[idx].set_next(Some(old_head));
        self[old_head].set_prev(Some(idx));

        self[idx].set_prev(None);

        self.lru_head = idx;
    }

    fn move_to_front(&mut self, idx: usize) {
        // Return early to prevent root from getting put into node list
        if ROOT_NODE_IDX == idx {
            return;
        }
        self.remove_arbitrary_node(idx);
        self.insert_at_head(idx);
    }

    pub fn empty_slots(&self) -> usize {
        // Not sure if having a parent is the best way to denote this but its only for uci
        // output anyway so whatevs
        self.node_list.len() - self.node_list.iter().filter_map(|n| n.parent()).count()
    }

    fn expand(&mut self, ptr: usize, board: &HistorizedBoard) {
        assert!(self[ptr].edges().is_empty() && !self[ptr].is_terminal());
        self[ptr].set_edges(
            board.legal_moves().into_iter().map(|m| Edge::new(m, None)).collect::<Vec<_>>().into_boxed_slice(),
        );
    }

    fn evaluate(&mut self, ptr: usize, board: &HistorizedBoard) -> f32 {
        self[ptr].evaluate().unwrap_or_else(|| board.scaled_eval())
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

            let child_ptr = self[ptr].edges()[edge_idx].child().unwrap_or_else(|| {
                let child_ptr = self.insert(board, Some(ptr), edge_idx);
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
        self[ptr].edges().iter().max_by(|&e1, &e2| f(e1).partial_cmp(&f(e2)).unwrap())
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

                q + CPUCT * policy * (parent_edge_visits as f32).sqrt() / (1 + child.visits()) as f32
            })
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(index, _)| index)
            .unwrap()
    }

    pub fn print_uci(&self, nodes: u64, search_start: Instant, max_depth: u64, avg_depth: u64, root: usize) {
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
        println!();
    }

    pub fn start_search(
        &mut self,
        board: &HistorizedBoard,
        halt: &AtomicBool,
        search_type: SearchType,
        report: bool,
    ) -> Move {
        self[ROOT_NODE_IDX] = Node::new(board.game_state(), board.hash(), None, u32::MAX as usize);

        self.root_visits = 0;
        self.root_total_score = 0.;

        let search_start = Instant::now();

        let mut total_depth = 0;
        let mut max_depth = 0;
        let mut running_avg_depth = 0;

        loop {
            self.depth = 0;

            let mut b = board.clone();

            let u = self.playout(ROOT_NODE_IDX, &mut b, self.root_visits);
            self.root_total_score += u;
            self.root_visits += 1;

            self.nodes += 1;
            max_depth = self.depth.max(max_depth);

            total_depth += self.depth;

            if self.nodes % 256 == 0
                && (halt.load(Ordering::Relaxed) || search_type.hard_stop(self.nodes, &search_start))
            {
                break;
            }

            if total_depth / self.nodes > running_avg_depth && report {
                running_avg_depth = total_depth / self.nodes;
                self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes, ROOT_NODE_IDX);
            }

            if self.nodes % 4096 == 0 && search_type.should_stop(self.nodes, &search_start, total_depth / self.nodes) {
                break;
            }
        }

        if report {
            self.print_uci(self.nodes, search_start, max_depth, total_depth / self.nodes, ROOT_NODE_IDX);
        }

        self.final_move_selection(ROOT_NODE_IDX).unwrap().m()
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str += format!("Head: {:?}\n", self.lru_head).as_str();
        str += format!("Tail: {:?}\n", self.lru_tail).as_str();
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
