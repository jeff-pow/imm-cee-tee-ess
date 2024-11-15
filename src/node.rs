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

    obs_error: Option<f32>,
    nn_utility: Option<f32>,

    prev: Option<ArenaIndex>,
    next: Option<ArenaIndex>,
    parent: Option<ArenaIndex>,
    edge_idx: u8,

    pub last_delta_sum: f32,
    pub last_bias_weight: f32,
}

impl Node {
    pub fn new(game_state: GameState, parent: Option<ArenaIndex>, edge_idx: usize) -> Self {
        Self {
            game_state,
            edges: Box::new([]),
            obs_error: None,
            nn_utility: None,
            prev: None,
            next: None,
            parent,
            edge_idx: edge_idx as u8,
            last_delta_sum: 0.,
            last_bias_weight: 0.,
        }
    }

    pub fn set_nn_utility(&mut self, value: f32) {
        self.nn_utility = Some(value);
    }

    pub const fn nn_utility(&self) -> f32 {
        self.nn_utility.unwrap()
    }

    pub fn set_obs_error(&mut self, value: f32) {
        self.obs_error = Some(value);
    }

    pub const fn obs_error(&self) -> f32 {
        self.obs_error.unwrap()
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
}
