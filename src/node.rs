use crate::{arena::ArenaIndex, chess_move::Move};

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

    m: Move,
    first_child: Option<ArenaIndex>,
    num_children: u8,
    policy: f32,

    visits: i32,
    total_score: f32,

    prev: Option<ArenaIndex>,
    next: Option<ArenaIndex>,
    parent: Option<ArenaIndex>,
}

impl Node {
    pub const fn new(game_state: GameState, parent: Option<ArenaIndex>, m: Move, policy: f32) -> Self {
        Self {
            game_state,
            prev: None,
            next: None,
            parent,
            total_score: 0.0,
            visits: 0,
            m,
            policy,
            first_child: None,
            num_children: 0,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.game_state.is_terminal()
    }

    pub const fn evaluate(&self) -> Option<f32> {
        self.game_state.evaluate()
    }

    pub const fn has_children(&self) -> bool {
        // Theoretically you only need one of these checks but extra
        // confidence never hurt anyone :)
        self.num_children > 0 && self.first_child.is_some()
    }

    pub const fn first_child(&self) -> ArenaIndex {
        self.first_child.unwrap()
    }

    pub fn num_children(&self) -> usize {
        usize::from(self.num_children)
    }

    pub fn children(&self) -> impl Iterator<Item = ArenaIndex> {
        let x = usize::from(self.first_child.unwrap());
        (x..x + usize::from(self.num_children)).map(usize::into)
    }

    pub fn remove_children(&mut self) {
        self.num_children = 0;
        self.first_child = None;
    }

    pub const fn parent(&self) -> Option<ArenaIndex> {
        self.parent
    }

    pub fn set_parent(&mut self, parent: Option<ArenaIndex>) {
        self.parent = parent;
    }

    pub fn should_expand(&self) -> bool {
        self.game_state == GameState::Ongoing && self.num_children == 0
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

    /// Remove parent node status
    pub fn make_root(&mut self) {
        self.parent = None;
        self.m = Move::NULL;
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

    pub fn policy(&self) -> f32 {
        self.policy
    }

    pub fn m(&self) -> Move {
        self.m
    }
}
