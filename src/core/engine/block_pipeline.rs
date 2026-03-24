use crate::core::dag::BlockNode;
use crate::core::errors::CoreError;
use super::engine::Engine;
use super::validation;
use super::state_apply;

/// Process a block through the consensus pipeline:
/// 1. Validate block
/// 2. Add to DAG
/// 3. Run GHOSTDAG consensus
/// 4. Update virtual block state
/// 5. Apply block transactions to state
/// 6. Update finality
/// 7. Prune old blocks
pub fn process_block(engine: &mut Engine, mut block: BlockNode) -> Result<(), CoreError> {
    let block_hash = block.id.clone();

    // 1. Calculate difficulty for block
    block.difficulty = engine.calculate_difficulty(block.timestamp);

    // 2. Validate block (including PoW)
    validation::validate_block(engine, &block)?;

    // 3. Check and mark genesis
    let is_genesis = block.parents.is_empty();
    if is_genesis {
        if engine.genesis_already_set() {
            return Err(CoreError::ConsensusError);
        }
        engine.set_genesis_hash(block_hash.clone());
    }

    // 4. Insert block to DAG
    engine.dag_mut().add_block(block.clone())?;

    // 5. Run GHOSTDAG consensus algorithm
    let ghostdag = engine.ghostdag().clone();
    let dag_mut = engine.dag_mut();
    ghostdag.process_block(dag_mut, &block_hash);

    // 6. Validate coinbase reward now that we have the correct blue_score
    if let Some(processed_block) = engine.dag().get_block(&block_hash) {
        validation::validate_coinbase_reward_final(processed_block)?;
    }

    // 7. Persist to storage
    if let Some(stored_block) = engine.dag().get_block(&block_hash).cloned() {
        engine.storage_mut().put_block(stored_block);
    }

    // 7. Update state using virtual block
    let virtual_block = engine.ghostdag().build_virtual_block(engine.dag());
    if let Some(vb_hash) = virtual_block.selected_parent.clone() {
        engine.state_mut().set_finalizing_block(vb_hash);
        engine.state_mut().update_virtual_score(virtual_block.blue_score);
    } else if let Some(vb_hash) = engine.dag().get_all_hashes().into_iter().next() {
        engine.state_mut().set_finalizing_block(vb_hash);
        engine.state_mut().update_virtual_score(virtual_block.blue_score);
    }

    // 8. Apply block transactions to state and remove confirmed transactions from mempool
    if let Some(block) = engine.dag().get_block(&block_hash).cloned() {
        state_apply::apply_block_to_state(engine.state_mut(), &block)?;

        let confirmed_tx_ids: Vec<_> = block.transactions.iter().map(|tx| tx.id.clone()).collect();
        engine.mempool_mut().remove_confirmed(&confirmed_tx_ids);
    }

    // 9. Update finality
    engine.update_finality();

    // 10. Prune old blocks
    engine.prune();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crypto::Hash;
    use std::collections::HashSet;

    #[test]
    fn test_genesis_block_processing() {
        let mut engine = Engine::new();

        // Create coinbase transaction for genesis
        let coinbase_tx = crate::core::state::transaction::Transaction {
            id: Hash::new(b"coinbase"),
            inputs: Vec::new(), // Coinbase has no inputs
            outputs: vec![crate::core::state::transaction::TxOutput {
                value: 100, // Initial reward for DAA score 0
                pubkey_hash: Hash::new(b"miner"),
            }],
        };

        let genesis = BlockNode {
            id: Hash::new(b"genesis"),
            parents: HashSet::new(),
            children: HashSet::new(),
            selected_parent: None,
            blue_set: HashSet::new(),
            red_set: HashSet::new(),
            blue_score: 0,
            timestamp: 1000,
            difficulty: 1000,
            nonce: 0,
            transactions: vec![coinbase_tx],
        };

        let result = process_block(&mut engine, genesis);
        assert!(result.is_ok());
        assert_eq!(engine.get_block_count(), 1);
        assert!(engine.get_genesis().is_some());
    }

    #[test]
    fn test_duplicate_genesis_rejected() {
        let mut engine = Engine::new();

        let coinbase_tx = crate::core::state::transaction::Transaction {
            id: Hash::new(b"coinbase1"),
            inputs: Vec::new(),
            outputs: vec![crate::core::state::transaction::TxOutput {
                value: 100,
                pubkey_hash: Hash::new(b"miner1"),
            }],
        };

        let genesis = BlockNode {
            id: Hash::new(b"genesis"),
            parents: HashSet::new(),
            children: HashSet::new(),
            selected_parent: None,
            blue_set: HashSet::new(),
            red_set: HashSet::new(),
            blue_score: 0,
            timestamp: 1000,
            difficulty: 1000,
            nonce: 0,
            transactions: vec![coinbase_tx],
        };

        let _ = process_block(&mut engine, genesis.clone());

        let another_coinbase_tx = crate::core::state::transaction::Transaction {
            id: Hash::new(b"coinbase2"),
            inputs: Vec::new(),
            outputs: vec![crate::core::state::transaction::TxOutput {
                value: 100,
                pubkey_hash: Hash::new(b"miner2"),
            }],
        };

        let another_genesis = BlockNode {
            id: Hash::new(b"another"),
            parents: HashSet::new(),
            children: HashSet::new(),
            selected_parent: None,
            blue_set: HashSet::new(),
            red_set: HashSet::new(),
            blue_score: 0,
            timestamp: 2000,
            difficulty: 1000,
            nonce: 0,
            transactions: vec![another_coinbase_tx],
        };

        let result = process_block(&mut engine, another_genesis);
        assert!(matches!(result, Err(CoreError::ConsensusError)));
    }
}
