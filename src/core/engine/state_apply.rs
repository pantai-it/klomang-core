use crate::core::dag::BlockNode;
use crate::core::state::BlockchainState;
use crate::core::errors::CoreError;

/// Apply a block's transactions to the blockchain state
///
/// Updates UTXO set based on:
/// - Remove spent outputs (from transaction inputs)
/// - Add new outputs (from transaction outputs)
pub fn apply_block_to_state(state: &mut BlockchainState, block: &BlockNode) -> Result<(), CoreError> {
    state.apply_block(block)
}