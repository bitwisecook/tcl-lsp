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

//! The SCCP parameter-constant seed for one procedure, from the collected call-site evidence and who else can call it.

use super::CallSiteEvidence;
use crate::ir::Module as IrModule;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tcl_registry::Traits;

/// Everything the interprocedural seed needs to know about *who else* can
/// call this unit's procedures.
pub(crate) struct UnitCallerView<'a> {
    /// Registry-declared boundaries the file itself crosses
    /// ([`scan_unit_linkage`]).
    pub linkage: Traits,
    /// Whether a host supplied cross-file call-site evidence for this unit.
    /// With evidence, `merged` is the whole project's view and a
    /// registry-declared boundary does not have to be treated as an unknown
    /// caller; without it, the unit is on its own and any boundary sinks the
    /// seed.
    pub has_cross_file_evidence: bool,
    /// Prepared trust projection for the declared identity of retained
    /// procedures. This is intentionally narrower than the optimiser's
    /// command-mutation projection: an unresolved command or an external
    /// body can make builtin effects opaque without itself proving that a
    /// retained procedure name was rebound.
    pub proc_binding_trust: &'a crate::command_binding::ProcBindingTrustProjection,
    /// Actual analysed source bytes for declaration correlation.
    pub source: Option<&'a str>,
    /// The procedures the file's own code can enter: reachable over direct
    /// calls from the top level (method and lambda bodies count as entry
    /// points). `None` when every procedure may be entered. A procedure
    /// outside the set runs only if a caller this unit cannot see calls it,
    /// so the callers it does see — its own recursive call, say — say nothing
    /// of its parameters on entry (#2522).
    pub entered: Option<&'a std::collections::HashSet<crate::ir::ProcedureId>>,
}

/// Boundaries that publish this file's commands to callers **no** host
/// enumeration can bound — another checkout can `package require` this one,
/// or `namespace import` from it.  Distinct from `LOADS_EXTERNAL_UNIT`, whose
/// implied caller is normally a project file the host has already scanned.
const UNBOUNDABLE_BOUNDARIES: Traits = Traits::PROVIDES_PACKAGE.union(Traits::EXPORTS_COMMAND);

impl UnitCallerView<'_> {
    /// Whether a registry-declared boundary rules out seeding outright — see
    /// [`params_constants_from_native_call_sites`]'s gate for the full rule.
    fn declines_seeding(&self) -> bool {
        if self.linkage.intersects(UNBOUNDABLE_BOUNDARIES) {
            return true;
        }
        !self.has_cross_file_evidence && self.linkage.intersects(Traits::LOADS_EXTERNAL_UNIT)
    }
}

/// Build the SCCP `param_constants` seed for `qname` from collected call-site
/// literals: bind `(param, 0)` only when every caller passes the same single
/// literal at that position.
///
/// Beyond the per-slot literal-uniformity test
/// ([`CalleeEvidence::uniform_literal_at`]), two whole-module gates must also
/// hold — each closes a way the recorded call sites could be an incomplete
/// picture of every real caller (an unproven "every caller I found agrees"
/// says nothing about callers no scan could see):
///
/// - `!proc_binding_trust.trusts_proc_binding(qname)` — `qname`'s own
///   binding may have been perturbed by `rename` / `interp alias` / a
///   dynamic proc redefinition anywhere in the module, so a call reaching
///   it at runtime need not be one this scan attributed to it (and vice
///   versa). The projection deliberately does not inherit broader
///   command-effect opacity from an unrelated unresolved or external body.
/// - **No registry-declared unit boundary the evidence cannot cover.** The
///   two kinds are treated differently, because a host's enumeration can only
///   ever bound one of them:
///   - `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
///     `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) publish
///     this file's commands as an API surface. Their consumers need not be in
///     the host's project at all — another checkout can `package require`
///     this one — so no enumeration bounds them and the seed is declined
///     **unconditionally**.
///   - `LOADS_EXTERNAL_UNIT` (`source`, `load`, `package require`,
///     `auto_load`, `auto_import`) says another unit's script runs here and
///     can call back in. That unit is normally a project file, so a host that
///     supplied cross-file evidence
///     ([`UnitCallerView::has_cross_file_evidence`]) has already contributed
///     its call sites and the seed proceeds on the union; with no host view
///     it is a blind spot and the seed is declined.
///
/// - **The scan enumerated every caller** ([`CallSiteEvidence::
///   record_unenumerable_caller`]). A dispatch word (`$cmd args`) is resolved by
///   *value*, so an enumerable one is ordinary evidence attributed to each
///   name it may hold — deliberately not a module-wide wildcard, which is
///   what lets `$cmd prod` keep a sound seed while `$cmd dev` retracts only
///   the proc it can reach. But a dispatch whose values cannot be
///   enumerated, or a script a command receives only as a *value* (`eval
///   $script`, `apply $fn`), names a caller of *something* this scan cannot
///   identify — and since it could be any procedure with any argument, every
///   seed in the unit is withdrawn.
#[cfg(test)]
pub(crate) fn params_constants_from_call_sites(
    params: &[String],
    evidence: &CallSiteEvidence,
    qname: &str,
    view: &UnitCallerView<'_>,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    if view.declines_seeding() {
        return None;
    }
    if !view.proc_binding_trust.trusts_proc_binding(qname) {
        return None;
    }
    let callee = evidence.get(qname)?;
    let mut consts: HashMap<(String, crate::ssa::Version), LatticeValue> = HashMap::new();
    for (index, pname) in params.iter().enumerate() {
        // Only a *trailing* `args` is Tcl's variadic catch-all
        // (`TclCreateProc`, `generic/tclProc.c`): in `proc f {args x}` the
        // first word is an ordinary parameter and `x` is required.  Every
        // position from a trailing `args` on absorbs an unbounded, per-call
        // word list, so no single position beyond it can be literal-uniform.
        if pname == "args" && index + 1 == params.len() {
            break;
        }
        if let Some(value) = callee.uniform_literal_at(index) {
            consts.insert(
                (pname.clone(), 0),
                LatticeValue::Const(ConstValue::String(value.to_owned())),
            );
        }
    }
    if consts.is_empty() {
        None
    } else {
        Some(consts)
    }
}

/// Caller contents projected through the selected native activation plan.
/// Names identify logical formal inputs, never physical cell or object identity;
/// consumers still need the captured incoming binding before using a seed.
pub(crate) fn params_constants_from_native_call_sites(
    module: &IrModule,
    id: crate::ir::ProcedureId,
    procedure: &crate::ir::Procedure,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    evidence: &CallSiteEvidence,
    view: &UnitCallerView<'_>,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, bind_formal_arguments, parse_formal_parameters_in,
    };

    if view.declines_seeding()
        || !view
            .proc_binding_trust
            .trusts_proc_binding(&procedure.qualified_name)
    {
        return None;
    }
    let grammar = grammar?;
    // Jim's ordinary value binding can write a retained static fallback.
    // This logical caller table does not carry that activation's physical
    // formal binding, so it cannot project a fresh incoming slot for Jim.
    if grammar != tcl_dialect::ParameterGrammar::Tcl {
        return None;
    }
    let formals = parse_formal_parameters_in(&procedure.params_raw, grammar).ok()?;
    let mut names = BTreeSet::new();
    if formals.iter().any(|formal| !names.insert(&formal.name)) {
        return None;
    }
    if !view.has_cross_file_evidence && view.entered.is_some_and(|entered| !entered.contains(&id)) {
        return None;
    }
    let callee = evidence.get_for_body(module, id)?;
    if callee.opaque_caller
        || view.source.is_some_and(|source| {
            callee.source_implementations.iter().any(|allocation| {
                !allocation.matches_source_declaration(source, procedure.span.start())
            })
        })
    {
        return None;
    }
    let mut agreed: Option<BTreeMap<usize, String>> = None;
    for &count in &callee.arg_counts {
        let bindings = bind_formal_arguments(&formals, count, grammar).ok()?;
        let mut values = BTreeMap::new();
        for binding in bindings {
            match binding {
                FormalArgumentBinding::Value {
                    parameter,
                    argument,
                } => {
                    if let Some(value) = callee.uniform_literal_at(argument) {
                        values.insert(parameter, value.to_owned());
                    }
                }
                FormalArgumentBinding::Default { parameter } => {
                    values.insert(parameter, formals[parameter].default.clone()?);
                }
                FormalArgumentBinding::CallerLink { .. } => return None,
                FormalArgumentBinding::Rest {
                    parameter, name, ..
                } => {
                    if parameter + 1 != formals.len() || name != formals[parameter].name {
                        return None;
                    }
                }
            }
        }
        if let Some(agreed) = &mut agreed {
            agreed.retain(|parameter, value| values.get(parameter) == Some(value));
        } else {
            agreed = Some(values);
        }
    }
    let constants: HashMap<_, _> = agreed?
        .into_iter()
        .map(|(parameter, value)| {
            (
                (formals[parameter].name.clone(), 0),
                LatticeValue::Const(ConstValue::String(value)),
            )
        })
        .collect();
    (!constants.is_empty()).then_some(constants)
}
