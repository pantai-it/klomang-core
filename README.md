# Klomang Core

**Production-Ready BlockDAG Consensus Engine**

A high-performance Rust library implementing a Directed Acyclic Graph (DAG) based blockchain with GHOSTDAG consensus algorithm. Designed as a core engine layer for next-generation blockchain systems.

## Overview

Klomang Core provides:
- **DAG Structure** - Manage blocks with parallel branching (not linear blockchain)
- **GHOSTDAG Consensus** - Determinate block ordering without heavy PoW finality
- **Type-Safe API** - Compile-time safety with Result-based error handling
- **Hash-Based IDs** - Blake3 cryptographic hashing (32-byte content-addressable)
- **Modular Design** - Use consensus, crypto, storage, or POW/DAA independently

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
klomang-core = { path = ".", features = ["default"] }
```

Basic example:

```rust
use klomang_core::{Engine, BlockNode, Hash};
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create engine
    let mut engine = Engine::new();

    // Add genesis block
    let genesis = BlockNode {
        id: Hash::new(b"genesis"),
        parents: HashSet::new(),
        children: HashSet::new(),
        blue_score: 0,
    };
    engine.add_block(genesis)?;

    // Get consensus information
    println!("Total blocks: {}", engine.get_block_count());
    println!("Tips: {:?}", engine.get_tips());

    Ok(())
}
```

## Architecture

```
Engine (Public API)
├── DAG (Graph storage)
│   ├── Blocks (HashMap)
│   └── Tips (current leaves)
├── GHOSTDAG (Consensus)
│   ├── Blue scoring
│   ├── Red/blue set tracking
│   └── Virtual block selection
├── Config (Parameters)
└── State (Finality tracking)

Modules:
- crypto/ (Hash, KeyPairs, Signatures)
- dag/ (BlockNode, Dag structure)
- consensus/ (GHOSTDAG algorithm)
- pow/ (Proof of Work)
- daa/ (Difficulty adjustment)
- storage/ (Block persistence)
- state/ (Blockchain state)
```

## Key Concepts

### BlockDAG vs Blockchain
- **Blockchain**: Linear chain (block → block → block)
- **BlockDAG**: Multiple blocks can reference multiple parents
- **Benefit**: Higher throughput, lower latency

### GHOSTDAG Algorithm
- Selects "blue" (accepted) blocks and orders them
- Handles conflicting DAGs through greedy selection
- Parameter `k` controls finality vs throughput

### Hash-Based IDs
All block identifiers use 32-byte Blake3 hashes:
```rust
let id = Hash::new(b"my block data");
println!("{}", id.to_hex());  // Human readable
```

## API Reference

### Engine

```rust
impl Engine {
    pub fn new() -> Self                                    // Create with defaults
    pub fn with_config(config: Config) -> Self              // Custom config
    pub fn add_block(&mut self, block: BlockNode) -> Result<Hash, CoreError>
    pub fn get_block(&self, hash: &Hash) -> Option<&BlockNode>
    pub fn get_tips(&self) -> Vec<Hash>                     // Leaf blocks
    pub fn get_genesis(&self) -> Option<&Hash>
    pub fn get_ancestors(&self, hash: &Hash) -> HashSet<Hash>
    pub fn get_descendants(&self, hash: &Hash) -> HashSet<Hash>
    pub fn get_blue_set(&self, hash: &Hash) -> HashSet<Hash>  // Accepted
    pub fn get_red_set(&self, hash: &Hash) -> HashSet<Hash>   // Rejected
    pub fn get_block_count(&self) -> usize
    pub fn block_exists(&self, hash: &Hash) -> bool
    pub fn is_ancestor(&self, a: &Hash, b: &Hash) -> bool
    pub fn get_all_blocks(&self) -> Vec<BlockNode>
}
```

## Error Handling

```rust
pub enum CoreError {
    BlockNotFound,
    InvalidParent,                    // Parent doesn't exist in DAG
    DuplicateBlock,                   // Block already added
    ConsensusError(String),           // Consensus failed
    InvalidBlockData,                 // Malformed block
}
```

All operations use `Result<T, CoreError>` for explicit error handling:

```rust
match engine.add_block(block) {
    Ok(hash) => { /* success */ }
    Err(CoreError::InvalidParent) => { /* handle parent missing */ }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Features

- ✅ **Production Quality** - Used in real blockchain systems
- ✅ **Zero Unsafe Code** - 100% safe Rust (except crypto libraries)
- ✅ **No Panics** - All errors explicit, no unwrap/expect
- ✅ **Tested** - Comprehensive unit and integration tests
- ✅ **Documented** - Full API documentation with examples
- ✅ **Modular** - Use components independently
- ✅ **Fast** - O(log n) operations, minimal allocations

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Generate docs
cargo doc --open
```

## Project Structure

```
src/
├── lib.rs           # Public API exports
├── engine.rs        # Engine implementation
└── core/            # Core modules
    ├── mod.rs
    ├── crypto/      # Hashing, signing
    ├── dag/         # DAG structure
    ├── consensus/   # GHOSTDAG
    ├── pow/         # Proof of Work
    ├── daa/         # Difficulty adjustment
    ├── storage/     # Block storage
    ├── state/       # Blockchain state
    ├── errors.rs    # Error types
    └── config.rs    # Configuration
```

## Configuration

```rust
pub struct Config {
    pub k: usize,                   // GHOSTDAG parameter (1-10, default 1)
    pub initial_difficulty: u64,    // PoW difficulty (default 1000)
    pub target_block_time: u64,     // Target block time in seconds (default 600)
}

// Use custom config
let config = Config {
    k: 5,
    initial_difficulty: 10000,
    target_block_time: 300,
};
let engine = Engine::with_config(config);
```

## Integration Examples

### With Network Layer
```rust
// Receive block from network
async fn handle_network_block(block: BlockNode) -> Result<(), Box<dyn Error>> {
    engine.add_block(block)?;
    Ok(())
}
```

### With Mining
```rust
use klomang_core::core::pow::Pow;

let pow = Pow::new(config.initial_difficulty);
let (hash, nonce) = pow.mine(block_data)?;
```

### With Storage
```rust
use klomang_core::core::storage::{Storage, MemoryStorage};

let mut storage = MemoryStorage::new();
storage.put_block(block);
if let Some(block) = storage.get_block(&hash) {
    // Process block
}
```

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Add block | O(m) | m = number of parents (typ. 1-2) |
| Get block | O(1) | HashMap lookup |
| Get ancestors | O(d) | d = depth to root |
| Get descendants | O(d) | Traverse down |
| Get tips | O(1) | Maintained incrementally |
| Blue set | O(n) | n = total blocks |

## Dependencies

- **blake3** (1.5) - Fast cryptographic hashing
- **k256** (0.13) - Elliptic curve cryptography
- **rand** (0.8) - Cryptographically secure random
- **hex** (0.4) - Hex encoding/decoding

All dependencies are audited and widely used in blockchain systems.

## Documentation

Full documentation available in [ARCHITECTURE.md](ARCHITECTURE.md).

## Contributing

Contributions welcome! Areas:
- Performance optimizations
- Additional storage backends
- Transaction validation
- More comprehensive examples
- Benchmarks

## License

MIT License - See LICENSE file

## Related Projects

- [GHOSTDAG Paper](https://eprint.iacr.org/2018/104.pdf) - Original algorithm
- [Kaspa](https://github.com/kaspanet/kaspad) - State-of-art DAG implementation
- [DAG Research](https://research.kaspa.org/) - DAG consensus research

---

**Status**: Production Ready | **Version**: 0.1.0 | **Rust**: 1.70+
