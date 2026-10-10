//! Compile-time coverage witness for durable compiler artefacts.
//!
//! The compiler owns the exhaustive no-`..` field witnesses, including private
//! retained inputs. Explorer reviews every published inventory name as a view
//! or an explicit exclusion. The inventory exposes names, not mutable inputs.

pub use tcl_compiler::durable_inventory::{
    COMPILATION_UNIT_FIELDS, FUNCTION_UNIT_FIELDS, assert_durable_field_inventory,
};

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
        artifact: "CompilationUnit::transfers",
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
        artifact: "FunctionUnit::semantic_value_projection",
        view: Some("semantic"),
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
        artifact: "FunctionUnit::irules_event_body",
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
        artifact: "FunctionUnit::source_metadata_input",
        view: None,
        exclusion: Some(
            "the immutable source/profile/availability input retained by the compiler's \
             metadata context; consumers use the read-only source_metadata_input accessor \
             rather than serialising or replacing this execution-independent input",
        ),
    },
    ArtifactCoverage {
        artifact: "FunctionUnit::source_config",
        view: None,
        exclusion: Some(
            "the exact lexer configuration retained for source-context validation; \
             consumers use the read-only source_lexer_config accessor, and sourceMap \
             reports positions produced under that configuration rather than an editable copy",
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

#[cfg(test)]
mod tests {
    use super::{ARTIFACT_COVERAGE, COMPILATION_UNIT_FIELDS, FUNCTION_UNIT_FIELDS};
    use crate::views::VIEW_META;

    /// Every compiler-owned durable field has exactly one reviewed destination
    /// or explicit exclusion, including private retained source inputs.
    #[test]
    fn every_durable_field_is_reviewed_in_the_coverage_table() {
        for field in COMPILATION_UNIT_FIELDS.iter().chain(FUNCTION_UNIT_FIELDS) {
            assert_eq!(
                ARTIFACT_COVERAGE
                    .iter()
                    .filter(|artifact| artifact.artifact == *field)
                    .count(),
                1,
                "{field} must have exactly one ARTIFACT_COVERAGE destination or exclusion"
            );
        }
    }

    #[test]
    fn reviewed_durable_fields_belong_to_the_compiler_inventory() {
        for artifact in ARTIFACT_COVERAGE {
            if matches!(
                artifact.artifact.split_once("::"),
                Some(("CompilationUnit" | "FunctionUnit", _))
            ) {
                assert!(
                    COMPILATION_UNIT_FIELDS
                        .iter()
                        .chain(FUNCTION_UNIT_FIELDS)
                        .any(|field| *field == artifact.artifact),
                    "{} is not a current compiler-owned durable field",
                    artifact.artifact
                );
            }
        }
    }

    #[test]
    fn every_reviewed_artifact_has_a_view_or_reasoned_exclusion() {
        assert!(!ARTIFACT_COVERAGE.is_empty());
        for artifact in ARTIFACT_COVERAGE {
            assert!(artifact.view.is_some() ^ artifact.exclusion.is_some());
            if let Some(reason) = artifact.exclusion {
                assert!(
                    !reason.trim().is_empty(),
                    "{} needs an exclusion purpose",
                    artifact.artifact
                );
            }
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
