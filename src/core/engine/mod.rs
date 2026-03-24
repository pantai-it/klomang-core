// Internal modules
mod engine;
mod block_pipeline;
mod validation;
mod state_apply;

// Public crate-level modules
pub(crate) mod mempool;
pub(crate) mod ordering;
pub(crate) mod errors;

// Public API re-exports
pub use engine::Engine;
pub use errors::EngineError;