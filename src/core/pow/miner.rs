use crate::core::crypto::Hash;
use super::hash::{calculate_hash, is_valid_pow};

pub struct Pow {
    pub difficulty: u64,
}

impl Pow {
    pub fn new(difficulty: u64) -> Self {
        Self { difficulty }
    }

    /// Calculate target from difficulty
    pub fn target(&self) -> u64 {
        u64::MAX / self.difficulty.max(1)
    }

    /// Mine a block by finding a valid nonce
    pub fn mine(&self, block_data: &[u8]) -> Option<(Hash, u64)> {
        let target = self.target();
        for nonce in 0..u64::MAX {
            let mut data = block_data.to_vec();
            data.extend_from_slice(&nonce.to_le_bytes());
            let hash = calculate_hash(&data);
            if is_valid_pow(&hash, target) {
                return Some((hash, nonce));
            }
            // Prevent infinite loop in tests
            if nonce > 1_000_000 {
                return None;
            }
        }
        None
    }

    /// Validate PoW for a given hash
    pub fn validate(&self, hash: &Hash) -> bool {
        let target = self.target();
        is_valid_pow(hash, target)
    }
}
