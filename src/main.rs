use std::env;

fn main() {
    if env::args().any(|x| x == *"bench") {
        imm_cee_tee_ess::bench();
    } else {
        imm_cee_tee_ess::main_loop();
    }
}
