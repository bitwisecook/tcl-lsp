// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The interprocedural pass over a finished unit: summaries, the memoised taint cascade and per-body taint reruns.

use super::optimiser_source_owner::OriginalInterproceduralTaintRequest;
use super::{
    CompilationUnit, FunctionUnit, OriginalTaintCallback, OriginalTaintCascadeCallback,
    SourcePipelineTrace, TaintCascadeCallback,
};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ssa::ValueKey;
use crate::taint::{
    TaintGraph, TaintLattice, TaintPropagationInputs, TaintSourceContext,
    instance_classes_for_function, propagate_taints,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl FunctionUnit {
    /// Re-run interprocedural taint propagation for this unit against `ia`,
    /// returning the taint lattice (it does **not** mutate `self.taints`).
    ///
    /// The taint lattice is keyed by SSA [`ValueKey`] (variable + version) and is
    /// **span-free**, so the result is offset-independent: an offset-0 baseline
    /// unit yields byte-identical taints to its rebased counterpart.  This is the
    /// property the salsa-native `taint_cascade` memo relies on — it propagates
    /// over the offset-0 baseline and installs the result into the rebased unit.
    /// This function-only entry retains its supplied metadata but cannot import
    /// Module-owned procedure or global-receiver facts.
    #[must_use]
    pub fn interproc_taints(
        &self,
        registry: &CommandRegistry,
        ia: &InterproceduralAnalysis,
        _dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> HashMap<ValueKey, TaintLattice> {
        self.interproc_taints_with_source(
            registry,
            ia,
            TaintSourceContext::for_function(registry, self),
            true,
        )
    }

    fn interproc_taints_with_source(
        &self,
        registry: &CommandRegistry,
        ia: &InterproceduralAnalysis,
        source: TaintSourceContext<'_>,
        include_globals: bool,
    ) -> HashMap<ValueKey, TaintLattice> {
        let instance_classes =
            instance_classes_for_function(&self.cfg, registry, Some(ia), include_globals, source);
        propagate_taints(
            &TaintGraph::new(&self.cfg, &self.ssa, &self.sccp),
            TaintPropagationInputs {
                registry,
                rendered_props: Some(&self.rendered_props),
                interproc: Some(ia),
                source,
                param_taints: None,
                taint_summaries: None,
                instance_classes: &instance_classes,
            },
        )
    }
}

impl CompilationUnit {
    /// Re-run one actual body's taints with its complete current source owner.
    /// Missing metadata or a foreign body ID cannot borrow a function-only context.
    #[must_use]
    pub fn original_interproc_taints_for_body(
        &self,
        body: Option<crate::ir::ProcedureId>,
        registry: &CommandRegistry,
        analysis: &InterproceduralAnalysis,
    ) -> Option<HashMap<ValueKey, TaintLattice>> {
        let function = self.function_for_body(body)?;
        let source = TaintSourceContext::for_module_function(registry, &self.ir_module, function);
        source.metadata_context()?;
        source.module()?;
        let owner = self.ir_module.retained_source_bindings.as_deref()?;
        if analysis.global_instance_class_owner.as_deref() != Some(owner)
            || analysis.global_instance_class_input.as_ref()
                != self.ir_module.source_metadata_input.as_ref()
        {
            return None;
        }
        (source.body_id() == body).then(|| {
            function.interproc_taints_with_source(registry, analysis, source, body.is_some())
        })
    }

    /// Populate [`InterproceduralAnalysis`] via
    /// [`crate::interprocedural::build_interprocedural_analysis`]. Call after
    /// [`Self::build_for`] when a consumer (optimiser, compiler-checks)
    /// needs proc summaries.
    ///
    /// Re-runs `propagate_taints` on every function unit using the
    /// freshly-built summary + the requested `dialect`, so
    /// inter-procedural taint transfer and dialect-specific source
    /// handling take effect.
    #[must_use]
    pub fn with_interprocedural(
        mut self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let original_owner = self.optimiser_source_owner.clone().filter(|owner| {
            owner.matches_unit(&self)
                && owner.matches_registry(registry)
                && owner.matches_dialect(dialect)
        });
        self.optimiser_source_owner = None;

        #[cfg(any(test, debug_assertions))]
        let mut phase_trace = SourcePipelineTrace::new("interprocedural", self.source.len());
        #[cfg(any(test, debug_assertions))]
        {
            phase_trace.procedures = self.procedures.len();
            phase_trace.phase("initial");
        }
        // Object-handle → class map (SSA/VTA-derived) so a `$g walk … -command
        // cb` instance-method callback becomes a call-graph / reachability edge.
        let Some(identities) =
            crate::realm::document_realm_bindings_for_module(&self.ir_module, registry)
        else {
            self.interproc = None;
            return self;
        };
        let object_types = crate::object_types::object_handle_classes(&self, registry);
        // The unit's own proven command-identity facts, so the call-graph scan
        // classifies a rebound head as the command it is.
        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("object-types");
        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("original-realm");
        let interproc = crate::interprocedural::build_interprocedural_analysis_for_unit(
            &self,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeCandidates::candidates(&object_types),
            &identities,
        );

        // Re-run taint with the new summary + dialect. We borrow
        // `interproc` immutably while each function unit re-runs
        // `propagate_taints`.
        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("analysis");
        let top_instance_classes = instance_classes_for_function(
            &self.top_level.cfg,
            registry,
            Some(&interproc),
            false,
            TaintSourceContext::for_module_function(registry, &self.ir_module, &self.top_level),
        );
        self.top_level.taints = Arc::new(propagate_taints(
            &TaintGraph::new(
                &self.top_level.cfg,
                &self.top_level.ssa,
                &self.top_level.sccp,
            ),
            TaintPropagationInputs {
                registry,
                rendered_props: Some(&self.top_level.rendered_props),
                interproc: Some(&interproc),
                source: TaintSourceContext::for_module_function(
                    registry,
                    &self.ir_module,
                    &self.top_level,
                ),
                param_taints: None,
                taint_summaries: None,
                instance_classes: &top_instance_classes,
            },
        ));
        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("top-level-taint");
        let source_bodies: HashSet<_> = self
            .ir_module
            .original_source_procedure_bodies()
            .map(|(id, _)| *id)
            .collect();
        for (id, fu) in self.procedures.iter_mut().chain(self.body_units.iter_mut()) {
            if !source_bodies.contains(id) {
                continue;
            }
            let instance_classes = instance_classes_for_function(
                &fu.cfg,
                registry,
                Some(&interproc),
                true,
                TaintSourceContext::for_module_function(registry, &self.ir_module, fu),
            );
            fu.taints = Arc::new(propagate_taints(
                &TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp),
                TaintPropagationInputs {
                    registry,
                    rendered_props: Some(&fu.rendered_props),
                    interproc: Some(&interproc),
                    source: TaintSourceContext::for_module_function(registry, &self.ir_module, fu),
                    param_taints: None,
                    taint_summaries: None,
                    instance_classes: &instance_classes,
                },
            ));
        }

        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("procedure-taints");
        self.interproc = Some(interproc);
        if let Some(owner) = original_owner {
            self.retain_enriched_original_optimiser_owner(&owner);
        }
        self
    }

    /// Like [`Self::with_interprocedural`] but routes each **procedure's**
    /// interprocedural taint re-run through `taint_cb`, a salsa-native memo (the
    /// `tcl-lsp-db` `taint_cascade` query).  The interprocedural summary is still
    /// built whole-module here (it is the memo's *input*); only the per-procedure
    /// `propagate_taints` re-run is memoised, so a procedure whose baseline and
    /// reachable-callee summaries are unchanged across an edit reuses its cached
    /// taints instead of re-propagating.
    ///
    /// `taint_cb(id, display, &interproc)` returns the procedure's taints, or `None` to
    /// fall back to a fresh [`FunctionUnit::interproc_taints`] (e.g. a procedure
    /// the lattice memo didn't intern).  The top level is always re-run fresh.
    /// Byte-identical to [`Self::with_interprocedural`] provided `taint_cb`
    /// reproduces `propagate_taints` against the full summary — guarded by the
    /// `compiler_check` corpus differential and the taint-cascade edit tests.
    #[must_use]
    pub fn with_interprocedural_memoized(
        self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        taint_cb: &mut TaintCascadeCallback<'_>,
    ) -> Self {
        self.with_interprocedural_callback(
            registry,
            dialect,
            &mut OriginalTaintCallback::Legacy(taint_cb),
        )
    }

    /// Memoise actual body taint reports under the private computed summary
    /// request. A legacy or foreign callback result remains reporting-only.
    #[must_use]
    pub fn with_original_interprocedural_memoized(
        self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        taint_cb: &mut OriginalTaintCascadeCallback<'_>,
    ) -> Self {
        self.with_interprocedural_callback(
            registry,
            dialect,
            &mut OriginalTaintCallback::Original(taint_cb),
        )
    }

    fn with_interprocedural_callback(
        mut self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        taint_cb: &mut OriginalTaintCallback<'_, '_>,
    ) -> Self {
        let original_owner = self.original_optimiser_prepared_unit(registry, dialect);
        self.optimiser_source_owner = None;
        let mut original_callbacks = true;

        let Some(identities) =
            crate::realm::document_realm_bindings_for_module(&self.ir_module, registry)
        else {
            self.interproc = None;
            return self;
        };
        let object_types = crate::object_types::object_handle_classes(&self, registry);
        // The unit's own proven command-identity facts, so the call-graph scan
        // classifies a rebound head as the command it is.
        let interproc = crate::interprocedural::build_interprocedural_analysis_for_unit(
            &self,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeCandidates::candidates(&object_types),
            &identities,
        );

        // Top level is built fresh (no offset-0 lattice key), so its taint
        // re-run stays inline.
        // A refused Module owner retains the bounded function-only projection;
        // it cannot reconstruct global receiver or procedure correspondence.
        let top_taints = self
            .original_interproc_taints_for_body(None, registry, &interproc)
            .unwrap_or_else(|| {
                self.top_level
                    .interproc_taints(registry, &interproc, dialect)
            });
        self.top_level.taints = Arc::new(top_taints);

        let request = OriginalInterproceduralTaintRequest::from_computed_summary(
            &self,
            &interproc,
            original_owner.as_ref(),
            dialect,
        );

        // Compute each procedure's taints (memoised via `taint_cb`, or fresh on a
        // miss) before mutating, so the immutable `&self.procedures` borrow the
        // fallback needs doesn't overlap the write-back.
        let mut new_taints: Vec<(crate::ir::ProcedureId, Arc<HashMap<ValueKey, TaintLattice>>)> =
            Vec::with_capacity(self.procedures.len() + self.body_units.len());
        for (qname, fu) in self.procedures.iter().chain(self.body_units.iter()) {
            if self.original_procedure_for_body(*qname).is_none() {
                continue;
            }
            let returned = match taint_cb {
                OriginalTaintCallback::Legacy(callback) => {
                    callback(*qname, &fu.name, &interproc).map(|taints| (taints, false))
                }
                OriginalTaintCallback::Original(callback) => callback(&request, *qname, &fu.name)
                    .map(|report| {
                        let current = request.accepts(*qname, registry, &report);
                        (Arc::clone(report.taints()), current)
                    }),
            };
            let taints = if let Some((taints, issued)) = returned {
                original_callbacks &= issued;
                taints
            } else {
                Arc::new(
                    self.original_interproc_taints_for_body(Some(*qname), registry, &interproc)
                        .unwrap_or_else(|| fu.interproc_taints(registry, &interproc, dialect)),
                )
            };
            new_taints.push((*qname, taints));
        }
        for (qname, taints) in new_taints {
            if let Some(fu) = self.procedures.get_mut(&qname) {
                fu.taints = taints;
            } else if let Some(fu) = self.body_units.get_mut(&qname) {
                fu.taints = taints;
            }
        }

        self.interproc = Some(interproc);
        if original_callbacks && let Some(owner) = original_owner {
            self.retain_enriched_original_optimiser_owner(&owner);
        }
        self
    }
}
