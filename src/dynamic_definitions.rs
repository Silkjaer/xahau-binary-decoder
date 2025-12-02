//! Dynamic field definitions loaded from JSON
//! 
//! Loads field definitions from definitions.json (JS binary codec format) or
//! server_definitions.json at runtime, allowing the decoder to stay in sync with network changes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Global definitions instance - loaded once at startup
static DEFINITIONS: OnceLock<Definitions> = OnceLock::new();

/// Field definition from server_definitions.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldDef {
    pub nth: i32,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(rename = "isVLEncoded")]
    pub is_vl_encoded: bool,
    #[serde(rename = "isSerialized")]
    pub is_serialized: bool,
    #[serde(rename = "isSigningField")]
    pub is_signing_field: bool,
}

/// Type information
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future use
pub struct TypeInfo {
    pub name: String,
    pub code: u8,
}

/// Complete definitions parsed from JSON
#[derive(Debug)]
pub struct Definitions {
    /// Map from (type_code, field_code) -> field name
    pub field_by_code: HashMap<(u8, u8), String>,
    
    /// Map from field name -> (type_code, field_code, FieldDef)
    pub field_by_name: HashMap<String, (u8, u8, FieldDef)>,
    
    /// Map from type name -> type code
    pub type_codes: HashMap<String, u8>,
    
    /// Map from type code -> type name
    pub type_names: HashMap<u8, String>,
    
    /// Transaction type codes: name -> code
    pub transaction_types: HashMap<String, u16>,
    
    /// Transaction type names: code -> name
    pub transaction_type_names: HashMap<u16, String>,
    
    /// Ledger entry type codes: name -> code
    pub ledger_entry_types: HashMap<String, u16>,
    
    /// Ledger entry type names: code -> name
    pub ledger_entry_type_names: HashMap<u16, String>,
    
    /// Transaction result codes: code -> name (e.g., 0 -> "tesSUCCESS")
    pub transaction_result_names: HashMap<i16, String>,
}

impl Definitions {
    /// Parse definitions from JSON format
    /// Supports both formats:
    /// 1. Flat format (JS binary codec): direct TYPES, FIELDS, etc. at root
    /// 2. Server definitions format: wrapped in "result" object
    pub fn from_json(json: &str) -> Result<Self, String> {
        let parsed: Value = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {}", e))?;
        
        // Try flat format first (JS binary codec), fall back to server definitions format
        let root = if parsed.get("result").is_some() {
            parsed.get("result").ok_or("Missing 'result' field")?
        } else {
            &parsed
        };
        
        let mut defs = Definitions {
            field_by_code: HashMap::new(),
            field_by_name: HashMap::new(),
            type_codes: HashMap::new(),
            type_names: HashMap::new(),
            transaction_types: HashMap::new(),
            transaction_type_names: HashMap::new(),
            ledger_entry_types: HashMap::new(),
            ledger_entry_type_names: HashMap::new(),
            transaction_result_names: HashMap::new(),
        };
        
        // Parse TYPES
        // Note: Only type codes 0-255 are used in binary format
        // High codes like Transaction (10001) are wrappers, skip them
        if let Some(types) = root.get("TYPES").and_then(|v| v.as_object()) {
            for (name, code) in types {
                if let Some(code) = code.as_i64() {
                    // Skip type codes > 255 (they can't appear in binary serialization)
                    if code < 0 || code > 255 {
                        continue;
                    }
                    let code = code as u8;
                    defs.type_codes.insert(name.clone(), code);
                    defs.type_names.insert(code, name.clone());
                }
            }
        }
        
        // Parse FIELDS
        if let Some(fields) = root.get("FIELDS").and_then(|v| v.as_array()) {
            for field in fields {
                if let Some(arr) = field.as_array() {
                    if arr.len() >= 2 {
                        let name = arr[0].as_str().unwrap_or("").to_string();
                        if let Ok(field_def) = serde_json::from_value::<FieldDef>(arr[1].clone()) {
                            // Get type code from type name
                            if let Some(&type_code) = defs.type_codes.get(&field_def.field_type) {
                                let field_code = field_def.nth as u8;
                                defs.field_by_code.insert((type_code, field_code), name.clone());
                                defs.field_by_name.insert(name, (type_code, field_code, field_def));
                            }
                        }
                    }
                }
            }
        }
        
        // Parse TRANSACTION_TYPES
        if let Some(tx_types) = root.get("TRANSACTION_TYPES").and_then(|v| v.as_object()) {
            for (name, code) in tx_types {
                if let Some(code) = code.as_i64() {
                    let code = code as u16;
                    defs.transaction_types.insert(name.clone(), code);
                    defs.transaction_type_names.insert(code, name.clone());
                }
            }
        }
        
        // Parse LEDGER_ENTRY_TYPES
        if let Some(le_types) = root.get("LEDGER_ENTRY_TYPES").and_then(|v| v.as_object()) {
            for (name, code) in le_types {
                if let Some(code) = code.as_i64() {
                    let code = code as u16;
                    defs.ledger_entry_types.insert(name.clone(), code);
                    defs.ledger_entry_type_names.insert(code, name.clone());
                }
            }
        }
        
        // Parse TRANSACTION_RESULTS (code -> name for TransactionResult field)
        if let Some(tx_results) = root.get("TRANSACTION_RESULTS").and_then(|v| v.as_object()) {
            for (name, code) in tx_results {
                if let Some(code) = code.as_i64() {
                    // Transaction results can be negative (tec, tef, tem, ter codes)
                    defs.transaction_result_names.insert(code as i16, name.clone());
                }
            }
        }
        
        Ok(defs)
    }
    
    /// Load from a file path
    pub fn from_file(path: &str) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        Self::from_json(&json)
    }
    
    /// Get field name from type and field codes
    pub fn get_field_name(&self, type_code: u8, field_code: u8) -> Option<&str> {
        self.field_by_code.get(&(type_code, field_code)).map(|s| s.as_str())
    }
    
    /// Get transaction type name from code
    pub fn get_transaction_type_name(&self, code: u16) -> Option<&str> {
        self.transaction_type_names.get(&code).map(|s| s.as_str())
    }
    
    /// Get ledger entry type name from code
    pub fn get_ledger_entry_type_name(&self, code: u16) -> Option<&str> {
        self.ledger_entry_type_names.get(&code).map(|s| s.as_str())
    }
    
    /// Get transaction result name from code (e.g., 0 -> "tesSUCCESS")
    pub fn get_transaction_result_name(&self, code: i16) -> Option<&str> {
        self.transaction_result_names.get(&code).map(|s| s.as_str())
    }
    
    /// Check if a type is VL-encoded (reserved for future use)
    #[allow(dead_code)]
    pub fn is_vl_encoded(&self, type_code: u8, _field_code: u8) -> bool {
        // VL-encoded types: Blob (7), AccountID (8), Vector256 (19)
        matches!(type_code, 7 | 8 | 19)
    }
}

/// Initialize definitions from JSON string
pub fn init_definitions(json: &str) -> Result<(), String> {
    // Idempotent - silently succeed if already initialized
    if DEFINITIONS.get().is_some() {
        return Ok(());
    }
    let defs = Definitions::from_json(json)?;
    // Use get_or_init pattern to avoid race condition
    DEFINITIONS.get_or_init(|| defs);
    Ok(())
}

/// Initialize definitions from file path
pub fn init_definitions_from_file(path: &str) -> Result<(), String> {
    // Idempotent - silently succeed if already initialized
    if DEFINITIONS.get().is_some() {
        return Ok(());
    }
    let defs = Definitions::from_file(path)?;
    // Use get_or_init pattern to avoid race condition
    DEFINITIONS.get_or_init(|| defs);
    Ok(())
}

/// Get the global definitions instance
pub fn get_definitions() -> Option<&'static Definitions> {
    DEFINITIONS.get()
}

/// Default definitions JSON (embedded fallback)
/// This is used if no external definitions are provided
pub const DEFAULT_DEFINITIONS: &str = include_str!("../definitions.json");

/// Initialize with default embedded definitions
pub fn init_default_definitions() -> Result<(), String> {
    init_definitions(DEFAULT_DEFINITIONS)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_definitions_server_format() {
        // Test server definitions format (with "result" wrapper)
        let json = r#"{
            "result": {
                "TYPES": {
                    "UInt16": 1,
                    "UInt32": 2,
                    "Hash256": 5
                },
                "FIELDS": [
                    ["TransactionType", {"nth": 2, "type": "UInt16", "isVLEncoded": false, "isSerialized": true, "isSigningField": true}],
                    ["Flags", {"nth": 2, "type": "UInt32", "isVLEncoded": false, "isSerialized": true, "isSigningField": true}]
                ],
                "TRANSACTION_TYPES": {
                    "Payment": 0,
                    "SetHook": 22
                },
                "LEDGER_ENTRY_TYPES": {
                    "AccountRoot": 97,
                    "HookDefinition": 68
                }
            }
        }"#;
        
        let defs = Definitions::from_json(json).unwrap();
        
        assert_eq!(defs.get_field_name(1, 2), Some("TransactionType"));
        assert_eq!(defs.get_field_name(2, 2), Some("Flags"));
        assert_eq!(defs.get_transaction_type_name(0), Some("Payment"));
        assert_eq!(defs.get_transaction_type_name(22), Some("SetHook"));
        assert_eq!(defs.get_ledger_entry_type_name(97), Some("AccountRoot"));
        assert_eq!(defs.get_ledger_entry_type_name(68), Some("HookDefinition"));
    }
    
    #[test]
    fn test_parse_definitions_flat_format() {
        // Test flat format (JS binary codec format - no "result" wrapper)
        let json = r#"{
            "TYPES": {
                "UInt16": 1,
                "UInt32": 2,
                "Hash256": 5
            },
            "FIELDS": [
                ["TransactionType", {"nth": 2, "type": "UInt16", "isVLEncoded": false, "isSerialized": true, "isSigningField": true}],
                ["Flags", {"nth": 2, "type": "UInt32", "isVLEncoded": false, "isSerialized": true, "isSigningField": true}]
            ],
            "TRANSACTION_TYPES": {
                "Payment": 0,
                "SetHook": 22
            },
            "LEDGER_ENTRY_TYPES": {
                "AccountRoot": 97,
                "HookDefinition": 68
            },
            "TRANSACTION_RESULTS": {
                "tesSUCCESS": 0,
                "tecPATH_DRY": 18
            }
        }"#;
        
        let defs = Definitions::from_json(json).unwrap();
        
        assert_eq!(defs.get_field_name(1, 2), Some("TransactionType"));
        assert_eq!(defs.get_field_name(2, 2), Some("Flags"));
        assert_eq!(defs.get_transaction_type_name(0), Some("Payment"));
        assert_eq!(defs.get_transaction_type_name(22), Some("SetHook"));
        assert_eq!(defs.get_ledger_entry_type_name(97), Some("AccountRoot"));
        assert_eq!(defs.get_ledger_entry_type_name(68), Some("HookDefinition"));
        assert_eq!(defs.get_transaction_result_name(0), Some("tesSUCCESS"));
        assert_eq!(defs.get_transaction_result_name(18), Some("tecPATH_DRY"));
    }
}

