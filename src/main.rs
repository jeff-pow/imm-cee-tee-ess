// #![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// #![allow(
//     clippy::cast_sign_loss,
//     clippy::module_name_repetitions,
//     clippy::cast_possible_truncation,
//     clippy::cast_precision_loss,
//     clippy::cast_possible_wrap,
//     clippy::large_stack_frames
// )]

use std::env;

fn main() {
    if env::args().any(|x| x == *"bench") {
        imm_cee_tee_ess::bench();
    } else {
        imm_cee_tee_ess::main_loop();
    }
}
