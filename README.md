# Xahau Decoder

Fast native binary decoder for Xahau/XRPL ledger data, built with Rust and napi-rs.

## Features

- 🦀 Native Rust implementation for maximum performance
- 📦 Node.js bindings via napi-rs
- 🔄 Batch decoding for efficient processing
- 📋 Full Xahau field definitions (Hooks, URITokens, etc.)

## Prerequisites

- **Node.js 16+** (required)
- **Rust toolchain** (only needed when installing from git or building from source)

## Installation

### From npm (recommended - no Rust required)

```bash
npm install xahau-binary-decoder
```

Pre-built binaries are included for all supported platforms, so no Rust installation is needed.

### From git (requires Rust)

If installing directly from GitHub, you'll need to build from source:

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install from git
npm install git+https://github.com/Silkjaer/xahau-binary-decoder.git
```

The build will run automatically during `npm install` via the `postinstall` script.

### Development setup

```bash
# Clone the repository
git clone https://github.com/Silkjaer/xahau-binary-decoder.git
cd xahau-binary-decoder

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies and build
npm install
npm run build
```

## Usage

```javascript
const decoder = require('xahau-decoder');

// FIRST: Load definitions (one of these options)

// Option 1: Embedded definitions (always works, might be stale)
decoder.loadDefaultDefinitions();

// Option 2: Load from local file
decoder.loadDefinitionsFromFile('./definitions.json');

// Option 3: Fetch latest from network (recommended for production AFTER sync-2.4.0, for now use the bundled)
const response = await fetch('https://xahau.network/server_definitions.json');
decoder.loadDefinitions(await response.text());

// Check what's loaded
console.log(decoder.getDefinitionsInfo());
// { field_count: 200+, transaction_type_count: 50+, ... }

// THEN: Decode transactions
const tx = decoder.decodeTransaction('1200002280000000...');
console.log(tx);

// Batch decode (more efficient for bulk processing)
const transactions = decoder.decodeTransactionsBatch([
  '1200002280000000...',
  '1200002280000001...',
]);

// Decode ledger objects
const obj = decoder.decodeLedgerObject('...');
```

## Performance

Expected performance improvements over JavaScript decoder:
- Single decode: 5-10x faster
- Batch decode: 10-20x faster (avoids per-call overhead)

## API

### `decodeTransaction(hexString: string): object`
Decode a single transaction from hex-encoded binary.

### `decodeLedgerObject(hexString: string): object`
Decode a single ledger object from hex-encoded binary.

### `decodeTransactionsBatch(hexStrings: string[]): object[]`
Decode multiple transactions in a single call (most efficient for bulk processing).

### `decodeLedgerObjectsBatch(hexStrings: string[]): object[]`
Decode multiple ledger objects in a single call.

### `sha512Half(hexString: string): string`
Compute SHA-512/256 hash (used for transaction hashing).

### `version(): string`
Get the module version.

## Field Definitions

Field definitions are based on [Xahau server_definitions.json](https://xahau.network/server_definitions.json).

Supported Xahau-specific types:
- Transaction types: SetHook, Invoke, ClaimReward, Import, URITokenMint, etc.
- Ledger entry types: HookDefinition, HookState, URIToken, etc.
- Fields: HookHash, HookNamespace, HookStateData, etc.

## Building for Production

```bash
# Build release version
npm run build

# The .node file will be created for your platform
```

## Cross-compilation

To build for multiple platforms:

```bash
# Build for Linux (from macOS)
rustup target add x86_64-unknown-linux-gnu
npm run build -- --target x86_64-unknown-linux-gnu
```

## License

MIT
