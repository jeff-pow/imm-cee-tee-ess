use self::network::Network;

pub mod network;
pub mod util;

pub const INPUT_SIZE: usize = 768 * 4;
pub const L1_SIZE: usize = 256;

static NET: Network = unsafe { std::mem::transmute(*include_bytes!("../../bins/raw.bin")) };
