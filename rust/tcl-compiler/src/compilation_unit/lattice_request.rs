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

//! `LatticeRequest`: one procedure's baseline-lattice build request for the memo callback, and the encoding of its parameter seeds.

use super::{FunctionUnit, original_lattice_worker};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// One procedure's **offset-0** baseline-lattice build request, handed to the
/// [`ProcLatticeCache`] callback by [`CompilationUnit::build_for_memoized`].
///
/// The body has already been normalised to offset 0 (every span shifted by
/// `-body_offset`), so a shifted-but-unchanged procedure produces an identical
/// request — the salsa-native memo (`tcl-lsp-db`'s `function_lattice`) keys on
/// it position-independently for ordinary procedures. Event requests also retain
/// their full original descriptor, and the builder rebases the returned unit back to
/// the procedure's real offset.  Carries exactly what
/// [`crate::cfg_builder::build_cfg_function_with_upvars`] +
/// [`FunctionUnit::build`] consume: the offset-0 body, the qualified name, the
/// parameter list, the module-wide upvar / proc-param context (so the rebuilt
/// CFG is identical to the whole-module build's), and the analysis dialect.
pub struct LatticeRequest<'a> {
    pub(crate) original_worker_request:
        Option<&'a original_lattice_worker::OriginalLatticeRequestSeal>,
    /// Genuine lowered Module that issued this source body coordinate.
    pub module: &'a crate::ir::Module,
    /// Exact current body for memo-result joins; neutral cached facts need no ID key.
    pub body_id: crate::ir::ProcedureId,
    /// Qualified procedure name (e.g. `::foo::bar`).
    pub qname: &'a str,
    /// The procedure body, normalised to offset 0.
    pub body: &'a crate::ir::Script,
    /// Actual authored document whose retained carriers own the body proofs.
    pub source: &'a str,
    /// Original body base before lexical spans were normalised to zero.
    pub original_body_offset: u32,
    /// Actual first executable body byte, independently of its normalisation base.
    pub executable_body_offset: u32,
    /// Exact procedure body source, absent for unrepresented synthesised bodies.
    pub body_source: Option<&'a str>,
    /// Sealed original event descriptor; absent for ordinary procedures.
    pub irules_event_body: Option<&'a Arc<crate::ir::SourceIrulesEventBody>>,
    /// Complete module mutation obligations used by the lattice fold owner.
    pub command_trust: &'a crate::command_binding::ModuleCommandMutations,
    /// The procedure's declared parameters.
    pub params: &'a [String],
    /// Body-keyed caller-frame summaries from [`crate::cfg_builder::prepare_cfg_context`].
    pub upvar_procs: &'a HashMap<crate::ir::ProcedureId, crate::cfg_builder::upvar_info::UpvarInfo>,
    /// Original parameter lists keyed by the same issued body IDs.
    pub proc_params: &'a HashMap<crate::ir::ProcedureId, Vec<String>>,
    /// Outer-place MAY summaries keyed by the same issued body IDs.
    pub global_write_procs:
        &'a HashMap<crate::ir::ProcedureId, crate::cfg_builder::global_write_info::GlobalWriteInfo>,
    /// Closed module command state used to resolve effective direct-call
    /// targets and `interp alias` argument prefixes.
    pub command_bindings: &'a crate::command_binding::ModuleCommandBindings,
    /// Actual original Module/source owner for procedure-effect selection.
    pub procedure_summary_owners: &'a crate::cfg_builder::OriginalProcedureSummaryOwners,
    /// Complete original metadata input for the shared body CFG owner.
    /// Absence refuses metadata; labels cannot recreate availability or scope.
    pub source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Exact offset-zero lexer configuration used to lower and rebuild this
    /// body. This is carried independently of `dialect`: callers may layer
    /// grammar overrides without changing the registry profile.
    pub lexer_config: tcl_lexer::LexerConfig,
    /// Analysis dialect — selects the registry the lattice pipeline runs under.
    /// `None` when the build named no dialect (plain Tcl).
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Whether command spellings must retain ordinary runtime dispatch rather
    /// than acquiring registry-declared builtin control-flow edges.
    pub plain_command_dispatch: bool,
    /// Interprocedural caller-uniform-literal SCCP seeds for this procedure
    /// (from [`params_constants_from_native_call_sites`]), encoded in the
    /// deterministic, hashable `(param, version, string)` form the memo key
    /// interns (see [`encode_param_constants`]).  Empty means "no seeds".
    /// Position-independent — keyed by parameter name + SSA version, never by
    /// span — so it rebases trivially with the rest of the offset-0 unit.
    pub param_constants: &'a [(String, u32, String)],
    /// Fully-qualified names of every class defined in the compilation unit
    /// (sorted), so the type-propagation pass can recognise a `TclOO` / itcl
    /// constructor call (`Foo new`) and type it `OBJECT(::ns::Foo)`.  A
    /// whole-unit fact, identical for every procedure, so it is folded into the
    /// memo key: adding or removing a class anywhere invalidates each
    /// procedure's lattice (a new class can change any body's constructor
    /// typing).  Sourced from [`crate::signature_scan`] so the standalone and
    /// incremental builds derive an identical set from the same source.
    pub known_classes: &'a [String],
    /// Literal variable-trace target names from [`crate::ir::Module::
    /// traced_variables`] (sorted, mirroring `known_classes`) — a
    /// whole-module fact SCCP's trace-safety gate needs (see
    /// [`crate::sccp::sccp`]).  Folded into the memo key like
    /// `known_classes`: a trace installed anywhere in the module can change
    /// any procedure's SCCP result, so adding/removing one must invalidate
    /// every cached lattice, not just the procedure whose body carries the
    /// `trace` call.
    pub traced_variables: &'a [String],
    /// The analysis context's memo identity for the module — the
    /// command-binding evidence, the registry and overlay generations, the
    /// tier, and the evaluator revision — so the value-transfer driver's
    /// answers are keyed by every fact that can change them
    /// (`docs/design/compiler/value-transfers.md` § *One invocation, one
    /// context*). Folded into the memo key like `traced_variables`: a
    /// `rename` anywhere in the module re-keys every procedure's lattice.
    pub analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    /// [`crate::ir::Module::has_dynamic_variable_trace`] — `true` when a
    /// variable-trace install/remove call targets a non-literal name
    /// anywhere in the module, which SCCP must treat as "every variable is
    /// potentially traced". Whole-module, folded into the memo key
    /// alongside `traced_variables`.
    pub has_dynamic_variable_trace: bool,
}

impl LatticeRequest<'_> {
    /// Preserve this body's original metadata purpose under its exact current
    /// declaration and module. Missing supplied input remains unavailable.
    #[must_use]
    pub fn original_metadata_context(
        &self,
        registry: &CommandRegistry,
    ) -> crate::registry_invocation::OwnedInvocationMetadataContext {
        use crate::registry_invocation::OwnedInvocationMetadataContext as Metadata;
        let Some(procedure) = self.original_source_procedure() else {
            return Metadata::Unavailable;
        };
        let Some(retained) = self.module.retained_source_bindings.as_ref() else {
            return Metadata::Unavailable;
        };
        if self.lexer_config != self.module.lexer_config.nested().normalized()
            || self
                .dialect
                .or_else(|| registry.profile())
                .map(tcl_dialect::DialectProfile::cache_key)
                != self
                    .module
                    .dialect_profile
                    .map(tcl_dialect::DialectProfile::cache_key)
            || retained
                .positioned_for_module(self.module, registry)
                .is_none()
            || !retained.matches_original_procedure_summary(self.body_id, procedure)
        {
            return Metadata::Unavailable;
        }
        match &self.module.source_entry.metadata_context {
            Metadata::Standalone if self.source_metadata_input.is_none() => Metadata::Standalone,
            Metadata::Supplied(context) if self.source_metadata_input.is_none() => {
                Metadata::Supplied(Arc::clone(context))
            }
            Metadata::SuppliedSource(original) => {
                if crate::registry_invocation::InvocationMetadataContext::for_module(
                    registry,
                    self.module,
                )
                .is_none()
                {
                    return Metadata::Unavailable;
                }
                let Some(input) = self.source_metadata_input else {
                    return Metadata::Unavailable;
                };
                if input != original.as_ref() && input != &original.for_nested_source() {
                    return Metadata::Unavailable;
                }
                Metadata::for_source_input(Some(input))
            }
            _ => Metadata::Unavailable,
        }
    }

    /// Genuine original declaration in this request's current source and issuer.
    /// Independent declaration cards retain their purpose; no allocation follows.
    #[must_use]
    pub fn original_source_procedure(&self) -> Option<&crate::ir::Procedure> {
        (self.module.source.try_text().ok()? == self.source).then_some(())?;
        let procedure = self
            .module
            .original_source_procedure_bodies()
            .find_map(|(id, procedure)| (*id == self.body_id).then_some(procedure))?;
        if procedure.qualified_name != self.qname
            || procedure.params != self.params
            || procedure.span.start() != self.original_body_offset
            || procedure.body_offset != self.executable_body_offset
            || procedure.body_source.as_deref() != self.body_source
        {
            return None;
        }
        let mut body = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut body, -i64::from(procedure.span.start()));
        (&body == self.body).then_some(procedure)
    }
}

/// Salsa-native per-procedure lattice memo used by
/// [`CompilationUnit::build_for_memoized`].
///
/// `cache(request)` returns the **offset-0** [`FunctionUnit`] for `request`
/// (building it on a miss, reusing a memoised one on a hit).  The builder
/// rebases the returned unit to the procedure's real position, so the result is
/// byte-identical to [`CompilationUnit::build_for_with_config`]; only the
/// redundant lattice recompute for an unchanged procedure body is skipped.  The
/// caller owns the backing store and its eviction policy.
pub type ProcLatticeCache<'a> = dyn FnMut(&LatticeRequest<'_>) -> FunctionUnit + 'a;

/// Encode interprocedural `param_constants` (caller-uniform-literal SCCP seeds)
/// into the deterministic, hashable form interned into the salsa-native
/// `FnLatticeKey`.  [`params_constants_from_native_call_sites`] only ever emits
/// `Const(String)` seeds, so the encoding is a sorted vec of `(param, version,
/// string)` triples; sorting makes the encoding independent of hash-map
/// iteration order so equal seeds always intern to the same key.
///
/// `None` input (no seeds) encodes to an empty vec.  Returns `None` only if a
/// seed has some other lattice shape — a shape we can't faithfully intern, so
/// the caller falls back to a fresh build rather than silently dropping it.
/// Defensive: the current producer never emits a non-string seed.
#[must_use]
pub(crate) fn encode_param_constants(
    param_constants: Option<&HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>>,
) -> Option<Vec<(String, u32, String)>> {
    use crate::analyses::{ConstValue, LatticeValue};
    let Some(map) = param_constants else {
        return Some(Vec::new());
    };
    let mut out = Vec::with_capacity(map.len());
    for ((name, version), val) in map {
        let LatticeValue::Const(ConstValue::String(s)) = val else {
            return None;
        };
        out.push((name.clone(), *version, s.clone()));
    }
    out.sort();
    Some(out)
}

/// Inverse of [`encode_param_constants`]: rebuild the SCCP seed map a
/// `function_lattice` build feeds to [`FunctionUnit::build_with_param_constants`].
/// An empty slice (no seeds) decodes to `None`.
#[must_use]
pub fn decode_param_constants(
    encoded: &[(String, u32, String)],
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    if encoded.is_empty() {
        return None;
    }
    Some(
        encoded
            .iter()
            .map(|(name, version, s)| {
                (
                    (name.clone(), *version),
                    LatticeValue::Const(ConstValue::String(s.clone())),
                )
            })
            .collect(),
    )
}
