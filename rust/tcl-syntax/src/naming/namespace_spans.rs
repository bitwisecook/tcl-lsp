// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Written namespace ranges selected from original bytes and retained geometry.

use std::ops::Range;

use super::{NativeNameContext, NativeNameProtocol};

/// A complete source word selecting this independently retained namespace.
/// The address is round-tripped before the shared native source renderer runs;
/// this provides no namespace existence, entered frame or edit permission.
#[must_use]
pub fn native_namespace_source_word(
    protocol: NativeNameProtocol,
    wanted: NativeNameContext<'_>,
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let value = match protocol {
        NativeNameProtocol::C(_) => super::native_namespace_full_name_bytes(wanted.namespace),
        NativeNameProtocol::Jim084 => {
            let mut value = b"::".to_vec();
            value.extend_from_slice(wanted.jim_namespace_object?);
            value
        }
    };
    selects(protocol, NativeNameContext::root(), &value, wanted).then_some(())?;
    crate::backslash::native_literal_source_word(
        &value,
        channel,
        config,
        protocol.string_protocol(),
    )
}

fn selects(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> bool {
    match protocol {
        NativeNameProtocol::C(_) => protocol
            .namespace_address_path(current, original)
            .is_ok_and(|path| path == *wanted.namespace),
        NativeNameProtocol::Jim084 => wanted.jim_namespace_object.is_some_and(|wanted| {
            protocol
                .jim_namespace_canonical_input(current, original)
                .is_ok_and(|selected| selected.selected() == wanted)
        }),
    }
}

/// The shortest nonempty original prefix selecting the exact wanted namespace.
/// Only the original operand is interpreted. Retained contexts supply no namespace
/// existence, entered frame, command token or native compiler authority.
#[must_use]
pub fn native_written_namespace_prefix_extent(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> Option<Range<usize>> {
    let root = match protocol {
        NativeNameProtocol::C(_) => wanted.namespace.is_root(),
        NativeNameProtocol::Jim084 => wanted.jim_namespace_object?.is_empty(),
    };
    if root {
        return None;
    }
    match protocol {
        NativeNameProtocol::C(_) => {
            let mut selected = protocol.namespace_address_path(current, original).ok()?;
            loop {
                if selected == *wanted.namespace {
                    break;
                }
                selected = selected.parent()?;
            }
        }
        NativeNameProtocol::Jim084 => {
            let selected = protocol
                .jim_namespace_canonical_input(current, original)
                .ok()?;
            let mut object = selected.selected();
            let wanted = wanted.jim_namespace_object?;
            loop {
                if object == wanted {
                    break;
                }
                if object.is_empty() {
                    return None;
                }
                let parent = protocol.namespace_qualifier_bytes(object);
                if parent == object {
                    return None;
                }
                object = parent;
            }
        }
    }
    (1..=original.len())
        .find(|&end| selects(protocol, current, &original[..end], wanted))
        .map(|end| 0..end)
}

/// The last namespace component written in the original prefix, with its exact
/// retained parent selected independently. A relative word omitting that component
/// has no range. Opaque Jim namespace objects without a supported parent decline.
#[must_use]
pub fn native_written_namespace_member_extent(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
) -> Option<Range<usize>> {
    let end = native_written_namespace_prefix_extent(protocol, current, original, wanted)?.end;
    let start = match protocol {
        NativeNameProtocol::C(_) => {
            let parent = wanted.namespace.parent()?;
            let parent = NativeNameContext::new(&parent);
            (0..end).rfind(|&start| {
                if start == 0 {
                    current.namespace == parent.namespace
                } else {
                    selects(protocol, current, &original[..start], parent)
                }
            })?
        }
        NativeNameProtocol::Jim084 => {
            let object = wanted.jim_namespace_object?;
            if object.contains(&0) {
                return None;
            }
            let parent = protocol.namespace_qualifier_bytes(object);
            let root = NativeNameContext::root();
            let parent = NativeNameContext::with_jim_namespace(root.namespace, parent);
            let start = (0..end).rfind(|&start| {
                if start == 0 {
                    current.jim_namespace_object.unwrap_or(b"")
                        == parent.jim_namespace_object.unwrap_or(b"")
                } else {
                    selects(protocol, current, &original[..start], parent)
                }
            })?;
            // Jim canonicalisation preserves a trailing separator, so the
            // parent prefix ends before its exact pair rather than after it.
            if original.get(start..end)?.starts_with(b"::") {
                start + 2
            } else {
                start
            }
        }
    };
    (start < end).then_some(start..end)
}

/// Byte projection for a namespace component that may be written in an input.
/// It supplies no occurrence identity, source span, namespace existence or edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNamespaceOperandRename {
    /// The operand omits the component or selects a different namespace.
    Unchanged,
    /// Original units with exactly the written component replaced.
    Replace(Vec<u8>),
}

/// Rename one namespace component through the original operand's own purpose
/// geometry. The new component and complete selected result must round-trip;
/// constructed paths and Jim objects are never recovered from display strings.
#[must_use]
pub fn native_namespace_operand_rename(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    original: &[u8],
    wanted: NativeNameContext<'_>,
    new_tail: &[u8],
) -> Option<NativeNamespaceOperandRename> {
    if new_tail.is_empty() {
        return None;
    }
    let expected = match protocol {
        NativeNameProtocol::C(_) => {
            let selected = protocol.namespace_address_path(current, original).ok()?;
            if wanted.namespace.is_root() {
                return None;
            }
            if !selected
                .as_segments()
                .starts_with(wanted.namespace.as_segments())
            {
                return Some(NativeNamespaceOperandRename::Unchanged);
            }
            let parent = wanted.namespace.parent()?;
            let changed = tcl_core_types::ByteNamespacePath::from_segments(
                parent
                    .as_segments()
                    .iter()
                    .map(tcl_core_types::NameBytes::as_bytes)
                    .chain(std::iter::once(new_tail)),
            );
            if protocol
                .namespace_address_path(NativeNameContext::new(&parent), new_tail)
                .ok()?
                != changed
            {
                return None;
            }
            let mut components = selected.as_segments().to_vec();
            components[wanted.namespace.as_segments().len() - 1] = new_tail.into();
            NativeNamespaceRenameResult::C(tcl_core_types::ByteNamespacePath::from_segments(
                components,
            ))
        }
        NativeNameProtocol::Jim084 => {
            let wanted = wanted.jim_namespace_object?;
            if wanted.is_empty() || wanted.contains(&0) {
                return None;
            }
            let selected = protocol
                .jim_namespace_canonical_input(current, original)
                .ok()?;
            if !jim_under(protocol, selected.selected(), wanted) {
                return Some(NativeNamespaceOperandRename::Unchanged);
            }
            let parent = protocol.namespace_qualifier_bytes(wanted);
            let mut changed = parent.to_vec();
            if !changed.is_empty() {
                changed.extend_from_slice(b"::");
            }
            changed.extend_from_slice(new_tail);
            if new_tail.contains(&0) || protocol.namespace_qualifier_bytes(&changed) != parent {
                return None;
            }
            let parent_context =
                NativeNameContext::with_jim_namespace(NativeNameContext::root().namespace, parent);
            if protocol
                .jim_namespace_canonical_input(parent_context, new_tail)
                .ok()?
                .selected()
                != changed
            {
                return None;
            }
            changed.extend_from_slice(selected.selected().get(wanted.len()..)?);
            NativeNamespaceRenameResult::Jim(changed)
        }
    };
    let Some(extent) = native_written_namespace_member_extent(protocol, current, original, wanted)
    else {
        let inherited = match protocol {
            NativeNameProtocol::C(_) => current
                .namespace
                .as_segments()
                .starts_with(wanted.namespace.as_segments()),
            NativeNameProtocol::Jim084 => jim_under(
                protocol,
                current.jim_namespace_object?,
                wanted.jim_namespace_object?,
            ),
        };
        return inherited.then_some(NativeNamespaceOperandRename::Unchanged);
    };
    let mut replacement = original.to_vec();
    replacement.splice(extent, new_tail.iter().copied());
    namespace_rename_matches_expected(protocol, current, &replacement, expected)?
        .then_some(NativeNamespaceOperandRename::Replace(replacement))
}

/// Compare the independently computed result with the same-purpose replacement.
/// Projection failure remains unavailable; this supplies no namespace lookup.
fn namespace_rename_matches_expected(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    replacement: &[u8],
    expected: NativeNamespaceRenameResult,
) -> Option<bool> {
    Some(match expected {
        NativeNamespaceRenameResult::C(expected) => {
            protocol.namespace_address_path(current, replacement).ok()? == expected
        }
        NativeNamespaceRenameResult::Jim(expected) => {
            protocol
                .jim_namespace_canonical_input(current, replacement)
                .ok()?
                .selected()
                == expected
        }
    })
}

enum NativeNamespaceRenameResult {
    C(tcl_core_types::ByteNamespacePath),
    Jim(Vec<u8>),
}

fn jim_under(protocol: NativeNameProtocol, selected: &[u8], wanted: &[u8]) -> bool {
    let mut selected = selected;
    loop {
        if selected == wanted {
            return true;
        }
        if selected.is_empty() {
            return false;
        }
        let parent = protocol.namespace_qualifier_bytes(selected);
        if parent == selected {
            return false;
        }
        selected = parent;
    }
}

/// Select which independently retained lookup home names the actual called
/// slot. A global fallback is used only when that same original input selects
/// the exact slot there. This supplies no command existence or identity proof.
#[must_use]
pub fn native_command_lookup_context_for_slot<'a>(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'a>,
    original: &[u8],
    slot: &tcl_core_types::ByteCommandSlot,
) -> Option<NativeNameContext<'a>> {
    if protocol.is_jim084() {
        if !slot.namespace.is_root() {
            return None;
        }
        let keys = protocol.jim_command_lookup_keys(current, original).ok()?;
        if keys.first() == Some(&slot.simple) {
            return Some(current);
        }
        return (keys.get(1) == Some(&slot.simple)).then_some(NativeNameContext::root());
    }
    if protocol
        .command_lookup_slot(current, original)
        .ok()
        .as_ref()
        == Some(slot)
    {
        return Some(current);
    }
    let root = NativeNameContext::root();
    (protocol.command_lookup_slot(root, original).ok().as_ref() == Some(slot)).then_some(root)
}

/// Whether a dynamic operand beginning with these independently produced
/// literal units may name the wanted namespace or one of its descendants.
/// Only an absolute fixed C prefix can rule this out. Missing geometry and
/// unaudited partial Jim object construction remain May; this grants no name
/// identity, namespace existence, source extent or edit.
#[must_use]
pub fn native_namespace_literal_prefix_may_select(
    protocol: NativeNameProtocol,
    prefix: &[u8],
    wanted: NativeNameContext<'_>,
) -> bool {
    if !prefix.starts_with(b"::") {
        return true;
    }
    let Ok(projection) = protocol.namespace_address_input(NativeNameContext::root(), prefix) else {
        return true;
    };
    if prefix.contains(&0) {
        // An absolute native CString terminator fixes the selected operand
        // before any unknown suffix. Reuse complete geometry in that case.
        return native_written_namespace_prefix_extent(
            protocol,
            NativeNameContext::root(),
            prefix,
            wanted,
        )
        .is_some();
    }
    let NativeNameProtocol::C(_) = protocol else {
        return true;
    };
    let Ok(selected) =
        protocol.namespace_address_path(NativeNameContext::root(), projection.selected())
    else {
        return true;
    };
    let fixed = selected.as_segments();
    let wanted = wanted.namespace.as_segments();
    let complete = prefix.ends_with(b"::");
    let prefix_count = if complete {
        fixed.len()
    } else {
        fixed.len().saturating_sub(1)
    };
    if fixed[..prefix_count.min(wanted.len())] != wanted[..prefix_count.min(wanted.len())] {
        return false;
    }
    if complete || wanted.len() <= prefix_count {
        return true;
    }
    fixed.last().is_none_or(|partial| {
        wanted[prefix_count]
            .as_bytes()
            .starts_with(partial.as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;
    use tcl_dialect::TclVersion;

    #[test]
    fn dynamic_namespace_prefix_may_retains_uncertainty_and_rules_out_fixed_siblings() {
        let wanted = ByteNamespacePath::from_segments(["old", "inner"]);
        let wanted = NativeNameContext::new(&wanted);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            for prefix in [
                b"".as_slice(),
                b"relative::",
                b"::ol",
                b"::old::",
                b"::old::in",
                b"::old::inner::deep::",
            ] {
                assert!(
                    native_namespace_literal_prefix_may_select(protocol, prefix, wanted),
                    "{prefix:?}"
                );
            }
            for prefix in [
                b"::other::".as_slice(),
                b"::old::wrong",
                b"::old::other::",
                b"::old::in\0",
            ] {
                assert!(
                    !native_namespace_literal_prefix_may_select(protocol, prefix, wanted),
                    "{prefix:?}"
                );
            }
            assert!(native_namespace_literal_prefix_may_select(
                protocol,
                b"::old::inner\0ignored",
                wanted
            ));
        }
        let root = NativeNameContext::root();
        assert!(native_namespace_literal_prefix_may_select(
            NativeNameProtocol::Jim084,
            b"::other::",
            NativeNameContext::with_jim_namespace(root.namespace, b"old")
        ));
    }

    #[test]
    fn namespace_spans_preserve_original_colon_runs_and_retained_components() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeNameProtocol::C(version);
            let wanted = ByteNamespacePath::from_segments(["app", "old"]);
            let wanted = NativeNameContext::new(&wanted);
            assert_eq!(
                native_written_namespace_member_extent(
                    protocol,
                    NativeNameContext::root(),
                    b"::::app:::old::leaf",
                    wanted
                ),
                Some(10..13)
            );
            let current = ByteNamespacePath::from_segments(["app"]);
            assert_eq!(
                native_written_namespace_member_extent(
                    protocol,
                    NativeNameContext::new(&current),
                    b"old::leaf",
                    wanted
                ),
                Some(0..3)
            );
            let parent = NativeNameContext::new(&current);
            assert_eq!(
                native_written_namespace_member_extent(protocol, parent, b"old::leaf", parent),
                None
            );
        }
    }

    #[test]
    fn namespace_spans_do_not_recover_display_colliding_scope_components() {
        // Implementation proof: naming.namespace.original-member-extent
        // docs/design/analysis/name-resolution-proofs/namespace-original-member-extent.md
        let first = ByteNamespacePath::from_segments(["a:", "b"]);
        let second = ByteNamespacePath::from_segments(["a", ":b"]);
        let protocol = NativeNameProtocol::C(TclVersion::V9_0);
        for path in [&first, &second] {
            assert_eq!(
                native_written_namespace_prefix_extent(
                    protocol,
                    NativeNameContext::root(),
                    b"::a:::b",
                    NativeNameContext::new(path)
                ),
                None
            );
        }
        let jim = NativeNameProtocol::Jim084;
        let root = NativeNameContext::root();
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"app::old");
        assert_eq!(
            native_written_namespace_member_extent(jim, root, b"::app::old::leaf", wanted),
            Some(7..10)
        );
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"app:::old");
        assert_eq!(
            native_written_namespace_member_extent(jim, root, b"::app:::old::leaf", wanted),
            Some(8..11),
            "the colon belonging to Jim's parent object remains outside the member"
        );
    }
}

#[cfg(test)]
mod namespace_source_word_tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;
    use tcl_dialect::TclVersion;

    #[test]
    fn namespace_source_words_reselect_exact_opaque_components_and_jim_objects() {
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            let config = tcl_lexer::LexerConfig {
                escapes: protocol.string_protocol().escape_syntax(),
                ..tcl_lexer::LexerConfig::default()
            };
            for units in [
                b"n\xed\xa0\x80".as_slice(),
                b"n\xed\xa0\x81",
                b"n\xc0\x80tail",
            ] {
                let path = ByteNamespacePath::from_segments([units]);
                let word = native_namespace_source_word(
                    protocol,
                    NativeNameContext::new(&path),
                    tcl_lexer::SourceChannel::Document,
                    config,
                )
                .unwrap();
                let produced = crate::backslash::native_source_string_bytes_channel_in(
                    &word.as_bytes()[1..word.len() - 1],
                    tcl_lexer::SourceChannel::Document,
                    config.escapes,
                    protocol.string_protocol(),
                )
                .unwrap();
                assert_eq!(
                    protocol
                        .namespace_address_path(NativeNameContext::root(), &produced)
                        .unwrap(),
                    path
                );
            }
            let raw_zero = ByteNamespacePath::from_segments([b"n\0tail".as_slice()]);
            assert!(
                native_namespace_source_word(
                    protocol,
                    NativeNameContext::new(&raw_zero),
                    tcl_lexer::SourceChannel::NativeValue,
                    config
                )
                .is_none()
            );
            let ambiguous = ByteNamespacePath::from_segments([b"a:".as_slice(), b"b"]);
            assert!(
                native_namespace_source_word(
                    protocol,
                    NativeNameContext::new(&ambiguous),
                    tcl_lexer::SourceChannel::Document,
                    config
                )
                .is_none()
            );
        }
        let protocol = NativeNameProtocol::Jim084;
        let config = tcl_lexer::LexerConfig::for_dialect("jimtcl");
        let root = NativeNameContext::root();
        let context = NativeNameContext::with_jim_namespace(root.namespace, b"n\0tail");
        assert!(
            native_namespace_source_word(
                protocol,
                context,
                tcl_lexer::SourceChannel::NativeValue,
                config
            )
            .is_none(),
            "Jim absolute CString namespace cannot recover a counted NUL object"
        );
        let context = NativeNameContext::with_jim_namespace(root.namespace, b"n::child");
        assert!(
            native_namespace_source_word(
                protocol,
                context,
                tcl_lexer::SourceChannel::Document,
                config
            )
            .is_some()
        );
    }
}

#[cfg(test)]
mod original_namespace_rename_tests {
    use super::*;
    use tcl_core_types::ByteNamespacePath;

    #[test]
    fn original_namespace_rename_preserves_colon_runs_and_omitted_components() {
        // Implementation proof: naming.namespace.original-member-extent
        // docs/design/analysis/name-resolution-proofs/namespace-original-member-extent.md
        for version in tcl_dialect::TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            let wanted = ByteNamespacePath::from_segments([b"a".as_slice(), b"old"]);
            assert_eq!(
                native_namespace_operand_rename(
                    protocol,
                    NativeNameContext::root(),
                    b"::a::::old::p",
                    NativeNameContext::new(&wanted),
                    b"new"
                ),
                Some(NativeNamespaceOperandRename::Replace(
                    b"::a::::new::p".to_vec()
                ))
            );
            assert_eq!(
                native_namespace_operand_rename(
                    protocol,
                    NativeNameContext::new(&wanted),
                    b"p",
                    NativeNameContext::new(&wanted),
                    b"new"
                ),
                Some(NativeNamespaceOperandRename::Unchanged)
            );
            assert!(
                native_namespace_operand_rename(
                    protocol,
                    NativeNameContext::root(),
                    b"::a::old::p",
                    NativeNameContext::new(&wanted),
                    b"two::parts"
                )
                .is_none()
            );
            assert!(
                native_namespace_operand_rename(
                    protocol,
                    NativeNameContext::root(),
                    b"::a::old::p",
                    NativeNameContext::new(&wanted),
                    b"new\0tail"
                )
                .is_none()
            );
        }
        let root = NativeNameContext::root();
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"a::old");
        assert_eq!(
            native_namespace_operand_rename(
                NativeNameProtocol::Jim084,
                root,
                b"::a::old::p",
                wanted,
                b"new"
            ),
            Some(NativeNamespaceOperandRename::Replace(
                b"::a::new::p".to_vec()
            ))
        );
        assert_eq!(
            native_namespace_operand_rename(
                NativeNameProtocol::Jim084,
                wanted,
                b"p",
                wanted,
                b"new"
            ),
            Some(NativeNamespaceOperandRename::Unchanged)
        );
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"app:::old");
        assert_eq!(
            native_namespace_operand_rename(
                NativeNameProtocol::Jim084,
                root,
                b"::::app:::old::p",
                wanted,
                b"new"
            ),
            Some(NativeNamespaceOperandRename::Replace(
                b"::::app:::new::p".to_vec()
            ))
        );
    }

    #[test]
    fn original_namespace_rename_keeps_distinct_opaque_components_and_actual_lookup_home() {
        let protocol = NativeNameProtocol::C(tcl_dialect::TclVersion::V8_6);
        let wanted = ByteNamespacePath::from_segments([b"n\xed\xa0\x80".as_slice()]);
        assert_eq!(
            native_namespace_operand_rename(
                protocol,
                NativeNameContext::root(),
                b"::n\xed\xa0\x80::p",
                NativeNameContext::new(&wanted),
                b"new"
            ),
            Some(NativeNamespaceOperandRename::Replace(b"::new::p".to_vec()))
        );
        assert_eq!(
            native_namespace_operand_rename(
                protocol,
                NativeNameContext::root(),
                b"::n\xed\xa0\x81::p",
                NativeNameContext::new(&wanted),
                b"new"
            ),
            Some(NativeNamespaceOperandRename::Unchanged)
        );
        let current = ByteNamespacePath::from_segments([b"caller".as_slice()]);
        let slot = protocol
            .command_lookup_slot(NativeNameContext::root(), b"old::p")
            .unwrap();
        let selected = native_command_lookup_context_for_slot(
            protocol,
            NativeNameContext::new(&current),
            b"old::p",
            &slot,
        )
        .unwrap();
        assert!(selected.namespace.is_root());
        assert!(
            native_command_lookup_context_for_slot(
                protocol,
                NativeNameContext::new(&current),
                b"other::p",
                &slot
            )
            .is_none()
        );
    }
}

/// Whether a purpose-selected command slot's namespace is the wanted namespace
/// or its descendant. The simple command name is excluded. Jim flat keys with
/// opaque `CString` parent geometry decline; this is geometry, not existence.
#[must_use]
pub fn native_command_slot_is_under_namespace(
    protocol: NativeNameProtocol,
    slot: &tcl_core_types::ByteCommandSlot,
    wanted: NativeNameContext<'_>,
) -> Option<bool> {
    match protocol {
        NativeNameProtocol::C(_) => Some(
            slot.namespace
                .as_segments()
                .starts_with(wanted.namespace.as_segments()),
        ),
        NativeNameProtocol::Jim084 => {
            if !slot.namespace.is_root() || slot.simple.as_bytes().contains(&0) {
                return None;
            }
            Some(jim_under(
                protocol,
                protocol.namespace_qualifier_bytes(slot.simple.as_bytes()),
                wanted.jim_namespace_object?,
            ))
        }
    }
}

/// Whether the explicit namespace of a runtime variable root is the wanted
/// namespace or its descendant. The simple C variable key and array index are
/// separate; unqualified/local names omit the namespace component entirely.
/// Jim root-flat keys decline without a separately established relationship.
/// This pure root purpose establishes no cell, compiler entry or frame.
#[must_use]
pub fn native_variable_root_is_under_namespace(
    protocol: NativeNameProtocol,
    current: NativeNameContext<'_>,
    root: &[u8],
    wanted: NativeNameContext<'_>,
) -> Option<bool> {
    match protocol.variable_root_geometry(current, root) {
        super::NativeVariableRootGeometry::Local(_) => Some(false),
        super::NativeVariableRootGeometry::CNamespace { namespace, .. } => Some(
            namespace
                .as_segments()
                .starts_with(wanted.namespace.as_segments()),
        ),
        // A Jim absolute variable key is one counted root-flat key, without
        // an intrinsic namespace/tail partition. Namespace-tail reporting
        // cannot donate that relationship to this variable purpose.
        super::NativeVariableRootGeometry::JimAbsolute(_) => None,
    }
}

#[cfg(test)]
mod operand_namespace_purpose_tests {
    use super::*;
    #[test]
    fn namespace_membership_excludes_command_and_variable_simple_tails() {
        // Implementation proof: naming.namespace.original-member-extent
        // docs/design/analysis/name-resolution-proofs/namespace-original-member-extent.md
        for version in tcl_dialect::TclVersion::ALL {
            let protocol = NativeNameProtocol::C(version);
            let wanted = tcl_core_types::ByteNamespacePath::from_segments(["old"]);
            let wanted = NativeNameContext::new(&wanted);
            for (written, under) in [
                (b"::old".as_slice(), false),
                (b"::old::p", true),
                (b"::other::old", false),
                (b"::old::inner::p", true),
            ] {
                let slot = protocol
                    .command_lookup_slot(NativeNameContext::root(), written)
                    .unwrap();
                assert_eq!(
                    native_command_slot_is_under_namespace(protocol, &slot, wanted),
                    Some(under)
                );
                assert_eq!(
                    native_variable_root_is_under_namespace(
                        protocol,
                        NativeNameContext::root(),
                        written,
                        wanted
                    ),
                    Some(under)
                );
            }
        }
        let protocol = NativeNameProtocol::Jim084;
        let root = NativeNameContext::root();
        let wanted = NativeNameContext::with_jim_namespace(root.namespace, b"old");
        for (written, under) in [
            (b"::old".as_slice(), false),
            (b"::old::p", true),
            (b"::other::old", false),
        ] {
            assert!(matches!(
                protocol.command_lookup_slot(root, written),
                Err(super::super::NameProjectionUnavailable::PurposeNotModelled)
            ));
            let keys = protocol.jim_command_lookup_keys(root, written).unwrap();
            let [key] = keys.as_slice() else {
                panic!("absolute Jim input has one root-flat candidate")
            };
            let slot = tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                key.clone(),
            );
            assert_eq!(
                native_command_slot_is_under_namespace(protocol, &slot, wanted),
                Some(under)
            );
            assert_eq!(
                native_variable_root_is_under_namespace(protocol, root, written, wanted),
                None
            );
        }
        let caller = NativeNameContext::with_jim_namespace(root.namespace, b"caller");
        let selected = |simple: &[u8]| {
            tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                simple.into(),
            )
        };
        assert_eq!(
            native_command_lookup_context_for_slot(
                protocol,
                caller,
                b"old::p",
                &selected(b"caller::old::p")
            )
            .unwrap()
            .jim_namespace_object,
            Some(b"caller".as_slice())
        );
        assert_eq!(
            native_command_lookup_context_for_slot(
                protocol,
                caller,
                b"old::p",
                &selected(b"old::p")
            )
            .unwrap()
            .jim_namespace_object,
            Some(b"".as_slice())
        );
        assert!(
            native_command_lookup_context_for_slot(
                protocol,
                caller,
                b"old::p",
                &selected(b"other::p")
            )
            .is_none()
        );
    }
}

/// Proposed namespace geometry, without a source producer or publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNamespaceRenameTarget {
    /// Exact C components.
    C(tcl_core_types::ByteNamespacePath),
    /// Exact Jim flat namespace object units.
    Jim(tcl_core_types::NameBytes),
}

/// Propose one final-component replacement using the same complete namespace
/// projector as source edits. The proposal can be compared for collisions;
/// it establishes no namespace existence, current lifetime or edit permission.
#[must_use]
pub fn native_namespace_rename_target(
    protocol: NativeNameProtocol,
    wanted: NativeNameContext<'_>,
    new_tail: &[u8],
) -> Option<NativeNamespaceRenameTarget> {
    let original = match protocol {
        NativeNameProtocol::C(_) => super::native_namespace_full_name_bytes(wanted.namespace),
        NativeNameProtocol::Jim084 => {
            let mut original = b"::".to_vec();
            original.extend_from_slice(wanted.jim_namespace_object?);
            original
        }
    };
    let NativeNamespaceOperandRename::Replace(replacement) = native_namespace_operand_rename(
        protocol,
        NativeNameContext::root(),
        &original,
        wanted,
        new_tail,
    )?
    else {
        return None;
    };
    match protocol {
        NativeNameProtocol::C(_) => Some(NativeNamespaceRenameTarget::C(
            protocol
                .namespace_address_path(NativeNameContext::root(), &replacement)
                .ok()?,
        )),
        NativeNameProtocol::Jim084 => Some(NativeNamespaceRenameTarget::Jim(
            protocol
                .jim_namespace_canonical_input(NativeNameContext::root(), &replacement)
                .ok()?
                .selected()
                .into(),
        )),
    }
}

#[cfg(test)]
mod namespace_rename_target_tests {
    use super::*;
    #[test]
    fn namespace_rename_targets_replace_one_opaque_component_and_reject_multiple_components() {
        // Implementation proof: naming.namespace.original-member-extent
        // docs/design/analysis/name-resolution-proofs/namespace-original-member-extent.md
        let protocol = NativeNameProtocol::C(tcl_dialect::TclVersion::V8_6);
        let wanted =
            tcl_core_types::ByteNamespacePath::from_segments([b"app".as_slice(), b"n\xed\xa0\x80"]);
        let result =
            native_namespace_rename_target(protocol, NativeNameContext::new(&wanted), b"new")
                .unwrap();
        assert_eq!(
            result,
            NativeNamespaceRenameTarget::C(tcl_core_types::ByteNamespacePath::from_segments([
                "app", "new"
            ]))
        );
        for tail in [b"".as_slice(), b"a::b", b"a\0hidden"] {
            assert!(
                native_namespace_rename_target(protocol, NativeNameContext::new(&wanted), tail)
                    .is_none()
            );
        }
        let root = NativeNameContext::root();
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(
            native_namespace_rename_target(
                jim,
                NativeNameContext::with_jim_namespace(root.namespace, b"app::old"),
                b"new"
            ),
            Some(NativeNamespaceRenameTarget::Jim(
                b"app::new".as_slice().into()
            ))
        );
        for tail in [b"a::b".as_slice(), b"a\0hidden"] {
            assert!(
                native_namespace_rename_target(
                    jim,
                    NativeNameContext::with_jim_namespace(root.namespace, b"app::old"),
                    tail
                )
                .is_none()
            );
        }
    }
}
