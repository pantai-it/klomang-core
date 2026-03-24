use crate::core::dag::Dag;
use crate::core::crypto::Hash;

pub struct Daa {
    pub target_time: u64,
    pub window_size: usize,
}

impl Daa {
    pub fn new(target_time: u64, window_size: usize) -> Self {
        Self {
            target_time,
            window_size,
        }
    }

    /// Calculate next difficulty based on last N blocks
    pub fn calculate_next_difficulty(&self, dag: &Dag, _current_timestamp: u64) -> u64 {
        let all_blocks: Vec<Hash> = dag.get_all_hashes().into_iter().collect();
        if all_blocks.is_empty() {
            return 1000; // initial difficulty
        }

        // Get recent blocks sorted by timestamp
        let mut recent_blocks: Vec<_> = all_blocks
            .into_iter()
            .filter_map(|h| dag.get_block(&h).map(|b| (h, b.timestamp, b.difficulty)))
            .collect();
        recent_blocks.sort_by_key(|(_, ts, _)| *ts);

        // Take last window_size blocks
        let window: Vec<_> = recent_blocks.into_iter().rev().take(self.window_size).collect();
        if window.len() < 2 {
            return window.last().map(|(_, _, diff)| *diff).unwrap_or(1000);
        }

        // Calculate average block time
        let mut total_time = 0u64;
        for i in 1..window.len() {
            if window[i-1].1 > window[i].1 {
                total_time += window[i-1].1 - window[i].1;
            }
        }
        let avg_block_time = total_time as f64 / (window.len() - 1) as f64;

        // Get current difficulty from latest block
        let current_difficulty = window[0].2 as f64;

        // Adjust difficulty
        let target = self.target_time as f64;
        let adjustment = target / avg_block_time;
        let new_difficulty = (current_difficulty * adjustment) as u64;

        // Clamp to reasonable bounds
        new_difficulty.max(1).min(1_000_000)
    }
}
