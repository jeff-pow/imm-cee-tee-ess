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
}

impl Node {
    pub fn new(game_state: GameState, hash: u64) -> Self {
        Self { game_state, edges: Box::new([]), hash }
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
}
