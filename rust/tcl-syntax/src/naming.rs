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

/// Does `name` contain a `::` namespace separator (i.e. is it qualified)?
#[must_use]
pub fn is_qualified(name: &[u8]) -> bool {
    tcl_core_types::naming::has_namespace_separator(name)
}

/// Is the **written** `name` absolute, that is rooted at the global namespace?
///
/// `TclGetNamespaceForQualName` (`tclNamesp.c`) starts its component walk at
/// the global namespace exactly when the name begins with two colons, and then
/// skips every further colon of that run. A leading run of two or more colons
/// is therefore the root marker (`::a`, `:::a`, and `::` alone, which names
/// the global namespace or its `{}` entity), while a single leading `:` is the
/// first character of a simple name, so `:a` is relative. A separator later in
/// the name qualifies it without rooting it: `a::b` is relative.
///
/// This reads a written name. A constructed key carries its own rooting
/// convention ([`root_unrooted_key`], [`unroot_rooted_key`]), and Jim's flat
/// names theirs ([`jim_global_variable_key_bytes`]).
///
/// ```
/// use tcl_syntax::naming::is_absolute;
/// assert!(is_absolute("::a"));
/// assert!(is_absolute(":::a"));
/// assert!(is_absolute("::"));
/// assert!(!is_absolute(":a"));
/// assert!(!is_absolute("a::b"));
/// assert!(!is_absolute(""));
/// ```
#[must_use]
pub fn is_absolute(name: &str) -> bool {
    is_absolute_bytes(name.as_bytes())
}

/// Counted-byte twin of [`is_absolute`], for names that need not be UTF-8.
#[must_use]
pub fn is_absolute_bytes(name: &[u8]) -> bool {
    name.starts_with(b"::")
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

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
/// A run of **two or more** colons is one separator (all consecutive colons are
/// consumed), while a lone interior colon is an ordinary name character, so
/// `a:::b` → `[a, b]` and `a:b` → `[a:b]`. A trailing separator drops its empty
/// tail (`a::b::` → `[a, b]`); callers that care about the `{}`-named cmd/var a
/// trailing `::` denotes test for it themselves. Mirrors
/// `TclGetNamespaceForQualName`'s component walk (`tclNamesp.c`).
#[must_use]
pub fn qualifier_segments(name: &[u8]) -> Vec<&[u8]> {
    qualifier_component_extents(name)
        .map(|extent| &name[extent])
        .collect()
}

/// Exact original byte extents of the nonempty components of a written name.
/// The same owner supplies [`qualifier_segments`]: a complete run of at least
/// two colons is one separator; a lone colon remains a component character.
/// Leading and trailing separators produce no empty component. Consumers that
/// distinguish a namespace operand from an empty terminal command or variable
/// must retain that independently selected purpose.
///
/// Extents retain original Unicode/NUL bytes and all separator widths. This
/// iterator does not select a native input prefix, invert a constructed key,
/// establish a namespace or grant an editable source container.
///
/// ```
/// use tcl_syntax::naming::qualifier_component_extents;
/// assert_eq!(qualifier_component_extents(b"::::a:::b::").collect::<Vec<_>>(),
///            vec![4..5, 8..9]);
/// assert_eq!(qualifier_component_extents(b"a:b").collect::<Vec<_>>(),
///            vec![0..3]);
/// ```
pub fn qualifier_component_extents(
    name: &[u8],
) -> impl Iterator<Item = std::ops::Range<usize>> + '_ {
    let mut next = 0;
    std::iter::from_fn(move || {
        while next < name.len() {
            let start = next;
            let mut at = start;
            while at < name.len() {
                if name[at] == b':' && name.get(at + 1) == Some(&b':') {
                    let end = at;
                    at += 2;
                    while name.get(at) == Some(&b':') {
                        at += 1;
                    }
                    next = at;
                    if start < end {
                        return Some(start..end);
                    }
                    break;
                }
                at += 1;
            }
            if at == name.len() && next == start {
                next = at;
                return Some(start..at);
            }
        }
        None
    })
}

/// The exact byte extent of the final written namespace separator run.
/// Each complete run of two or more colons separates components; a lone colon
/// remains component data. The counted input is not truncated or normalised.
/// This pure span query supplies no dialect, native input extent, namespace,
/// source edit, command lookup or publication authority.
#[must_use]
pub fn written_separator_extent(name: &[u8]) -> Option<std::ops::Range<usize>> {
    let mut previous_end = 0;
    let mut separator = None;
    for component in qualifier_component_extents(name) {
        if component.start > previous_end {
            separator = Some(previous_end..component.start);
        }
        previous_end = component.end;
    }
    if previous_end < name.len() {
        separator = Some(previous_end..name.len());
    }
    separator
}

/// Does `name` end with a namespace separator (a run of ≥2 colons)? In a
/// command or variable name such a trailing `::` names the `{}` (empty) entity
/// in the qualified namespace (`TclGetNamespaceForQualName`), so the simple name
/// is `""` rather than the last [`qualifier_segments`] element.
#[must_use]
pub fn ends_with_separator(name: &[u8]) -> bool {
    name.len() >= 2 && name[name.len() - 1] == b':' && name[name.len() - 2] == b':'
}

/// The simple (tail) part of a **written** command name, for the
/// *definition* direction: empty when the name is empty or ends in a
/// separator run — such a spelling addresses the empty-string `{}` command
/// inside its full qualifier chain (`proc x:: {} {}` defines `::x::`;
/// `rename foo x::` binds `::x::`; `rename bar ::` the global `{}` command
/// — all tclsh 8.6/9.0-pinned) — otherwise the last colon-run
/// segment, or the whole name when unqualified.  The resolution-direction
/// counterparts apply the same rule (`Namespaces::home_of` in the runtime,
/// native command-name protocols), so definition and dispatch can never
/// disagree about which command a trailing-separator spelling names.
#[must_use]
pub fn written_command_tail(name: &[u8]) -> &[u8] {
    written_separator_extent(name).map_or(name, |separator| &name[separator.end..])
}

/// Borrowed components of a written command name, including an empty tail.
/// These bytes select no native purpose, namespace token or command identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenCommandParts<'a> {
    /// Whether the written name starts with a root marker.
    pub absolute: bool,
    /// Nonempty namespace components before the simple name.
    pub qualifiers: Vec<&'a [u8]>,
    /// Exact simple name, empty after a trailing separator run.
    pub simple: &'a [u8],
}

/// Decompose counted written bytes without selecting a native input extent.
#[must_use]
pub fn written_command_parts(name: &[u8]) -> WrittenCommandParts<'_> {
    let mut qualifiers = qualifier_segments(name);
    if !ends_with_separator(name) {
        qualifiers.pop();
    }
    WrittenCommandParts {
        absolute: is_absolute_bytes(name),
        qualifiers,
        simple: written_command_tail(name),
    }
}

/// [`qualifier_segments`] over a `&str`, as owned `String`s — the one shared
/// segment split for consumers that hold qualified names as strings (the
/// compiler's interprocedural/optimiser namespace walks). Same colon-run rule:
/// `::a:::b::` → `["a", "b"]`.
///
/// ```
/// use tcl_syntax::naming::qualifier_segments_owned;
/// assert_eq!(qualifier_segments_owned("::a::b::cmd"), vec!["a", "b", "cmd"]);
/// assert_eq!(qualifier_segments_owned("a:::b"), vec!["a", "b"]);
/// assert!(qualifier_segments_owned("::").is_empty());
/// ```
#[must_use]
pub fn qualifier_segments_owned(name: &str) -> Vec<String> {
    qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| {
            core::str::from_utf8(s)
                .expect("subslice of valid UTF-8")
                .to_owned()
        })
        .collect()
}

/// Canonicalise a **written** command / variable word into the constructed-key
/// convention: a colon run of ≥2 is one separator (the whole run
/// collapses to `::`), a lone `:` is an ordinary name character, and a
/// *trailing* separator survives as `::` — it names the empty-string (`{}`)
/// command/variable in the qualified namespace, which is a real, addressable
/// entity (`proc x:: {} {}` defines it; `x::` / `x:::` / `::x::` all call it —
/// tclsh 8.6/9.0-verified, `TclGetNamespaceForQualName`, invariant 8.4→9.1).
///
/// An absolute input yields a rooted key (`::a:::b` → `::a::b`, `:::` → `::` —
/// the global `{}` command); a relative input yields a canonical *relative*
/// suffix (`a:::b` → `a::b`, `x::` → `x::`, `:` → `:`) for the caller to join
/// under a namespace key with one exact `::`.
///
/// This is for **written words only** — never re-apply it to a constructed
/// key: a key holding an all-colon *segment* (a namespace or command
/// legitimately named `:`) would collapse into its parent, which is exactly
/// how a `proc :` would collide with the empty-named `::` key.
///
/// ```
/// use tcl_syntax::naming::canonical_written_command;
/// assert_eq!(canonical_written_command("::a:::b"), "::a::b");
/// assert_eq!(canonical_written_command(":::"), "::");
/// assert_eq!(canonical_written_command("::x::"), "::x::");
/// assert_eq!(canonical_written_command("x:::"), "x::");
/// assert_eq!(canonical_written_command(":"), ":");
/// assert_eq!(canonical_written_command("a:b"), "a:b");
/// assert_eq!(canonical_written_command("cmd"), "cmd");
/// ```
#[must_use]
pub fn canonical_written_command(name: &str) -> String {
    let segs: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    let trailing = if ends_with_separator(name.as_bytes()) {
        "::"
    } else {
        ""
    };
    if is_absolute(name) {
        if segs.is_empty() {
            // Two-or-more colons alone: the global namespace's `{}` command.
            return "::".to_owned();
        }
        return format!("::{}{trailing}", segs.join("::"));
    }
    if segs.is_empty() {
        // A relative word cannot begin with a separator, so an empty segment
        // list means the word itself is empty.
        return String::new();
    }
    format!("{}{trailing}", segs.join("::"))
}

/// The simple tail selected by the authored constructed-key convention.
/// The input is an analytical presentation, independent of a written native
/// name or an actual namespace object's retained component path.
///
/// C's `namespace tail` consumes a whole colon run as one written separator.
/// This authored convention preserves a lone-colon simple name in examples
/// such as `":::"` and an empty simple name after `"::x::"`.
///
/// Joining arbitrary native components is not injective: `["a:", "b"]` and
/// `["a", ":b"]` both render as `"::a:::b"`. This helper cannot recover their
/// original holders or grant a native lookup identity. Native consumers retain
/// the exact component path and namespace incarnation independently.
///
/// ```
/// use tcl_syntax::naming::key_tail;
/// assert_eq!(key_tail("::a::b"), "b");
/// assert_eq!(key_tail(":::"), ":");            // "::" + ":"
/// assert_eq!(key_tail("::::::"), ":");         // ns `:` + proc `:`
/// assert_eq!(key_tail("::a:::"), ":");         // ns `a` + proc `:`
/// assert_eq!(key_tail("::x::"), "");           // the empty-named command
/// assert_eq!(key_tail("::"), "");              // global `{}` command key
/// assert_eq!(key_tail(":"), ":");              // bare simple name
/// assert_eq!(key_tail("a:b"), "a:b");
/// assert_eq!(key_tail("cmd"), "cmd");
/// ```
#[must_use]
pub fn key_tail(name: &str) -> &str {
    let bytes = name.as_bytes();
    if bytes.len() < 2 {
        return name;
    }
    if bytes.iter().all(|&b| b == b':') {
        // All-colon key.  The construction grammar — `"::"` root (2), plus 3
        // per `:`-named namespace level (`"::" + ":"`), plus a final `":"`
        // simple (1) or `""` simple (0) — makes length ≡ 0 (mod 3) the `:`
        // simple name and ≡ 2 (mod 3) the empty one; a lone `:` (length 1) is
        // the bare simple name itself, and other lengths are unconstructible.
        return match bytes.len() {
            n if n % 3 == 0 => &name[n - 1..],
            _ => "",
        };
    }
    // A constructed key's separator is a `::` whose suffix holds no further
    // `::` run and whose prefix is itself a valid namespace key.  Testing only
    // whether the prefix appears to end in a separator is insufficient:
    // `::a:::` is the valid key for namespace `:` below `::a`, despite its
    // final two bytes. Validate the construction grammar recursively (without
    // calling `key_tail`) so any depth of literal-colon segments is retained.
    let rooted = name.starts_with("::");
    let prefix_ok = |i: usize| {
        if rooted {
            i == 0 || (i > 2 && constructed_namespace_path_is_valid(&bytes[2..i]))
        } else {
            i > 0 && constructed_namespace_path_is_valid(&bytes[..i])
        }
    };
    let mut saw_empty_suffix = false;
    let mut i = bytes.len().saturating_sub(2);
    loop {
        if bytes[i] == b':' && bytes[i + 1] == b':' && prefix_ok(i) {
            let suffix = &name[i + 2..];
            if !suffix.contains("::") {
                if !suffix.is_empty() {
                    return suffix;
                }
                saw_empty_suffix = true;
            }
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    if saw_empty_suffix { "" } else { name }
}

/// Whether `path` is a non-empty sequence of constructed namespace segments.
/// Segments themselves contain no `::` run and are joined by exactly `::`;
/// they may contain a lone colon, including the whole segment `:`.  Colon runs
/// therefore have more than one possible split, so recognise the grammar with
/// a small position work-list instead of parsing a written Tcl name.
fn constructed_namespace_path_is_valid(path: &[u8]) -> bool {
    if path.is_empty() {
        return false;
    }
    let mut starts = vec![false; path.len() + 1];
    starts[0] = true;
    for start in 0..path.len() {
        if !starts[start] {
            continue;
        }
        for end in start + 1..=path.len() {
            if end >= start + 2 && path[end - 2] == b':' && path[end - 1] == b':' {
                break;
            }
            if end == path.len() {
                return true;
            }
            if end + 1 < path.len() && path[end] == b':' && path[end + 1] == b':' {
                starts[end + 2] = true;
            }
        }
    }
    false
}

/// Select an analytical holder and tail from a constructed display key.
/// This projection is not an inverse of native path construction: `["a:", "b"]`
/// and `["a", ":b"]` both render as `"::a:::b"`. Native lookup retains its
/// original component path and namespace incarnation instead. The holder of a root-level key is `"::"`
/// (`key_holder_and_tail("::x")` → `("::", "x")`), and the holder chain of a
/// colon-named nesting is preserved (`"::::::"` — proc `:` in the namespace
/// named `:` — → `(":::", ":")`).
///
/// ```
/// use tcl_syntax::naming::key_holder_and_tail;
/// assert_eq!(key_holder_and_tail("::a::b"), ("::a", "b"));
/// assert_eq!(key_holder_and_tail("::x"), ("::", "x"));
/// assert_eq!(key_holder_and_tail(":::"), ("::", ":"));
/// assert_eq!(key_holder_and_tail(":::::p"), (":::", "p"));
/// assert_eq!(key_holder_and_tail("::a:::::p"), ("::a:::", "p"));
/// assert_eq!(key_holder_and_tail("::::::"), (":::", ":"));
/// assert_eq!(key_holder_and_tail("::x::"), ("::x", ""));
/// assert_eq!(key_holder_and_tail("::"), ("::", ""));
/// ```
#[must_use]
pub fn key_holder_and_tail(key: &str) -> (&str, &str) {
    let tail = key_tail(key);
    if tail.len() + 2 > key.len() {
        // A bare simple name (no separator): the holder is unknown/relative.
        return ("", tail);
    }
    let holder = &key[..key.len() - tail.len() - 2];
    if holder.is_empty() {
        ("::", tail)
    } else {
        (holder, tail)
    }
}

/// Select analytical segments from a constructed namespace display key, one
/// [`key_holder_and_tail`] step per level. The selected split cannot recover
/// every native component path because `"::"`-joining is not injective.
/// A legitimately colon-named segment survives (`":::"` → `[":"]`,
/// `"::a::b"` → `["a", "b"]`, `"::"` → `[]`).  Accepts rooted or unrooted
/// keys.  Contrast [`qualifier_segments`], the *written-name* split, which
/// collapses colon runs.
///
/// ```
/// use tcl_syntax::naming::key_segments;
/// assert_eq!(key_segments("::a::b"), ["a", "b"]);
/// assert_eq!(key_segments("a::b"), ["a", "b"]);
/// assert_eq!(key_segments(":::"), [":"]);
/// assert_eq!(key_segments("::::::"), [":", ":"]);
/// assert!(key_segments("::").is_empty());
/// assert!(key_segments("").is_empty());
/// ```
#[must_use]
pub fn key_segments(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor: String = if key.starts_with("::") || key.bytes().all(|b| b == b':') {
        key.to_owned()
    } else {
        format!("::{key}")
    };
    while !cursor.is_empty() && cursor != "::" {
        let (holder, tail) = key_holder_and_tail(&cursor);
        out.push(tail.to_owned());
        if holder == cursor {
            break;
        }
        cursor = holder.to_owned();
    }
    out.reverse();
    out
}

/// Possible suffix labels of a constructed analytical key, retaining the
/// complete input first. Each suffix follows the existing [`key_segments`]
/// projection; lone colon segments and a trailing empty segment survive.
/// Empty suffixes are omitted and repeated labels appear once.
///
/// These labels are a conservative metadata index, not written Tcl spellings,
/// namespace paths, command lookup candidates or proof of a selected binding.
#[must_use]
pub fn key_suffixes(key: &str) -> Vec<String> {
    let mut suffixes = vec![key.to_owned()];
    let segments = key_segments(key);
    for start in 0..segments.len() {
        let suffix = segments[start..].join("::");
        if !suffix.is_empty() && !suffixes.contains(&suffix) {
            suffixes.push(suffix);
        }
    }
    suffixes
}

/// Whether a definition written as `simple_name` inside the namespace whose
/// **written segments** are `ns_segments` can be reached by *any* absolute
/// (fully-qualified) written form.
///
/// Unaddressable shapes (tclsh 8.6/9.0-verified, invariant in C 8.4→9.1):
/// an all-colon simple name (only `:` is writable — a written colon run of ≥2
/// is a separator, so no absolute spelling can end in that name), or any
/// namespace segment that is itself all-colons (`namespace eval :` — its
/// contents are reachable only relatively, e.g. `namespace inscope : :`; the
/// rendered `[namespace which]` form `::::::` does not resolve).  The
/// empty-string name (`proc {} {} {}` / `proc x:: {} {}`) **is** addressable
/// (`::` / `::x::`).
#[must_use]
pub fn is_absolutely_addressable(ns_segments: &[&str], simple_name: &str) -> bool {
    let all_colons = |s: &str| !s.is_empty() && s.bytes().all(|b| b == b':');
    !all_colons(simple_name) && !ns_segments.iter().any(|seg| all_colons(seg))
}

/// Strip a variable reference's substitution sigil (`$`, `${…}`) while
/// **keeping** any array-index suffix — the form an evaluator needs to read the
/// actual variable (`$arr(idx)` → `arr(idx)`, `${v}` → `v`, `$x` → `x`). Unlike
/// [`normalise_var_name`], the `(idx)` is preserved.
///
/// ```
/// use tcl_syntax::naming::var_reference;
/// assert_eq!(var_reference("$arr(idx)"), "arr(idx)");
/// assert_eq!(var_reference("${v}"), "v");
/// assert_eq!(var_reference("$x"), "x");
/// ```
#[must_use]
pub fn var_reference(name: &str) -> &str {
    var_reference_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`var_reference`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn var_reference_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    match braced_var_name(name, style) {
        Some(inner) => inner,
        None => name.strip_prefix('$').unwrap_or(name),
    }
}

/// The name inside a word whose **whole text** is one `${…}` reference.
///
/// Two readings, in this order, and the order is the point:
///
/// 1. The closer located by the shared owner
///    [`tcl_lexer::braced_var_name_end`] under `style` — the free-text
///    reading, which is what a scanner walking raw source needs.
/// 2. Failing that, a word that nonetheless *is* a whole `${…}` span:
///    it opens `${` and ends `}`, so some lexer already delimited it, and a
///    span is **correct by construction** under whatever release produced it.
///
/// Step 2 exists because a token span and a free-text scan can disagree about
/// the same bytes when the caller's `style` is not the one that produced the
/// span. Under an 8.x dialect the lexer emits `${a{b}` as one `Var` (the name
/// `a{b`, with `c}` following as ordinary word text); reading that span with
/// the 9.x rule finds no closer at all, and answering "not a reference" would
/// name `{a{b}` — a variable the source never spells. Trusting the delimiters
/// the lexer already chose is strictly better than inventing that name, and it
/// can never override step 1: the owner returning `Unterminated` is exactly
/// the case where no closer exists under `style`.
///
/// A word with text *after* the closer is not one reference; step 1 still
/// yields its name (the caller decides what the remainder means), which is why
/// this returns only the name.
fn braced_var_name(word: &str, style: tcl_dialect::BracedVarStyle) -> Option<&str> {
    if let Some((inner, _)) = split_braced_var_ref(word, style) {
        return Some(inner);
    }
    word.strip_prefix("${").and_then(|w| w.strip_suffix('}'))
}

/// Split a word that **begins** with a `${…}` reference into its inner name
/// and the text following the closing brace.
///
/// This is the one place `tcl-syntax` decodes the brace form. The closer is
/// located by the shared owner [`tcl_lexer::braced_var_name_end`] under
/// `style`, never by a local scan: three independent private scans here
/// (`normalise_var_name` stripping the **last** `}` — the 9.x answer, at
/// every release — while `split_array_name` and `element_var_name_braced`
/// take the **first** — the 8.x answer, at every release) would disagree
/// with each other on the same bytes, so the crate would answer
/// `${a{b}c}` two ways.
///
/// `None` when the word does not open with `${`, or when the reference never
/// closes — an unterminated name is `Tcl_ParseVarName`'s
/// [`tcl_lexer::MISSING_CLOSE_BRACE_FOR_VAR`] error, not a name running to
/// end-of-input, so callers fall back to their non-braced reading rather than
/// inventing a name.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_syntax::naming::split_braced_var_ref;
/// // 9.x tracks nesting and `\X` pairs …
/// assert_eq!(
///     split_braced_var_ref("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     Some(("a{b}c", "")),
/// );
/// // … while the 8.x family ends the name at the first literal `}`.
/// assert_eq!(
///     split_braced_var_ref("${a{b}c}", BracedVarStyle::FirstClose),
///     Some(("a{b", "c}")),
/// );
/// assert_eq!(split_braced_var_ref("$a", BracedVarStyle::Tcl9Nesting), None);
/// assert_eq!(split_braced_var_ref("${a", BracedVarStyle::Tcl9Nesting), None);
/// ```
#[must_use]
pub fn split_braced_var_ref(
    word: &str,
    style: tcl_dialect::BracedVarStyle,
) -> Option<(&str, &str)> {
    if !word.starts_with("${") {
        return None;
    }
    // `2` is the byte just past the `${`, i.e. where the name starts. The
    // returned offset always lands on an ASCII `}`, so both slices cut on a
    // UTF-8 character boundary.
    match tcl_lexer::braced_var_name_end(word.as_bytes(), 2, style) {
        tcl_lexer::BracedVarEnd::Closed(end) => Some((&word[2..end], &word[end + 1..])),
        tcl_lexer::BracedVarEnd::Unterminated => None,
    }
}

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_syntax::naming::normalise_var_name;
/// assert_eq!(normalise_var_name("$foo"), "foo");
/// assert_eq!(normalise_var_name("${bar}"), "bar");
/// assert_eq!(normalise_var_name("$arr(idx)"), "arr");
/// assert_eq!(normalise_var_name("plain"), "plain");
/// ```
///
/// Uses the **default** `${…}` close rule
/// ([`tcl_dialect::BracedVarStyle::default`] — 9.x nesting), which is the rule
/// a document with no explicit dialect is lexed under, so the name recovered
/// here agrees with the span the lexer produced. A caller that has already
/// resolved the document's dialect must pass it through
/// [`normalise_var_name_for_style`] instead: under an 8.x dialect the closer
/// moves, and reading it with the 9.x rule names a variable the source never
/// mentions.
#[must_use]
pub fn normalise_var_name(name: &str) -> &str {
    normalise_var_name_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`normalise_var_name`] under an explicitly resolved `${…}` close rule.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_syntax::naming::normalise_var_name_for_style;
/// assert_eq!(
///     normalise_var_name_for_style("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     "a{b}c",
/// );
/// assert_eq!(
///     normalise_var_name_for_style("${a{b}c}", BracedVarStyle::FirstClose),
///     "a{b",
/// );
/// ```
#[must_use]
pub fn normalise_var_name_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    // The selected braced-name owner determines where the reference ends;
    // the shared closed-element partition decides whether parentheses denote
    // an array. Unmatched opens and suffix text remain scalar name bytes.
    split_array_name_for_style(name, style).0
}

/// Fixed identifier subset for emitting a variable-name word without quoting:
/// `[A-Za-z_][A-Za-z0-9_:]*`. This construction policy is deliberately narrower
/// than Tcl's accepted names and supplies no reference parsing or lookup proof.
///
/// ```
/// use tcl_syntax::naming::is_static_variable_source_word;
/// assert!(is_static_variable_source_word("ns::var"));
/// assert!(!is_static_variable_source_word("1abc"));
/// assert!(!is_static_variable_source_word("has space"));
/// ```
#[must_use]
pub fn is_static_variable_source_word(text: &str) -> bool {
    let mut iter = text.bytes();
    let Some(first) = iter.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    iter.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
}

/// Whether the complete source fragment is one variable reference under its grammar.
/// This shape supplies no selected cell, completed read or invocation authority.
#[must_use]
pub fn is_whole_variable_reference(source: &str, config: tcl_lexer::LexerConfig) -> bool {
    matches!(
        tcl_lexer::word_parts::whole_var_ref(source.as_bytes(), config),
        Ok(Some(_))
    )
}

/// One complete reference's literal name, without a separately substituted index.
/// Combined names such as `${a(k)}` remain exact name data; the selected lookup
/// must independently decide scalar/element semantics and authenticate the read.
#[must_use]
pub fn whole_variable_reference_literal_name(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<&str> {
    let reference = tcl_lexer::word_parts::whole_var_ref(source.as_bytes(), config).ok()??;
    reference.index.is_none().then_some(())?;
    core::str::from_utf8(reference.name).ok()
}

/// A whole scalar reference's literal name under the selected source grammar.
/// Separate and combined array references decline; empty and literal sigil names
/// remain data. The caller must independently authenticate the source and read.
#[must_use]
pub fn whole_scalar_variable_reference_name(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<&str> {
    let name = whole_variable_reference_literal_name(source, config)?;
    split_element_ref_bytes(name.as_bytes())
        .is_none()
        .then_some(name)
}

/// A whole original substitution's analytical variable root under its grammar.
///
/// The selected lexer owns sigils, brace closers and separate array indexes.
/// A braced combined name is partitioned by [`split_element_ref_bytes`]; an
/// unmatched parenthesis remains part of its scalar name. UTF-8 and opaque
/// byte references use the same path, retaining NUL and literal dollar bytes.
/// Compound text declines; malformed references retain the scanner's error.
/// This source projection supplies no native cell, input extent or read proof.
pub fn variable_reference_root_bytes(
    source: &[u8],
    config: tcl_lexer::LexerConfig,
) -> Result<Option<&[u8]>, &'static str> {
    let Some(reference) = tcl_lexer::word_parts::whole_var_ref(source, config)? else {
        return Ok(None);
    };
    let root = if reference.index.is_some() {
        reference.name
    } else {
        split_element_ref_bytes(reference.name).map_or(reference.name, |(root, _)| root)
    };
    Ok(Some(root))
}

/// Whether an array-element key is a compile-time literal — no `$` variable
/// substitution, `[…]` command substitution, or `\` backslash substitution,
/// so the element it names is a single fixed variable (`arr(k)`, `arr(1)`),
/// not a runtime-selected or -decoded one (`arr($i)`, `arr(\x41)`).
///
/// Backslash sequences substitute in an unbraced word (`incr arr(\x41)`
/// increments `arr(A)` — tclsh 8.6/9.0 verified), so treating them as
/// literal would split one runtime element across two SSA symbols. They stay
/// on the conflated base instead; a *braced* word (`set {arr(\x41)} v`)
/// suppresses substitution and its key is literal via
/// [`element_var_name_braced`]'s flag.
#[must_use]
pub fn array_key_is_literal(key: &str) -> bool {
    !key.contains('$') && !key.contains('[') && !key.contains('\\')
}

/// The **SSA variable name** of a reference or write-target word: the
/// element-qualified form `base(key)` for a *constant-keyed* array element,
/// the bare base otherwise.
///
/// Array elements are independent variables at runtime (each has its own
/// value and intrep), so a constant-keyed element gets its own name — and
/// therefore its own SSA symbol, type-lattice cell, and shimmer tracking.
/// A dynamic key (`arr($i)`) may select any element, so it stays on the
/// conflated base name (the SSA build fans its def over the array's known
/// elements). A `${…}`-braced reference substitutes nothing inside the
/// braces, so its key is always literal — `${arr($i)}` names the element
/// whose key is the two characters `$i`.
///
/// ```
/// use tcl_syntax::naming::element_var_name;
/// assert_eq!(element_var_name("$arr(k)"), "arr(k)");
/// assert_eq!(element_var_name("$arr(\\x41)"), "arr", "backslash keys substitute");
/// assert_eq!(element_var_name("arr(k)"), "arr(k)");
/// assert_eq!(element_var_name("$arr($i)"), "arr");
/// assert_eq!(element_var_name("${arr($i)}"), "arr($i)");
/// assert_eq!(element_var_name("${arr}(k)"), "arr");
/// assert_eq!(element_var_name("$plain"), "plain");
/// ```
#[must_use]
pub fn element_var_name(name: &str) -> &str {
    element_var_name_braced(name, false)
}

/// [`element_var_name`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn element_var_name_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    element_var_name_braced_for_style(name, false, style)
}

/// [`element_var_name`] for a word whose **delimiters make its content a
/// literal name** — a brace-quoted write target (`set {a($x)} v`) or the
/// `${…}` reference form, both of which suppress every substitution inside.
///
/// With `braced_literal` set, the content **is** the variable name, verbatim:
/// no `$` sigil to strip, no `${…}` form to unwrap, and the array key is
/// literal whatever it spells.  Tcl keeps such a name distinct from the
/// `$`-less one it looks like (tclsh 9.0.4 / 8.6.14, byte-identical):
///
/// ```text
/// set {$n} v ; set {$n}   -> v
/// info exists {$n} / n    -> 1 / 0     (two different variables)
/// set n other ; set {$n}  -> v         (unaffected)
/// set i 5 ; set {arr($i)} 1
/// array names arr         -> {$i}      (the literal key, not `arr(5)`)
/// info exists arr(5)      -> 0
/// ```
///
/// Stripping the sigil here keyed every consumer — defs, reads,
/// W210/W211/W220, rename, semantic highlighting — on the wrong variable.
///
/// ```
/// use tcl_syntax::naming::element_var_name_braced;
/// // A brace-quoted word: the content is the name, verbatim.
/// assert_eq!(element_var_name_braced("$n", true), "$n");
/// assert_eq!(element_var_name_braced("${n}", true), "${n}");
/// assert_eq!(element_var_name_braced("a b", true), "a b");
/// assert_eq!(element_var_name_braced("arr($i)", true), "arr($i)");
/// assert_eq!(element_var_name_braced("$arr(k)", true), "$arr(k)");
/// // Unbraced: the ordinary substituting spellings.
/// assert_eq!(element_var_name_braced("$n", false), "n");
/// assert_eq!(element_var_name_braced("$arr($i)", false), "arr");
/// ```
#[must_use]
pub fn element_var_name_braced(name: &str, braced_literal: bool) -> &str {
    element_var_name_braced_for_style(name, braced_literal, tcl_dialect::BracedVarStyle::default())
}

/// [`element_var_name_braced`] under an explicitly resolved `${…}` close rule.
///
/// The brace form is delimited by [`split_braced_var_ref`], i.e. by the shared
/// owner, rather than by a first-`}` scan that answered for 8.x at every
/// release — a harvest keyed on `a{b` where the lexer spanned `a{b}c` drops the
/// use and lets a live write be reported dead.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_syntax::naming::element_var_name_braced_for_style;
/// assert_eq!(
///     element_var_name_braced_for_style("${a{b}c}", false, BracedVarStyle::Tcl9Nesting),
///     "a{b}c",
/// );
/// assert_eq!(
///     element_var_name_braced_for_style("${a{b}c}", false, BracedVarStyle::FirstClose),
///     "a{b",
/// );
/// ```
#[must_use]
pub fn element_var_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> &str {
    if braced_literal {
        // The delimiters suppressed every substitution, so the content is the
        // whole name — including a leading `$` and any `(key)`, which is
        // literal by construction.
        return name;
    }
    // `${…}` brace form: the inner text is the whole (literal) name.
    if let Some(inner) = braced_var_name(name, style) {
        return inner;
    }
    let base = name.strip_prefix('$').unwrap_or(name);
    match split_array_name_for_style(name, style) {
        (_, Some(key)) if array_key_is_literal(key) => base,
        _ => normalise_var_name_for_style(name, style),
    }
}

/// [`normalise_var_name`] for a word whose delimiters make its content a
/// literal name — see [`element_var_name_braced`] for the oracle.
///
/// The *base* name, so an array element still loses its `(key)` suffix
/// (`{arr($i)}` is element `$i` of the array `arr`); what a braced word does
/// **not** lose is its substitution sigil, because there was no substitution:
/// `{$n}` names the variable `$n`, and `{$arr(k)}` element `k` of the array
/// `$arr`.
///
/// ```
/// use tcl_syntax::naming::normalise_var_name_braced;
/// assert_eq!(normalise_var_name_braced("$n", true), "$n");
/// assert_eq!(normalise_var_name_braced("${n}", true), "${n}");
/// assert_eq!(normalise_var_name_braced("arr($i)", true), "arr");
/// assert_eq!(normalise_var_name_braced("$arr(k)", true), "$arr");
/// assert_eq!(normalise_var_name_braced("$n", false), "n");
/// ```
#[must_use]
pub fn normalise_var_name_braced(name: &str, braced_literal: bool) -> &str {
    normalise_var_name_braced_for_style(
        name,
        braced_literal,
        tcl_dialect::BracedVarStyle::default(),
    )
}

/// [`normalise_var_name_braced`] under an explicitly resolved `${…}` close
/// rule.
#[must_use]
pub fn normalise_var_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> &str {
    if !braced_literal {
        return normalise_var_name_for_style(name, style);
    }
    split_array_name_braced_for_style(name, true, style).0
}

/// [`split_array_name`] for a word whose delimiters make its content a
/// literal name — see [`element_var_name_braced`] for the oracle.
///
/// ```
/// use tcl_syntax::naming::split_array_name_braced;
/// assert_eq!(split_array_name_braced("arr($i)", true), ("arr", Some("$i")));
/// assert_eq!(split_array_name_braced("$n", true), ("$n", None));
/// assert_eq!(split_array_name_braced("$arr(k)", true), ("$arr", Some("k")));
/// assert_eq!(split_array_name_braced("$arr(k)", false), ("arr", Some("k")));
/// ```
#[must_use]
pub fn split_array_name_braced(name: &str, braced_literal: bool) -> (&str, Option<&str>) {
    split_array_name_braced_for_style(name, braced_literal, tcl_dialect::BracedVarStyle::default())
}

/// [`split_array_name_braced`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn split_array_name_braced_for_style(
    name: &str,
    braced_literal: bool,
    style: tcl_dialect::BracedVarStyle,
) -> (&str, Option<&str>) {
    if !braced_literal {
        return split_array_name_for_style(name, style);
    }
    // A braced-literal word carries no sigil to strip, so the element split is
    // the owner's rule applied to the whole word — call it rather than
    // re-deriving it here.
    match split_element_ref(name) {
        Some((array, elem)) => (array, Some(elem)),
        None => (name, None),
    }
}

/// Whether a variable's **name** can only ever appear in source inside
/// quoting — `{$n}` / `${$n}`, `{a b}` / `${a b}` — because writing it bare
/// would substitute, split the word, or end the command.
///
/// Such a name is legal and distinct in Tcl (`set {$n} v` creates the
/// variable `$n`, unrelated to `n`), but every occurrence of it carries
/// delimiters that are *not* part of the recorded name span, and the
/// delimiters a **new** name needs depend on that new name.  A rewrite is
/// therefore not a matter of substituting new text into the recorded spans,
/// which is what the rename edit builder does — so rename refuses instead of
/// emitting an edit set that would produce `set q} 1`.
///
/// A namespace-qualified bareword (`::ns::v`) needs no quoting; neither does
/// an array element whose base and key are barewords (`arr(k)`), since it is
/// keyed by its base.
///
/// ```
/// use tcl_syntax::naming::var_name_requires_quoting;
/// assert!(var_name_requires_quoting("$n"));
/// assert!(var_name_requires_quoting("a b"));
/// assert!(var_name_requires_quoting("[gen]"));
/// assert!(!var_name_requires_quoting("n"));
/// assert!(!var_name_requires_quoting("::ns::v"));
/// assert!(!var_name_requires_quoting("arr"));
/// ```
#[must_use]
pub fn var_name_requires_quoting(name: &str) -> bool {
    name.is_empty()
        || name.contains([
            '$', '[', ']', '{', '}', '"', '\\', ';', ' ', '\t', '\n', '\r',
        ])
}

/// Return `true` when `${name}` would successfully look up `name` under the
/// given `${…}` delimiting rule.
///
/// `nesting` selects the dialect's `Tcl_ParseVarName` brace-form parser
/// (`LexerConfig::braced_var`):
///
/// - **`true` — Tcl 9.x** (tcl9.0.1 `tclParse.c`, the `braceCount` loop):
///   `\X` consumes 2 source chars, both kept in the lookup name — so a `}`
///   preceded by `\` does not end the span — and `{` / `}` are tracked with
///   a depth counter; only a `}` at depth 0 ends the var-name span.
///   Unreachable: a `}` at depth 0, a trailing lone `\`, or unbalanced `{`.
/// - **`false` — the 8.x family** (8.6.14 `tclParse.c:1466`,
///   tclsh-verified): the name is everything up to the FIRST literal `}`,
///   with no nesting and no backslash processing — so any name containing a
///   `}` is unreachable, and `{` / `\` are ordinary name characters.
///
/// Drives the W215 reachability check.
#[must_use]
pub fn is_brace_substitutable(name: &str, nesting: bool) -> bool {
    if name.is_empty() {
        return true; // `${}` looks up the var literally named "".
    }
    if !nesting {
        // 8.x: the first literal `}` closes the form, so a name containing
        // one can never be spelt; everything else is verbatim.
        return !name.contains('}');
    }
    let b = name.as_bytes();
    let n = b.len();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < n {
        match b[i] {
            b'}' if depth == 0 => return false,
            b'\\' => {
                if i + 1 >= n {
                    return false;
                }
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    depth == 0
}

/// Return `true` when `$name` would lex as a single bare variable token.
///
/// A name is one or more `::`-separated segments of **ASCII** alphanumerics
/// or `_`, with an optional leading `::` — `TclIsBareword` is ASCII-only in
/// every Tcl (8.6.14 and 9.0.1 `tclParse.c`; the man page: "Letters and
/// digits are only the standard ASCII ones"), so `$café` names the variable
/// `caf`. Used to decide between the bare `$name` and brace `${name}` forms
/// in quick fixes — a Unicode-permissive rule here would let a
/// `${café}` → `$café` rewrite silently change which variable is read.
///
/// ```
/// use tcl_syntax::naming::is_bare_var_name;
/// assert!(is_bare_var_name("foo"));
/// assert!(is_bare_var_name("ns::bar"));
/// assert!(is_bare_var_name("::baz"));
/// assert!(!is_bare_var_name("has-dash"));
/// assert!(!is_bare_var_name("café"));
/// assert!(!is_bare_var_name(""));
/// ```
#[must_use]
pub fn is_bare_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let s = name.strip_prefix("::").unwrap_or(name);
    if s.is_empty() {
        return false;
    }
    for segment in s.split("::") {
        if segment.is_empty() {
            return false;
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return false;
        }
    }
    true
}

/// Return `true` when *word* is the braced indirect-array-element idiom
/// `${name}(index)`.
///
/// Tcl parses `${name}(index)` as the brace-form substitution `${name}`
/// (which the lexer ends at the `}`) concatenated with the *literal* text
/// `(index)`.  In a variable-name position (the target of `set` / `incr` /
/// `append` / `lappend` / `unset` / `info exists` / `vwait`) the resulting
/// string `<value-of-name>(index)` names element `index` of the array whose
/// name is held in the scalar `name` — the standard "array name kept in a
/// variable" idiom (e.g. `set ${token}(status) eof`).  The braces are
/// essential: the bare `$name(index)` is a *direct* array reference, a
/// different construct, so this returns `false` for it.
///
/// This is the discriminator that keeps the W216 (brace-then-paren) and W212
/// (substitution-where-name-expected) checks from false-positiving on the
/// indirect idiom.
///
/// ```
/// use tcl_syntax::naming::is_braced_indirect_array_ref;
/// assert!(is_braced_indirect_array_ref("${token}(status)"));
/// assert!(!is_braced_indirect_array_ref("$arr(idx)"));
/// assert!(!is_braced_indirect_array_ref("${x}"));
/// assert!(!is_braced_indirect_array_ref("${}(x)"));
/// ```
#[must_use]
pub fn is_braced_indirect_array_ref(word: &str) -> bool {
    if !word.starts_with("${") {
        return false;
    }
    let bytes = word.as_bytes();
    let n = bytes.len();
    // Walk to the depth-0 `}` that closes the brace-form variable name.
    let mut i = 2usize;
    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    while i < n {
        match bytes[i] {
            b'\\' if i + 1 < n => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    close = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    // No closing brace, or an empty name (`${}(…)`) — not the idiom.  The
    // closing `}` must sit past the `${` and the (non-empty) name.
    let Some(close) = close.filter(|&c| c > 2) else {
        return false;
    };
    // The closing `}` must be immediately followed by `(…)` running to the
    // end of the word.
    let rest = &bytes[close + 1..];
    rest.len() >= 2 && rest[0] == b'(' && *rest.last().unwrap() == b')'
}

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_syntax::naming::normalise_qualified_name;
/// assert_eq!(normalise_qualified_name("foo"), "::foo");
/// assert_eq!(normalise_qualified_name("ns::bar"), "::ns::bar");
/// assert_eq!(normalise_qualified_name("::baz"), "::baz");
/// assert_eq!(normalise_qualified_name(""), "");
/// assert_eq!(normalise_qualified_name("::::x"), "::x");
/// ```
#[must_use]
pub fn normalise_qualified_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Share the one canonical `::` segmentation. Each segment is a subslice of a
    // `&str` split on ASCII `::`, so it is valid UTF-8.
    let parts: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    if parts.is_empty() {
        return "::".to_owned();
    }
    format!("::{}", parts.join("::"))
}

/// Join a namespace `prefix` and a (possibly-relative) `name` into a fully
/// qualified, `::`-rooted name — the canonical Tcl qualification rule:
///
/// * An **absolute** `name` (leading `::`) ignores the prefix entirely —
///   `qualify("::ns", "::other::C")` is `::other::C`, never re-prefixed.
/// * A relative `name` resolves under `prefix`, which may be given rooted
///   (`::a::b`) or unrooted (`a::b`).  The *written* `name`'s colon runs
///   collapse ([`canonical_written_command`]); the `prefix` is a constructed
///   key and is joined verbatim.
/// * An empty / root prefix roots the name at `::`.
///
/// The one shared join for the analyser / signature-scan / class-lattice
/// qualifiers, so the absolute-name rule cannot drift between them.
#[must_use]
pub fn qualify(prefix: &str, name: &str) -> String {
    // Canonicalise the *written* name once (colon-run rule, trailing-separator
    // preservation), then join with one exact separator — never re-parse the
    // joined result, which would collapse a legitimately colon-named segment
    // into its parent (`proc :` inside `namespace eval :`).
    let canonical = canonical_written_command(name);
    if is_absolute(name) {
        return canonical;
    }
    // The prefix is a *constructed* namespace key (possibly unrooted); use it
    // verbatim apart from rooting.  (Stripping is single-shot: the key of a
    // namespace named `:` is `":::"`, whose unrooted form is `":"`.)
    let p = prefix.strip_prefix("::").unwrap_or(prefix);
    if p.is_empty() {
        if canonical.is_empty() {
            // The empty-name entity (`proc {} {} {}`) at the root — see
            // [`ends_with_separator`]; `::x::` is its form inside `::x`.
            return "::".to_owned();
        }
        return format!("::{canonical}");
    }
    format!("::{p}::{canonical}")
}

/// Qualify a written C namespace name under a constructed namespace key.
/// Trailing separators select the namespace itself; empty relative input
/// retains the current namespace. This is an authored naming abstraction,
/// without native namespace existence or execution authority.
#[must_use]
pub fn qualify_namespace(prefix: &str, written: &str) -> String {
    if is_absolute(written) {
        return normalise_qualified_name(written);
    }
    let parts = qualifier_segments_owned(written);
    if parts.is_empty() {
        let relative = unroot_rooted_key(prefix).unwrap_or(prefix);
        return if relative.is_empty() {
            "::".to_owned()
        } else {
            format!("::{relative}")
        };
    }
    qualify(prefix, &parts.join("::"))
}

/// Byte-preserving qualification for command glob patterns and names.
/// The prefix is constructed; only the written name's colon runs collapse.
#[must_use]
pub fn qualify_bytes(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    let mut result = if is_absolute_bytes(name) {
        b"::".to_vec()
    } else {
        let prefix = prefix.strip_prefix(b"::").unwrap_or(prefix);
        let mut rooted = b"::".to_vec();
        rooted.extend_from_slice(prefix);
        if !prefix.is_empty() {
            rooted.extend_from_slice(b"::");
        }
        rooted
    };
    for (index, segment) in qualifier_segments(name).into_iter().enumerate() {
        if index != 0 {
            result.extend_from_slice(b"::");
        }
        result.extend_from_slice(segment);
    }
    if ends_with_separator(name) && !name.iter().all(|byte| *byte == b':') {
        result.extend_from_slice(b"::");
    }
    result
}

/// Add the global root marker to an **unrooted constructed key**.
///
/// This is not written-name canonicalisation: the input has already been
/// constructed from namespace segments, so every byte must be retained.  In
/// particular, an unrooted key may itself begin with `::` when its first
/// segment contains literal colons; prepend exactly one root marker rather
/// than using [`canonical_written_command`] or testing whether it already
/// looks rooted.
///
/// ```
/// use tcl_syntax::naming::root_unrooted_key;
///
/// assert_eq!(root_unrooted_key(""), "::");
/// assert_eq!(root_unrooted_key("matrix"), "::matrix");
/// assert_eq!(root_unrooted_key(":::p"), ":::::p");
/// ```
#[must_use]
pub fn root_unrooted_key(key: &str) -> String {
    format!("::{key}")
}

/// Counted-byte twin of [`root_unrooted_key`]: every byte of the constructed
/// key is retained after exactly one prepended root marker.
#[must_use]
pub fn root_unrooted_key_bytes(key: &[u8]) -> Vec<u8> {
    let mut rooted = Vec::with_capacity(key.len() + 2);
    rooted.extend_from_slice(b"::");
    rooted.extend_from_slice(key);
    rooted
}

/// String twin of [`rooted_member_key_bytes`]: neither key is reparsed.
///
/// ```
/// use tcl_syntax::naming::rooted_member_key;
/// assert_eq!(rooted_member_key("::", "v"), "::v");
/// assert_eq!(rooted_member_key("::a", "b"), "::a::b");
/// assert_eq!(rooted_member_key(":::", ":"), "::::::");
/// ```
#[must_use]
pub fn rooted_member_key(rooted_namespace: &str, member: &str) -> String {
    if rooted_namespace == "::" {
        format!("::{member}")
    } else {
        format!("{rooted_namespace}::{member}")
    }
}

/// A member's rooted reporting key under a rooted constructed namespace key:
/// the root's members take one marker, any other namespace one separator.
/// Neither input is reparsed; this supplies no lookup or table identity.
///
/// ```
/// use tcl_syntax::naming::rooted_member_key_bytes;
/// assert_eq!(rooted_member_key_bytes(b"::", b"v"), b"::v");
/// assert_eq!(rooted_member_key_bytes(b"::a:", b":v"), b"::a::::v");
/// ```
#[must_use]
pub fn rooted_member_key_bytes(rooted_namespace: &[u8], member: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(rooted_namespace.len() + member.len() + 2);
    key.extend_from_slice(rooted_namespace);
    if rooted_namespace != b"::" {
        key.extend_from_slice(b"::");
    }
    key.extend_from_slice(member);
    key
}

/// A member's key under an **unrooted constructed namespace key**, the
/// counterpart of [`rooted_member_key`] for tables keyed without the root
/// marker. The root is the empty key there, so its members stand alone; any
/// other namespace takes one separator. Neither input is reparsed.
///
/// ```
/// use tcl_syntax::naming::unrooted_member_key;
/// assert_eq!(unrooted_member_key("", "v"), "v");
/// assert_eq!(unrooted_member_key("a", "b"), "a::b");
/// assert_eq!(unrooted_member_key("a::b", ""), "a::b::");
/// assert_eq!(unrooted_member_key(":", ":"), "::::");
/// ```
#[must_use]
pub fn unrooted_member_key(unrooted_namespace: &str, member: &str) -> String {
    if unrooted_namespace.is_empty() {
        member.to_owned()
    } else {
        format!("{unrooted_namespace}::{member}")
    }
}

/// Remove exactly the root marker from a **rooted constructed key**.
/// This is the inverse of [`root_unrooted_key`], preserving colon-named
/// segments rather than canonicalising the spelling as a written name.
///
/// ```
/// use tcl_syntax::naming::{root_unrooted_key, unroot_rooted_key};
/// for key in ["", "N", ":", ":::p"] {
///     assert_eq!(unroot_rooted_key(&root_unrooted_key(key)), Some(key));
/// }
/// assert_eq!(unroot_rooted_key("relative"), None);
/// ```
#[must_use]
pub fn unroot_rooted_key(key: &str) -> Option<&str> {
    key.strip_prefix("::")
}

/// Remove one root marker from a counted constructed key, retaining all other bytes.
#[must_use]
pub fn unroot_rooted_key_bytes(key: &[u8]) -> Option<&[u8]> {
    key.strip_prefix(b"::")
}

/// A written name without its leading colons, for a display label or a
/// registry spelling: `::a::b` is `a::b`, `:::a` is `a`. It drops a single
/// leading colon too, so it is not a lookup key; a lookup uses
/// [`unroot_rooted_key`] or [`canonical_written_command`].
///
/// ```
/// use tcl_syntax::naming::without_leading_colons;
/// assert_eq!(without_leading_colons("::switch"), "switch");
/// assert_eq!(without_leading_colons(":::a::b"), "a::b");
/// assert_eq!(without_leading_colons("plain"), "plain");
/// ```
#[must_use]
pub fn without_leading_colons(name: &str) -> &str {
    name.trim_start_matches(':')
}

/// A literal fragment of a partly dynamic name without the colons at either
/// end, which belong to the separators joining it to its neighbours:
/// `::ns::` is `ns`, `a::b` stays `a::b`.
///
/// ```
/// use tcl_syntax::naming::without_surrounding_colons;
/// assert_eq!(without_surrounding_colons("::ns::"), "ns");
/// assert_eq!(without_surrounding_colons("a::b"), "a::b");
/// assert_eq!(without_surrounding_colons("::"), "");
/// ```
#[must_use]
pub fn without_surrounding_colons(fragment: &str) -> &str {
    fragment.trim_matches(':')
}

/// The analyser's flat key for a variable held by a named scope: the scope's
/// own key, one separator, the variable's base name. The scope key is used
/// verbatim, so this is a pair rendered flat and not a Tcl name.
///
/// ```
/// use tcl_syntax::naming::scope_variable_key;
/// assert_eq!(scope_variable_key("::", "x"), "::::x");
/// assert_eq!(scope_variable_key("::ns::p", "x"), "::ns::p::x");
/// ```
#[must_use]
pub fn scope_variable_key(scope: &str, variable: &str) -> String {
    format!("{scope}::{variable}")
}

/// A command lookup base supplied by a consumer's context owner.
/// Coordinates carry no namespace liveness or binding evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLookupBase<T> {
    /// The current namespace or an ordered namespace-path entry.
    Context(T),
    /// The interpreter's actual global namespace, resolved when reached.
    Root,
}

/// Visit current/path/root bases for a relative command, or root alone for
/// an absolute command. Consumers project their own coordinate representation.
pub fn command_lookup_bases<T>(
    absolute: bool,
    current: T,
    path: impl IntoIterator<Item = T>,
) -> impl Iterator<Item = CommandLookupBase<T>> {
    let mut contexts = std::iter::once(current).chain(path).fuse();
    let mut root_pending = true;
    std::iter::from_fn(move || {
        if !absolute && let Some(context) = contexts.next() {
            return Some(CommandLookupBase::Context(context));
        }
        std::mem::take(&mut root_pending).then_some(CommandLookupBase::Root)
    })
}

/// Candidate qualified names for Tcl's real bareword command/procedure
/// resolution, in priority order: the current namespace first, then global.
///
/// The `namespace path`-free specialisation of
/// [`command_resolution_candidates`] — see there for the full rule.  Kept
/// as the common entry point for consumers that do not model
/// `namespace path` (the static analyser, the optimiser's interprocedural
/// proc-identity resolution).
///
/// ```
/// use tcl_syntax::naming::bareword_resolution_candidates;
/// assert_eq!(bareword_resolution_candidates("::ns", "::foo"), vec!["::foo"]);
/// assert_eq!(
///     bareword_resolution_candidates("::ns", "inner::p"),
///     vec!["::ns::inner::p", "::inner::p"],
/// );
/// assert_eq!(bareword_resolution_candidates("::ns", "foo"), vec!["::ns::foo", "::foo"]);
/// assert_eq!(bareword_resolution_candidates("::", "foo"), vec!["::foo"]);
/// ```
#[must_use]
pub fn bareword_resolution_candidates(namespace: &str, cmd_name: &str) -> Vec<String> {
    command_resolution_candidates::<&str>(namespace, &[], cmd_name)
}

/// Candidate qualified names for Tcl's command resolution, in priority
/// order, including the current namespace's `namespace path`.
///
/// This is the **canonical encoding of C Tcl's command-lookup order**
/// (`Tcl_FindCommand`, `generic/tclNamesp.c`, Tcl 9.0.4) — the algorithm
/// every backend must agree with (behaviour pinned against tclsh 8.6.16
/// and 9.0.4 by `tests/command_resolution_conformance.rs`):
///
/// * An absolute name (`::foo`, `::ns::foo`) is taken as-is — one
///   candidate; the path is **not** consulted.
/// * Any relative name — bare (`foo`) *or* with embedded qualifiers
///   (`inner::p`) — is resolved against the current namespace first, then
///   each `namespace path` entry in order, then the global namespace.
///   The dispatch target is the first candidate that **exists**; mere
///   existence of an intermediate *namespace* does not commit resolution
///   (calling `inner::p` from `::outer` reaches `::inner::p` even when
///   the namespace `::outer::inner` exists but holds no `p`).
/// * No implicit ancestor walk: a bare `helper` inside `::a::b` never
///   reaches `::a::helper` unless `::a` is on the `namespace path`.
///
/// `path` entries may be given rooted (`::pathns`) or as written
/// (`pathns`): a *relative* entry is **current-namespace-relative only**
/// — `namespace path inner` inside `::outer` means `::outer::inner`,
/// never `::inner` (tclsh 8.6.16 / 9.0.4: the set errors `namespace
/// "inner" not found in "::outer"` when `::outer::inner` does not exist,
/// even with `::inner` present — namespace names have no global
/// fallback, unlike command names).  Consumers that do not model
/// `namespace path` pass `&[]` (equivalently, use
/// [`bareword_resolution_candidates`]).
///
/// ```
/// use tcl_syntax::naming::command_resolution_candidates;
/// // path slots between the current namespace and global:
/// assert_eq!(
///     command_resolution_candidates("::c", &["::pathns"], "helper"),
///     vec!["::c::helper", "::pathns::helper", "::helper"],
/// );
/// // a relative path entry is current-namespace-relative (tclsh-pinned):
/// assert_eq!(
///     command_resolution_candidates("::d", &["pathq"], "sub::q"),
///     vec!["::d::sub::q", "::d::pathq::sub::q", "::sub::q"],
/// );
/// // absolute names ignore the path entirely:
/// assert_eq!(
///     command_resolution_candidates("::d", &["::pathq"], "::sub::q"),
///     vec!["::sub::q"],
/// );
/// // duplicates collapse, keeping the highest-priority position:
/// assert_eq!(
///     command_resolution_candidates("::", &["::"], "foo"),
///     vec!["::foo"],
/// );
/// ```
#[must_use]
pub fn command_resolution_candidates<S: AsRef<str>>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
) -> Vec<String> {
    let absolute = cmd_name.starts_with("::");
    let canonical_cmd = canonical_written_command(cmd_name);
    let selected_path = path.iter().map(|entry| {
        let entry = entry.as_ref();
        if entry.starts_with("::") {
            normalise_qualified_name(entry)
        } else {
            let canonical_entry = qualifier_segments_owned(entry).join("::");
            if namespace.is_empty() || namespace == "::" {
                format!("::{canonical_entry}")
            } else if namespace.starts_with("::") {
                format!("{namespace}::{canonical_entry}")
            } else {
                format!("::{namespace}::{canonical_entry}")
            }
        }
    });
    let mut out = Vec::with_capacity(path.len() + 2);
    for base in command_lookup_bases(absolute, namespace.to_owned(), selected_path) {
        let candidate = match base {
            CommandLookupBase::Root if absolute => canonical_cmd.clone(),
            CommandLookupBase::Root => format!("::{canonical_cmd}"),
            CommandLookupBase::Context(base) if base.is_empty() || base == "::" => {
                format!("::{canonical_cmd}")
            }
            CommandLookupBase::Context(base) if base.starts_with("::") => {
                format!("{base}::{canonical_cmd}")
            }
            CommandLookupBase::Context(base) => format!("::{base}::{canonical_cmd}"),
        };
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

/// Ordered command candidates using retained constructed namespace keys.
///
/// The command operand is written syntax; current and path namespaces are
/// already selected contexts. Literal-colon components in those keys remain
/// data. A candidate key does not certify a globally callable source spelling.
#[must_use]
pub fn command_resolution_candidates_from_namespace_keys<S: AsRef<str>>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
) -> Vec<String> {
    let absolute = cmd_name.starts_with("::");
    let command = canonical_written_command(cmd_name);
    let mut candidates = Vec::with_capacity(path.len() + 2);
    for base in command_lookup_bases(absolute, namespace, path.iter().map(AsRef::as_ref)) {
        let key = match base {
            CommandLookupBase::Root if absolute => command.clone(),
            CommandLookupBase::Root => format!("::{command}"),
            CommandLookupBase::Context(context) if context.is_empty() || context == "::" => {
                format!("::{command}")
            }
            CommandLookupBase::Context(context) if context.starts_with("::") => {
                format!("{context}::{command}")
            }
            CommandLookupBase::Context(context) => format!("::{context}::{command}"),
        };
        if !candidates.contains(&key) {
            candidates.push(key);
        }
    }
    candidates
}

/// Resolve a command name the way C Tcl's `Tcl_FindCommand` does: walk
/// [`command_resolution_candidates`] in priority order and return the
/// first candidate for which `exists` is true.
///
/// `exists` is the caller's command table — the analyser's collected
/// definitions, a compilation unit's proc map, or a live interpreter's
/// registry.  Returns `None` when no candidate exists (Tcl would raise
/// `invalid command name` / fall through to `unknown`).
///
/// ```
/// use tcl_syntax::naming::resolve_command_with;
/// let defined = ["::inner::p"];
/// let exists = |q: &str| defined.contains(&q);
/// // local candidate absent -> falls back to the global command:
/// assert_eq!(
///     resolve_command_with::<&str, _>("::outer", &[], "inner::p", exists),
///     Some("::inner::p".to_string()),
/// );
/// // nothing defined -> unresolved:
/// assert_eq!(resolve_command_with::<&str, _>("::outer", &[], "nope", exists), None);
/// ```
#[must_use]
pub fn resolve_command_with<S: AsRef<str>, F: FnMut(&str) -> bool>(
    namespace: &str,
    path: &[S],
    cmd_name: &str,
    mut exists: F,
) -> Option<String> {
    command_resolution_candidates(namespace, path, cmd_name)
        .into_iter()
        .find(|candidate| exists(candidate))
}

/// Split a **resolved** variable name into `(array, element)`, or `None` when
/// it names a scalar / whole array.
///
/// This is `TclObjLookupVarEx`'s unparsed-element rule verbatim
/// (`tclVar.c(9.0.4):683-686`): the name is an array element when it is longer
/// than one byte, ends in `)`, and contains a `(`. The array name is
/// everything before the **first** `(` and the element everything between it
/// and the final `)`.
///
/// Note what the rule does *not* require: the `(` may sit at offset 0, so a
/// **zero-length array name** is legal — `set (x) 5` writes element `x` of the
/// array named `""`, exactly as tclsh does. A consumer that adds
/// a "base must be non-empty" test silently demotes those references to
/// ordinary scalars.
///
/// Unlike [`split_array_name`] this takes a name that has already been
/// resolved — no `$` / `${…}` sigil is stripped — so a variable genuinely
/// *named* `$foo(x)` splits as array `$foo`, element `x`.
///
/// ```
/// use tcl_syntax::naming::split_element_ref;
/// assert_eq!(split_element_ref("arr(foo)"), Some(("arr", "foo")));
/// assert_eq!(split_element_ref("arr"), None);
/// // Zero-length array name and zero-length element are both legal.
/// assert_eq!(split_element_ref("(x)"), Some(("", "x")));
/// assert_eq!(split_element_ref("arr()"), Some(("arr", "")));
/// assert_eq!(split_element_ref("()"), Some(("", "")));
/// // Not an element reference: no `(`, or nothing after it closes.
/// assert_eq!(split_element_ref(")"), None);
/// assert_eq!(split_element_ref("a(b"), None);
/// ```
#[must_use]
pub fn split_element_ref(name: &str) -> Option<(&str, &str)> {
    // Both halves are cut at an ASCII `(` / `)`, so each offset is a UTF-8
    // character boundary and the byte split re-slices `name` directly.
    let (array, _) = split_element_ref_bytes(name.as_bytes())?;
    let open = array.len();
    Some((&name[..open], &name[open + 1..name.len() - 1]))
}

/// [`split_element_ref`] over raw bytes, for the byte-oriented runtime (a
/// variable name is not required to be UTF-8 there).
#[must_use]
pub fn split_element_ref_bytes(name: &[u8]) -> Option<(&[u8], &[u8])> {
    if name.last() != Some(&b')') {
        return None;
    }
    // C also demands `len > 1`, which is implied here: a one-byte name ending
    // in `)` cannot also contain the `(` this scan requires.
    let open = name.iter().position(|&c| c == b'(')?;
    Some((&name[..open], &name[open + 1..name.len() - 1]))
}

/// Split a Tcl variable reference into `(base, element)`.
///
/// Strips `$` / `${…}` substitution sigils first, then separates the
/// optional `(element)` array-index suffix from the base name.  Returns
/// `(base, None)` for scalar references.  Follows the brace-form rule that
/// `${arr}(foo)` is the scalar `arr` followed by literal `(foo)`, whereas
/// `${arr(foo)}` *is* the array element `arr(foo)`.
///
/// ```
/// use tcl_syntax::naming::split_array_name;
/// assert_eq!(split_array_name("arr"), ("arr", None));
/// assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
/// assert_eq!(split_array_name("${arr}"), ("arr", None));
/// ```
#[must_use]
pub fn split_array_name(name: &str) -> (&str, Option<&str>) {
    split_array_name_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`split_array_name`] under an explicitly resolved `${…}` close rule.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_syntax::naming::split_array_name_for_style;
/// // 9.x: the nested pair is inside the name, so this is the scalar `a{b}c`.
/// assert_eq!(
///     split_array_name_for_style("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     ("a{b}c", None),
/// );
/// // 8.x: the name ends at the first `}` and `c}` is ordinary word text.
/// assert_eq!(
///     split_array_name_for_style("${a{b}c}", BracedVarStyle::FirstClose),
///     ("a{b", None),
/// );
/// ```
#[must_use]
pub fn split_array_name_for_style(
    name: &str,
    style: tcl_dialect::BracedVarStyle,
) -> (&str, Option<&str>) {
    // `${…}` brace form: only the chars inside the braces are the reference;
    // an `(idx)` *inside* the braces is an element, one *after* `}` is not.
    if let Some(inner) = braced_var_name(name, style) {
        return match split_element_ref(inner) {
            Some((array, elem)) => (array, Some(elem)),
            None => (inner, None),
        };
    }
    // No closing brace — fall through (gated on `"}" in base`).
    let base = name.strip_prefix('$').unwrap_or(name);
    match split_element_ref(base) {
        Some((array, elem)) => (array, Some(elem)),
        None => (base, None),
    }
}

/// Split an editor's incomplete array-name fragment into source prefixes.
/// Neither prefix is an evaluated name, complete reference or variable lookup.
#[must_use]
pub fn array_variable_completion_prefix(partial: &str) -> Option<(&str, &str)> {
    let (name, index) = partial.split_once('(')?;
    (!name.is_empty()).then_some((name, index))
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
