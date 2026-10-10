// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object Index lookup over retained native option-table declarations.
//!
//! The supported Rust backend owns immutable static option declarations. The
//! table's address identifies that declaration while its static lifetime keeps
//! every entry readable; it never licenses a caller-supplied C address. Native
//! ABI callbacks instead retain their independently issued `NativeIndexTable`.

use std::rc::Rc;
use tcl_core_types::{
    NativeIndexCache, NativeIndexLookupFlags, NativeIndexTable, NativeIndexUnavailable,
};
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueError;

/// Actual C `GetIndex` protocol, independently selected from source grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeIndexLookupProtocol(TclVersion);

/// Retained immutable declaration from the supported native backend.
#[derive(Clone)]
pub struct NativeStaticIndexTable {
    entries: StaticEntries,
    stride: usize,
}

#[derive(Clone)]
enum StaticEntries {
    Words(&'static [&'static str]),
    Bytes(&'static [&'static [u8]]),
}

impl NativeStaticIndexTable {
    /// Issue a supported-backend receipt for a real immutable declaration.
    /// C pointer stride comes from this backend's actual C ABI layout.
    #[must_use]
    pub fn supported_backend(words: &'static [&'static str]) -> Self {
        Self {
            entries: StaticEntries::Words(words),
            stride: std::mem::size_of::<*const std::ffi::c_char>(),
        }
    }

    /// Issue a supported-backend receipt for a static byte declaration.
    #[must_use]
    pub fn supported_backend_bytes(words: &'static [&'static [u8]]) -> Self {
        Self {
            entries: StaticEntries::Bytes(words),
            stride: std::mem::size_of::<*const std::ffi::c_char>(),
        }
    }

    /// Select an actual supported-backend layout made of pointer fields.
    /// The declaration contains the original ordered fields; the selected byte
    /// stride advances over those fields. Non-pointer field layouts require
    /// their own guarded `NativeIndexTable` producer, rather than guessed reads.
    /// The stride is an equality key and never authorizes a pointer dereference.
    pub fn with_entry_stride(mut self, stride: usize) -> Result<Self, ValueError> {
        let pointer_size = std::mem::size_of::<*const std::ffi::c_char>();
        if stride < pointer_size || !stride.is_multiple_of(pointer_size) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Index struct layout",
            ));
        }
        self.stride = stride;
        Ok(self)
    }

    fn len(&self) -> usize {
        let fields = match self.entries {
            StaticEntries::Words(words) => words.len(),
            StaticEntries::Bytes(words) => words.len(),
        };
        fields.div_ceil(self.stride / std::mem::size_of::<*const std::ffi::c_char>())
    }
    fn entry(&self, index: usize) -> Option<&'static [u8]> {
        let field =
            index.checked_mul(self.stride / std::mem::size_of::<*const std::ffi::c_char>())?;
        match self.entries {
            StaticEntries::Words(words) => words.get(field).map(|word| word.as_bytes()),
            StaticEntries::Bytes(words) => words.get(field).copied(),
        }
    }

    /// Equality key for this actual static declaration, without C lookup authority.
    #[must_use]
    pub fn declaration_identity(&self) -> usize {
        self.identity()
    }
    /// Count entries reached by the retained declaration's selected pointer stride.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.len()
    }
    /// Borrow the original static bytes at a selected entry, preserving counted extent.
    /// Returns `None` when the index or its stride-scaled field is out of range.
    #[must_use]
    pub fn original_entry(&self, index: usize) -> Option<&'static [u8]> {
        self.entry(index)
    }
    /// The original static literal pointer is only an equality key.
    #[must_use]
    pub fn literal_identity(&self, index: usize) -> Option<usize> {
        self.entry(index).map(|entry| entry.as_ptr() as usize)
    }

    /// Retain this declaration and selected entry as a primary cache receipt.
    #[must_use]
    pub fn cache(&self, index: usize) -> NativeIndexCache {
        NativeIndexCache::new(Rc::new(self.clone()), self.stride, index)
    }
}

impl NativeIndexTable for NativeStaticIndexTable {
    fn identity(&self) -> usize {
        match self.entries {
            StaticEntries::Words(words) => words.as_ptr() as usize,
            StaticEntries::Bytes(words) => words.as_ptr() as usize,
        }
    }
    fn word(&self, index: usize, stride: usize) -> Result<Rc<[u8]>, NativeIndexUnavailable> {
        if stride != self.stride {
            return Err(NativeIndexUnavailable);
        }
        self.entry(index)
            .map(|word| Rc::from(tcl_core_types::c_string_extent(word)))
            .ok_or(NativeIndexUnavailable)
    }
}

/// Native lookup success or Tcl diagnostic, independently of an unavailable object capability.
pub type NativeIndexLookupOutcome = Result<(Option<usize>, Option<NativeIndexCache>), Vec<u8>>;

impl NativeIndexLookupProtocol {
    /// Pure lookup input extent for this independently selected C recipe.
    /// It neither materializes an object nor creates or validates an Index cache.
    #[must_use]
    pub fn selected_input(self, original: &[u8]) -> &[u8] {
        match self.0 {
            TclVersion::V8_4
            | TclVersion::V8_5
            | TclVersion::V8_6
            | TclVersion::V9_0
            | TclVersion::V9_1 => tcl_core_types::c_string_extent(original),
        }
    }

    /// Actual original descriptor release.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.0
    }

    /// Validate flag availability before any original getter or cache access.
    pub fn validate_flags(self, flags: NativeIndexLookupFlags) -> Result<(), ValueError> {
        if (flags.temporary_table || flags.null_ok)
            && !matches!(self.0, TclVersion::V9_0 | TclVersion::V9_1)
        {
            return Err(ValueError::CommandProtocolUnavailable("native Index flags"));
        }
        Ok(())
    }

    /// `TEMP_TABLE` bypasses both a matching cache and its installation. Foreign
    /// physical origins remain a typed boundary before string materialization.
    pub fn cached_index_with_flags(
        self,
        cache: Option<(NativeIndexCache, TclVersion)>,
        table: &NativeStaticIndexTable,
        flags: NativeIndexLookupFlags,
    ) -> Result<Option<usize>, ValueError> {
        self.validate_flags(flags)?;
        let hit = self.cached_index(cache, table)?;
        Ok(if flags.temporary_table { None } else { hit })
    }

    /// Selected successful lookup, including the C9 native no-index sentinel.
    /// A missing cache receipt means `TEMP_TABLE` or `NULL_OK` did not install one.
    pub fn lookup_with_flags(
        self,
        bytes: Option<&[u8]>,
        table: &NativeStaticIndexTable,
        flags: NativeIndexLookupFlags,
        noun: &'static str,
    ) -> Result<NativeIndexLookupOutcome, ValueError> {
        self.validate_flags(flags)?;
        let bytes = bytes.unwrap_or_default();
        if flags.null_ok && tcl_core_types::c_string_extent(bytes).is_empty() {
            return Ok(Ok((None, None)));
        }
        Ok(self.lookup(bytes, table, flags.exact, noun).map(|cache| {
            (
                Some(cache.index()),
                (!flags.temporary_table).then_some(cache),
            )
        }))
    }

    /// Authenticate origin before materialization, and probe matching table and
    /// offset before applying the current EXACT flag. A native cache hit ignores
    /// changed lookup flags.
    pub fn cached_index(
        self,
        cache: Option<(NativeIndexCache, TclVersion)>,
        table: &NativeStaticIndexTable,
    ) -> Result<Option<usize>, ValueError> {
        let Some((cache, version)) = cache else {
            return Ok(None);
        };
        if version != self.0 {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Index origin",
            ));
        }
        Ok((cache.table_identity() == table.identity()
            && cache.stride() == table.stride
            && cache.index() < table.len())
        .then_some(cache.index()))
    }

    /// Native `GetIndex` diagnostic metadata after a reached failed lookup.
    #[must_use]
    pub fn error_code(self, noun: &[u8], original: &[u8]) -> Vec<u8> {
        if self.0 == TclVersion::V8_4 {
            return b"NONE".to_vec();
        }
        let mut code = b"TCL LOOKUP INDEX ".to_vec();
        tcl_syntax::list::append_list_element(&mut code, noun, true);
        code.push(b' ');
        tcl_syntax::list::append_list_element(
            &mut code,
            tcl_core_types::c_string_extent(original),
            true,
        );
        code
    }

    /// Native `CString` matching after the original object's checked updater.
    /// Failure leaves its primary cache untouched.
    pub fn lookup(
        self,
        bytes: &[u8],
        table: &NativeStaticIndexTable,
        exact: bool,
        noun: &'static str,
    ) -> Result<NativeIndexCache, Vec<u8>> {
        let words: Vec<&[u8]> = (0..table.len())
            .map(|index| table.entry(index).expect("static table entry"))
            .collect();
        let matcher = if exact {
            tcl_cmd_core::prefix::OptionTable::exact_only(noun, &words)
        } else {
            tcl_cmd_core::prefix::OptionTable::abbreviating(noun, &words)
        };
        matcher
            .index_of(self.selected_input(bytes))
            .map(|index| table.cache(index))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_info_option_lookup_is_separate_from_ensemble_and_jim_dispatch() {
        // naming.info.original-root-and-explicit-nons-command-inventory
        // docs/design/analysis/name-resolution-proofs/info-original-root-and-explicit-nons-command-inventory.md
        // naming.info.original-missing-and-empty-selector-dispatch
        // docs/design/analysis/name-resolution-proofs/info-original-missing-and-empty-selector-dispatch.md
        // Pure purpose selection; both backends compare the whole original
        // six-provider inventory programs independently of this API control.
        for engine in [
            "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim", "irules",
        ] {
            let dialect = crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment(engine).unit_profile(),
            );
            assert_eq!(
                dialect.native_info_original_option_protocol().is_some(),
                engine == "tcl8.4"
            );
            let expected_usage: Option<&[u8]> = match engine {
                "tcl8.4" => Some(b"option ?arg arg ...?"),
                "tcl8.5" => Some(b"subcommand ?argument ...?"),
                "tcl8.6" | "tcl9.0" | "tcl9.1" => Some(b"subcommand ?arg ...?"),
                "jim" | "irules" => None,
                _ => unreachable!(),
            };
            assert_eq!(
                dialect.native_info_original_missing_selector_usage(),
                expected_usage
            );
        }
        let mut unknown = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
        unknown.core_point = None;
        unknown.native_family = None;
        assert!(unknown.native_info_original_option_protocol().is_none());
        assert!(
            unknown
                .native_info_original_missing_selector_usage()
                .is_none()
        );
    }

    #[test]
    fn selected_trace_type_bytes_keep_raw_and_encoded_zero_distinct() {
        // Native proof: naming.trace.original-type-wrong-arity-inputs
        // docs/design/analysis/name-resolution-proofs/trace-original-type-wrong-arity-inputs.md
        for version in tcl_dialect::TclVersion::ALL {
            let protocol = crate::InvocationDialect::for_version(version)
                .native_index_lookup_protocol()
                .unwrap();
            for (raw, encoded, canonical) in [
                (
                    b"var\0\xff".as_slice(),
                    b"var\xc0\x80\xff".as_slice(),
                    "variable",
                ),
                (
                    b"com\0\xff".as_slice(),
                    b"com\xc0\x80\xff".as_slice(),
                    "command",
                ),
                (
                    b"exec\0\xff".as_slice(),
                    b"exec\xc0\x80\xff".as_slice(),
                    "execution",
                ),
            ] {
                let selected =
                    tcl_cmd_core::trace::resolve_type_bytes(protocol.selected_input(raw)).unwrap();
                assert_eq!(selected.canonical_name(), canonical);
                assert!(
                    tcl_cmd_core::trace::resolve_type_bytes(protocol.selected_input(encoded))
                        .is_err()
                );
                assert_eq!(protocol.selected_input(encoded), encoded);
            }
        }
        assert!(
            crate::InvocationDialect::of_profile(
                crate::model::ingress::resolve_environment("jim").unit_profile()
            )
            .native_index_lookup_protocol()
            .is_none()
        );
    }
}

impl crate::InvocationDialect {
    /// C8.4 info dispatches its original selector through GetIndexFromObj's
    /// option table. C8.5+ dispatches through the actual namespace ensemble;
    /// Jim has its separately selected command-inventory and helper protocol.
    /// This selection grants no table/header object or lookup result.
    #[must_use]
    pub fn native_info_original_option_protocol(self) -> Option<NativeIndexLookupProtocol> {
        self.native_index_lookup_protocol()
            .filter(|protocol| protocol.version() == TclVersion::V8_4)
    }

    /// Missing-selector usage of the selected original C info dispatcher.
    /// C8.4's option command and C8.5's ensemble retain their own wording;
    /// Jim's independent dispatch protocol owns its own arity rendering.
    #[must_use]
    pub fn native_info_original_missing_selector_usage(self) -> Option<&'static [u8]> {
        if self.native_index_lookup_protocol()?.version() == TclVersion::V8_4 {
            Some(b"option ?arg arg ...?")
        } else {
            Some(
                self.native_ensemble_configuration_protocol()?
                    .missing_selector_usage(),
            )
        }
    }

    /// `GetIndex` is a C native object operation; Jim uses its distinct Enum owner.
    #[must_use]
    pub fn native_index_lookup_protocol(self) -> Option<NativeIndexLookupProtocol> {
        match self.native_string_protocol()? {
            tcl_syntax::native_string::NativeStringProtocol::C(version) => {
                Some(NativeIndexLookupProtocol(version))
            }
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => None,
        }
    }
}
