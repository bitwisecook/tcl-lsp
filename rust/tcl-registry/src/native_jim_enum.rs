// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim GetEnum and CompareStringImmediate recipes over retained declarations.
use super::native_index_lookup::NativeStaticIndexTable;
use tcl_core_types::NativeJimOptionCache;
use tcl_syntax::native_string::NativeStringProtocol;

/// Actual Jim flags. Every bit participates in the original cache equality key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJimEnumFlags(pub i32);
impl NativeJimEnumFlags {
    /// Request Jim's native bad/ambiguous option diagnostic on lookup failure.
    pub const ERROR_MESSAGE: i32 = 1;
    /// Allow a unique counted-prefix abbreviation after exact `CString` matching.
    pub const ABBREVIATE: i32 = 2;
    /// Select ordinary option flags, always requesting diagnostics.
    /// `exact` disables the abbreviation bit.
    #[must_use]
    pub const fn options(exact: bool) -> Self {
        Self(Self::ERROR_MESSAGE | if exact { 0 } else { Self::ABBREVIATE })
    }
}
/// Pure Jim enum and immediate-string comparison rules over retained static tables.
#[derive(Debug, Clone, Copy)]
pub struct NativeJimEnumProtocol;
impl NativeJimEnumProtocol {
    /// Consume an Enum hit only for the same retained table and complete flags value.
    /// `ComparedString` primaries and mismatched declarations return `None`.
    #[must_use]
    pub fn cached_index(
        self,
        cache: Option<&NativeJimOptionCache>,
        table: &NativeStaticIndexTable,
        flags: NativeJimEnumFlags,
    ) -> Option<usize> {
        match cache? {
            NativeJimOptionCache::Enum {
                entry,
                flags: cached,
            } if *cached == flags.0 && entry.table_identity() == table.declaration_identity() => {
                Some(entry.index())
            }
            _ => None,
        }
    }
    /// Jim exact comparison is `CString`; abbreviation uses counted arglen with strncmp.
    ///
    /// # Errors
    /// Returns `None` for a silent miss, or the native byte diagnostic when
    /// `ERROR_MESSAGE` is selected and the name is missing or ambiguous.
    pub fn lookup(
        self,
        original: &[u8],
        table: &NativeStaticIndexTable,
        flags: NativeJimEnumFlags,
        noun: Option<&[u8]>,
    ) -> Result<NativeJimOptionCache, Option<Vec<u8>>> {
        let arg = tcl_core_types::c_string_extent(original);
        let mut matched = None;
        let mut ambiguous = false;
        for index in 0..table.entry_count() {
            let word = tcl_core_types::c_string_extent(
                table.original_entry(index).expect("retained table entry"),
            );
            if arg == word {
                return Ok(NativeJimOptionCache::Enum {
                    entry: table.cache(index),
                    flags: flags.0,
                });
            }
            if flags.0 & NativeJimEnumFlags::ABBREVIATE != 0
                && word.starts_with(arg)
                && original.len() == arg.len()
            {
                if arg == b"-" {
                    break;
                }
                if matched.is_some() {
                    ambiguous = true;
                    break;
                }
                matched = Some(index);
            }
        }
        if !ambiguous && let Some(index) = matched {
            return Ok(NativeJimOptionCache::Enum {
                entry: table.cache(index),
                flags: flags.0,
            });
        }
        if flags.0 & NativeJimEnumFlags::ERROR_MESSAGE == 0 {
            return Err(None);
        }
        let mut message = if ambiguous {
            b"ambiguous ".to_vec()
        } else {
            b"bad ".to_vec()
        };
        message.extend_from_slice(noun.unwrap_or(b"option"));
        message.extend_from_slice(b" \"");
        message.extend_from_slice(arg);
        message.extend_from_slice(b"\": must be ");
        let mut words: Vec<_> = (0..table.entry_count())
            .map(|index| {
                tcl_core_types::c_string_extent(
                    table.original_entry(index).expect("retained entry"),
                )
            })
            .collect();
        words.sort_by(|a, b| match (*a == b"--", *b == b"--") {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        });
        for (index, word) in words.iter().enumerate() {
            if index > 0 && index + 1 == words.len() {
                message.extend_from_slice(b"or ");
            }
            message.extend_from_slice(word);
            if index + 1 < words.len() {
                message.extend_from_slice(b", ");
            }
        }
        Err(Some(message))
    }
    /// Match a `ComparedString` primary against the actual selected static literal.
    /// Equal text from another literal grants no cache hit.
    /// Compare `CString` extents and retain a `ComparedString` receipt on equality.
    /// Returns `None` for a mismatch or an unavailable declaration entry.
    #[must_use]
    pub fn compared_hit(
        self,
        cache: Option<&NativeJimOptionCache>,
        table: &NativeStaticIndexTable,
        index: usize,
    ) -> bool {
        matches!(cache,Some(NativeJimOptionCache::ComparedString {literal_identity,..}) if Some(*literal_identity)==table.literal_identity(index))
    }
    /// Compare `CString` extents and retain a `ComparedString` receipt on equality.
    /// Returns `None` for a mismatch or an unavailable declaration entry.
    #[must_use]
    pub fn compare(
        self,
        original: &[u8],
        table: &NativeStaticIndexTable,
        index: usize,
    ) -> Option<NativeJimOptionCache> {
        let word = table.original_entry(index)?;
        (tcl_core_types::c_string_extent(original) == tcl_core_types::c_string_extent(word)).then(
            || NativeJimOptionCache::ComparedString {
                entry: table.cache(index),
                literal_identity: table
                    .literal_identity(index)
                    .expect("original static entry"),
            },
        )
    }
}
impl crate::InvocationDialect {
    /// Select genuine Jim enum rules from the actual Jim string issuer.
    /// C releases and unavailable issuers return `None`.
    #[must_use]
    pub fn native_jim_enum_protocol(self) -> Option<NativeJimEnumProtocol> {
        (self.native_string_protocol() == Some(NativeStringProtocol::Jim084))
            .then_some(NativeJimEnumProtocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORDS: &[&str] = &["provide", "present", "--"];
    #[test]
    fn genuine_jim_cache_key_retains_every_flags_bit_and_literal_identity() {
        let protocol = NativeJimEnumProtocol;
        let table = NativeStaticIndexTable::supported_backend(WORDS);
        let cache = protocol
            .lookup(b"pro", &table, NativeJimEnumFlags(3), None)
            .unwrap();
        assert_eq!(
            protocol.cached_index(Some(&cache), &table, NativeJimEnumFlags(3)),
            Some(0)
        );
        assert_eq!(
            protocol.cached_index(Some(&cache), &table, NativeJimEnumFlags(1)),
            None
        );
        assert_eq!(
            protocol.cached_index(Some(&cache), &table, NativeJimEnumFlags(7)),
            None
        );
        let compared = protocol.compare(b"provide\0tail", &table, 0).unwrap();
        assert!(protocol.compared_hit(Some(&compared), &table, 0));
        assert!(!protocol.compared_hit(Some(&compared), &table, 1));
        assert_eq!(
            protocol
                .lookup(b"p", &table, NativeJimEnumFlags(2), None)
                .unwrap_err(),
            None
        );
        assert_eq!(
            protocol
                .lookup(b"p", &table, NativeJimEnumFlags(3), None)
                .unwrap_err(),
            Some(b"ambiguous option \"p\": must be present, provide, or --".to_vec())
        );
    }
}
