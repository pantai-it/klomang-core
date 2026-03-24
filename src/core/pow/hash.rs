use crate::core::crypto::Hash;

/// Calculate hash of block header using Blake3
pub fn calculate_hash(header: &[u8]) -> Hash {
    Hash::new(header)
}

/// Check if hash meets the target difficulty
pub fn is_valid_pow(hash: &Hash, target: u64) -> bool {
    // Convert first 8 bytes of hash to u64 (little endian)
    let hash_bytes = hash.as_bytes();
    if hash_bytes.len() < 8 {
        return false;
    }
    let hash_val = u64::from_le_bytes(hash_bytes[0..8].try_into().unwrap_or([0u8; 8]));
    hash_val < target
}

/// Mine a block by finding a valid nonce
/// Returns the nonce if found, or None if not found within reasonable attempts
pub fn mine_block(header: &[u8], target: u64) -> Option<u64> {
    for nonce in 0..u64::MAX {
        let mut header_with_nonce = header.to_vec();
        header_with_nonce.extend_from_slice(&nonce.to_le_bytes());
        let hash = calculate_hash(&header_with_nonce);
        if is_valid_pow(&hash, target) {
            return Some(nonce);
        }
        // Prevent infinite loop in tests, but in practice this should be interruptible
        if nonce > 1_000_000 {
            return None;
        }
    }
    None
}