//! Selected C array-search handle conversion, distinct from numeric getters.

use tcl_core_types::{NativeArraySearchAbi, NativeArraySearchCache, NativeHashWordWidth};
use tcl_dialect::TclVersion;

/// Original lookup failure, before Eval applies its own propagation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeArraySearchFailure {
    /// The original spelling has no valid search prefix and decimal ID.
    IllegalIdentifier,
    /// The parsed handle names a different variable operand.
    WrongVariable,
    /// No currently active cursor matches the original handle.
    MissingSearch,
}

/// Pure native handle recipe; actual engine selection belongs to its issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeArraySearchProtocol {
    version: TclVersion,
    abi: NativeArraySearchAbi,
}
impl NativeArraySearchProtocol {
    /// Select a pure recipe with independently supplied C ABI facts.
    #[must_use]
    pub fn for_tcl_version(version: TclVersion, abi: NativeArraySearchAbi) -> Option<Self> {
        (abi.int_bits == 32).then_some(Self { version, abi })
    }
    /// Exact physical cache origin.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }
    /// C8 installs the resident-only array-search primary; C9 does not.
    #[must_use]
    pub fn caches_handle(self) -> bool {
        self.version < TclVersion::V9_0
    }
    /// C9's start-search formatter produces an unknown-count String primary.
    /// Legacy start-search results retain an untyped resident string.
    #[must_use]
    pub fn start_handle_has_string_primary(self) -> bool {
        self.version >= TclVersion::V9_0
    }
    /// Installation validates origin, not an active search or array identity.
    #[must_use]
    pub fn accepts_cache_origin(self, origin: TclVersion) -> bool {
        self.caches_handle() && self.version == origin
    }
    /// C-string boundary, independent of UTF storage and name-key extent.
    #[must_use]
    pub fn c_string(self, bytes: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(bytes)
    }
    /// Parse the exact decimal strtoul stage, including ERANGE saturation.
    pub fn parse(self, bytes: &[u8]) -> Result<NativeArraySearchCache, NativeArraySearchFailure> {
        let bytes = self.c_string(bytes);
        if !bytes.starts_with(b"s-") {
            return Err(NativeArraySearchFailure::IllegalIdentifier);
        }
        let mut position = 2;
        while bytes
            .get(position)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c'))
        {
            position += 1;
        }
        let negative = bytes.get(position) == Some(&b'-');
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        let first = position;
        let maximum = match self.abi.unsigned_long {
            NativeHashWordWidth::Bits32 => u64::from(u32::MAX),
            NativeHashWordWidth::Bits64 => u64::MAX,
        };
        let mut magnitude = 0_u64;
        let mut overflow = false;
        while let Some(byte @ b'0'..=b'9') = bytes.get(position) {
            match magnitude
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            {
                Some(value) if value <= maximum && !overflow => magnitude = value,
                _ => {
                    magnitude = maximum;
                    overflow = true;
                }
            }
            position += 1;
        }
        if position == first || bytes.get(position) != Some(&b'-') {
            return Err(NativeArraySearchFailure::IllegalIdentifier);
        }
        if negative && !overflow {
            magnitude = magnitude.wrapping_neg() & maximum;
        }
        let low = u32::try_from(magnitude & u64::from(u32::MAX)).expect("masked native int");
        Ok(NativeArraySearchCache {
            id: i32::from_ne_bytes(low.to_ne_bytes()),
            name_offset: position + 1,
        })
    }
    /// Original name comparison after an actually reached handle conversion.
    #[must_use]
    pub fn is_for_variable(self, bytes: &[u8], cache: NativeArraySearchCache, name: &[u8]) -> bool {
        self.c_string(bytes)
            .get(cache.name_offset..)
            .is_some_and(|tail| tail == self.c_string(name))
    }
    /// Handle production uses the array operand's original `CString` spelling.
    #[must_use]
    pub fn handle(self, id: i32, name: &[u8]) -> Vec<u8> {
        let mut bytes = format!("s-{id}-").into_bytes();
        bytes.extend_from_slice(self.c_string(name));
        bytes
    }
    /// Exact primitive message; the completion owner propagates it separately.
    #[must_use]
    pub fn failure_message(
        self,
        failure: NativeArraySearchFailure,
        handle: &[u8],
        name: &[u8],
    ) -> Vec<u8> {
        use NativeArraySearchFailure as F;
        let mut result = match failure {
            F::IllegalIdentifier => b"illegal search identifier \"".to_vec(),
            F::WrongVariable => b"search identifier \"".to_vec(),
            F::MissingSearch => b"couldn't find search \"".to_vec(),
        };
        result.extend_from_slice(self.c_string(handle));
        result.push(b'"');
        if failure == F::WrongVariable {
            result.extend_from_slice(b" isn't for variable \"");
            result.extend_from_slice(self.c_string(name));
            result.push(b'"');
        }
        result
    }
    /// C8.4 leaves primitive errorCode unchanged; later engines set ARRAYSEARCH.
    #[must_use]
    pub fn sets_lookup_error_code(self) -> bool {
        self.version != TclVersion::V8_4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    #[test]
    fn native_original_handle_cache_and_failures_match_sixty_five_c_rows() {
        let inputs: &[&[u8]] = &[
            b"s-1-a",
            b"s-01-a",
            b"s-+1-a",
            b"s- 1-a",
            b"s--1-a",
            b"s-4294967297-a",
            b"s-18446744073709551617-a",
            b"s-1-other",
            b"s-1-a\0tail",
            b"BAD",
            b"s-1",
            b"s--a",
            b"s-1-a\xff",
        ];
        let captures = [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_array_search/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_array_search/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_array_search/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_array_search/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_array_search/9.1.0.tsv"),
            ),
        ];
        let mut rows = 0;
        for (version, capture) in captures {
            let protocol = NativeArraySearchProtocol::for_tcl_version(
                version,
                NativeArraySearchAbi {
                    unsigned_long: NativeHashWordWidth::Bits64,
                    int_bits: 32,
                },
            )
            .unwrap();
            for line in capture.lines() {
                let fields: Vec<_> = line.split('\t').collect();
                let bytes = inputs[fields[0].parse::<usize>().unwrap()];
                let parsed = protocol.parse(bytes);
                if protocol.caches_handle()
                    && let Ok(cache) = parsed
                {
                    assert_eq!(fields[2], "array search", "{version:?}: {line}");
                    assert_eq!(cache.id, fields[3].parse::<i32>().unwrap());
                    assert_eq!(cache.name_offset, fields[4].parse::<usize>().unwrap());
                } else {
                    assert_eq!(fields[2], "none", "{version:?}: {line}");
                }
                let failure = match parsed {
                    Err(failure) => Some(failure),
                    Ok(cache) if !protocol.is_for_variable(bytes, cache, b"a") => {
                        Some(NativeArraySearchFailure::WrongVariable)
                    }
                    Ok(cache)
                        if (protocol.caches_handle() && cache.id == 1)
                            || (!protocol.caches_handle()
                                && protocol.c_string(bytes) == b"s-1-a") =>
                    {
                        None
                    }
                    Ok(_) => Some(NativeArraySearchFailure::MissingSearch),
                };
                let result = failure.map_or_else(
                    || b"1".to_vec(),
                    |failure| protocol.failure_message(failure, bytes, b"a"),
                );
                let hex: String = result.iter().fold(String::new(), |mut output, byte| {
                    write!(output, "{byte:02x}").unwrap();
                    output
                });
                assert_eq!(hex, fields[5], "{version:?}: {line}");
                assert_eq!(
                    usize::from(failure.is_some()),
                    fields[1].parse::<usize>().unwrap()
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 65);
    }
    #[test]
    fn unsigned_long_width_is_independent_and_missing_int_width_withdraws() {
        let select = |width| {
            NativeArraySearchProtocol::for_tcl_version(
                TclVersion::V8_6,
                NativeArraySearchAbi {
                    unsigned_long: width,
                    int_bits: 32,
                },
            )
            .unwrap()
        };
        assert_eq!(
            select(NativeHashWordWidth::Bits32)
                .parse(b"s-4294967297-a")
                .unwrap()
                .id,
            -1
        );
        assert_eq!(
            select(NativeHashWordWidth::Bits64)
                .parse(b"s-4294967297-a")
                .unwrap()
                .id,
            1
        );
        assert_eq!(
            select(NativeHashWordWidth::Bits64)
                .parse(b"s--18446744073709551617-a")
                .unwrap()
                .id,
            -1
        );
        assert!(
            NativeArraySearchProtocol::for_tcl_version(
                TclVersion::V9_0,
                NativeArraySearchAbi {
                    unsigned_long: NativeHashWordWidth::Bits64,
                    int_bits: 64
                }
            )
            .is_none()
        );
    }
}
