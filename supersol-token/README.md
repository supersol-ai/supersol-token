# SuperSol (SSOL) Token Program

SuperSol (SSOL) Token program for SuperSol Layer 2 blockchain.

## Overview

`supersol-token` is a reimplementation of the SPL Token program, one of the most popular programs on Solana. The purpose is to have a Layer 2 token `SuperSol (SSOL)` on SuperSol blockchain, while being fully compatible with the original implementation &mdash; i.e., support the exact same instruction and account layouts as SPL Token, byte for byte.

## Features

- `no_std` crate
- Same instruction and account layout as SPL Token
- Minimal CU usage
- Enhanced flash loan functionality
- Concentrated liquidity support
- Price oracle integration
- Emergency pause mechanism
- Liquidity mining rewards

## Project Structure

```
supersol-token/
├── src/
│   ├── shared_liquidity/     # Shared liquidity implementation
│   │   ├── flash_loan.rs     # Flash loan functionality
│   │   ├── concentrated.rs   # Concentrated liquidity
│   │   ├── oracle.rs         # Price oracle
│   │   ├── pool.rs          # Liquidity pool
│   │   └── ...
│   ├── lib.rs               # Library entry point
│   ├── processor.rs         # Instruction processor
│   └── ...
├── tests/
│   ├── integration/         # Integration tests
│   └── unit/               # Unit tests
└── ...
```

## Development

### Prerequisites

- Rust 1.70.0 or later
- Solana CLI tools
- Anchor Framework

### Building

```bash
cargo build-bpf
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration
```

### Documentation

```bash
# Generate documentation
cargo doc --no-deps --document-private-items
```

## Architecture

### Core Components

1. **Shared Liquidity Layer**

   - Flash loan implementation
   - Concentrated liquidity
   - Price oracle integration
   - Emergency pause mechanism

2. **Token Management**

   - Mint creation and management
   - Account management
   - Transfer operations

3. **Security Features**
   - Emergency pause
   - Fee management
   - Access control

## License

The code is licensed under the [Apache License Version 2.0](LICENSE)
