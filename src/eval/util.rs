use super::{L1_SIZE, NET};

// Credit to akimbo. This function streamlines the assembly generated and prevents unnecessary
// redundant loads and stores to the same simd vectors. Does sparse matmul.
pub fn f32_update(acc: &mut [f32], adds: &[usize], subs: &[usize]) {
    const REGISTERS: usize = 8;
    const ELEMENTS_PER_LOOP: usize = REGISTERS * 256 / 32;
    const _: () = assert!(L1_SIZE % ELEMENTS_PER_LOOP == 0);
    assert_eq!(acc.len(), L1_SIZE);

    let mut regs = [0f32; ELEMENTS_PER_LOOP];

    for i in 0..L1_SIZE / ELEMENTS_PER_LOOP {
        let offset = ELEMENTS_PER_LOOP * i;

        for (reg, &j) in regs.iter_mut().zip(acc[offset..].iter()) {
            *reg = j;
        }

        for &add in adds {
            let weights = &NET.ft.weights[add];

            for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
                *reg += w;
            }
        }

        for &sub in subs {
            let weights = &NET.ft.weights[sub];

            for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
                *reg -= w;
            }
        }

        for (a, &r) in acc[offset..].iter_mut().zip(regs.iter()) {
            *a = r;
        }
    }
}
//pub fn update(acc: &mut [i16], adds: &[usize], subs: &[usize]) {
//assert_eq!(acc.len(), L1_SIZE);
//const REGISTERS: usize = 8;
//const ELEMENTS_PER_LOOP: usize = REGISTERS * 256 / 16;
//
//let mut regs = [0i16; ELEMENTS_PER_LOOP];
//
//for i in 0..L1_SIZE / ELEMENTS_PER_LOOP {
//    let offset = ELEMENTS_PER_LOOP * i;
//
//    for (reg, &j) in regs.iter_mut().zip(acc[offset..].iter()) {
//        *reg = j;
//    }
//
//    for &add in adds {
//        let weights = &NET.ft.weights[add];
//
//        for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
//            *reg += w;
//        }
//    }
//
//    for &sub in subs {
//        let weights = &NET.ft.weights[sub];
//
//        for (reg, &w) in regs.iter_mut().zip(weights[offset..].iter()) {
//            *reg -= w;
//        }
//    }
//
//    for (a, &r) in acc[offset..].iter_mut().zip(regs.iter()) {
//        *a = r;
//    }
//}
//}
