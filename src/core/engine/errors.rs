use crate::core::errors::CoreError;

#[derive(Debug, Clone, PartialEq)]
pub enum EngineError {
    Core(CoreError),
    Validation(String),
    Mempool(String),
    StateApply(String),
    Ordering(String),
}

impl From<CoreError> for EngineError {
    fn from(err: CoreError) -> Self {
        EngineError::Core(err)
    }
}