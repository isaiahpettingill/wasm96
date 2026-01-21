//! Loader utilities for wasm96-core.
//!
//! Responsibilities:
//! - Detect whether the provided ROM bytes are a `.wasm` binary or `.wat` text.
//! - If it looks like WAT, convert it to WASM bytes (via the `wat` crate).
//!
//! Notes:
//! - libretro provides the ROM bytes; extension sniffing is unreliable in some setups,
//!   so we sniff the bytes themselves.
//! - We accept leading whitespace/comments for WAT as best-effort.

/// Error returned by loader helpers.
#[derive(Debug)]
pub enum LoadError {
    /// The input was empty or otherwise not recognized as WASM/WAT.
    UnrecognizedFormat,
    /// WAT parsing failed.
    WatParseFailed(wat::Error),
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadError::UnrecognizedFormat => {
                write!(f, "unrecognized ROM format (expected wasm or wat)")
            }
            LoadError::WatParseFailed(e) => write!(f, "failed to parse WAT: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// What kind of module the loader inferred from the bytes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DetectedFormat {
    Wasm,
    Wat,
}

/// Detect format and normalize to valid WASM bytes.
pub fn normalize_to_wasm(rom_bytes: &[u8]) -> Result<Detected, LoadError> {
    let format = detect_format(rom_bytes).ok_or(LoadError::UnrecognizedFormat)?;

    match format {
        DetectedFormat::Wasm => Ok(Detected {
            format,
            wasm_bytes: rom_bytes.to_vec(),
        }),
        DetectedFormat::Wat => {
            let bytes = wat::parse_bytes(rom_bytes).map_err(LoadError::WatParseFailed)?;
            Ok(Detected {
                format,
                wasm_bytes: bytes.into(),
            })
        }
    }
}

/// Result of normalizing (detecting + possibly converting) the input.
#[derive(Clone, Debug)]
pub struct Detected {
    pub format: DetectedFormat,
    /// Always valid WASM bytes (for WASM/WAT inputs).
    pub wasm_bytes: Vec<u8>,
}

/// Best-effort detection.
///
/// Rules:
/// - If the first 4 bytes are `\0asm`, treat as WASM.
/// - Else, after stripping UTF-8 BOM / leading whitespace, if the first non-ws byte is `(`,
///   treat as WAT (common WAT starts with `(module ...)`).
///
/// This intentionally avoids requiring valid UTF-8 for WAT; `wat::parse_bytes` accepts bytes.
pub fn detect_format(bytes: &[u8]) -> Option<DetectedFormat> {
    if is_wasm_magic(bytes) {
        return Some(DetectedFormat::Wasm);
    }

    // Check for "(...)" after skipping common whitespace/BOM.
    let i = skip_bom_and_leading_ws(bytes);
    if i < bytes.len() && bytes[i] == b'(' {
        return Some(DetectedFormat::Wat);
    }

    None
}

fn is_wasm_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == *b"\0asm"
}

fn skip_bom_and_leading_ws(bytes: &[u8]) -> usize {
    let mut i = 0;

    // UTF-8 BOM: EF BB BF
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        i = 3;
    }

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            _ => break,
        }
    }

    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wasm_magic() {
        assert_eq!(
            detect_format(b"\0asm\x01\x00\x00\x00"),
            Some(DetectedFormat::Wasm)
        );
    }

    #[test]
    fn detects_wat_with_whitespace() {
        assert_eq!(detect_format(b"   \n\t(module)"), Some(DetectedFormat::Wat));
    }

    #[test]
    fn detects_wat_with_bom() {
        assert_eq!(
            detect_format(b"\xEF\xBB\xBF(module)"),
            Some(DetectedFormat::Wat)
        );
    }

    #[test]
    fn unrecognized_returns_none() {
        assert_eq!(detect_format(b"not wasm"), None);
    }

    #[test]
    fn normalize_to_wasm_accepts_wat_and_wasm() {
        let wat = br#"(module (func (export "setup")))"#;
        let d1 = normalize_to_wasm(wat).unwrap();
        assert_eq!(d1.format, DetectedFormat::Wat);
        assert!(is_wasm_magic(&d1.wasm_bytes));

        let wasm = wat::parse_bytes(wat).unwrap();
        let d2 = normalize_to_wasm(wasm.as_ref()).unwrap();
        assert_eq!(d2.format, DetectedFormat::Wasm);
        assert_eq!(d2.wasm_bytes, wasm.as_ref().to_vec());
    }
}
