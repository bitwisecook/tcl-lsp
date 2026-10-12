// SPDX-License-Identifier: AGPL-3.0-or-later
//! Physical private error headers, independent of public return-options overlays.

use tcl_syntax::native_string::NativeStringProtocol;

/// Actual C interpreter's private List and Dictionary header availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeErrorObjectsProtocol {
    strings: NativeStringProtocol,
}
impl NativeErrorObjectsProtocol {
    /// Native member and updater recipe for these original headers.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }
    /// A private return-options Dictionary exists only after a modern return producer.
    #[must_use]
    pub const fn has_return_options(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(
                tcl_dialect::TclVersion::V8_5
                    | tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }
    /// TIP 348 creates an empty private List header at interpreter construction.
    #[must_use]
    pub const fn has_error_stack(self) -> bool {
        matches!(
            self.strings,
            NativeStringProtocol::C(
                tcl_dialect::TclVersion::V8_6
                    | tcl_dialect::TclVersion::V9_0
                    | tcl_dialect::TclVersion::V9_1
            )
        )
    }
}
impl crate::InvocationDialect {
    /// Issue private C header storage from the actual native core only.
    #[must_use]
    pub fn native_error_objects_protocol(self) -> Option<NativeErrorObjectsProtocol> {
        let strings = self.native_string_protocol()?;
        strings.tcl_version()?;
        Some(NativeErrorObjectsProtocol { strings })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_private_header_fields_match_five_native_constructor_and_save_windows() {
        // Native proof: naming.error.original-private-return-options-header
        // docs/design/analysis/name-resolution-proofs/error-original-private-return-options-header.md
        // Native proof: naming.error.original-private-error-stack-header
        // docs/design/analysis/name-resolution-proofs/error-original-private-error-stack-header.md
        let fixture = include_str!("../tests/data/native_error_headers/observations.tsv");
        let mut rows = 0;
        for (name, version) in [
            ("8.4.20", tcl_dialect::TclVersion::V8_4),
            ("8.5.19", tcl_dialect::TclVersion::V8_5),
            ("8.6.18", tcl_dialect::TclVersion::V8_6),
            ("9.0.4", tcl_dialect::TclVersion::V9_0),
            ("9.1.0", tcl_dialect::TclVersion::V9_1),
        ] {
            let recipe = crate::InvocationDialect::for_version(version)
                .native_error_objects_protocol()
                .unwrap();
            let controls: Vec<_> = fixture
                .lines()
                .filter_map(|line| line.strip_prefix(&format!("{name}\t")))
                .collect();
            assert!(!controls.is_empty());
            for line in controls {
                let fields: Vec<_> = line.split('\t').collect();
                rows += 1;
                match fields[0] {
                    "initial-options" => assert_eq!(
                        fields[1],
                        if recipe.has_return_options() {
                            "absent"
                        } else {
                            "absent-field"
                        }
                    ),
                    "initial-stack" => {
                        assert_eq!(
                            fields[1],
                            if recipe.has_error_stack() {
                                "none"
                            } else {
                                "absent-field"
                            }
                        );
                        if recipe.has_error_stack() {
                            assert_eq!(fields[2], "1");
                        }
                    }
                    "options-before" => assert_eq!(&fields[1..], &["dict", "1", "none", "1"]),
                    "stack-before" => assert_eq!(&fields[1..], &["list", "1", "2", "none", "1"]),
                    "options-saved" | "stack-saved" => assert_eq!(&fields[1..], &["2", "1"]),
                    "options-mutated" | "stack-mutated" | "options-restored" | "stack-restored" => {
                        assert_eq!(&fields[1..], &["1", "1", "1"]);
                    }
                    _ => panic!("unexpected native header stage"),
                }
            }
        }
        assert_eq!(rows, 38);
        assert!(
            crate::InvocationDialect::of_profile(
                crate::model::resolve_environment("jim").unit_profile()
            )
            .native_error_objects_protocol()
            .is_none()
        );
    }
}
