// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Variable and command name normalisation.
//!
//! These helpers live in the compiler-facing crates because they are
//! consumed by the expression parser and lowering — not by the lexer
//! itself.
//!
//! The `::`-qualifier split ([`qualifier_segments`] / [`is_qualified`]) owns
//! C-style written namespace segmentation. [`NativeNameProtocol`] selects the
//! purpose-specific native input extent before that segmentation. Jim command
//! and namespace objects have separate flat composition rules. Constructed
//! namespace segments are retained directly, without reparsing display text.

mod compacted_source;
pub use compacted_source::{AuthoredCompactName, AuthoredCompactNameSequence};

mod scaffold;
pub use scaffold::{
    GeneratedScaffoldNamespace, PACKAGE_SCAFFOLD_NAMESPACE_CASES,
    PACKAGE_SCAFFOLD_NAMESPACE_POLICY, ScaffoldNamespacePolicy, compiled_unit_procedure_name,
    scaffold_namespace_from_package_name, suggested_data_group_name,
};
mod authored_source;
pub use authored_source::{
    authored_source_command_candidates, authored_source_command_slot,
    authored_source_command_spelling, authored_source_namespace_path,
};
mod substitution_candidates;
pub use substitution_candidates::{
    SubstitutionVariableCandidates, substitution_variable_candidates,
};
mod aliases;
mod execution_policy;
mod vendor_source;
pub use aliases::{
    c_family_local_alias_name_bytes, global_local_name_bytes, variable_local_name_bytes,
};
pub use execution_policy::{
    ExecutionNamePolicy, ExecutionVariableNameProjection, MeasuredBigIpNameScope,
    ObservedBigIpNamePolicy, ObservedVariableNamePurpose,
};
pub use vendor_source::{
    VendorSourceFormalExtent, VendorSourceNameAuthority, VendorSourceNamePolicy,
    VendorSourceNamePurpose, vendor_scalar_source_reference, vendor_source_formal_extents,
    vendor_source_literal_units,
};
mod alias_chain;
pub use alias_chain::alias_chain_loops;
mod import_wrapper_hint;
pub use import_wrapper_hint::{
    AuthoredImportAliasSpelling, AuthoredImportWrapperHint, authored_import_wrapper_hint,
};

mod array_elements;
mod command_resolution;
mod constructed_keys;
mod qualification;
mod variable_quoting;
mod variable_references;
mod written_names;

pub use array_elements::{
    array_variable_completion_prefix, element_var_name, element_var_name_braced,
    element_var_name_braced_for_style, element_var_name_for_style, normalise_var_name_braced,
    normalise_var_name_braced_for_style, split_array_name, split_array_name_braced,
    split_array_name_braced_for_style, split_array_name_for_style, split_element_ref,
    split_element_ref_bytes,
};
pub use command_resolution::{
    CommandLookupBase, bareword_resolution_candidates, command_lookup_bases,
    command_resolution_candidates, command_resolution_candidates_from_namespace_keys,
    resolve_command_with,
};
pub use constructed_keys::{
    canonical_written_command, is_absolutely_addressable, key_holder_and_tail, key_segments,
    key_suffixes, key_tail,
};
pub use qualification::{
    normalise_qualified_name, qualify, qualify_bytes, qualify_namespace, root_unrooted_key,
    root_unrooted_key_bytes, rooted_member_key, rooted_member_key_bytes, scope_variable_key,
    unroot_rooted_key, unroot_rooted_key_bytes, unrooted_member_key, without_leading_colons,
    without_surrounding_colons,
};
pub use variable_quoting::{
    is_bare_var_name, is_brace_substitutable, is_braced_indirect_array_ref,
    var_name_requires_quoting,
};
pub use variable_references::{
    array_key_is_literal, is_static_variable_source_word, is_whole_variable_reference,
    normalise_var_name, normalise_var_name_for_style, split_braced_var_ref, var_reference,
    var_reference_for_style, variable_reference_root_bytes, whole_scalar_variable_reference_name,
    whole_variable_reference_literal_name,
};
pub use written_names::{
    WrittenCommandParts, ends_with_separator, is_absolute, is_absolute_bytes, is_qualified,
    qualifier_component_extents, qualifier_segments, qualifier_segments_owned,
    written_command_parts, written_command_tail, written_separator_extent,
};

/// Literal namespace-prefix tail for conditional authoring hints.
/// Preserves the written remainder; supplies no qualification or cell identity.
#[must_use]
pub fn authored_namespace_hint_tail<'a>(name: &'a str, namespace: &str) -> Option<&'a str> {
    let tail = authored_namespace_hint_tail_bytes(name.as_bytes(), namespace.as_bytes())?;
    // The tail follows an ASCII `::`, so its start is a character boundary.
    Some(&name[name.len() - tail.len()..])
}

/// Counted-byte twin of [`authored_namespace_hint_tail`], for keys that need
/// not be UTF-8: the bytes of `name` after the literal prefix `namespace` and
/// exactly one `::`.
///
/// Both inputs are constructed keys, compared byte for byte. A constructed key
/// joins its segments with exactly `::`, and Jim keeps a namespaced command
/// under the one flat key `ns::name` (`JimQualifyName`, `jim.c`), so the
/// members of a namespace are the keys that extend its key by one `::`. No
/// colon run is collapsed and no root marker is added or removed: the tail of
/// `a:::b` under `a` is `:b`, and `::a::b` is not under `a`. An empty
/// `namespace` selects nothing, because the root owns every key and has no
/// prefix to remove.
///
/// ```
/// use tcl_syntax::naming::authored_namespace_hint_tail_bytes;
/// assert_eq!(authored_namespace_hint_tail_bytes(b"a::b", b"a"), Some(b"b".as_slice()));
/// assert_eq!(authored_namespace_hint_tail_bytes(b"a::b::c", b"a"), Some(b"b::c".as_slice()));
/// assert_eq!(authored_namespace_hint_tail_bytes(b"a::", b"a"), Some(b"".as_slice()));
/// assert_eq!(authored_namespace_hint_tail_bytes(b"ab::c", b"a"), None);
/// assert_eq!(authored_namespace_hint_tail_bytes(b"a", b"a"), None);
/// assert_eq!(authored_namespace_hint_tail_bytes(b"::b", b""), None);
/// ```
#[must_use]
pub fn authored_namespace_hint_tail_bytes<'a>(
    name: &'a [u8],
    namespace: &[u8],
) -> Option<&'a [u8]> {
    if namespace.is_empty() {
        return None;
    }
    name.strip_prefix(namespace)?.strip_prefix(b"::")
}

mod autoload;
pub use autoload::{autoload_command_candidates, native_autoload_command_candidates};
mod authored_namespace_component;
mod command_geometry;
pub use authored_namespace_component::{
    AuthoredNamespaceComponent, AuthoredNamespaceOccurrencePurpose,
    authored_namespace_component_extent,
};
mod reporting;
mod root_source;
pub use root_source::native_command_root_source_spelling;
mod script_name_query;
pub use script_name_query::original_script_name_query_support;
mod script_command_inventory;
pub use script_command_inventory::original_script_command_inventory_support;
mod script_command_presence;
pub use script_command_presence::{
    ScriptRootCommandPresencePlan, original_command_presence_support,
};
mod script_namespace_snapshot;
pub use reporting::{
    ReportedQNameNamespacePurpose, reported_namespace_contains,
    reported_namespace_strictly_contains, reported_qname_namespace,
};
pub use script_namespace_snapshot::original_namespace_variable_snapshot_support;
mod compiled_variables;
mod variable_geometry;
pub use variable_geometry::{
    NativeVariableBridgeUnavailable, NativeVariableNamespaceProjection, NativeVariableRootGeometry,
    VariableLookupBase, variable_namespace_lookup_bases,
};
mod variable_spelling;
pub use variable_spelling::{
    NativeScalarSourceSpelling, array_source_reference, native_scalar_source_spelling,
    scalar_source_reference, scalar_source_reference_followed_by,
    variable_source_reference_with_tail,
};
mod jim_enumeration;
mod jim_table_key;
pub use jim_table_key::NativeJimCommandTableKey;
mod tcloo_link;
pub use tcloo_link::{
    NativeOoLinkAliasGeometry, native_oo_link_alias_geometry, native_oo_link_call_geometry,
};
mod oo_variables;
pub use compiled_variables::{
    NativeCompiledScalarName, NativeCompiledVariableAuthority, NativeCompiledVariableEnvironment,
    NativeCompiledVariableLookup, NativeCompiledVariableProtocol, NativeCompiledVariableRecipe,
};
pub use oo_variables::{
    NativeOoPrivateVariablePurpose, NativeOoVariableError, NativeOoVariableResolverPurpose,
    NativeOoVariableSlotOperation, NativeOoVariableSlotSelection, NativeOoVarnameLookup,
    apply_native_oo_slot_records, apply_native_oo_variable_slot,
    native_oo_explicit_variable_local_name, native_oo_private_variable_matches,
    native_oo_variable_key_report, native_oo_variable_resolver_matches, native_oo_variable_slot,
    native_oo_varname_lookup, native_oo_varname_lookup_bytes, validate_native_oo_variable,
};
mod namespace_spans;
pub use namespace_spans::{
    NativeNamespaceOperandRename, NativeNamespaceRenameTarget,
    native_command_lookup_context_for_slot, native_command_slot_is_under_namespace,
    native_namespace_literal_prefix_may_select, native_namespace_operand_rename,
    native_namespace_rename_target, native_namespace_source_word,
    native_variable_root_is_under_namespace, native_written_namespace_member_extent,
    native_written_namespace_prefix_extent,
};
mod hidden_invocation;
pub use hidden_invocation::{NativeHiddenInvocationGrammar, NativeHiddenInvocationOption};
mod native;
pub use native::{
    AuthoredNamespaceImportControl, NamePolicyAuthority, NamePolicyProtocol,
    NameProjectionUnavailable, NativeCommandNamespaceRoute, NativeCommandSlotProjection,
    NativeCommandVisibilityDiagnostic, NativeCommandVisibilityFailure,
    NativeDictionaryMissingKeyError, NativeDictionaryMissingKeyOperation,
    NativeJimNamespaceCommandQueryCandidate, NativeJimNamespaceConstruction,
    NativeJimNamespaceParentResult, NativeNameContext, NativeNameProjection, NativeNameProtocol,
    NativeNamePurpose, NativeNameQualification, NativeNameReportPurpose,
    NativeNamespaceLookupError, NativeNamespaceLookupOperation, NativeNamespaceOperationError,
    NativeNamespacePatternParts, NativeNamespacePatternSource, NativeNamespaceReportParts,
    NativeNamespaceTextResult, NativeRenameDiagnostic, NativeRenameFailure,
    NativeVariableDiagnosticOperation, NativeVariableDiagnosticProjection,
    NativeVariableDiagnosticReason, NativeVariableFailureSite, NativeVariableInputForm,
    NativeVariableProjection, NativeVariableTraceNames, NativeVariableTraceReportingInput,
    checked_command_slot_utf8, checked_namespace_path_utf8, constructed_command_address_bytes,
    native_command_full_name_bytes, native_command_source_spelling, native_command_source_word,
    native_constant_failure_verb, native_jim_namespace_source_spelling,
    native_namespace_full_name_bytes, native_namespace_key_bytes, native_namespace_source_spelling,
    native_procedure_compilation_name_input, native_qualified_inventory_report_bytes,
    report_native_c_variable_value_name, report_native_dictionary_missing_key,
    report_native_name_bytes, report_native_namespace_creation_conflict,
    report_native_namespace_creation_error, report_native_namespace_lookup_error,
    report_native_namespace_operation_error, report_native_variable_access_trace_names,
    report_native_variable_diagnostic, report_native_variable_diagnostic_at,
};

/// Rooted slot for an authored support-command registration label.
/// Empty tails select the last nonempty written component. Counted opaque bytes
/// remain data; this supplies no native publication, lookup or existence proof.
#[must_use]
pub fn support_command_registration_slot(
    written: &[u8],
) -> Option<tcl_core_types::ByteCommandSlot> {
    let mut parts = written_command_parts(written);
    let simple = if parts.simple.is_empty() {
        parts.qualifiers.pop()?
    } else {
        parts.simple
    };
    Some(tcl_core_types::ByteCommandSlot::new(
        tcl_core_types::ByteNamespacePath::from_segments(parts.qualifiers),
        tcl_core_types::NameBytes::from(simple),
    ))
}

/// Checked compatibility key for the exact native namespace components.
/// Opaque UTF-8 or an ambiguous constructed-key round trip declines; the
/// original byte path remains authoritative. This grants no native lookup.
#[must_use]
pub fn checked_namespace_path_key(path: &tcl_core_types::ByteNamespacePath) -> Option<String> {
    let segments = checked_namespace_path_utf8(path).ok()?;
    let key = root_unrooted_key(&segments.join("::"));
    (key_segments(&key) == segments).then_some(key)
}

/// Jim's flat global-variable key from a rooted constructed namespace.
///
/// Absolute written names beginning with `::` discard every leading colon.
/// Relative names prepend the namespace after removing its one root marker;
/// literal colon segments inside that constructed namespace stay intact.
/// Names and namespace segments retain their exact bytes, including NUL.
#[must_use]
pub fn jim_global_variable_key_bytes(rooted_namespace: &[u8], name: &[u8]) -> Vec<u8> {
    if name.starts_with(b"::") {
        let start = name
            .iter()
            .position(|byte| *byte != b':')
            .unwrap_or(name.len());
        return name[start..].to_vec();
    }
    let prefix = rooted_namespace
        .strip_prefix(b"::")
        .unwrap_or(rooted_namespace);
    let mut key = Vec::with_capacity(prefix.len() + name.len() + 2);
    key.extend_from_slice(prefix);
    if !prefix.is_empty() {
        key.extend_from_slice(b"::");
    }
    key.extend_from_slice(name);
    key
}

/// Unicode projection of [`jim_global_variable_key_bytes`].
/// The namespace is a rooted constructed key, not a written name to parse.
#[must_use]
pub fn jim_global_variable_key(rooted_namespace: &str, name: &str) -> String {
    String::from_utf8(jim_global_variable_key_bytes(
        rooted_namespace.as_bytes(),
        name.as_bytes(),
    ))
    .expect("joining Unicode namespace and name bytes preserves Unicode")
}

/// Conservative dynamic-target hazard over an unattributed name presentation.
/// The caller supplies the brace marker. This does not establish a decoded
/// value, original word, static definition, variable lookup or cell identity.
#[must_use]
pub fn unattributed_dynamic_written_variable_target(name: &str, braced_literal: bool) -> bool {
    if braced_literal {
        return false;
    }
    if name.starts_with('$') {
        return true;
    }
    let base = match name.find('(') {
        Some(index) => &name[..index],
        None => name,
    };
    base.is_empty() || base.contains('$') || base.contains('[')
}

/// A bounded alias hint from unattributed reference and value presentations.
/// This does not prove a read value, selected frame, cell or absence of other
/// targets. Consumers must retain unresolved dynamic-name alternatives.
#[must_use]
pub fn unattributed_variable_alias_hint<'a>(
    token: &str,
    config: tcl_lexer::LexerConfig,
    lookup: impl FnOnce(&str) -> Option<&'a str>,
) -> Option<&'a str> {
    let reference = tcl_lexer::word_parts::whole_var_ref(token.as_bytes(), config).ok()??;
    if reference.index.is_some() || split_element_ref_bytes(reference.name).is_some() {
        return None;
    }
    let reference = core::str::from_utf8(reference.name).ok()?;
    let value = lookup(reference)?;
    (!value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':'))
    .then_some(value)
}

/// Whether a source variable name belongs to an ordinary activation's table.
/// C qualifiers select namespace cells; Jim only roots a name with initial `::`.
/// The empty name remains local. The caller owns the policy and input extent;
/// this supplies no Native frame, cell, lookup or variable-presence receipt.
#[must_use]
pub fn source_variable_name_is_frame_local(
    name: &[u8],
    policy: tcl_dialect::VariableLookupPolicy,
) -> bool {
    match policy {
        tcl_dialect::VariableLookupPolicy::Tcl => !is_qualified(name),
        tcl_dialect::VariableLookupPolicy::Jim => !name.starts_with(b"::"),
    }
}

/// Source-local geometry shared by every supported variable lookup policy.
/// This does not select a runtime policy, activation, cell or presence fact.
#[must_use]
pub fn source_variable_name_is_frame_local_for_all_policies(name: &[u8]) -> bool {
    source_variable_name_is_frame_local(name, tcl_dialect::VariableLookupPolicy::Tcl)
        && source_variable_name_is_frame_local(name, tcl_dialect::VariableLookupPolicy::Jim)
}

/// The TextMate-compatible variable-name body used by the generated editor
/// grammars. A namespace separator is a run of *two or more* colons, matching
/// [`qualifier_segments`] exactly; keeping the rendered fragment beside the
/// segmentation owner prevents editor regexes from quietly reverting to an
/// exactly-two-colon interpretation.
///
/// `TextMate` grammars intentionally retain the conservative ASCII identifier
/// surface used by their historical variable rule. Tcl itself permits broader
/// braced names, which are handled by the separate `${…}` grammar rule.
#[must_use]
pub fn textmate_variable_name_body() -> &'static str {
    r"(?:[:]{2,})?(?:[A-Za-z_][A-Za-z0-9_]*[:]{2,})*[A-Za-z_][A-Za-z0-9_]*"
}

/// Static display token for an unquoted source definition name. Excluding
/// delimiters prevents a keyword used as data from consuming a quote/body.
/// This is a conservative display subset, not Tcl name acceptance or lookup.
#[must_use]
pub fn textmate_unquoted_entity_name() -> &'static str {
    r#"[^\s"{}\[\];$]+"#
}

/// Static `TextMate` braced-variable display fragment, including the empty name.
/// This preserves the historical first-close projection. It is exact for
/// [`tcl_dialect::BracedVarStyle::FirstClose`]; nested Tcl 9 names and escaped
/// closers need semantic tokens for exact extents. No lookup or cell follows.
#[must_use]
pub fn textmate_braced_variable_reference() -> &'static str {
    r"\$\{[^}]*\}"
}

/// True when `word` carries a variable / command substitution anywhere —
/// e.g. `rename ::$c mystr` or `rename foo bar[x]` — so it cannot be
/// resolved to a static command name at compile time.
///
/// ```
/// use tcl_syntax::naming::is_dynamic_word;
/// assert!(!is_dynamic_word("foo"));
/// assert!(is_dynamic_word("::$c"));
/// assert!(is_dynamic_word("bar[x]"));
/// ```
#[must_use]
pub fn is_dynamic_word(word: &str) -> bool {
    word.contains('$') || word.contains('[')
}

/// True when a word whose **token kind** is known carries a substitution.
///
/// A brace-quoted word never substitutes, so its content is literal by
/// construction however many `$` or `[` characters it holds: in
/// `namespace path {$ns ::a}` the first entry is a namespace literally named
/// `$ns`.  [`is_dynamic_word`] alone re-scans the *reconstructed* content —
/// braces already stripped — and so mistakes such a word for a dynamic one,
/// making the whole command abstain.
///
/// Only braces suppress substitution: a `"…"`-quoted word does substitute, so
/// it is scanned like a bare word.
///
/// ```
/// use tcl_syntax::naming::word_is_dynamic;
/// assert!(word_is_dynamic("$ns ::a", false));
/// assert!(!word_is_dynamic("$ns ::a", true));
/// ```
#[must_use]
pub fn word_is_dynamic(word: &str, braced_literal: bool) -> bool {
    !braced_literal && is_dynamic_word(word)
}

/// The offset-keyed synthetic identity markers the analyser mints for
/// constructs whose real name is a run-time fact: a dynamic
/// `namespace eval $ns { … }` scope (`@dynns@<offset>`), a dynamic
/// `oo::define $cls …` target (`@dynclass@<offset>`), and a path-less
/// `interp create` (`@autoname@<offset>`).  The `<offset>` is the minting
/// token's **absolute byte offset**, so two lexically unrelated occurrences
/// never collide and the identity is deterministic for a given source text.
///
/// Kept in one place so [`rebase_synthetic_offset_names`] and every minting
/// site agree on the exact marker spellings.
pub const SYNTHETIC_OFFSET_MARKERS: [&str; 3] = ["@dynns@", "@dynclass@", "@autoname@"];

/// Shift the `<offset>` of every embedded `@dynns@N` / `@dynclass@N` /
/// `@autoname@N` marker in `s` by `delta`, but only for tokens `is_minted`
/// recognises — the exact names one isolated analysis pass actually minted.
///
/// This is the string half of the per-item span rebase: an isolated
/// proc-body analysis mints these names from **body-relative** offsets (so
/// the memoised fragment stays offset-invariant), and the graft rebases them
/// to the absolute offsets a whole-file walk would have minted.  The
/// `is_minted` gate keeps a *literal* source name that happens to be spelled
/// like a marker (`proc @dynns@5 {} {}` is legal Tcl) untouched unless it
/// textually collides with a genuinely minted name — the same ambiguity the
/// minting scheme itself already accepts.
///
/// Returns `None` when nothing was rewritten (the overwhelming common case),
/// so callers can skip the reallocation.
///
/// ```
/// use tcl_syntax::naming::rebase_synthetic_offset_names;
/// let minted = |t: &str| t == "@dynns@55";
/// assert_eq!(
///     rebase_synthetic_offset_names("::hook::@dynns@55::x", 5340, minted).as_deref(),
///     Some("::hook::@dynns@5395::x"),
/// );
/// // `@dynns@550` is a different token — the digit run is matched whole.
/// assert_eq!(rebase_synthetic_offset_names("::@dynns@550::x", 5340, minted), None);
/// ```
#[must_use]
pub fn rebase_synthetic_offset_names<F: Fn(&str) -> bool>(
    s: &str,
    delta: u32,
    is_minted: F,
) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out: Option<String> = None;
    // Bytes of `s` already copied into `out` (only meaningful once `out` is).
    let mut copied = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }
        let Some(marker) = SYNTHETIC_OFFSET_MARKERS
            .iter()
            .find(|m| s[i..].starts_with(**m))
        else {
            i += 1;
            continue;
        };
        let digits_start = i + marker.len();
        let digits_len = bytes[digits_start..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let digits_end = digits_start + digits_len;
        // The whole digit run is the offset, so `@dynns@5` can never be
        // misread out of `@dynns@52`.
        if let Ok(n) = s[digits_start..digits_end].parse::<u32>()
            && is_minted(&s[i..digits_end])
        {
            let dst = out.get_or_insert_with(|| String::with_capacity(s.len() + 8));
            dst.push_str(&s[copied..i]);
            dst.push_str(marker);
            dst.push_str(&n.saturating_add(delta).to_string());
            copied = digits_end;
        }
        i = digits_end.max(i + 1);
    }
    let mut dst = out?;
    dst.push_str(&s[copied..]);
    Some(dst)
}

/// Shared command-resolution conformance vectors.
///
/// One table of `(namespace, namespace path, defined commands, call,
/// expected winner)` rows — the executable ground truth for
/// [`resolve_command_with`](super::naming::resolve_command_with) and for
/// every backend that re-implements the rule over its own command store
/// (the analyser's post-walk settlement, the bytecode VM's dispatch, the
/// WASM runtime's namespace tree).  Each consumer runs the *same* rows, so
/// a change to the rule that lands in one implementation but not another
/// fails that consumer's conformance test rather than drifting silently.
///
/// The rows themselves are pinned against real tclsh by
/// `tests/command_resolution_conformance.rs` in this crate, which renders
/// each row with [`conformance::vector_script`] and diffs the interpreter's
/// answer against `want`.
pub mod conformance {
    use crate::release_expectations::{PerRelease, newest_release};
    use tcl_dialect::TclVersion;

    /// One resolution scenario: from `ns` (with `path` as its
    /// `namespace path`), a call to `call` with exactly `defs` defined
    /// must dispatch `want` (`None` = `invalid command name`).
    #[derive(Debug, Clone)]
    pub struct ResolutionVector {
        /// `::`-rooted current namespace (`::` = global).
        pub ns: String,
        /// `namespace path` entries for `ns`, in order (each `::`-rooted).
        pub path: Vec<String>,
        /// Every defined command, `::`-rooted.
        pub defs: Vec<String>,
        /// The call text as written (bare, relative-qualified, or absolute).
        pub call: String,
        /// The winner per modelled release — a single value when every
        /// release agrees, release-tagged columns when they do not (see
        /// [`crate::release_expectations`]).
        pub wants: PerRelease,
        /// 1-based line in the vector file (for failure messages).
        pub line: usize,
    }

    impl ResolutionVector {
        /// The `::`-rooted winner on `release`, or `None` for
        /// `invalid command name`.
        #[must_use]
        pub fn want_for(&self, release: TclVersion) -> Option<String> {
            let column = self.wants.get(release);
            (column != "-").then(|| column.to_owned())
        }

        /// The `::`-rooted winner on the newest modelled release — the
        /// column the release-agnostic pure resolver asserts against.
        #[must_use]
        pub fn want(&self) -> Option<String> {
            self.want_for(newest_release())
        }
    }

    /// The raw vector table (see the file header for the format).
    pub const RAW: &str = include_str!("../tests/data/command_resolution_vectors.txt");

    /// Parse [`RAW`] into vectors.
    ///
    /// # Panics
    /// On a malformed row — the file is repo-controlled test data, so a
    /// parse failure is a bug in the row, not an input condition.
    #[must_use]
    pub fn vectors() -> Vec<ResolutionVector> {
        let split_list = |field: &str| -> Vec<String> {
            if field == "-" {
                Vec::new()
            } else {
                field.split(',').map(|s| s.trim().to_string()).collect()
            }
        };
        let mut out = Vec::new();
        for (idx, raw_line) in RAW.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            assert!(
                fields.len() == 5,
                "vector line {}: expected 5 |-separated fields, got {}: {raw_line:?}",
                idx + 1,
                fields.len(),
            );
            out.push(ResolutionVector {
                ns: fields[0].to_string(),
                path: split_list(fields[1]),
                defs: split_list(fields[2]),
                call: fields[3].to_string(),
                wants: PerRelease::parse(fields[4]).unwrap_or_else(|why| {
                    panic!("vector line {}: {why}", idx + 1);
                }),
                line: idx + 1,
            });
        }
        assert!(!out.is_empty(), "no conformance vectors parsed");
        out
    }

    /// `run(ns_key, body)`: a command string that evaluates `body` inside
    /// `ns_key`, built by nesting `namespace eval {tail} {…}` from the root
    /// down.
    ///
    /// Namespace / definition fields are **constructed keys** (see the vector
    /// -file header): a key may hold a colon-named segment (`":::"` is the
    /// namespace — or proc — named `:`; `"::"` names the empty-string `{}`
    /// proc), which has no absolute written spelling.  Everything
    /// is therefore created *relatively*, descending the key's holder chain one
    /// `namespace eval` at a time with brace-quoted tails.
    fn run(ns_key: &str, body: &str) -> String {
        if ns_key.is_empty() || ns_key == "::" {
            return body.to_owned();
        }
        let (holder, tail) = crate::naming::key_holder_and_tail(ns_key);
        run(holder, &format!("namespace eval {{{tail}}} {{ {body} }}"))
    }

    /// The **setup** half of a vector's script: create every namespace it
    /// mentions, define each command as a proc returning its own qualified
    /// name, and apply the `namespace path`.
    ///
    /// Split out from [`vector_script`] so a backend can pair it with its own
    /// result capture.  The runtime port needs exactly that: its `if` is gated
    /// on the bignum tower being linked, so the `if`-based capture
    /// [`vector_script`] uses would make the whole conformance gate unrunnable
    /// in a tower-less build, surfacing as `invalid command name "if"`.
    /// Every command emitted here (`namespace`, `proc`,
    /// `return`) is tower-free, so a backend that can only manage
    /// `set`/`catch` can still run all the vectors.
    #[must_use]
    pub fn vector_setup(v: &ResolutionVector) -> String {
        use std::fmt::Write as _;
        let mut script = String::new();
        // Every namespace referenced anywhere must exist up front: the call
        // namespace, each path entry, and each definition's holder.
        let ensure_ns = |ns: &str, script: &mut String| {
            if ns != "::" && !ns.is_empty() {
                let _ = writeln!(script, "{}", run(ns, ""));
            }
        };
        ensure_ns(&v.ns, &mut script);
        for p in &v.path {
            ensure_ns(p, &mut script);
        }
        for def in &v.defs {
            let (holder, tail) = crate::naming::key_holder_and_tail(def);
            ensure_ns(holder, &mut script);
            let _ = writeln!(
                script,
                "{}",
                run(
                    holder,
                    &format!("proc {{{tail}}} {{}} {{ return {{{def}}} }}")
                ),
            );
        }
        if !v.path.is_empty() {
            let entries = v.path.join(" ");
            let _ = writeln!(
                script,
                "{}",
                run(&v.ns, &format!("namespace path [list {entries}]")),
            );
        }
        script
    }

    /// The **call** half of a vector's script: the call text as written,
    /// evaluated in the vector's current namespace.  Pair with
    /// [`vector_setup`].
    #[must_use]
    pub fn vector_call(v: &ResolutionVector) -> String {
        run(&v.ns, &v.call)
    }

    /// Render a vector as a runnable Tcl script whose **output** is the
    /// dispatched command's qualified name, or `-` on
    /// `invalid command name`.
    ///
    /// [`vector_setup`] plus an `if`/`catch` capture and a `puts`.  Shared by
    /// the tclsh pin test and every backend that has a full command set, so all
    /// of them execute byte-identical scripts.  A backend without `if` composes
    /// [`vector_setup`] and [`vector_call`] itself instead.
    #[must_use]
    pub fn vector_script(v: &ResolutionVector) -> String {
        use std::fmt::Write as _;
        let mut script = vector_setup(v);
        let _ = writeln!(
            script,
            "if {{[catch {{{}}} __r]}} {{ set __r - }}",
            vector_call(v),
        );
        script.push_str("puts $__r\n");
        script
    }
}

/// Possible rooted names in unattributed text, for conservative edit refusal.
/// Delimiter and sigil scanning is deliberately broader than executable Tcl;
/// these fragments grant no name lookup, variable access or editable span.
pub fn unattributed_rooted_name_candidates(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| character.is_whitespace() || "{}[];\"".contains(character))
        .map(|token| token.trim_start_matches(['$', '{']).trim_end_matches('}'))
        .filter(|token| token.starts_with("::"))
        .map(|token| token.split_once('(').map_or(token, |(root, _)| root))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod shared_boundary_tests;

#[cfg(test)]
mod support_registration_tests;
