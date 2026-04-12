//! Shared compilation artefacts for a single source document.
//!
//! Built once per diagnostics cycle, consumed by the analyser,
//! optimiser, shimmer analysis, taint engine, and compiler checks.
//!
//! Ported from `core/compiler/compilation_unit.py` (C31). This
//! strip lands the [`CompilationUnit`] / [`FunctionUnit`] facade
//! types and the `build_for` entry point that drives the landed
//! pipeline (lower → CFG → SSA → def-use → SCCP). Heavier
//! analyses (interprocedural, memory-SSA, execution-intent,
//! rendered-properties) plug in through accessor methods that
//! return `Option<&T>` — `None` when the analysis hasn't been
//! run on this unit yet.
//!
//! The Python facade also owns class-name extraction and
//! connection-scope analysis; those are follow-ups.

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::build_cfg;
use crate::def_use::{build_def_use_chains, DefUseResult};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::lowering::lower_to_ir;
use crate::memory_ssa::{build_memory_ssa, MemorySSAFunction};
use crate::sccp::{sccp, SccpResult};
use crate::ssa::{build_ssa, SsaFunction};

// ---------------------------------------------------------------------------
// Per-function analysis bundle
// ---------------------------------------------------------------------------

/// Analysis artefacts for one function (top-level or procedure).
#[derive(Debug, Clone)]
pub struct FunctionUnit {
    /// Qualified function name (e.g. `::top`, `::foo::bar`).
    pub name: String,
    /// Control-flow graph.
    pub cfg: CfgFunction,
    /// SSA form.
    pub ssa: SsaFunction,
    /// Def-use chains.
    pub def_use: DefUseResult,
    /// SCCP result: lattice values, executable blocks, constant
    /// branches.
    pub sccp: SccpResult,
    /// Optional memory-SSA annotations (populated on demand).
    pub memory_ssa: Option<MemorySSAFunction>,
}

impl FunctionUnit {
    /// Build per-function analyses from a CFG + its source
    /// parameters. Does *not* populate `memory_ssa`; call
    /// [`FunctionUnit::with_memory_ssa`] when the caller needs
    /// it.
    #[must_use]
    pub fn build(name: impl Into<String>, cfg: CfgFunction, registry: &CommandRegistry) -> Self {
        let ssa = build_ssa(&cfg, registry);
        let def_use = build_def_use_chains(&ssa, Some(&cfg));
        let sccp = sccp(&cfg, &ssa, None);
        Self {
            name: name.into(),
            cfg,
            ssa,
            def_use,
            sccp,
            memory_ssa: None,
        }
    }

    /// Populate memory-SSA on demand. Returns `self` for chaining.
    #[must_use]
    pub fn with_memory_ssa(mut self) -> Self {
        self.memory_ssa = Some(build_memory_ssa(&self.ssa));
        self
    }
}

// ---------------------------------------------------------------------------
// Module-level compilation unit
// ---------------------------------------------------------------------------

/// Complete compilation artefacts for a source document.
///
/// Built once, consumed many times across the diagnostics cycle.
#[derive(Debug, Clone)]
pub struct CompilationUnit {
    /// Source text (kept so downstream passes that need raw
    /// lexing can re-scan ranges without reparsing).
    pub source: String,
    /// IR module produced by lowering.
    pub ir_module: IrModule,
    /// Module-level CFG.
    pub cfg_module: CfgModule,
    /// Top-level script analysis.
    pub top_level: FunctionUnit,
    /// Per-procedure analyses keyed by qualified name.
    pub procedures: HashMap<String, FunctionUnit>,
    /// Interprocedural summary (optional — populated when the
    /// interprocedural pass has been run).
    pub interproc: Option<InterproceduralAnalysis>,
}

impl CompilationUnit {
    /// Build a [`CompilationUnit`] by running the landed pipeline
    /// end-to-end: `lower_to_ir` → `build_cfg` → per-function
    /// SSA / def-use / SCCP.
    ///
    /// `defer_top_level = false` gives analyses the fully-inlined
    /// CFG; passing `true` matches the codegen behaviour where
    /// top-level `foreach` / `catch` / `try` are compiled as
    /// opaque calls.
    #[must_use]
    pub fn build_for(source: &str, registry: &CommandRegistry, defer_top_level: bool) -> Self {
        let ir_module = lower_to_ir(source, registry);
        let cfg_module = build_cfg(&ir_module, defer_top_level);
        let top_level = FunctionUnit::build("::top", cfg_module.top_level.clone(), registry);
        let mut procedures: HashMap<String, FunctionUnit> = HashMap::new();
        for (qname, cfg) in &cfg_module.procedures {
            procedures.insert(
                qname.clone(),
                FunctionUnit::build(qname, cfg.clone(), registry),
            );
        }
        Self {
            source: source.to_owned(),
            ir_module,
            cfg_module,
            top_level,
            procedures,
            interproc: None,
        }
    }

    /// Populate [`InterproceduralAnalysis`] via
    /// [`build_interprocedural_analysis`]. Call after
    /// [`build_for`] when a consumer (optimiser, compiler-checks)
    /// needs proc summaries.
    #[must_use]
    pub fn with_interprocedural(
        mut self,
        registry: &CommandRegistry,
        dialect: Option<&str>,
    ) -> Self {
        self.interproc = Some(crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
        ));
        self
    }

    /// Populate memory-SSA on the top-level and every procedure.
    #[must_use]
    pub fn with_memory_ssa(mut self) -> Self {
        self.top_level = self.top_level.with_memory_ssa();
        let mut out: HashMap<String, FunctionUnit> = HashMap::with_capacity(self.procedures.len());
        for (k, fu) in self.procedures.drain() {
            out.insert(k, fu.with_memory_ssa());
        }
        self.procedures = out;
        self
    }

    /// Return the function unit for a qualified name, searching
    /// top-level + procedures.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&FunctionUnit> {
        if name == "::top" {
            return Some(&self.top_level);
        }
        self.procedures.get(name)
    }

    /// Iterate over every function unit in the module (top-level
    /// first, then procedures in insertion order).
    pub fn functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        std::iter::once(&self.top_level).chain(self.procedures.values())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn build_for_empty_source() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        assert_eq!(cu.source, "");
        assert_eq!(cu.top_level.name, "::top");
        assert!(cu.procedures.is_empty());
    }

    #[test]
    fn build_for_single_statement() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        assert!(!cu.top_level.cfg.blocks.is_empty());
        // SCCP should mark entry executable for a non-empty CFG.
        assert!(cu.top_level.sccp.executable_blocks.contains(&cu.top_level.ssa.entry));
    }

    #[test]
    fn build_for_captures_procedures() {
        let cu = CompilationUnit::build_for(
            "proc greet {name} {puts $name}",
            &registry(),
            false,
        );
        assert!(!cu.procedures.is_empty());
        assert!(cu.function("::greet").is_some());
        assert!(cu.function("::top").is_some());
    }

    #[test]
    fn with_memory_ssa_populates_optional() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false).with_memory_ssa();
        assert!(cu.top_level.memory_ssa.is_some());
    }

    #[test]
    fn functions_iterator_yields_top_plus_procs() {
        let cu = CompilationUnit::build_for(
            "proc foo {} {return 1}\nproc bar {} {return 2}",
            &registry(),
            false,
        );
        let count = cu.functions().count();
        assert_eq!(count, cu.procedures.len() + 1);
    }
}
