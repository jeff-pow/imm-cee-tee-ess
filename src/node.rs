use crate::{arena::ArenaIndex, edge::Edge};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum GameState {
    #[expect(unused)]
    Won,
    Draw,
    Lost,
    #[default]
    Ongoing,
}
const _: () = assert!(size_of::<GameState>() == size_of::<Option<GameState>>());

impl GameState {
    const fn evaluate(self) -> Option<f32> {
        match self {
            Self::Won => Some(1.),
            Self::Draw => Some(0.5),
            Self::Lost => Some(0.),
            Self::Ongoing => None,
        }
    }

    fn is_terminal(self) -> bool {
        self != Self::Ongoing
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Node {
    game_state: GameState,
    edges: Box<[Edge]>,

    visits: i32,
    total_score: f32,

    prev: Option<ArenaIndex>,
    next: Option<ArenaIndex>,
    parent: Option<ArenaIndex>,
    edge_idx: u8,
}

impl Node {
    pub fn new(game_state: GameState, parent: Option<ArenaIndex>, edge_idx: usize) -> Self {
        Self {
            game_state,
            edges: Box::new([]),
            prev: None,
            next: None,
            parent,
            edge_idx: edge_idx as u8,
            total_score: 0.0,
            visits: 0,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.game_state.is_terminal()
    }

    pub const fn evaluate(&self) -> Option<f32> {
        self.game_state.evaluate()
    }

    pub fn should_expand(&self) -> bool {
        self.game_state == GameState::Ongoing && self.edges.is_empty()
    }

    pub const fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut [Edge] {
        &mut self.edges
    }

    pub fn set_edges(&mut self, edges: Box<[Edge]>) {
        self.edges = edges;
    }

    pub const fn prev(&self) -> Option<ArenaIndex> {
        self.prev
    }

    pub fn set_prev(&mut self, prev: Option<ArenaIndex>) {
        self.prev = prev;
    }

    pub const fn next(&self) -> Option<ArenaIndex> {
        self.next
    }

    pub fn set_next(&mut self, next: Option<ArenaIndex>) {
        self.next = next;
    }

    pub const fn parent(&self) -> Option<ArenaIndex> {
        self.parent
    }

    pub fn parent_edge_idx(&self) -> usize {
        self.edge_idx.into()
    }

    /// Remove parent node status
    pub fn make_root(&mut self) {
        self.parent = None;
        self.edge_idx = u8::MAX;
    }

    pub fn set_game_state(&mut self, game_state: GameState) {
        self.game_state = game_state;
    }

    pub fn q(&self) -> f32 {
        assert_ne!(
            0, self.visits,
            "User must specify FPU if node hasn't been visited before."
        );
        self.total_score / self.visits as f32
    }

    pub fn update_stats(&mut self, u: f32) {
        self.visits += 1;
        self.total_score += u;
    }

    pub const fn visits(&self) -> i32 {
        self.visits
    }

    pub const fn total_score(&self) -> f32 {
        self.total_score
    }
}
