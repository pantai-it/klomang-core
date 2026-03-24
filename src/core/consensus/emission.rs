/// DAG-based emission system with hard cap
///
/// Calculates block rewards based on DAA score (blue score)
/// Ensures total supply never exceeds MAX_SUPPLY

const MAX_SUPPLY: u128 = 600_000_000;

/// Calculate block reward for a given DAA score
/// Reward decreases exponentially but never goes below 1
pub fn block_reward(daa_score: u64) -> u64 {
    if daa_score == 0 {
        return 100; // Initial reward for genesis
    }

    // Exponential decay: reward = 100 * 2^(-daa_score/100000)
    // But use integer arithmetic to avoid floating point
    let halvings = daa_score / 100_000;
    let mut reward = 100u64;

    // Apply halvings (divide by 2 for each halving)
    for _ in 0..halvings.min(50) { // Cap halvings to prevent underflow
        if reward <= 1 {
            break;
        }
        reward /= 2;
    }

    reward.max(1)
}

/// Calculate total coins emitted up to a given DAA score
/// This is an approximation since DAG structure is complex
pub fn total_emitted(daa_score: u64) -> u128 {
    if daa_score == 0 {
        return 0;
    }

    // Approximate total emitted as sum of geometric series
    // For simplicity, assume average reward and multiply by daa_score
    // In reality, this would need to track actual emission history

    let mut total = 0u128;
    let mut current_score = 1u64;

    while current_score <= daa_score {
        let reward = block_reward(current_score) as u128;
        if total + reward > MAX_SUPPLY {
            total = MAX_SUPPLY;
            break;
        }
        total += reward;
        current_score += 1;

        // Prevent infinite loop
        if current_score > daa_score + 1_000_000 {
            break;
        }
    }

    total.min(MAX_SUPPLY)
}

/// Calculate capped reward that won't exceed total supply
/// Returns the actual reward amount that can be issued
pub fn capped_reward(daa_score: u64) -> u64 {
    let current_total = total_emitted(daa_score);
    let base_reward = block_reward(daa_score) as u128;

    if current_total + base_reward > MAX_SUPPLY {
        // Calculate remaining supply
        let remaining = MAX_SUPPLY.saturating_sub(current_total);
        remaining as u64
    } else {
        base_reward as u64
    }
}

/// Get maximum supply constant
pub fn max_supply() -> u128 {
    MAX_SUPPLY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_reward() {
        assert_eq!(block_reward(0), 100);
    }

    #[test]
    fn test_reward_halving() {
        assert_eq!(block_reward(100_000), 50);
        assert_eq!(block_reward(200_000), 25);
        assert_eq!(block_reward(300_000), 12);
    }

    #[test]
    fn test_minimum_reward() {
        assert_eq!(block_reward(5_000_000), 1); // After many halvings
    }

    #[test]
    fn test_total_emitted_increases() {
        assert!(total_emitted(100) > total_emitted(50));
    }

    #[test]
    fn test_max_supply_cap() {
        assert!(total_emitted(1_000_000) <= MAX_SUPPLY);
    }

    #[test]
    fn test_capped_reward() {
        // For very high DAA scores, reward should be capped
        let high_score = 10_000_000;
        let reward = capped_reward(high_score);
        assert!(reward <= 600_000_000); // Should not exceed max supply in a single reward
    }

    #[test]
    fn test_max_supply_constant() {
        assert_eq!(max_supply(), 600_000_000);
    }
}