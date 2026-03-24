use crate::core::crypto::Hash;
use crate::core::state::transaction::Transaction;
use crate::core::errors::CoreError;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Mempool {
    txs: HashMap<Hash, Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            txs: HashMap::new(),
        }
    }

    /// Submit a transaction to the mempool
    pub fn submit_tx(&mut self, tx: Transaction) -> Result<(), CoreError> {
        let tx_id = tx.id.clone();
        if self.txs.contains_key(&tx_id) {
            return Err(CoreError::TransactionError("Duplicate transaction".into()));
        }
        // TODO: validate tx (double spend, etc.)
        self.txs.insert(tx_id, tx);
        Ok(())
    }

    /// Remove confirmed transactions from mempool
    pub fn remove_confirmed(&mut self, tx_ids: &[Hash]) {
        for tx_id in tx_ids {
            self.txs.remove(tx_id);
        }
    }

    /// Select transactions for inclusion in a block
    pub fn select_txs_for_block(&self, _max_size: usize) -> Vec<Transaction> {
        // Simple selection: all txs
        // TODO: Implement size-based selection and priority ordering
        self.txs.values().cloned().collect()
    }

    /// Get a transaction by ID
    pub fn get_tx(&self, tx_id: &Hash) -> Option<&Transaction> {
        self.txs.get(tx_id)
    }

    /// Get the number of transactions in mempool
    pub fn tx_count(&self) -> usize {
        self.txs.len()
    }
}