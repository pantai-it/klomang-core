use std::fmt;

/// Core engine errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Block with the given hash was not found in DAG
    BlockNotFound,
    /// Parent block does not exist in DAG
    InvalidParent,
    /// Block already exists in DAG
    DuplicateBlock,
    /// Consensus processing failed
    ConsensusError,
    /// Transaction validation failed
    TransactionError(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CoreError::BlockNotFound => write!(f, "Block not found"),
            CoreError::InvalidParent => write!(f, "Invalid parent"),
            CoreError::DuplicateBlock => write!(f, "Duplicate block"),
            CoreError::ConsensusError => write!(f, "Consensus error"),
            CoreError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
        }
    }
}

impl std::error::Error for CoreError {}