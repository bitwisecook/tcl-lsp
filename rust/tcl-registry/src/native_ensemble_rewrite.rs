// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual ensemble usage rewrite reset order, independent of handler parsers.

use crate::InvocationDialect;
use tcl_dialect::TclVersion;

/// Reached native evaluation event; internal forwarding invokes emit no
/// ordinary lookup event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsembleRewriteResetEvent {
    /// A successfully admitted bytecode body begins execution.
    BytecodeEntry,
    /// Ordinary object-vector evaluation is about to resolve its command.
    BeforeOrdinaryLookup,
    /// Ordinary command lookup has selected an actual command.
    AfterSuccessfulOrdinaryLookup,
}

/// Explicit logical evaluation capability, without native ensemble authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalEnsembleRewriteProvider {
    /// The authored F5 Tcl 8.4 core has no native ensemble rewrite state.
    Tcl84CoreSimulation,
}

/// Selected reset recipe. Tcl 8.5 resets after successful command lookup;
/// newer C engines reset before lookup and on bytecode entry. Tcl 8.4 and Jim
/// have no C ensemble rewrite state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEnsembleRewriteProtocol {
    version: Option<TclVersion>,
}

impl NativeEnsembleRewriteProtocol {
    /// Whether the reached event withdraws the active usage rewrite.
    /// A failed lookup does not emit an after-success event.
    #[must_use]
    pub const fn resets_at(self, event: EnsembleRewriteResetEvent) -> bool {
        matches!(
            (self.version, event),
            (
                Some(TclVersion::V8_5),
                EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup
            ) | (
                Some(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1),
                EnsembleRewriteResetEvent::BeforeOrdinaryLookup
                    | EnsembleRewriteResetEvent::BytecodeEntry,
            )
        )
    }
}

impl InvocationDialect {
    /// Select actual reset order from authenticated engine axes.
    #[must_use]
    pub fn native_ensemble_rewrite_protocol(self) -> Option<NativeEnsembleRewriteProtocol> {
        self.native_string_protocol()
            .map(|protocol| NativeEnsembleRewriteProtocol {
                version: protocol.tcl_version(),
            })
    }

    /// Select actual evaluation or an explicitly installed logical provider.
    /// Compatibility versions alone do not grant a native reset recipe.
    #[must_use]
    pub fn ensemble_rewrite_protocol(
        self,
        logical: Option<LogicalEnsembleRewriteProvider>,
    ) -> Option<NativeEnsembleRewriteProtocol> {
        if let Some(protocol) = self.native_ensemble_rewrite_protocol() {
            return Some(protocol);
        }
        let LogicalEnsembleRewriteProvider::Tcl84CoreSimulation = logical?;
        self.authored_f5_tcl84_core()?;
        Some(NativeEnsembleRewriteProtocol {
            version: Some(TclVersion::V8_4),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_events_retain_native_lookup_and_execution_order() {
        use EnsembleRewriteResetEvent as Event;
        let mut count = 0;
        for (version, observations) in [
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_ensemble_rewrite/reset-8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_ensemble_rewrite/reset-8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_ensemble_rewrite/reset-9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_ensemble_rewrite/reset-9.1.0.tsv"),
            ),
        ] {
            let protocol = InvocationDialect::for_version(version)
                .native_ensemble_rewrite_protocol()
                .unwrap();
            for row in observations.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                let receipt: Vec<_> = fields[1].split(' ').collect();
                assert_eq!(fields[2], "0", "{version:?}: {row}");
                assert_eq!(receipt[0], "before=1", "{version:?}: {row}");
                let absent_after = receipt[2] == "after=0";
                match fields[0] {
                    "lookup" => {
                        let absent_at_lookup = receipt[1] == "lookup=0";
                        assert_eq!(
                            protocol.resets_at(Event::BeforeOrdinaryLookup),
                            absent_at_lookup,
                            "{version:?}: {row}"
                        );
                        assert_eq!(
                            protocol.resets_at(Event::AfterSuccessfulOrdinaryLookup),
                            !absent_at_lookup && absent_after,
                            "{version:?}: {row}"
                        );
                    }
                    "bytecode" => assert_eq!(
                        protocol.resets_at(Event::BytecodeEntry),
                        absent_after,
                        "{version:?}: {row}"
                    ),
                    "missing" => assert_eq!(
                        protocol.resets_at(Event::BeforeOrdinaryLookup),
                        absent_after,
                        "{version:?}: {row}"
                    ),
                    "invoke" => assert!(!absent_after, "{version:?}: {row}"),
                    _ => unreachable!(),
                }
                count += 1;
            }
        }
        assert_eq!(count, 16);
        let legacy = InvocationDialect::for_version(TclVersion::V8_4)
            .native_ensemble_rewrite_protocol()
            .unwrap();
        let jim =
            InvocationDialect::of_profile(crate::model::resolve_environment("jim").unit_profile())
                .native_ensemble_rewrite_protocol()
                .unwrap();
        for event in [
            Event::BytecodeEntry,
            Event::BeforeOrdinaryLookup,
            Event::AfterSuccessfulOrdinaryLookup,
        ] {
            assert!(!jim.resets_at(event));
            assert!(!legacy.resets_at(event));
        }
    }

    #[test]
    fn array_default_uses_the_actual_private_worker_only_at_accepted_outer_arity() {
        use crate::invocation_words::InvocationWords;
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection, NativeCompilationWordShape, NativeNamedInvocationProtocol,
        };
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        let context = NativeCompilationContext {
            frame: NativeCompilationFrame::ProcedureCode,
            mode: NativeCompilationMode::BytecodeObject,
            ..Default::default()
        };
        for arguments in [
            vec!["default", "get", "a", "EXTRA"],
            vec!["default", "get", "a"],
        ] {
            let spec = registry
                .resolve_invocation("array", &arguments, registry.own_surface_query())
                .unwrap()
                .facts()
                .native_compilation
                .unwrap();
            let shapes = vec![NativeCompilationWordShape::Literal; arguments.len()];
            for version in [TclVersion::V9_0, TclVersion::V9_1] {
                assert!(
                    matches!(spec.select(InvocationWords::literals("array", &arguments), &shapes, Some(InvocationDialect::for_version(version)), context), NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::Direct } if lookup.slot == "::tcl::array::default")
                );
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("array", &arguments),
                    &shapes,
                    Some(InvocationDialect::for_version(TclVersion::V8_6)),
                    context
                ),
                NativeCompilationSelection::Generic
            );
            let broad = ["default", "get"];
            assert_eq!(
                spec.select(
                    InvocationWords::literals("array", &broad),
                    &[NativeCompilationWordShape::Literal; 2],
                    Some(InvocationDialect::for_version(TclVersion::V9_0)),
                    context
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}
