# Klomang Core - BlockDAG Engine

**Production-ready core blockchain engine implementing GHOSTDAG consensus**

A Rust library providing a robust, type-safe implementation of a BlockDAG (Directed Acyclic Graph) blockchain engine with GHOSTDAG consensus algorithm. Designed for use as a core layer in blockchain systems requiring parallel block production and DAG-based ordering.

## Features

- ✅ **Hash-based Block IDs** - Blake3 cryptographic hashing (no string keys)
- ✅ **DAG Data Structure** - Efficient graph storage with parent-child relationships
- ✅ **GHOSTDAG Consensus** - Handles parallel blocks and fork resolution
- ✅ **Type Safety** - All operations return `Result<T, CoreError>` 
- ✅ **Production Ready** - No placeholders, comprehensive error handling
- ✅ **Zero Unsafe Code** - Pure safe Rust
- ✅ **Modular Design** - Use only what you need (crypto, consensus, storage, etc.)

## Quick Start

```rust
use klomang_core::{Engine, BlockNode, Hash};
use std::collections::HashSet;

fn main() {
    let mut engine = Engine::new();

    // Create genesis block
    let genesis = BlockNode {
        id: Hash::new(b"genesis"),
        parents: HashSet::new(),
        children: HashSet::new(),
        blue_score: 0,
    };

    // Add to DAG
    engine.add_block(genesis)?;

    // Create child block
    let block1 = BlockNode {
        id: Hash::new(b"block1"),
        parents: engine.get_tips().iter().cloned().collect(),
        children: HashSet::new(),
        blue_score: 0,
    };

    engine.add_block(block1)?;

    // Query consensus state
    let blue_set = engine.get_blue_set(&engine.get_genesis().unwrap());
    println!("Accepted blocks: {:?}", blue_set.len());
}
```

## Architecture

### Core Engine (`Engine`)

Main entry point for blockchain operations:

```rust
pub struct Engine {
    dag: Dag,              // Block DAG storage
    ghostdag: GhostDag,    // Consensus processor
    config: Config,        // Network configuration
    genesis_hash: Option<Hash>,
}

impl Engine {
    pub fn new() -> Self { ... }
    pub fn add_block(&mut self, block: BlockNode) -> Result<Hash, CoreError> { ... }
    pub fn get_block(&self, hash: &Hash) -> Option<&BlockNode> { ... }
    pub fn get_tips(&self) -> Vec<Hash> { ... }
    pub fn get_genesis(&self) -> Option<&Hash> { ... }
    pub fn get_ancestors(&self, hash: &Hash) -> HashSet<Hash> { ... }
    pub fn get_descendants(&self, hash: &Hash) -> HashSet<Hash> { ... }
    pub fn get_blue_set(&self, hash: &Hash) -> HashSet<Hash> { ... }
    pub fn get_red_set(&self, hash: &Hash) -> HashSet<Hash> { ... }
}
```

### DAG (`Dag`)

Manages block graph:

```rust
pub struct Dag {
    blocks: HashMap<Hash, BlockNode>,
    tips: HashSet<Hash>,  // Blocks with no children
}

impl Dag {
    pub fn add_block(&mut self, block: BlockNode) { ... }
    pub fn get_block(&self, id: &Hash) -> Option<&BlockNode> { ... }
    pub fn get_tips(&self) -> &HashSet<Hash> { ... }
    pub fn is_ancestor(&self, a: &Hash, b: &Hash) -> bool { ... }
    pub fn get_ancestors(&self, id: &Hash) -> HashSet<Hash> { ... }
    pub fn get_descendants(&self, id: &Hash) -> HashSet<Hash> { ... }
    pub fn block_exists(&self, id: &Hash) -> bool { ... }
    pub fn get_block_count(&self) -> usize { ... }
}
```

### GHOSTDAG Consensus (`GhostDag`)

Implements block ordering and finality:

```rust
pub struct GhostDag {
    k: usize,  // Consensus parameter
}

impl GhostDag {
    pub fn new(k: usize) -> Self { ... }
    pub fn process_block(&self, dag: &mut Dag, block_hash: Hash) { ... }
    pub fn get_blue_set(&self, dag: &Dag, block: &Hash) -> HashSet<Hash> { ... }
    pub fn get_red_set(&self, dag: &Dag, block: &Hash) -> HashSet<Hash> { ... }
    pub fn get_virtual_block(&self, dag: &Dag) -> Option<Hash> { ... }
    pub fn get_ordering(&self, dag: &Dag) -> Vec<Hash> { ... }
}
```

### Hash (`Hash`)

Cryptographic block identifier:

```rust
pub struct Hash([u8; 32]);  // Blake3 hash

impl Hash {
    pub fn new(data: &[u8]) -> Self { ... }
    pub fn to_hex(&self) -> String { ... }
    pub fn as_bytes(&self) -> &[u8; 32] { ... }
}
```

Uses blake3 for fast, cryptographically secure hashing.

### Block (`BlockNode`)

```rust
pub struct BlockNode {
    pub id: Hash,                    // Unique block identifier
    pub parents: HashSet<Hash>,      // References to parent blocks
    pub children: HashSet<Hash>,     // References to child blocks
    pub blue_score: u64,             // Consensus ordering score
}
```

### Types / Modules

| Module | Purpose |
|--------|---------|
| `core::crypto` | Hash generation and cryptographic operations |
| `core::dag` | Directed Acyclic Graph implementation |
| `core::consensus` | GHOSTDAG consensus algorithm |
| `core::pow` | Proof of Work (mining support) |
| `core::daa` | Difficulty Adjustment Algorithm |
| `core::state` | Blockchain state tracking |
| `core::storage` | Block persistence (Memory/DB) |
| `core::config` | Configuration management |
| `core::errors` | Error types |

## Error Handling

All operations return proper `Result` types:

```rust
pub enum CoreError {
    BlockNotFound,
    InvalidParent,           // Parent doesn't exist
    DuplicateBlock,          // Block already in DAG
    ConsensusError(String),  // Consensus failure
    InvalidBlockData,        // Malformed block
}

// Usage
match engine.add_block(block) {
    Ok(hash) => println!("Block accepted: {}", hash.to_hex()),
    Err(CoreError::InvalidParent) => eprintln!("Parent not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Configuration

```rust
pub struct Config {
    pub k: usize,                   // GHOSTDAG parameter (1-10)
    pub initial_difficulty: u64,    // PoW difficulty
    pub target_block_time: u64,     // Target seconds between blocks
}

let config = Config {
    k: 1,
    initial_difficulty: 1000,
    target_block_time: 600,
};
let engine = Engine::with_config(config);
```

## Integration Points

### Network Layer
- `Engine::add_block()` - Process blocks from network
- `Engine::get_tips()` - Blocks to reference in new blocks

### Mining Layer
- `core::pow::Pow` - Mining/validation
- `core::daa::Daa` - Difficulty adjustment

### Storage Layer
- `core::storage::Storage` trait - Pluggable backend
- `core::storage::MemoryStorage` - In-memory implementation

### State Machine
- `core::state::BlockchainState` - Track finality
- `Engine::get_blue_set()` - Accepted transactions

## Building & Testing

```bash
# Build library
cargo build --release

# Run tests
cargo test

# Run binary
cargo run
```

## Performance

- **Block addition**: O(n) where n = number of parents (typically 1-2)
- **Ancestor lookup**: O(m) where m = DAG depth
- **Blue set calculation**: O(n) where n = total blocks
- **Memory**: ~1KB per block (plus references)

## Dependencies

- **blake3** - Cryptographic hashing
- **k256** - Elliptic curve signatures
- **rand** - Random number generation
- **hex** - Hex encoding/decoding

## Design Principles

1. **Type Safety First** - Use types to prevent bugs
2. **No Panics** - All errors are explicit `Result` types
3. **Zero Cost Abstractions** - No unnecessary allocations
4. **Modularity** - Use components independently
5. **Testing** - Comprehensive unit tests
6. **Documentation** - Everything is documented

## Future Extensions

- [ ] Persistent storage backend (RocksDB)
- [ ] Transaction inclusion in blocks
- [ ] UTxO state tracking
- [ ] Finality pruning
- [ ] Network serialization
- [ ] Schnorr signature validation

## License

MIT

## Contributing

PRs welcome! Focus on:
- Tests for new features
- Documentation
- Performance improvements
- Error handling
