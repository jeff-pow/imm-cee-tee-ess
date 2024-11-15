use crate::types::pieces::Color;

#[derive(Clone, Copy, Default)]
pub struct TableEntry {
    pub delta_utility_sum: f32,
    pub weight_sum: f32,
}

const NUM_ENTRIES: usize = 16384;
pub const ALPHA: f32 = 0.8;
pub const LAMBDA: f32 = 0.35;
#[derive(Clone)]
pub struct SubtreeBiasTable {
    table: Box<[[TableEntry; NUM_ENTRIES]; 2]>,
}

impl SubtreeBiasTable {
    pub fn update_bias(
        &mut self,
        stm: Color,
        pawn_hash: u64,
        subtree_value_bias_weight: f32,
        subtree_value_bias_delta_sum: f32,
    ) {
        let entry = self.index_mut(stm, pawn_hash);
        entry.delta_utility_sum += subtree_value_bias_weight;
        entry.weight_sum += subtree_value_bias_delta_sum;
    }

    pub fn bias(&self, stm: Color, pawn_hash: u64) -> f32 {
        let entry = self.index(stm, pawn_hash);
        if entry.weight_sum > 1e-3 {
            LAMBDA * entry.delta_utility_sum / entry.weight_sum
        } else {
            0.
        }
    }

    pub fn reset(&mut self) {
        self.table.iter_mut().flatten().for_each(|e| {
            *e = TableEntry {
                delta_utility_sum: 0.,
                weight_sum: 0.,
            }
        });
    }

    pub fn index(&self, stm: Color, pawn_hash: u64) -> TableEntry {
        self.table[stm][((u128::from(pawn_hash) * (NUM_ENTRIES as u128)) >> 64) as usize]
    }

    pub fn index_mut(&mut self, stm: Color, pawn_hash: u64) -> &mut TableEntry {
        &mut self.table[stm][((u128::from(pawn_hash) * (NUM_ENTRIES as u128)) >> 64) as usize]
    }
}

impl Default for SubtreeBiasTable {
    fn default() -> Self {
        Self {
            table: Box::new([[TableEntry::default(); NUM_ENTRIES]; 2]),
        }
    }
}
