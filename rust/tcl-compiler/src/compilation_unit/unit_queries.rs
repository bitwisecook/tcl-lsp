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

//! Selecting the functions, bodies, declarations and prepared CFGs of a finished unit.

use super::{CompilationUnit, FunctionUnit};
use crate::cfg::Function as CfgFunction;
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

impl CompilationUnit {
    /// Select the current top-level unit (`None`) or a Module-issued procedure,
    /// method or selected body unit through its matching actual store.
    /// Foreign IDs and function units with a mismatched body stamp are unavailable.
    #[must_use]
    pub fn function_for_body(&self, body: Option<crate::ir::ProcedureId>) -> Option<&FunctionUnit> {
        let Some(id) = body else {
            return self.top_level.body_id.is_none().then_some(&self.top_level);
        };
        if !self.ir_module.procedure_ids.owns(id) {
            return None;
        }
        let owners = [
            self.ir_module.procedures.contains_key(&id),
            self.ir_module.methods.contains_key(&id),
            self.ir_module.body_units.contains_key(&id),
            self.ir_module.carried_installed_lowerings.contains_key(&id),
        ];
        if owners.into_iter().filter(|owns| *owns).count() != 1 {
            return None;
        }
        let stores = [
            (
                &self.procedures,
                self.ir_module.procedures.contains_key(&id),
            ),
            (&self.methods, self.ir_module.methods.contains_key(&id)),
            (
                &self.body_units,
                self.ir_module.body_units.contains_key(&id),
            ),
            (
                &self.carried_installed_lowerings,
                self.ir_module.carried_installed_lowerings.contains_key(&id),
            ),
        ];
        let mut functions = stores
            .into_iter()
            .filter_map(|(store, owns)| owns.then(|| store.get(&id)).flatten());
        let function = functions.next()?;
        if functions.next().is_some() {
            return None;
        }
        (function.body_id == Some(id)).then_some(function)
    }

    /// Actual CFG prepared from an original source declaration under this issuer.
    /// Anonymous and installed-only bodies do not supply declaration evidence.
    #[must_use]
    pub fn original_procedure_cfg_for_body(
        &self,
        id: crate::ir::ProcedureId,
    ) -> Option<&CfgFunction> {
        self.ir_module
            .original_source_procedure_bodies()
            .any(|(body, _)| *body == id)
            .then_some(())?;
        self.prepared_original_cfg(id)
    }

    /// [`Self::original_procedure_cfg_for_body`] against an original-body set
    /// the caller computed once from `original_source_procedure_bodies`.
    pub(crate) fn original_procedure_cfg_in(
        &self,
        id: crate::ir::ProcedureId,
        original: &std::collections::BTreeSet<crate::ir::ProcedureId>,
    ) -> Option<&CfgFunction> {
        original.contains(&id).then_some(())?;
        self.prepared_original_cfg(id)
    }

    fn prepared_original_cfg(&self, id: crate::ir::ProcedureId) -> Option<&CfgFunction> {
        if self.ir_module.procedures.contains_key(&id) {
            self.cfg_module.procedures.get(&id)
        } else {
            self.source_body_cfgs.get(&id)
        }
    }

    /// Prepared CFG for an actual Installed analysis body, without declaration evidence.
    pub(crate) fn installed_procedure_cfg_for_body(
        &self,
        id: crate::ir::ProcedureId,
    ) -> Option<&CfgFunction> {
        (self.ir_module.procedure_ids.owns(id)
            && self
                .ir_module
                .installed_procedure_body_units
                .contains_key(&id)
            && self.ir_module.body_lowering(&id)?.body_kind
                == crate::ir::ProcedureBodyKind::Installed
            && self.body_lowering_function(id)?.body_id == Some(id))
        .then_some(())?;
        self.source_body_cfgs.get(&id)
    }

    /// The function of one body lowering, from the store its Module lowering
    /// occupies: a body unit's, or a carried Installed lowering's.
    #[must_use]
    pub fn body_lowering_function(&self, id: crate::ir::ProcedureId) -> Option<&FunctionUnit> {
        if self.ir_module.body_units.contains_key(&id) {
            self.body_units.get(&id)
        } else if self.ir_module.carried_installed_lowerings.contains_key(&id) {
            self.carried_installed_lowerings.get(&id)
        } else {
            None
        }
    }

    /// Every body lowering's function: each body unit, then each Installed
    /// lowering a unit carries. [`Self::body_units`] alone counts the units.
    pub fn body_lowering_functions(
        &self,
    ) -> impl Iterator<Item = (&crate::ir::ProcedureId, &FunctionUnit)> {
        self.body_units
            .iter()
            .chain(&self.carried_installed_lowerings)
    }

    /// Current readonly Native source declaration for diagnostic body traversal.
    /// This supplies no callable, entered frame, Native invocation or absence proof.
    #[must_use]
    pub(crate) fn original_native_declaration_for_function(
        &self,
        function: &FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<&crate::ir::Procedure> {
        let body = function.body_id?;
        if self.ir_module.source.try_text().ok() != Some(self.source.as_str())
            || !self.ir_module.procedure_ids.owns(body)
            || self
                .function_for_body(Some(body))
                .is_none_or(|current| !std::ptr::eq(current, function))
            || self.source_body_cfgs.get(&body) != Some(&function.cfg)
        {
            return None;
        }
        function.invocation_metadata_context_for_module(registry, &self.ir_module)?;
        let procedure = self.ir_module.body_lowering(&body)?;
        let card = self
            .ir_module
            .original_native_declaration_body_units
            .get(&body)?;
        card.matches_procedure(&self.ir_module, procedure)
            .then_some(procedure)
    }

    /// Static-read source advice from this same current readonly body producer.
    /// A separate equal-source interpretation cannot supply its allocation owner.
    pub(crate) fn original_native_declaration_static_read_alternatives(
        &self,
        function: &FunctionUnit,
        registry: &CommandRegistry,
    ) -> Option<crate::command_binding::OriginalProcedureStaticReadAlternatives> {
        let procedure = self.original_native_declaration_for_function(function, registry)?;
        let entry = &self.ir_module.source_entry;
        if entry.source_point_entry.is_some()
            || entry.unknown_entry
            || entry.native_entry.is_some()
            || entry.hosted_execution_context.is_some()
        {
            return None;
        }
        let bindings = self
            .ir_module
            .retained_source_bindings
            .as_deref()?
            .positioned_owner_for_module(&self.ir_module, registry)?;
        bindings.original_procedure_static_read_alternatives(
            procedure,
            &self.ir_module.source,
            registry,
            function.source_metadata_input()?,
        )
    }

    /// Actual original declaration, independently of whether it entered the runtime.
    #[must_use]
    pub fn original_procedure_for_body(
        &self,
        id: crate::ir::ProcedureId,
    ) -> Option<&crate::ir::Procedure> {
        self.ir_module
            .original_source_procedure_bodies()
            .find_map(|(body, declaration)| (*body == id).then_some(declaration))
    }

    /// Whether a memo candidate agrees with this original body's prepared CFG.
    /// A declined callback cannot supply a downstream baseline for a fresh unit.
    #[must_use]
    pub fn accepts_original_procedure_lattice(
        &self,
        id: crate::ir::ProcedureId,
        candidate: &FunctionUnit,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(procedure) = self.original_procedure_for_body(id) else {
            return false;
        };
        let Some(current) = self.function_for_body(Some(id)) else {
            return false;
        };
        let Some(retained) = self.ir_module.retained_source_bindings.as_deref() else {
            return false;
        };
        if retained
            .positioned_for_module(&self.ir_module, registry)
            .is_none()
            || !retained.matches_original_procedure_summary(id, procedure)
            || self.ir_module.source.try_text().ok() != Some(self.source.as_str())
            || self.original_procedure_cfg_for_body(id) != Some(&current.cfg)
            || candidate.sccp.reads_module
        {
            return false;
        }
        let mut positioned = candidate.clone();
        crate::lattice_rebase::rebase_function_unit(
            &mut positioned,
            i64::from(procedure.span.start()),
        );
        positioned.cfg == current.cfg
    }

    /// Select a uniquely named authored test fixture, then its actual issued body.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&FunctionUnit> {
        if name == "::top" {
            return (!self.ir_module.procedures.has_display(name))
                .then(|| self.function_for_body(None))
                .flatten();
        }
        if let Some((id, _)) = self.ir_module.procedures.unique_display(name) {
            return self.function_for_body(Some(*id));
        }
        // An ambiguous primary fixture cannot borrow a different source body.
        if self.ir_module.procedures.has_display(name) {
            return None;
        }
        // Readonly analysis bodies retain their own issued IDs and source cards.
        // Their reporting labels select a fixture, never a callable or frame.
        let mut original = self
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .filter_map(|(id, card)| {
                let procedure = self.ir_module.body_units.get(id)?;
                (self.ir_module.procedure_ids.owns(*id)
                    && card.reporting_label(&self.ir_module, procedure) == Some(name))
                .then_some(*id)
            });
        let id = original.next()?;
        if original.next().is_some() {
            return None;
        }
        self.function_for_body(Some(id))
    }

    /// Top-level first, followed by primary procedures ordered by reporting name.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut procs: Vec<&FunctionUnit> = self.procedures.values().collect();
        procs.sort_by(|a, b| a.name.cmp(&b.name));
        std::iter::once(&self.top_level).chain(procs)
    }

    /// The **narrowest** function unit whose source range covers `offset`
    /// (absolute): procedures, `TclOO` methods, and synthetic body units
    /// (`apply` lambdas, `namespace eval` bodies) all participate; the
    /// top-level unit is the containing fallback.  This is the locator a
    /// program-point query (e.g. the constant-`$cmd` value-provenance
    /// settlement) uses to find the SSA form that actually contains a
    /// site — a site inside a `namespace eval` body lives in that body's
    /// own unit, not in `::top`'s statement list.
    #[must_use]
    pub fn function_unit_at(&self, offset: u32) -> &FunctionUnit {
        let procs = self
            .procedures
            .iter()
            .filter_map(|(qname, fu)| Some((self.ir_module.procedures.get(qname)?.span, fu)));
        let methods = self
            .methods
            .iter()
            .filter_map(|(qname, fu)| Some((self.ir_module.methods.get(qname)?.span?, fu)));
        let bodies = self
            .body_lowering_functions()
            .filter_map(|(qname, fu)| Some((self.ir_module.body_lowering(qname)?.span, fu)));
        let mut best: Option<(u32, &FunctionUnit)> = None;
        for (range, fu) in procs.chain(methods).chain(bodies) {
            if !(range.start() <= offset && offset <= range.end()) {
                continue;
            }
            let width = range.end() - range.start();
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, fu));
            }
        }
        best.map_or(&self.top_level, |(_, fu)| fu)
    }

    /// Instance-variable names in scope for the named function when it is a
    /// `TclOO`/snit *method* body (class-level `variable` declarations plus
    /// the method's own) — the alias-shaped names the shimmer thunking pass
    /// abstains on (their writes escape to the object, so the local SSA
    /// version chain is not the whole story).  `None` for procs and the top
    /// level.
    #[must_use]
    pub fn method_instance_vars(
        &self,
        body_id: Option<crate::ir::ProcedureId>,
    ) -> Option<&std::collections::HashSet<String>> {
        self.ir_module
            .methods
            .get(&body_id?)
            .map(|m| &m.instance_vars)
    }

    /// The complete descriptive class roster from this actual source build.
    /// Source, full input/configuration, Registry and immutable Module owner
    /// must still match. Names grant no Native class or Normal result identity.
    pub(crate) fn retained_source_class_names(
        &self,
        registry: &CommandRegistry,
    ) -> Option<&HashSet<String>> {
        if self.ir_module.source != tcl_lexer::SourceImage::document(&self.source) {
            return None;
        }
        self.source_class_facts
            .as_deref()?
            .for_module(&self.ir_module, registry)
    }

    /// Top-level and primary procedure units that permit deep analysis.
    pub fn analysable_functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        self.functions().filter(|fu| !fu.complexity_guarded)
    }

    /// Top-level first, then primary procedures, then methods and additional
    /// bodies together. Each following group is ordered by reporting name.
    pub fn all_body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut extra: Vec<&FunctionUnit> = self
            .methods
            .values()
            .chain(self.body_lowering_functions().map(|(_, function)| function))
            .collect();
        extra.sort_by(|a, b| a.name.cmp(&b.name));
        self.functions().chain(extra)
    }

    /// All retained function units that permit deep analysis.
    pub fn analysable_body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        self.all_body_function_units()
            .filter(|fu| !fu.complexity_guarded)
    }

    /// Additional retained body lowerings, ordered by reporting name.
    pub fn body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut v: Vec<&FunctionUnit> = self
            .body_lowering_functions()
            .map(|(_, function)| function)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v.into_iter()
    }

    /// Methods and additional retained bodies that permit deep analysis,
    /// ordered by reporting name.
    pub fn analysable_methods_and_body_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut v: Vec<&FunctionUnit> = self
            .methods
            .values()
            .chain(self.body_lowering_functions().map(|(_, function)| function))
            .filter(|fu| !fu.complexity_guarded)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v.into_iter()
    }
}
