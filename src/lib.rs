// Klomang Core Engine
// Production-ready BlockDAG engine implementation

pub mod core;

// Re-export public API
pub use core::engine::Engine;
pub use core::crypto::Hash;
pub use core::dag::BlockNode;
pub use core::errors::CoreError;
pub use core::config::Config;
