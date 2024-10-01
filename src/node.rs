use crate::edge::Edge;

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
    hash: u64,

    prev: Option<usize>,
    next: Option<usize>,
    parent: Option<usize>,
    edge_idx: u8,
}

impl Node {
    pub fn new(game_state: GameState, hash: u64, parent: Option<usize>, edge_idx: usize) -> Self {
        Self {
            game_state,
            edges: Box::new([]),
            hash,
            prev: None,
            next: None,
            parent,
            edge_idx: edge_idx as u8,
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

    pub fn hash(&self) -> u64 {
        self.hash
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

    pub const fn prev(&self) -> Option<usize> {
        self.prev
    }

    pub fn set_prev(&mut self, prev: Option<usize>) {
        self.prev = prev;
    }

    pub const fn next(&self) -> Option<usize> {
        self.next
    }

    pub fn set_next(&mut self, next: Option<usize>) {
        self.next = next;
    }

    pub const fn parent(&self) -> Option<usize> {
        self.parent
    }

    pub fn parent_edge_idx(&self) -> usize {
        self.edge_idx.into()
    }

    pub fn set_parent(&mut self, parent: Option<usize>) {
        self.parent = parent;
    }

    pub fn copy_root_from(&mut self, old_root: Self) {
        self.game_state = old_root.game_state;
        self.edges = old_root.edges;
        self.hash = old_root.hash;
    }

    pub fn reset(&mut self) {
        self.game_state = GameState::default();
        self.edges = [].into();
        self.hash = 0;
        self.parent = None;
        self.edge_idx = u8::MAX;
    }
}
