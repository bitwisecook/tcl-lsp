// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native alias local-tail selection from original materialized operands.
use super::NativeNameProtocol;

/// The local binding selected by `global`. Jim leaves rooted names alone;
/// C scans its `CString` qualification and retains the original unqualified
/// object when no namespace separator occurred.
#[must_use]
pub fn global_local_name_bytes(protocol: NativeNameProtocol, original: &[u8]) -> Option<Vec<u8>> {
    if protocol.is_jim084() {
        return (!original.starts_with(b"::")).then(|| original.to_vec());
    }
    Some(c_local_tail(protocol, original))
}

/// The local alias selected by native namespace `variable`. C uses its
/// qualification tail; Jim uses `Jim_NamespaceTail` while retaining the full
/// canonical target name. Element-target validation remains separate.
#[must_use]
pub fn variable_local_name_bytes(protocol: NativeNameProtocol, original: &[u8]) -> Vec<u8> {
    if protocol.is_jim084() {
        protocol.namespace_tail_bytes(original).to_vec()
    } else {
        c_local_tail(protocol, original)
    }
}

/// Release-independent C local-alias projection for authored assistance.
/// An unqualified NUL-bearing name needs an exact release and remains opaque.
/// This supplies no native lookup or physical binding authority.
#[must_use]
pub fn c_family_local_alias_name_bytes(original: &[u8]) -> Option<Vec<u8>> {
    if let Some(tail) = c_qualification_tail(original) {
        Some(tail.to_vec())
    } else {
        (!original.contains(&0)).then(|| original.to_vec())
    }
}

fn c_local_tail(protocol: NativeNameProtocol, original: &[u8]) -> Vec<u8> {
    c_qualification_tail(original).map_or_else(
        || protocol.variable_root_input(original).selected().to_vec(),
        <[u8]>::to_vec,
    )
}

fn c_qualification_tail(original: &[u8]) -> Option<&[u8]> {
    let visible = &original[..original
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(original.len())];
    let mut tail = None;
    let mut at = 0;
    while at + 1 < visible.len() {
        if visible[at..].starts_with(b"::") {
            at += 2;
            while visible.get(at) == Some(&b':') {
                at += 1;
            }
            tail = Some(at);
        } else {
            at += 1;
        }
    }
    tail.map(|at| &visible[at..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;
    #[test]
    fn unversioned_c_alias_assistance_withdraws_release_dependent_names() {
        assert_eq!(
            c_family_local_alias_name_bytes(b"R:::v"),
            Some(b"v".to_vec())
        );
        assert_eq!(
            c_family_local_alias_name_bytes(b"R:::v\0tail"),
            Some(b"v".to_vec())
        );
        assert_eq!(c_family_local_alias_name_bytes(b"v\0tail"), None);
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::for_tcl_version(version);
            for name in [b"v".as_slice(), b"R:::v", b"R:::v\0tail"] {
                assert_eq!(
                    c_family_local_alias_name_bytes(name),
                    global_local_name_bytes(protocol, name)
                );
                assert_eq!(
                    c_family_local_alias_name_bytes(name),
                    Some(variable_local_name_bytes(protocol, name))
                );
            }
        }
    }
    #[test]
    fn jim_variable_alias_uses_native_tail_without_changing_full_target() {
        let jim = NativeNameProtocol::Jim084;
        assert_eq!(variable_local_name_bytes(jim, b"R:::v"), b"v");
        assert_eq!(variable_local_name_bytes(jim, b"R::v\0tail"), b"v");
        assert_eq!(
            variable_local_name_bytes(jim, b"v\0tail::other"),
            b"v\0tail::other"
        );
        assert_eq!(
            variable_local_name_bytes(jim, b"R::v\xc0\x80tail"),
            b"v\xc0\x80tail"
        );
        assert_eq!(variable_local_name_bytes(jim, b"R::v\xff"), b"v\xff");
    }

    #[test]
    fn local_tail_preserves_unqualified_full_object_but_qualified_cstring() {
        for version in TclVersion::ALL {
            let protocol = NativeNameProtocol::for_tcl_version(version);
            let expected: &[u8] = if version == TclVersion::V8_4 {
                b"a"
            } else {
                b"a\0z"
            };
            assert_eq!(
                global_local_name_bytes(protocol, b"a\0z").unwrap(),
                expected
            );
            assert_eq!(
                global_local_name_bytes(protocol, b"::ns::a\0z").unwrap(),
                b"a"
            );
            assert_eq!(variable_local_name_bytes(protocol, b"::ns:::a\0z"), b"a");
        }
        assert_eq!(
            global_local_name_bytes(NativeNameProtocol::Jim084, b"::ns::a"),
            None
        );
        assert_eq!(
            global_local_name_bytes(NativeNameProtocol::Jim084, b"ns::a\0z").unwrap(),
            b"ns::a\0z"
        );
    }

    #[test]
    fn local_alias_names_match_original_native_declarations() {
        let script = include_bytes!("../../tests/data/native_alias_names/probe.tcl");
        let expected = |protocol| {
            let mut output = String::new();
            for (label, global, name) in [
                ("a", true, b"::R:::v".as_slice()),
                ("b", false, b"::R:::v".as_slice()),
                ("c", true, b"R:::v".as_slice()),
                ("d", false, b"R:::v".as_slice()),
            ] {
                let local = if global {
                    global_local_name_bytes(protocol, name).unwrap_or_default()
                } else {
                    variable_local_name_bytes(protocol, name)
                };
                use std::fmt::Write as _;
                writeln!(output, "{label}:0:{}", std::str::from_utf8(&local).unwrap()).unwrap();
            }
            output.trim().to_owned()
        };
        let references = tcl_test_support::available_tclshs();
        assert!(!references.is_empty(), "a C Tcl reference is required");
        for reference in references {
            let version = reference.version;
            let actual = tcl_test_support::run_script(&reference.path, script)
                .expect("run original native declarations")
                .strict_text()
                .expect("clean native declaration result");
            assert_eq!(
                actual,
                expected(NativeNameProtocol::for_tcl_version(version)),
                "{}",
                reference.patchlevel
            );
        }
        if let Some(jim) = tcl_test_support::locate_jimsh().expect("valid Jim reference") {
            let actual = tcl_test_support::run_script(&jim.path, script)
                .expect("run original Jim declarations")
                .strict_text()
                .expect("clean Jim declaration result");
            assert_eq!(actual, expected(NativeNameProtocol::Jim084));
        }
    }
}
