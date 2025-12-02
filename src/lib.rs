//! Xahau Binary Decoder - Native Node.js addon
//! 
//! Provides high-performance binary deserialization for Xahau ledger data.
//! Loads field definitions dynamically from definitions.json (JS binary codec format).
//!
//! NOTE: Existing Rust crates (xrpl_binary_codec, xrpl-rust) only have SERIALIZATION,
//! not deserialization. This is a complete from-scratch implementation.

#![deny(clippy::all)]

mod deserializer;
mod dynamic_definitions;

use napi_derive::napi;
use serde_json::Value;

use crate::deserializer::Deserializer;
use crate::dynamic_definitions as dyndefs;

/// Decode a transaction from hex-encoded binary
#[napi]
pub fn decode_transaction(hex_data: String) -> napi::Result<Value> {
    Deserializer::decode_transaction_hex(&hex_data)
        .map_err(|e| napi::Error::from_reason(format!("{}", e)))
}

/// Decode a ledger object from hex-encoded binary
#[napi]
pub fn decode_ledger_object(hex_data: String) -> napi::Result<Value> {
    Deserializer::decode_ledger_object_hex(&hex_data)
        .map_err(|e| napi::Error::from_reason(format!("{}", e)))
}

/// Decode multiple transactions in a batch (more efficient - single FFI call)
#[napi]
pub fn decode_transactions_batch(hex_array: Vec<String>) -> napi::Result<Vec<Value>> {
    let mut results = Vec::with_capacity(hex_array.len());
    
    for hex_data in hex_array {
        let decoded = Deserializer::decode_transaction_hex(&hex_data)
            .map_err(|e| napi::Error::from_reason(format!("{}", e)))?;
        results.push(decoded);
    }
    
    Ok(results)
}

/// Decode multiple ledger objects in a batch (more efficient - single FFI call)
#[napi]
pub fn decode_ledger_objects_batch(hex_array: Vec<String>) -> napi::Result<Vec<Value>> {
    let mut results = Vec::with_capacity(hex_array.len());
    
    for hex_data in hex_array {
        let decoded = Deserializer::decode_ledger_object_hex(&hex_data)
            .map_err(|e| napi::Error::from_reason(format!("{}", e)))?;
        results.push(decoded);
    }
    
    Ok(results)
}

/// Compute SHA-512 hash (first 256 bits) - used for transaction hashing
#[napi]
pub fn sha512_half(hex_data: String) -> napi::Result<String> {
    use sha2::{Sha512, Digest};
    
    let bytes = hex::decode(&hex_data)
        .map_err(|e| napi::Error::from_reason(format!("Invalid hex: {}", e)))?;
    
    let hash = Sha512::digest(&bytes);
    let half: [u8; 32] = hash[..32].try_into().unwrap();
    
    Ok(hex::encode_upper(half))
}

/// Health check - returns version info
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Load definitions from JSON string (e.g., fetched from server_definitions endpoint)
#[napi]
pub fn load_definitions(json: String) -> napi::Result<()> {
    dyndefs::init_definitions(&json)
        .map_err(|e| napi::Error::from_reason(e))
}

/// Load definitions from a file path
#[napi]
pub fn load_definitions_from_file(path: String) -> napi::Result<()> {
    dyndefs::init_definitions_from_file(&path)
        .map_err(|e| napi::Error::from_reason(e))
}

/// Load embedded default definitions (fallback)
#[napi]
pub fn load_default_definitions() -> napi::Result<()> {
    dyndefs::init_default_definitions()
        .map_err(|e| napi::Error::from_reason(e))
}

/// Get info about loaded definitions
#[napi]
pub fn get_definitions_info() -> napi::Result<serde_json::Value> {
    let defs = dyndefs::get_definitions()
        .ok_or_else(|| napi::Error::from_reason("Definitions not loaded"))?;
    
    Ok(serde_json::json!({
        "field_count": defs.field_by_name.len(),
        "type_count": defs.type_codes.len(),
        "transaction_type_count": defs.transaction_types.len(),
        "ledger_entry_type_count": defs.ledger_entry_types.len(),
        "transaction_result_count": defs.transaction_result_names.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple_payment() {
        // Example Payment transaction hex
        let hex = "1200002280000000240000000161D4838D7EA4C680000000000000000000000000005553440000000000A407AF5856CCF3C42619DAA925813FC955C72983684000000000000014732103B6FCD7FAC4F665FE92415DD6E8450AAD52636A097F6F2B4B7F9B9B9B9B9B9B9B74473045022100...";
        // This would need a real transaction hex to test
    }
}
