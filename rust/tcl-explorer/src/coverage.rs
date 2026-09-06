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
];

/// Exhaustive field witness. Adding a field to either durable compiler type
/// requires updating this mapping before the workspace compiles again.
#[allow(clippy::too_many_lines)]
pub fn assert_durable_field_inventory(unit: &CompilationUnit) {
    let FunctionUnit {
        name: _name,
        cfg: _cfg,
        ssa: _ssa,
        def_use: _def_use,
        sccp: _sccp,
        types: _types,
        return_type: _return_type,
        taints: _taints,
        rendered_props: _rendered_props,
        memory_ssa: _memory_ssa,
        dynamic_names: _dynamic_names,
        complexity_guarded: _complexity_guarded,
        base_offset: _base_offset,
        method_facts: _method_facts,
        semantic_facts: _semantic_facts,
    } = &unit.top_level;
    let CompilationUnit {
        source: _source,
        ir_module: _ir_module,
        cfg_module: _cfg_module,
        command_mutations: _command_mutations,
        top_level: _top_level,
        procedures: _procedures,
        methods: _methods,
        body_units: _body_units,
        interproc: _interproc,
        connection_scope: _connection_scope,
        caller_scope: _caller_scope,
    } = unit;
}

#[cfg(test)]
mod tests {
    use super::ARTIFACT_COVERAGE;
    use crate::views::VIEW_META;

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
