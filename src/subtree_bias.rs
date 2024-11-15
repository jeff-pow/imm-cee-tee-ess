use crate::types::pieces::Color;

#[derive(Clone, Copy, Default)]
struct TableEntry {
    delta_utility_sum: f32,
    weight_sum: f32,
}

const NUM_ENTRIES: usize = 16384;
const ALPHA: f32 = 0.8;
const LAMBDA: f32 = 0.35;
#[derive(Clone)]
pub struct SubtreeBiasTable {
    table: Box<[[TableEntry; NUM_ENTRIES]; 2]>,
}

impl SubtreeBiasTable {
    pub fn update_bias(&mut self, stm: Color, pawn_hash: u64, obs_error: f32, child_visits: i32) -> f32 {
        self.table[stm][pawn_hash as usize % NUM_ENTRIES].delta_utility_sum +=
            obs_error * (child_visits as f32).powf(ALPHA);
        self.table[stm][pawn_hash as usize % NUM_ENTRIES].weight_sum += (child_visits as f32).powf(ALPHA);

        self.bias(stm, pawn_hash)
    }

    pub fn bias(&self, stm: Color, pawn_hash: u64) -> f32 {
        LAMBDA * self.table[stm][pawn_hash as usize % NUM_ENTRIES].delta_utility_sum
            / self.table[stm][pawn_hash as usize % NUM_ENTRIES].weight_sum
    }

    pub fn reset(&mut self) {
        self.table.iter_mut().flatten().for_each(|e| {
            *e = TableEntry {
                delta_utility_sum: 0.,
                weight_sum: 0.,
            }
        });
    }
}

impl Default for SubtreeBiasTable {
    fn default() -> Self {
        Self {
            table: Box::new([[TableEntry::default(); NUM_ENTRIES]; 2]),
        }
    }
}
