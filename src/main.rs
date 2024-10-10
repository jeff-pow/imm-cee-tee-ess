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

//fn write_quantized() {
//    static UNQUANTIZED: UnquantizedNetwork = unsafe { std::mem::transmute(*include_bytes!("../bins/raw.bin")) };
//    let quantized = UNQUANTIZED.quantize();
//
//    let mut f = File::create("threats.net").unwrap();
//    unsafe {
//        let ptr: *const Network = quantized.as_ref();
//        let slice_ptr: *const u8 = std::mem::transmute(ptr);
//        let slice = std::slice::from_raw_parts(slice_ptr, std::mem::size_of::<Network>());
//        f.write_all(slice).unwrap();
//    }
//}
