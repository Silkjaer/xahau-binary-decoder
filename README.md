# Xahau Binary Decoder

Fast native binary decoder for Xahau/XRPL ledger data, built with Rust and napi-rs.

## Features

- 🦀 Native Rust implementation (5-35x faster than JS)
- 📦 Node.js bindings via napi-rs
- 🔄 Batch decoding for efficient bulk processing
- 📋 Full Xahau field definitions (Hooks, URITokens, etc.)

## Installation

```bash
# Install Rust (required)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install from git
npm install git+https://github.com/Silkjaer/xahau-binary-decoder.git
```

The build will run automatically during `npm install` via the `postinstall` script.

For development:

```bash
git clone https://github.com/Silkjaer/xahau-binary-decoder.git
cd xahau-binary-decoder
npm install
npm run build
```

## Usage

```javascript
const decoder = require('xahau-binary-decoder');

// Load definitions (one of these options):

// Option 1: Bundled definitions (from xahau-binary-codec)
decoder.loadDefaultDefinitions();

// Option 2: Load from file
decoder.loadDefinitionsFromFile('./definitions.json');

// Option 3: Fetch from URL
const response = await fetch('https://xahau.network/server_definitions.json');
decoder.loadDefinitions(await response.text());

// Decode transactions
const tx = decoder.decodeTransaction('1200002280000000...');

// Batch decode (more efficient)
const transactions = decoder.decodeTransactionsBatch([
  '1200002280000000...',
  '1200002280000001...',
]);

// Decode ledger objects
const obj = decoder.decodeLedgerObject('...');
```

## API

- `decodeTransaction(hexString: string): object` - Decode a single transaction
- `decodeLedgerObject(hexString: string): object` - Decode a single ledger object
- `decodeTransactionsBatch(hexStrings: string[]): object[]` - Batch decode transactions
- `decodeLedgerObjectsBatch(hexStrings: string[]): object[]` - Batch decode ledger objects
- `sha512Half(hexString: string): string` - Compute SHA-512/256 hash
- `loadDefaultDefinitions()` - Load bundled definitions (from xahau-binary-codec)
- `loadDefinitions(json: string)` - Load definitions from JSON string
- `loadDefinitionsFromFile(path: string)` - Load definitions from file
- `getDefinitionsInfo()` - Get info about loaded definitions
- `version(): string` - Get module version

## Field Definitions

Bundled definitions are from the JavaScript [xahau-binary-codec](https://github.com/xahau/xahau-binary-codec) package. Supports all Xahau-specific types including Hooks, URITokens, and related transaction/ledger entry types.

## License

MIT
