// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Candidate keys of C Tcl's `init.tcl` autoload library.

use crate::{native_string::NativeStringProtocol, native_tcl_utf::NativeTclUtf};

/// Native resident-byte keys of C `init.tcl`'s `auto_qualify`.
///
/// A matched separator rebuilds the command through the selected native
/// Unicode encoder, as `regsub` does. With no match the original counted bytes
/// survive. Namespace bytes are the actual constructed context, not another
/// written name to normalise. This pure recipe grants no original-object
/// getter authority or actual library availability. Jim has no C recipe.
#[must_use]
pub fn native_autoload_command_candidates(
    protocol: NativeStringProtocol,
    command: &[u8],
    namespace: &[u8],
) -> Option<Vec<Vec<u8>>> {
    let NativeStringProtocol::C(version) = protocol else {
        return None;
    };
    let utf = NativeTclUtf::for_version(version);
    let (units, separators) = normalise_colon_runs(&utf.decode_units(command), u32::from(b':'));
    let cleaned = if separators == 0 {
        command.to_vec()
    } else {
        utf.encode_units(&units)?
    };
    Some(ordered_candidates(cleaned, separators, namespace))
}

fn normalise_colon_runs<T: Copy + Eq>(input: &[T], colon: T) -> (Vec<T>, usize) {
    let mut cleaned = Vec::with_capacity(input.len());
    let mut separators = 0;
    let mut at = 0;
    while at < input.len() {
        if input[at] != colon {
            cleaned.push(input[at]);
            at += 1;
            continue;
        }
        let start = at;
        while input.get(at) == Some(&colon) {
            at += 1;
        }
        if at - start >= 2 {
            cleaned.extend_from_slice(&[colon, colon]);
            separators += 1;
        } else {
            cleaned.push(colon);
        }
    }
    (cleaned, separators)
}

fn ordered_candidates(cleaned: Vec<u8>, separators: usize, namespace: &[u8]) -> Vec<Vec<u8>> {
    if let Some(unrooted) = cleaned.strip_prefix(b"::") {
        return vec![if separators > 1 {
            cleaned
        } else {
            unrooted.to_vec()
        }];
    }
    if separators == 0 {
        if namespace == b"::" {
            vec![cleaned]
        } else {
            vec![qualified(namespace, &cleaned), cleaned]
        }
    } else if namespace == b"::" {
        vec![qualified(b"", &cleaned)]
    } else {
        vec![qualified(namespace, &cleaned), qualified(b"", &cleaned)]
    }
}

fn qualified(namespace: &[u8], command: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(namespace.len() + command.len() + 2);
    key.extend_from_slice(namespace);
    key.extend_from_slice(b"::");
    key.extend_from_slice(command);
    key
}

/// Ordered C library autoload keys in the analytical Unicode domain.
///
/// This preserves Unicode while normalising command colon runs. Native
/// resident bytes use [`native_autoload_command_candidates`] and their actual
/// C string protocol. Candidate keys prove neither a loader nor a binding.
#[must_use]
pub fn autoload_command_candidates(command: &str, namespace: &str) -> Vec<String> {
    let (cleaned, separators) = normalise_colon_runs(command.as_bytes(), b':');
    ordered_candidates(cleaned, separators, namespace.as_bytes())
        .into_iter()
        .map(|key| String::from_utf8(key).expect("ASCII colon replacement preserves Unicode"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_autoload_keys_follow_original_regsub_rebuilding() {
        // Native proof: naming.autoload.separator-rebuild-opaque
        // docs/design/analysis/name-resolution-proofs/autoload-separator-rebuild-opaque.md
        // Native proof: naming.autoload.separator-rebuild-zero
        // docs/design/analysis/name-resolution-proofs/autoload-separator-rebuild-zero.md
        // Native proof: naming.autoload.root-prefix-rebuild-zero
        // docs/design/analysis/name-resolution-proofs/autoload-root-prefix-rebuild-zero.md
        // Native proof: naming.autoload.unqualified-opaque-preservation
        // docs/design/analysis/name-resolution-proofs/autoload-unqualified-opaque-preservation.md
        let fixtures = [
            include_str!("../../tests/data/native_autoload_keys/8.4.20.tsv"),
            include_str!("../../tests/data/native_autoload_keys/8.5.19.tsv"),
            include_str!("../../tests/data/native_autoload_keys/8.6.18.tsv"),
            include_str!("../../tests/data/native_autoload_keys/9.0.4.tsv"),
            include_str!("../../tests/data/native_autoload_keys/9.1.0.tsv"),
        ];
        let commands: [&[u8]; 5] = [
            b"::a\xff::b",
            b"a\0::b",
            b"::a\0x",
            b"a\xff",
            b"a\xc0\x80::b",
        ];
        for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
            use std::fmt::Write as _;
            let protocol = NativeStringProtocol::C(version);
            let mut observed = String::new();
            for (index, command) in commands.iter().enumerate() {
                write!(observed, "{index} code0").unwrap();
                for key in native_autoload_command_candidates(protocol, command, b"::n").unwrap() {
                    observed.push(' ');
                    for byte in key {
                        write!(observed, "{byte:02x}").unwrap();
                    }
                }
                observed.push('\n');
            }
            assert_eq!(observed, fixture, "{version:?}");
            assert_eq!(
                native_autoload_command_candidates(protocol, b"x", b"::n:::m"),
                Some(vec![b"::n:::m::x".to_vec(), b"x".to_vec()])
            );
        }
        assert!(
            native_autoload_command_candidates(NativeStringProtocol::Jim084, b"x", b"::").is_none()
        );
    }

    #[test]
    fn unicode_autoload_keys_match_actual_c_libraries() {
        let cases = [
            ("café", "::"),
            ("café", "::工具"),
            ("工具:::café", "::应用"),
            (":::café", "::工具"),
            (":::工具::::café", "::应用"),
            (":café", "::工具"),
            ("", "::工具"),
            ("::", "::工具"),
        ];
        let references = tcl_test_support::available_tclshs();
        assert!(!references.is_empty(), "a C Tcl reference is required");
        for reference in references {
            for (command, namespace) in cases {
                let invocation = crate::list::join_list(["auto_qualify", command, namespace]);
                let script = format!(
                    "foreach key [{invocation}] {{binary scan [encoding convertto utf-8 $key] H* hex; puts :$hex}}"
                );
                let native = tcl_test_support::run_script(&reference.path, script.as_bytes())
                    .expect("execute actual autoload library")
                    .strict_text()
                    .expect("clean autoload result");
                let actual = autoload_command_candidates(command, namespace)
                    .iter()
                    .map(|key| {
                        use std::fmt::Write as _;
                        let mut hex = String::from(":");
                        for byte in key.as_bytes() {
                            write!(hex, "{byte:02x}").expect("write String");
                        }
                        hex
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert_eq!(
                    actual, native,
                    "{}: {command:?}/{namespace:?}",
                    reference.patchlevel
                );
            }
        }
    }
}
