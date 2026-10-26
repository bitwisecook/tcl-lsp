// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded UTF-8 Logical source geometry, independently of Native naming authority.

use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};

fn source_units(current: &ByteNamespacePath, written: &[u8]) -> Option<()> {
    (std::str::from_utf8(written).is_ok()
        && !written.contains(&0)
        && current.as_segments().iter().all(|segment| {
            std::str::from_utf8(segment.as_bytes()).is_ok() && !segment.as_bytes().contains(&0)
        }))
    .then_some(())
}

/// Construct a bounded UTF-8 authored command slot under a retained Logical
/// source namespace. Written qualifiers use the shared parser; constructed
/// namespace components are appended without reparsing. No Native name recipe,
/// namespace existence, table binding or publication follows from this slot.
#[must_use]
pub fn authored_source_command_slot(
    current: &ByteNamespacePath,
    written: &[u8],
) -> Option<ByteCommandSlot> {

    source_units(current, written)?;
    let parts = super::qualifier_segments(written);
    let simple = super::written_command_tail(written);
    let parent_count = parts.len().checked_sub(usize::from(!simple.is_empty()))?;
    let parents = &parts[..parent_count];
    let mut namespace = if written.starts_with(b"::") {
        ByteNamespacePath::root()
    } else {
        current.clone()
    };
    for parent in parents {
        namespace.push(NameBytes::from(*parent));
    }
    Some(ByteCommandSlot::new(namespace, NameBytes::from(simple)))
}

/// Absolute insertion data for an already retained Logical source coordinate.
/// An ambiguous written round trip declines; this grants no Native lookup.
#[must_use]
pub fn authored_source_command_spelling(slot: &ByteCommandSlot) -> Option<String> {
    source_units(&slot.namespace, slot.simple.as_bytes())?;
    let namespace = super::checked_namespace_path_key(&slot.namespace)?;
    let simple = std::str::from_utf8(slot.simple.as_bytes()).ok()?;
    let spelling = super::qualify(&namespace, simple);
    (authored_source_command_slot(&ByteNamespacePath::root(), spelling.as_bytes())? == *slot)
        .then_some(spelling)
}

/// Ordered Logical source candidates from independently retained namespace
/// components. This is conditional metadata geometry, not native lookup.
#[must_use]
pub fn authored_source_command_candidates(
    current: &ByteNamespacePath,
    path: &[ByteNamespacePath],
    written: &[u8],
) -> Option<Vec<ByteCommandSlot>> {

    source_units(current, written)?;
    let mut result = Vec::new();
    let root = ByteNamespacePath::root();
    for base in super::command_lookup_bases(written.starts_with(b"::"), current, path) {
        let namespace = match base {
            super::CommandLookupBase::Context(namespace) => namespace,
            super::CommandLookupBase::Root => &root,
        };
        let key = authored_source_command_slot(namespace, written)?;
        if !result.contains(&key) {
            result.push(key);
        }
    }
    Some(result)
}

/// Retain a bounded authored namespace recipe. It provides source scope only;
/// a selected native input, allocated namespace or frame remains independent.
#[must_use]
pub fn authored_source_namespace_path(
    current: &ByteNamespacePath,
    written: &[u8],
) -> Option<ByteNamespacePath> {

    source_units(current, written)?;
    let mut namespace = if written.starts_with(b"::") {
        ByteNamespacePath::root()
    } else {
        current.clone()
    };
    for part in super::qualifier_segments(written) {
        if !part.is_empty() {
            namespace.push(NameBytes::from(part));
        }
    }
    Some(namespace)
}

#[cfg(test)]
mod tests {
    use super::{
        authored_source_command_candidates, authored_source_command_slot,
        authored_source_namespace_path,
    };
    use tcl_core_types::{ByteNamespacePath, NameBytes};

    #[test]
    fn original_logical_command_spelling_preserves_coordinate_round_trips() {
        let root = ByteNamespacePath::root();
        for (written, expected) in [
            (b"N:::helper".as_slice(), "::N::helper"),
            (b"N::", "::N::"),
            (b":single", ":::single"),
        ] {
            let slot = authored_source_command_slot(&root, written).unwrap();
            let spelling = super::authored_source_command_spelling(&slot);
            if written == b":single" {
                assert!(spelling.is_none());
            } else {
                assert_eq!(spelling.as_deref(), Some(expected));
                assert_eq!(
                    authored_source_command_slot(&root, spelling.unwrap().as_bytes()),
                    Some(slot)
                );
            }
        }
        let mut path = root.clone();
        path.push(NameBytes::from(b"literal::component".as_slice()));
        let slot = authored_source_command_slot(&path, b"helper").unwrap();
        assert!(super::authored_source_command_spelling(&slot).is_none());
    }

    #[test]
    fn original_authored_scope_preserves_constructed_components_and_empty_tail() {

        let mut current = ByteNamespacePath::root();
        current.push(NameBytes::from(&b"literal::component"[..]));
        let slot = authored_source_command_slot(&current, b"child:::command").unwrap();
        assert_eq!(
            slot.namespace.as_segments()[0].as_bytes(),
            b"literal::component"
        );
        assert_eq!(slot.namespace.as_segments()[1].as_bytes(), b"child");
        assert_eq!(slot.simple.as_bytes(), b"command");
        let empty = authored_source_command_slot(&current, b"child::").unwrap();
        assert!(empty.simple.as_bytes().is_empty());
        assert_eq!(empty.namespace, slot.namespace);
        let root = authored_source_command_slot(&current, b"::command").unwrap();
        assert!(root.namespace.is_root());
        let namespace = authored_source_namespace_path(&current, b"child").unwrap();
        assert_eq!(namespace, slot.namespace);
    }

    #[test]
    fn original_authored_candidates_keep_order_and_decline_unmodelled_units() {

        let mut current = ByteNamespacePath::root();
        current.push(NameBytes::from(&b"current"[..]));
        let mut search = ByteNamespacePath::root();
        search.push(NameBytes::from(&b"search"[..]));
        let candidates =
            authored_source_command_candidates(&current, &[search.clone()], b"command").unwrap();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].namespace, current);
        assert_eq!(candidates[1].namespace, search);
        assert!(candidates[2].namespace.is_root());
        assert_eq!(
            authored_source_command_candidates(&current, &[search], b"::command")
                .unwrap()
                .len(),
            1
        );
        for bytes in [&b"name\0suffix"[..], &b"name\xff"[..]] {
            assert!(authored_source_command_slot(&current, bytes).is_none());
            assert!(authored_source_namespace_path(&current, bytes).is_none());
        }
    }
    #[test]
    fn original_authored_unicode_scope_keeps_raw_components_and_canonical_colons() {

        // Conditional Logical geometry only; no Native name or table binding.
        let mut current = ByteNamespacePath::root();
        current.push(NameBytes::from("λ::retained".as_bytes()));
        let slot =
            authored_source_command_slot(&current, "café:::\u{feff}puts".as_bytes()).unwrap();
        assert_eq!(
            slot.namespace.as_segments()[0].as_bytes(),
            "λ::retained".as_bytes()
        );
        assert_eq!(
            slot.namespace.as_segments()[1].as_bytes(),
            "café".as_bytes()
        );
        assert_eq!(slot.simple.as_bytes(), "\u{feff}puts".as_bytes());
        let empty = authored_source_command_slot(&current, "café::".as_bytes()).unwrap();
        assert_eq!(empty.namespace, slot.namespace);
        assert!(empty.simple.as_bytes().is_empty());
        let namespace = authored_source_namespace_path(&current, "café".as_bytes()).unwrap();
        assert_eq!(namespace, slot.namespace);
        let root =
            authored_source_command_slot(&current, "::café::\u{feff}puts".as_bytes()).unwrap();
        assert_eq!(root.namespace.as_segments().len(), 1);
        assert_eq!(
            root.namespace.as_segments()[0].as_bytes(),
            "café".as_bytes()
        );
        assert_eq!(root.simple, slot.simple);
        let composed = authored_source_command_slot(&current, "é".as_bytes()).unwrap();
        let decomposed = authored_source_command_slot(&current, "e\u{301}".as_bytes()).unwrap();
        assert_ne!(composed.simple, decomposed.simple);
        let candidates = authored_source_command_candidates(
            &current,
            std::slice::from_ref(&namespace),
            "λ".as_bytes(),
        )
        .unwrap();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].namespace, current);
        assert_eq!(candidates[1].namespace, namespace);
        assert!(candidates[2].namespace.is_root());
        assert!(
            candidates
                .iter()
                .all(|slot| slot.simple.as_bytes() == "λ".as_bytes())
        );
    }

    #[test]
    fn original_authored_unicode_scope_refuses_invalid_written_and_retained_units() {

        let root = ByteNamespacePath::root();
        for written in [
            b"name\0suffix".as_slice(),
            b"name\xff".as_slice(),
            b"\xc0\x80".as_slice(),
            b"\xed\xa0\x80".as_slice(),
        ] {
            assert!(authored_source_command_slot(&root, written).is_none());
            assert!(authored_source_command_candidates(&root, &[], written).is_none());
            assert!(authored_source_namespace_path(&root, written).is_none());
        }
        for segment in [
            b"retained\0suffix".as_slice(),
            b"retained\xff".as_slice(),
            b"\xc0\x80".as_slice(),
        ] {
            let mut current = ByteNamespacePath::root();
            current.push(NameBytes::from(segment));
            assert!(authored_source_command_slot(&current, "λ".as_bytes()).is_none());
            assert!(authored_source_namespace_path(&current, "λ".as_bytes()).is_none());
            assert!(
                authored_source_command_candidates(&root, &[current], "λ".as_bytes()).is_none()
            );
        }
    }
}
