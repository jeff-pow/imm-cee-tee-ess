use crate::{arena::NodeIndex, chess_move::Move};

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

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Node {
    game_state: GameState,

    first_child: Option<NodeIndex>,
    num_children: u8,

    m: Move,
    policy: f32,

    visits: i32,
    total_score: f32,
}

impl Node {
    pub const fn new(game_state: GameState, m: Move, policy: f32) -> Self {
        Self {
            game_state,
            total_score: 0.0,
            visits: 0,
            m,
            policy,
            first_child: None,
            num_children: 0,
        }
    }

    pub fn clear(&mut self) {
        self.game_state = GameState::default();
        self.m = Move::NULL;
        self.policy = 0.0;
        self.visits = 0;
        self.total_score = 0.0;
        self.first_child = None;
        self.num_children = 0;
    }

    pub fn is_terminal(&self) -> bool {
        self.game_state.is_terminal()
    }

    pub const fn evaluate(&self) -> Option<f32> {
        self.game_state.evaluate()
    }

    pub fn should_expand(&self) -> bool {
        self.game_state == GameState::Ongoing && self.num_children == 0
    }

    pub const fn has_children(&self) -> bool {
        // Theoretically you only need one of these checks but extra
        // confidence never hurt anyone :)
        self.num_children > 0 && self.first_child.is_some()
    }

    pub const fn first_child(&self) -> Option<NodeIndex> {
        self.first_child
    }

    pub fn set_first_child(&mut self, first_child: NodeIndex) {
        self.first_child = Some(first_child);
    }

    pub const fn expand(&mut self, first_child: NodeIndex, num_children: u8) {
        self.first_child = Some(first_child);
        self.num_children = num_children;
    }

    pub fn num_children(&self) -> usize {
        usize::from(self.num_children)
    }

    pub fn children(&self) -> impl Iterator<Item = NodeIndex> {
        self.first_child
            .map(|first_child| {
                let start = usize::from(first_child);
                let end = start + usize::from(self.num_children);
                start..end
            })
            .into_iter()
            .flatten()
            .map(usize::into)
    }

    pub fn remove_children(&mut self) {
        self.num_children = 0;
        self.first_child = None;
    }

    /// Remove parent node status
    pub fn make_root(&mut self) {
        self.m = Move::NULL;
        self.policy = 1.0;
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

    pub const fn policy(&self) -> f32 {
        self.policy
    }

    pub const fn m(&self) -> Move {
        self.m
    }
}
