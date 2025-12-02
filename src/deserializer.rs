//! Complete Binary Deserializer for Xahau/XRPL
//! 
//! Uses dynamic definitions loaded from server_definitions.json
//! Handles all standard XRPL types plus Xahau extensions (Hooks, URITokens, etc.)
//! 
//! Reference: https://xrpl.org/serialization.html

use crate::dynamic_definitions::{get_definitions, Definitions};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
// std::io not needed - using custom BinaryReader

/// Deserialization error types
#[derive(Debug)]
#[allow(dead_code)] // Some variants reserved for future error handling
pub enum Error {
    UnexpectedEof,
    InvalidFieldHeader,
    InvalidTypeCode(u8),
    InvalidAmount,
    InvalidAccountId,
    Utf8Error,
    DefinitionsNotLoaded,
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedEof => write!(f, "Unexpected end of data"),
            Error::InvalidFieldHeader => write!(f, "Invalid field header"),
            Error::InvalidTypeCode(c) => write!(f, "Invalid type code: {}", c),
            Error::InvalidAmount => write!(f, "Invalid amount format"),
            Error::InvalidAccountId => write!(f, "Invalid account ID"),
            Error::Utf8Error => write!(f, "Invalid UTF-8"),
            Error::DefinitionsNotLoaded => write!(f, "Definitions not loaded - call loadDefinitions first"),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// XRPL Base58 alphabet
const BASE58_ALPHABET: &[u8] = b"rpshnaf39wBUDNEGHJKLM4PQRST7VWXYZ2bcdeCg65jkm8oFqi1tuvAxyz";

/// Binary reader with position tracking
pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[inline]
    #[allow(dead_code)] // Useful for debugging
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    #[inline]
    pub fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let val = self.data[self.pos];
        self.pos += 1;
        Ok(val)
    }

    #[inline]
    pub fn read_u16_be(&mut self) -> Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let val = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    #[inline]
    pub fn read_u32_be(&mut self) -> Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let val = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(val)
    }

    #[inline]
    pub fn read_u64_be(&mut self) -> Result<u64> {
        if self.pos + 8 > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let val = u64::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(val)
    }

    #[inline]
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    /// Read variable length integer (XRPL VL encoding)
    #[inline]
    pub fn read_vl_length(&mut self) -> Result<usize> {
        let first = self.read_u8()? as usize;

        if first <= 192 {
            Ok(first)
        } else if first <= 240 {
            let second = self.read_u8()? as usize;
            Ok(193 + (first - 193) * 256 + second)
        } else if first <= 254 {
            let second = self.read_u8()? as usize;
            let third = self.read_u8()? as usize;
            Ok(12481 + (first - 241) * 65536 + second * 256 + third)
        } else {
            Err(Error::Other("Invalid VL length prefix".into()))
        }
    }

    /// Read field header -> (type_code, field_code)
    #[inline]
    pub fn read_field_header(&mut self) -> Result<(u8, u8)> {
        let byte = self.read_u8()?;

        let mut type_code = (byte >> 4) & 0x0F;
        let mut field_code = byte & 0x0F;

        // Extended type code
        if type_code == 0 {
            type_code = self.read_u8()?;
        }

        // Extended field code
        if field_code == 0 {
            field_code = self.read_u8()?;
        }

        Ok((type_code, field_code))
    }
}

/// Main deserializer
pub struct Deserializer;

impl Deserializer {
    /// Deserialize transaction from hex string
    pub fn decode_transaction_hex(hex: &str) -> Result<Value> {
        let bytes = hex::decode(hex).map_err(|e| Error::Other(format!("Invalid hex: {}", e)))?;
        Self::decode_transaction(&bytes)
    }

    /// Deserialize transaction from bytes
    pub fn decode_transaction(data: &[u8]) -> Result<Value> {
        let defs = get_definitions().ok_or(Error::DefinitionsNotLoaded)?;
        let mut reader = BinaryReader::new(data);
        let mut result = Map::new();

        Self::decode_object(&mut reader, &mut result, defs)?;

        Ok(Value::Object(result))
    }

    /// Deserialize ledger object from hex string
    pub fn decode_ledger_object_hex(hex: &str) -> Result<Value> {
        let bytes = hex::decode(hex).map_err(|e| Error::Other(format!("Invalid hex: {}", e)))?;
        Self::decode_ledger_object(&bytes)
    }

    /// Deserialize ledger object from bytes
    pub fn decode_ledger_object(data: &[u8]) -> Result<Value> {
        let defs = get_definitions().ok_or(Error::DefinitionsNotLoaded)?;
        let mut reader = BinaryReader::new(data);
        let mut result = Map::new();

        Self::decode_object(&mut reader, &mut result, defs)?;

        Ok(Value::Object(result))
    }

    /// Decode an STObject
    fn decode_object(
        reader: &mut BinaryReader,
        result: &mut Map<String, Value>,
        defs: &Definitions,
    ) -> Result<()> {
        while !reader.is_empty() {
            let (type_code, field_code) = reader.read_field_header()?;

            // STObject end marker (type=14, field=1)
            if type_code == 0x0E && field_code == 1 {
                break;
            }

            // STArray end marker (type=15, field=1)
            if type_code == 0x0F && field_code == 1 {
                break;
            }

            let field_name = defs
                .get_field_name(type_code, field_code)
                .unwrap_or(&format!("Unknown_{}_{}", type_code, field_code))
                .to_string();

            let value = Self::decode_field(reader, type_code, field_code, &field_name, defs)?;
            result.insert(field_name, value);
        }

        Ok(())
    }

    /// Decode a single field by type
    fn decode_field(
        reader: &mut BinaryReader,
        type_code: u8,
        _field_code: u8,
        field_name: &str,
        defs: &Definitions,
    ) -> Result<Value> {
        match type_code {
            // UInt8
            16 => {
                let val = reader.read_u8()?;
                
                // Special handling for TransactionResult (convert to name like "tesSUCCESS")
                // TransactionResult codes in metadata are 0-199:
                //   - tesSUCCESS = 0
                //   - tecCLAIM = 100, tecPATH_PARTIAL = 101, etc.
                // Note: Failed transactions (tef*, tem*, ter*) have negative codes but
                // never appear in metadata since they don't get recorded on the ledger.
                if field_name == "TransactionResult" || field_name == "HookResult" {
                    if let Some(name) = defs.get_transaction_result_name(val as i16) {
                        return Ok(json!(name));
                    }
                }
                
                Ok(json!(val))
            }

            // UInt16
            1 => {
                let val = reader.read_u16_be()?;

                // Special handling for TransactionType and LedgerEntryType
                if field_name == "TransactionType" {
                    if let Some(name) = defs.get_transaction_type_name(val) {
                        // Alias: "Amendment" -> "EnableAmendment" to match ripple-binary-codec
                        if name == "Amendment" {
                            return Ok(json!("EnableAmendment"));
                        }
                        return Ok(json!(name));
                    }
                } else if field_name == "LedgerEntryType" {
                    if let Some(name) = defs.get_ledger_entry_type_name(val) {
                        return Ok(json!(name));
                    }
                }
                Ok(json!(val))
            }

            // UInt32
            2 => {
                let val = reader.read_u32_be()?;
                Ok(json!(val))
            }

            // UInt64
            3 => {
                let val = reader.read_u64_be()?;
                // Return as 16-char uppercase hex string (matches ripple-binary-codec)
                Ok(json!(format!("{:016X}", val)))
            }

            // Hash128 (e.g., EmailHash)
            4 => {
                let bytes = reader.read_bytes(16)?;
                // All zeros = empty (matches ripple-binary-codec for EmailHash)
                if bytes.iter().all(|&b| b == 0) {
                    return Ok(json!(""));
                }
                Ok(json!(hex::encode_upper(bytes)))
            }

            // Hash256
            5 => {
                let bytes = reader.read_bytes(32)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // Amount
            6 => Self::decode_amount(reader),

            // Blob (VL-encoded)
            7 => {
                let len = reader.read_vl_length()?;
                let bytes = reader.read_bytes(len)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // AccountID (VL-encoded)
            8 => {
                let len = reader.read_vl_length()?;
                let bytes = reader.read_bytes(len)?;
                
                // Handle 0-length AccountID: treat as black hole account (all zeros)
                // This matches ripple-binary-codec behavior for edge cases
                if bytes.is_empty() {
                    return Ok(json!("rrrrrrrrrrrrrrrrrrrrrhoLvTp"));
                }
                
                Ok(json!(encode_account_id(bytes)))
            }

            // STObject
            14 => {
                let mut obj = Map::new();
                Self::decode_object(reader, &mut obj, defs)?;
                Ok(Value::Object(obj))
            }

            // STArray
            15 => {
                let mut arr = Vec::new();
                loop {
                    if reader.is_empty() {
                        break;
                    }
                    let (tc, fc) = reader.read_field_header()?;

                    // Array end marker
                    if tc == 0x0F && fc == 1 {
                        break;
                    }

                    let inner_field_name = defs
                        .get_field_name(tc, fc)
                        .unwrap_or(&format!("Unknown_{}_{}", tc, fc))
                        .to_string();

                    let value = Self::decode_field(reader, tc, fc, &inner_field_name, defs)?;

                    let mut item = Map::new();
                    item.insert(inner_field_name, value);
                    arr.push(Value::Object(item));
                }
                Ok(Value::Array(arr))
            }

            // Hash160
            17 => {
                let bytes = reader.read_bytes(20)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // PathSet
            18 => Self::decode_pathset(reader),

            // Vector256
            19 => {
                let len = reader.read_vl_length()?;
                let count = len / 32;
                let mut hashes = Vec::with_capacity(count);
                for _ in 0..count {
                    let bytes = reader.read_bytes(32)?;
                    hashes.push(json!(hex::encode_upper(bytes)));
                }
                Ok(Value::Array(hashes))
            }

            // UInt96 (12 bytes = 96 bits)
            20 => {
                let bytes = reader.read_bytes(12)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // UInt192 (24 bytes = 192 bits)
            21 => {
                let bytes = reader.read_bytes(24)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // UInt384 (48 bytes = 384 bits)
            22 => {
                let bytes = reader.read_bytes(48)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // UInt512 (64 bytes = 512 bits)
            23 => {
                let bytes = reader.read_bytes(64)?;
                Ok(json!(hex::encode_upper(bytes)))
            }

            // Issue (Currency + Issuer) - XRPL mainnet, may not be in Xahau
            24 => {
                let currency_bytes = reader.read_bytes(20)?;
                let issuer_bytes = reader.read_bytes(20)?;
                Ok(json!({
                    "currency": decode_currency(currency_bytes),
                    "issuer": encode_account_id(issuer_bytes)
                }))
            }

            // XChainBridge - XRPL mainnet, may not be in Xahau
            25 => {
                // LockingChainDoor (AccountID with VL)
                let len1 = reader.read_vl_length()?;
                let locking_door = reader.read_bytes(len1)?;

                // LockingChainIssue (Issue)
                let locking_currency = reader.read_bytes(20)?;
                let locking_issuer = reader.read_bytes(20)?;

                // IssuingChainDoor (AccountID with VL)
                let len2 = reader.read_vl_length()?;
                let issuing_door = reader.read_bytes(len2)?;

                // IssuingChainIssue (Issue)
                let issuing_currency = reader.read_bytes(20)?;
                let issuing_issuer = reader.read_bytes(20)?;

                Ok(json!({
                    "LockingChainDoor": encode_account_id(locking_door),
                    "LockingChainIssue": {
                        "currency": decode_currency(locking_currency),
                        "issuer": encode_account_id(locking_issuer)
                    },
                    "IssuingChainDoor": encode_account_id(issuing_door),
                    "IssuingChainIssue": {
                        "currency": decode_currency(issuing_currency),
                        "issuer": encode_account_id(issuing_issuer)
                    }
                }))
            }

            _ => Err(Error::InvalidTypeCode(type_code)),
        }
    }

    /// Decode Amount field (XAH/XRP or IOU)
    fn decode_amount(reader: &mut BinaryReader) -> Result<Value> {
        let first_byte = reader.read_u8()?;

        if (first_byte & 0x80) == 0 {
            // Native currency (XAH/XRP)
            let remaining = reader.read_bytes(7)?;
            let mut amount_bytes = [0u8; 8];
            amount_bytes[0] = first_byte;
            amount_bytes[1..8].copy_from_slice(remaining);

            let raw = u64::from_be_bytes(amount_bytes);
            let is_positive = (raw & 0x4000000000000000) != 0;
            let drops = raw & 0x3FFFFFFFFFFFFFFF;

            if is_positive {
                Ok(json!(drops.to_string()))
            } else {
                Ok(json!(format!("-{}", drops)))
            }
        } else {
            // IOU
            let remaining = reader.read_bytes(7)?;
            let mut amount_bytes = [0u8; 8];
            amount_bytes[0] = first_byte;
            amount_bytes[1..8].copy_from_slice(remaining);

            let currency_bytes = reader.read_bytes(20)?;
            let issuer_bytes = reader.read_bytes(20)?;

            let raw = u64::from_be_bytes(amount_bytes);

            // Check for zero
            if raw == 0x8000000000000000 {
                return Ok(json!({
                    "value": "0",
                    "currency": decode_currency(currency_bytes),
                    "issuer": encode_account_id(issuer_bytes)
                }));
            }

            let is_positive = (raw & 0x4000000000000000) != 0;
            let exponent = (((raw >> 54) & 0xFF) as i32) - 97;
            let mantissa = raw & 0x3FFFFFFFFFFFFF;

            let sign = if is_positive { "" } else { "-" };
            let value = format_iou_value(sign, mantissa, exponent);

            Ok(json!({
                "value": value,
                "currency": decode_currency(currency_bytes),
                "issuer": encode_account_id(issuer_bytes)
            }))
        }
    }

    /// Decode PathSet
    fn decode_pathset(reader: &mut BinaryReader) -> Result<Value> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();

        loop {
            if reader.is_empty() {
                break;
            }

            let type_byte = reader.read_u8()?;

            if type_byte == 0x00 {
                // End of pathset
                if !current_path.is_empty() {
                    paths.push(Value::Array(std::mem::take(&mut current_path)));
                }
                break;
            } else if type_byte == 0xFF {
                // Path boundary
                if !current_path.is_empty() {
                    paths.push(Value::Array(std::mem::take(&mut current_path)));
                }
            } else {
                // Path element
                let mut element = Map::new();

                if (type_byte & 0x01) != 0 {
                    let account = reader.read_bytes(20)?;
                    element.insert("account".into(), json!(encode_account_id(account)));
                }
                if (type_byte & 0x10) != 0 {
                    let currency = reader.read_bytes(20)?;
                    element.insert("currency".into(), json!(decode_currency(currency)));
                }
                if (type_byte & 0x20) != 0 {
                    let issuer = reader.read_bytes(20)?;
                    element.insert("issuer".into(), json!(encode_account_id(issuer)));
                }

                current_path.push(Value::Object(element));
            }
        }

        Ok(Value::Array(paths))
    }
}

/// Encode 20-byte account ID to base58check address
pub fn encode_account_id(bytes: &[u8]) -> String {
    // Handle empty or invalid length
    if bytes.is_empty() {
        return String::new();
    }
    if bytes.len() != 20 {
        return hex::encode_upper(bytes);
    }

    // Add version byte (0x00 for mainnet accounts)
    let mut with_version = Vec::with_capacity(25);
    with_version.push(0x00);
    with_version.extend_from_slice(bytes);

    // Calculate checksum: first 4 bytes of double SHA-256
    let hash1 = Sha256::digest(&with_version);
    let hash2 = Sha256::digest(&hash1);
    with_version.extend_from_slice(&hash2[..4]);

    // Base58 encode
    base58_encode(&with_version)
}

/// Decode 20-byte currency code
pub fn decode_currency(bytes: &[u8]) -> String {
    if bytes.len() != 20 {
        return hex::encode_upper(bytes);
    }

    // All zeros = native currency (XAH/XRP)
    if bytes.iter().all(|&b| b == 0) {
        return "XAH".into();
    }

    // Standard 3-char currency:
    // - bytes 0-11: must be zero (first 12 bytes)
    // - bytes 12-14: ASCII characters (the 3-char code)
    // - bytes 15-19: must be zero (last 5 bytes)
    let is_standard = bytes[0..12].iter().all(|&b| b == 0)
        && bytes[15..20].iter().all(|&b| b == 0)
        && bytes[12..15].iter().all(|&b| b.is_ascii_graphic() || b == 0);

    if is_standard && bytes[12..15].iter().any(|&b| b != 0) {
        // Find the actual 3-char code (may have trailing zeros for codes < 3 chars)
        if let Ok(s) = std::str::from_utf8(&bytes[12..15]) {
            let trimmed = s.trim_end_matches('\0');
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    // Non-standard currency: return hex
    hex::encode_upper(bytes)
}

/// Format IOU value as clean decimal string (matches ripple-binary-codec output)
/// value = mantissa × 10^exponent
fn format_iou_value(sign: &str, mantissa: u64, exponent: i32) -> String {
    if mantissa == 0 {
        return "0".to_string();
    }

    let mantissa_str = mantissa.to_string();
    let mantissa_len = mantissa_str.len() as i32;
    
    // Where the decimal point should be from the LEFT of the mantissa string
    // exponent tells us: value = mantissa * 10^exponent
    // If exponent >= 0, we add zeros to the right
    // If exponent < 0, we move decimal point left
    
    let result = if exponent >= 0 {
        // Positive exponent: append zeros
        format!("{}{}", mantissa_str, "0".repeat(exponent as usize))
    } else {
        let shift = (-exponent) as usize;
        if shift >= mantissa_len as usize {
            // Decimal is before all digits: 0.000...{mantissa}
            let leading_zeros = shift - mantissa_len as usize;
            // Trim trailing zeros from mantissa
            let mantissa_trimmed = mantissa_str.trim_end_matches('0');
            if mantissa_trimmed.is_empty() {
                "0".to_string()
            } else {
                format!("0.{}{}", "0".repeat(leading_zeros), mantissa_trimmed)
            }
        } else {
            // Decimal is within the mantissa
            let decimal_pos = mantissa_len as usize - shift;
            let (integer_part, fractional_part) = mantissa_str.split_at(decimal_pos);
            let fractional_trimmed = fractional_part.trim_end_matches('0');
            if fractional_trimmed.is_empty() {
                integer_part.to_string()
            } else {
                format!("{}.{}", integer_part, fractional_trimmed)
            }
        }
    };
    
    format!("{}{}", sign, result)
}

/// Base58 encode using XRPL alphabet
fn base58_encode(data: &[u8]) -> String {
    // Convert to big integer
    let mut num = data.iter().fold(
        num_bigint::BigUint::from(0u32),
        |acc, &b| acc * 256u32 + b as u32,
    );

    let mut result = Vec::new();
    let base = num_bigint::BigUint::from(58u32);
    let zero = num_bigint::BigUint::from(0u32);

    while num > zero {
        let rem = (&num % &base).to_u32_digits();
        let idx = rem.first().copied().unwrap_or(0) as usize;
        result.push(BASE58_ALPHABET[idx]);
        num /= &base;
    }

    // Add leading 'r' for leading zero bytes
    for &b in data.iter() {
        if b == 0 {
            result.push(BASE58_ALPHABET[0]);
        } else {
            break;
        }
    }

    result.reverse();
    String::from_utf8(result).unwrap_or_else(|_| hex::encode_upper(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_currency_native() {
        let native = [0u8; 20];
        assert_eq!(decode_currency(&native), "XAH");
    }

    #[test]
    fn test_decode_currency_standard() {
        // Standard 3-char currency: bytes 12-14 contain the code
        let mut usd = [0u8; 20];
        usd[12] = b'U';
        usd[13] = b'S';
        usd[14] = b'D';
        assert_eq!(decode_currency(&usd), "USD");
        
        // Test EUR
        let mut eur = [0u8; 20];
        eur[12] = b'E';
        eur[13] = b'U';
        eur[14] = b'R';
        assert_eq!(decode_currency(&eur), "EUR");
    }

    #[test]
    fn test_vl_length() {
        let mut reader = BinaryReader::new(&[0x05]);
        assert_eq!(reader.read_vl_length().unwrap(), 5);

        let mut reader = BinaryReader::new(&[0xC1, 0x00]);
        assert_eq!(reader.read_vl_length().unwrap(), 193);
    }
    
    #[test]
    fn test_black_hole_account() {
        // 20 bytes of zeros = black hole account
        let zeros = [0u8; 20];
        let encoded = encode_account_id(&zeros);
        assert_eq!(encoded, "rrrrrrrrrrrrrrrrrrrrrhoLvTp");
    }
}
