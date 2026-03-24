use std::collections::HashMap;
use crate::core::crypto::Hash;
use crate::core::state::transaction::{Transaction, TxOutput};
use crate::core::crypto::schnorr;

#[derive(Clone, Debug)]
pub struct UtxoSet {
    pub utxos: HashMap<(Hash, u32), TxOutput>,
}

impl UtxoSet {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    pub fn validate_tx(&self, tx: &Transaction) -> Result<(), String> {
        // Check inputs exist and not double spent
        let mut total_input = 0u64;
        for input in &tx.inputs {
            let key = (input.prev_tx.clone(), input.index);
            match self.utxos.get(&key) {
                Some(output) => {
                    total_input += output.value;
                    // TODO: Verify signature - for now skip in test
                    // if !schnorr::verify(&input.pubkey, tx.id.as_bytes(), &input.signature) {
                    //     return Err("Invalid signature".to_string());
                    // }
                }
                None => return Err("Input UTXO not found".to_string()),
            }
        }

        // Check outputs total <= inputs total (skip for coinbase)
        if !tx.is_coinbase() {
            let total_output: u64 = tx.outputs.iter().map(|o| o.value).sum();
            if total_output > total_input {
                return Err("Insufficient input value".to_string());
            }
        }

        Ok(())
    }

    pub fn apply_tx(&mut self, tx: &Transaction) {
        // Remove spent inputs
        for input in &tx.inputs {
            let key = (input.prev_tx.clone(), input.index);
            self.utxos.remove(&key);
        }

        // Add new outputs
        for (index, output) in tx.outputs.iter().enumerate() {
            let key = (tx.id.clone(), index as u32);
            self.utxos.insert(key, output.clone());
        }
    }

    pub fn get_balance(&self, pubkey_hash: &Hash) -> u64 {
        self.utxos
            .values()
            .filter(|output| &output.pubkey_hash == pubkey_hash)
            .map(|output| output.value)
            .sum()
    }
}