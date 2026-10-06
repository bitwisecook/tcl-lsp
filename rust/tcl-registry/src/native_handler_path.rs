// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original nested ensemble dependencies of a reached normal value handler.

use crate::{InvocationArguments, native_compilation::NativeCompilerImplementationLookup};

/// Exact frozen operand selecting one audited nested native dispatch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeHandlerLookupPaths {
    /// Operand relative to the selected semantic subcommand's argument view.
    pub argument: usize,
    /// Authored literal selections. Missing or unknown values supply no path.
    pub alternatives: &'static [NativeHandlerLookupPath],
}

/// One original public-to-private dispatch chain, in actual lookup order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeHandlerLookupPath {
    /// Exact frozen selector value; abbreviations require separate evidence.
    pub value: &'static str,
    /// Each intermediate ensemble and terminal worker must independently match.
    pub lookups: &'static [NativeCompilerImplementationLookup],
}

impl NativeHandlerLookupPaths {
    /// Select descriptor dependencies without proving any live implementation.
    /// Dynamic values, expansion or unlisted selections retain uncertainty.
    #[must_use]
    pub fn select(
        &'static self,
        arguments: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<&'static [NativeCompilerImplementationLookup]> {
        arguments.exact_argv_len()?;
        let value = arguments.literal_at(argument_offset.checked_add(self.argument)?)?;
        let mut paths = self.alternatives.iter().filter(|path| path.value == value);
        let selected = paths.next()?;
        paths.next().is_none().then_some(selected.lookups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InvocationDialect, InvocationWord, native_compilation::NormalHandlerImplementationLookup,
    };
    use tcl_dialect::TclVersion;

    fn encoding_paths() -> &'static NativeHandlerLookupPaths {
        let registry = crate::CommandRegistry::build_default();
        let facts = registry
            .resolve_invocation(
                "binary",
                &["encode", "hex", "data"],
                registry.own_surface_query(),
            )
            .unwrap()
            .facts();
        let NormalHandlerImplementationLookup::RequiredPath(paths) = facts
            .normal_handler_implementation_lookup(Some(InvocationDialect::for_version(
                TclVersion::V8_6,
            )))
        else {
            panic!("audited encoding path descriptor");
        };
        paths
    }

    #[test]
    fn selected_encoding_path_retains_each_original_dispatch_edge() {
        let paths = encoding_paths();
        for format in ["hex", "base64", "uuencode"] {
            let words = ["encode", format, "data"];
            let selected = paths
                .select(InvocationArguments::literals(&words), 1)
                .unwrap();
            assert_eq!(selected.len(), 2);
            assert_eq!(selected[0].ensemble, "binary");
            assert_eq!(selected[0].slot, "::tcl::binary::encode");
            assert_eq!(selected[0].prepended, &["encode"]);
            assert_eq!(selected[1].ensemble, selected[0].slot);
            assert_eq!(selected[1].member, format);
            assert_eq!(selected[1].slot, format!("::tcl::binary::encode::{format}"));
            assert_eq!(selected[1].prepended, &["encode", format]);
        }
    }

    #[test]
    fn unknown_selector_cardinality_or_duplicate_path_cannot_supply_dependencies() {
        static DUPLICATE: NativeHandlerLookupPaths = NativeHandlerLookupPaths {
            argument: 0,
            alternatives: &[
                NativeHandlerLookupPath {
                    value: "hex",
                    lookups: &[],
                },
                NativeHandlerLookupPath {
                    value: "hex",
                    lookups: &[],
                },
            ],
        };
        let paths = encoding_paths();
        for words in [
            &[InvocationWord::Dynamic, InvocationWord::Literal("data")][..],
            &[InvocationWord::Literal("hex"), InvocationWord::Expanded][..],
            &[InvocationWord::Literal("hex"), InvocationWord::Opaque][..],
            &[
                InvocationWord::Literal("h"),
                InvocationWord::Literal("data"),
            ][..],
            &[
                InvocationWord::Literal("unknown"),
                InvocationWord::Literal("data"),
            ][..],
            &[][..],
        ] {
            assert!(
                paths
                    .select(InvocationArguments::structured(words), 0)
                    .is_none()
            );
        }
        assert!(
            DUPLICATE
                .select(InvocationArguments::literals(&["hex"]), 0)
                .is_none()
        );
        assert!(
            paths
                .select(InvocationArguments::literals(&["hex"]), usize::MAX)
                .is_none()
        );
    }

    #[test]
    fn encoding_path_requires_its_authored_actual_native_release() {
        let registry = crate::CommandRegistry::build_default();
        let words = ["encode", "hex", "data"];
        let facts = registry
            .resolve_invocation("binary", &words, registry.own_surface_query())
            .unwrap()
            .facts();
        for version in TclVersion::ALL {
            let lookup = facts.normal_handler_implementation_lookup(Some(
                InvocationDialect::for_version(version),
            ));
            assert_eq!(
                matches!(lookup, NormalHandlerImplementationLookup::RequiredPath(_)),
                version >= TclVersion::V8_6
            );
            if let NormalHandlerImplementationLookup::RequiredPath(paths) = lookup {
                assert_eq!(
                    paths
                        .select(InvocationArguments::literals(&words), facts.argument_offset)
                        .unwrap()
                        .len(),
                    2
                );
            } else {
                assert_eq!(lookup, NormalHandlerImplementationLookup::Unknown);
            }
        }
        for environment in ["jim", "f5-irules"] {
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            assert_eq!(
                facts.normal_handler_implementation_lookup(Some(InvocationDialect::of_profile(
                    profile
                ))),
                NormalHandlerImplementationLookup::Unknown
            );
        }
        assert_eq!(
            facts.normal_handler_implementation_lookup(None),
            NormalHandlerImplementationLookup::Unknown
        );
        assert_eq!(
            facts.native_compilation,
            registry
                .get_for_surface("binary", registry.own_surface_query())
                .unwrap()
                .native_compilation,
            "a normal path retains only the independently authored parent compiler descriptor"
        );
    }

    #[test]
    fn stock_inventory_retains_all_nested_edges_only_in_the_authored_release() {
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let slots = registry
                .stock_native_implementation_slots(InvocationDialect::for_version(version))
                .into_iter()
                .filter(|lookup| lookup.slot.starts_with("::tcl::binary::encode"))
                .collect::<Vec<_>>();
            if version < TclVersion::V8_6 {
                assert!(slots.is_empty(), "{version:?}");
                continue;
            }
            assert_eq!(slots.len(), 4, "{version:?}");
            for path in encoding_paths().alternatives {
                for lookup in path.lookups {
                    assert!(slots.contains(lookup), "{version:?}: {lookup:?}");
                }
            }
        }
        for environment in ["jim", "f5-irules"] {
            let profile = crate::model::ingress::resolve_environment(environment).unit_profile();
            assert!(
                registry
                    .stock_native_implementation_slots(InvocationDialect::of_profile(profile))
                    .iter()
                    .all(|lookup| !lookup.slot.starts_with("::tcl::binary::encode")),
                "{environment}"
            );
        }
    }

    #[test]
    fn reverse_registration_retains_prefix_without_fabricating_terminal_facts() {
        let registry = crate::CommandRegistry::build_default();
        let dialect = InvocationDialect::for_version(TclVersion::V8_6);
        for format in ["hex", "base64", "uuencode"] {
            let head = format!("::tcl::binary::encode::{format}");
            assert_eq!(
                registry.native_registration_operand_prefix(&head, dialect),
                Some(["encode", format].as_slice())
            );
            let words = crate::InvocationWords::literals(&head, &["data"]).with_dialect(dialect);
            // The semantic encode row still includes the format operand. Until
            // that layout can be rebased, a terminal worker supplies no facts.
            assert!(
                registry
                    .native_registration_invocation_facts(words)
                    .is_none()
            );
            assert!(registry.native_registration_success_facts(words).is_none());
            assert_eq!(
                registry
                    .native_compilation_for_registration(&head, dialect)
                    .expect("independently authored terminal compiler registration")
                    .compiler_hook_presence(dialect),
                Some(format == "hex")
            );
        }
        let words = crate::InvocationWords::literals("::tcl::binary::encode", &["hex", "data"])
            .with_dialect(dialect);
        assert_eq!(
            registry
                .native_registration_invocation_facts(words)
                .unwrap()
                .argument_offset,
            0
        );
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert!(
                registry
                    .native_registration_operand_prefix(
                        "::tcl::binary::encode::hex",
                        InvocationDialect::for_version(version),
                    )
                    .is_none()
            );
        }
    }
}
