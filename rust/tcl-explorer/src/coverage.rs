//! Compile-time coverage witness for durable compiler artefacts.
//!
//! The no-`..` destructures intentionally fail compilation when a durable
//! field is added to `CompilationUnit` or `FunctionUnit` before its Explorer
//! mapping is reviewed. The witness is not a second serialiser: it only pins
//! the inventory boundary and the owners that must be accounted for.

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};

/// A reviewed durable artefact and its Explorer destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactCoverage {
    /// Compiler field or stable stage name.
    pub artifact: &'static str,
    /// Canonical Explorer view id, or `None` for an explicit exclusion.
    pub view: Option<&'static str>,
    /// Exclusion reason, required when `view` is `None`.
    pub exclusion: Option<&'static str>,
}

/// Field-to-view map reviewed with the compiler pipeline inventory.
pub const ARTIFACT_COVERAGE: &[ArtifactCoverage] = &[
    ArtifactCoverage {
        artifact: "CompilationUnit::source",
        view: Some("sourceMap"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::ir_module",
        view: Some("ir"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::cfg_module",
        view: Some("cfg"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::top_level",
        view: Some("cfg"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::procedures",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::methods",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::body_units",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::interproc",
        view: Some("interproc"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::connection_scope",
        view: Some("connectionScope"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::caller_scope",
        view: Some("unitScope"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::cfg",
        view: Some("cfg"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::ssa",
        view: Some("ssa"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::def_use",
        view: Some("liveness"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::sccp",
        view: Some("sccp"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::types",
        view: Some("types"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::return_type",
        view: Some("types"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::taints",
        view: Some("taintFacts"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::rendered_props",
        view: Some("rendered"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::memory_ssa",
        view: Some("dataflow"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::dynamic_names",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::complexity_guarded",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::tier",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::base_offset",
        view: Some("sourceMap"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::method_facts",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::semantic_facts",
        view: Some("semantic"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "ExecutableWorldStateSsa",
        view: Some("worldSsa"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "BackendSelection",
        view: Some("wasm"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "WASM",
        view: Some("wasm"),
        exclusion: None,
    },
    ArtifactCoverage {
        // `asm.rs` hands this straight to
        // `codegen_module_with_command_mutations`, so the disassembly the
        // view shows is the one this fact selected.
        artifact: "CompilationUnit::command_mutations",
        view: Some("asm"),
        exclusion: None,
    },
    ArtifactCoverage {
        artifact: "CompilationUnit::declared_commands",
        view: None,
        exclusion: Some(
            "an input the Explorer itself supplies on `UnitBuildOptions` and the unit \
             echoes back, not an artefact the build produced; its effect is visible \
             wherever a declared command resolved (`ir`, `semantic`, `taint`)",
        ),
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::name",
        view: None,
        exclusion: Some(
            "the function's label, carried in the header of every per-function view \
             rather than being an artefact of its own",
        ),
    },
];

/// Declare a durable compiler type's field inventory once: a witness that
/// destructures it exhaustively, and the names that destructure bound.
///
/// Binding a field to `_name` and leaving it out of [`ARTIFACT_COVERAGE`]
/// satisfied the compile-time tripwire while the field stayed silently
/// uncovered — the one escape hatch the design had, and it had been used
/// three times (`CompilationUnit::command_mutations`,
/// `CompilationUnit::declared_commands`, `FunctionUnit::name` — #2081).
/// Naming each field once closes it: the destructure still fails to compile
/// when a field is added, and the name it forces the author to write is the
/// same one `every_durable_field_is_reviewed_in_the_coverage_table` then
/// demands a row for.
macro_rules! durable_inventory {
    ($witness:ident, $fields:ident, $ty:ident, $($field:ident),+ $(,)?) => {
        /// Every durable field of the type, as `Type::field`.
        pub const $fields: &[&str] =
            &[$(concat!(stringify!($ty), "::", stringify!($field))),+];

        fn $witness(value: &$ty) {
            let $ty { $($field: _),+ } = value;
        }
    };
}

durable_inventory!(
    witness_function_unit,
    FUNCTION_UNIT_FIELDS,
    FunctionUnit,
    name,
    cfg,
    ssa,
    def_use,
    sccp,
    types,
    return_type,
    taints,
    rendered_props,
    memory_ssa,
    dynamic_names,
    complexity_guarded,
    tier,
    base_offset,
    method_facts,
    semantic_facts,
);

durable_inventory!(
    witness_compilation_unit,
    COMPILATION_UNIT_FIELDS,
    CompilationUnit,
    source,
    ir_module,
    cfg_module,
    command_mutations,
    top_level,
    procedures,
    methods,
    body_units,
    interproc,
    connection_scope,
    caller_scope,
    declared_commands,
);

/// Exhaustive field witness. Adding a field to either durable compiler type
/// requires updating this mapping before the workspace compiles again.
pub fn assert_durable_field_inventory(unit: &CompilationUnit) {
    witness_function_unit(&unit.top_level);
    witness_compilation_unit(unit);
}

#[cfg(test)]
mod tests {
    use super::{ARTIFACT_COVERAGE, COMPILATION_UNIT_FIELDS, FUNCTION_UNIT_FIELDS};
    use crate::views::VIEW_META;

    /// The inventory boundary is only a boundary if crossing it is noticed.
    /// Every field the witness destructures must have a row — a view, or an
    /// exclusion that says why (#2081).
    #[test]
    fn every_durable_field_is_reviewed_in_the_coverage_table() {
        for field in COMPILATION_UNIT_FIELDS.iter().chain(FUNCTION_UNIT_FIELDS) {
            assert!(
                ARTIFACT_COVERAGE
                    .iter()
                    .any(|artifact| artifact.artifact == *field),
                "{field} is destructured by the witness but has no ARTIFACT_COVERAGE row: \
                 give it a view, or an exclusion saying why it has none"
            );
        }
    }

    #[test]
    fn every_reviewed_artifact_has_a_view_or_reasoned_exclusion() {
        assert!(!ARTIFACT_COVERAGE.is_empty());
        for artifact in ARTIFACT_COVERAGE {
            assert!(artifact.view.is_some() ^ artifact.exclusion.is_some());
            if let Some(view) = artifact.view {
                assert!(
                    VIEW_META.iter().any(|descriptor| descriptor.id == view),
                    "artifact {} references unknown Explorer view {}",
                    artifact.artifact,
                    view
                );
            }
        }
    }
}
