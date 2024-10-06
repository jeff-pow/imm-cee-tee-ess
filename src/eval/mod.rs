use self::network::Network;

pub mod network;
pub mod util;

pub const INPUT_SIZE: usize = 768;
pub const L1_SIZE: usize = 768;

static NET: Network = Network {
    ft: network::Layer {
        weights: [[0; 768]; 3072],
        bias: [0; 768],
    },
    l1: network::PerspectiveLayer {
        weights: [[[0; 16]; 768]; 2],
        bias: [0; 16],
    },
    l2: network::Layer {
        weights: [[0.; 16]; 16],
        bias: [0.; 16],
    },
    l3: network::Layer {
        weights: [[0.; 1]; 16],
        bias: [0.; 1],
    },
};
