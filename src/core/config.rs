pub struct Config {
    pub k: usize,
    pub initial_difficulty: u64,
    pub target_block_time: u64,
    pub finality_depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            k: 1,
            initial_difficulty: 1000,
            target_block_time: 600,
            finality_depth: 100,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}