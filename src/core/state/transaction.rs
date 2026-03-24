use crate::core::crypto::Hash;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TxInput {
    pub prev_tx: Hash,
    pub index: u32,
    pub signature: Vec<u8>,
    pub pubkey: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub pubkey_hash: Hash,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    pub id: Hash,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
}

impl Transaction {
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> Self {
        let mut tx = Self {
            id: Hash::new(&[]), // placeholder
            inputs,
            outputs,
        };
        tx.id = tx.calculate_id();
        tx
    }

    pub fn calculate_id(&self) -> Hash {
        let mut data = Vec::new();
        for input in &self.inputs {
            data.extend_from_slice(input.prev_tx.as_bytes());
            data.extend_from_slice(&input.index.to_be_bytes());
            data.extend_from_slice(&input.pubkey);
        }
        for output in &self.outputs {
            data.extend_from_slice(&output.value.to_be_bytes());
            data.extend_from_slice(output.pubkey_hash.as_bytes());
        }
        Hash::new(&data)
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty()
    }
}