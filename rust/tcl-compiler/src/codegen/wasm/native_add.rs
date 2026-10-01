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

//! The sealed native i64 addition: the premises its selection composes.
//!
//! Selection consumes common proof objects only. Every premise is evaluated
//! even after another has failed, so a compile that does not select the
//! addition records each premise it found wanting as a [`NativeDecline`], and
//! the selection and its record are one derivation that cannot disagree. A
//! premise whose input another premise rejects is not evaluated: the obstacle
//! is the one already recorded. While no pass the addition consumes is enabled
//! it has not been asked for, no proof is built, and the record is the
//! options' premises alone.

use std::collections::BTreeMap;

use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId};

use crate::analyses::{ConstValue, LatticeValue};
use crate::common_aot_plan::{
    ClosedProgramCoverageDecision, ClosedProgramCoverageEvidence, ClosedProgramStatementEvidence,
    CommonAotEnvironment, CommonAotProofPlan, DirectActualValue, DirectCallSiteId,
    DirectProcBodyDecision, DirectProcDecision, DirectProcEvidence, MaterialisableSlotDecision,
    NativeDecline, NativeDeclineReason, NativePremise, SemanticCallArgument, SemanticCallDecision,
    SsaValueIdentity,
};
use crate::compilation_unit::CompilationUnit;
use crate::intervals::Interval;
use crate::native_integer_proof::{
    NativeAddDecision, NativeAddExecution, NativeAddResult, NativeIntegerPolicy,
    NativeIntegerProof, prove_native_integer_adds,
};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::types::TypeShape;

use super::pipeline::{WasmCompileOptions, WasmNativeI64AddSelection};

/// The passes the addition consumes, in the order they are recorded.
const REQUIRED_PASSES: [SemanticOptimisationPassId; 5] = [
    SemanticOptimisationPassId::DirectProc,
    SemanticOptimisationPassId::MaterialisableSlot,
    SemanticOptimisationPassId::FrameElision,
    SemanticOptimisationPassId::NativeInteger,
    SemanticOptimisationPassId::SemanticOperationSpecialisation,
];

/// Select the sealed native i64 addition, or say which premises rejected it.
pub(super) fn select(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    options: WasmCompileOptions,
) -> Result<WasmNativeI64AddSelection, Vec<NativeDecline>> {
    let config = options.semantic_optimisations();
    let mut declines = Declines::default();
    reject_configuration(options, &mut declines);
    if !REQUIRED_PASSES.iter().any(|pass| config.is_enabled(*pass)) {
        // No pass the addition consumes is enabled, so it was not asked for:
        // the proofs stay unbuilt and the record is the options' answer.
        return Err(declines.into_vec());
    }

    let common = CommonAotProofPlan::build(
        unit,
        registry,
        unit.top_level.semantic_facts.context(),
        config,
        options.common_aot_environment(),
    );
    if !common.coverage_declines().is_empty() {
        declines.reject(
            NativePremise::Coverage,
            NativeDeclineReason::ExcludedSurfaces(common.coverage_declines().to_vec()),
            None,
        );
    }
    let coverage = match common.closed_program_coverage() {
        ClosedProgramCoverageDecision::Selected(coverage) => Some(coverage),
        ClosedProgramCoverageDecision::Declined(reason) => {
            declines.reject(
                NativePremise::ClosedProgram,
                NativeDeclineReason::ClosedProgram(*reason),
                None,
            );
            None
        }
    };
    let unit_admits = declines.is_empty();

    let mut composition = Composition {
        unit,
        registry,
        config,
        common: &common,
        coverage,
        proofs: BTreeMap::new(),
    };
    let mut any_call = false;
    for (site, decision) in common.direct_calls() {
        any_call = true;
        let mut site_declines = Declines::default();
        let selection = composition.site(site, decision, &mut site_declines);
        if let Some(selection) = selection
            && unit_admits
            && site_declines.is_empty()
        {
            return Ok(selection);
        }
        declines.extend(site_declines);
    }
    if !any_call {
        declines.reject(
            NativePremise::DirectCall,
            NativeDeclineReason::NoDirectCall,
            None,
        );
    }
    Err(declines.into_vec())
}

/// The premises the options answer, before any proof is consulted.
fn reject_configuration(options: WasmCompileOptions, declines: &mut Declines) {
    if !options.semantic_plans_enabled() {
        declines.reject(
            NativePremise::SemanticPlans,
            NativeDeclineReason::EvalOnlyTestHost,
            None,
        );
    }
    if options.is_standalone() {
        declines.reject(
            NativePremise::Packaging,
            NativeDeclineReason::StandaloneBootstrap,
            None,
        );
    }
    if options.common_aot_environment() != CommonAotEnvironment::SealedProgram {
        declines.reject(
            NativePremise::SealedProgram,
            NativeDeclineReason::HostedEnvironment,
            None,
        );
    }
    let config = options.semantic_optimisations();
    for pass in REQUIRED_PASSES {
        if !config.is_enabled(pass) {
            declines.reject(
                NativePremise::Pass(pass),
                NativeDeclineReason::PassDisabled,
                None,
            );
        }
    }
}

/// Rejected premises in evaluation order, one entry per premise and reason.
#[derive(Default)]
struct Declines(Vec<NativeDecline>);

impl Declines {
    fn reject(
        &mut self,
        premise: NativePremise,
        reason: NativeDeclineReason,
        site: Option<&DirectCallSiteId>,
    ) {
        self.record(NativeDecline {
            premise,
            reason,
            sites: site.cloned().into_iter().collect(),
        });
    }

    fn record(&mut self, decline: NativeDecline) {
        let held = self
            .0
            .iter_mut()
            .find(|held| held.premise == decline.premise && held.reason == decline.reason);
        let Some(held) = held else {
            self.0.push(decline);
            return;
        };
        for site in decline.sites {
            if !held.sites.contains(&site) {
                held.sites.push(site);
            }
        }
    }

    fn extend(&mut self, other: Self) {
        for decline in other.0 {
            self.record(decline);
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn into_vec(self) -> Vec<NativeDecline> {
        self.0
    }
}

/// The statements the closed program accounts for, as the addition reads them.
struct Covered {
    left: i64,
    right: i64,
    statements: u32,
}

/// The proofs one composition consults.
struct Composition<'a> {
    unit: &'a CompilationUnit,
    registry: &'a CommandRegistry,
    config: SemanticOptimisationConfig,
    common: &'a CommonAotProofPlan,
    coverage: Option<&'a ClosedProgramCoverageEvidence>,
    /// The integer proof per callee, since every call to one asks the same.
    proofs: BTreeMap<String, NativeIntegerProof>,
}

impl Composition<'_> {
    /// Evaluate every premise about one direct call, and build the selection
    /// it supports when none is rejected.
    fn site(
        &mut self,
        site: &DirectCallSiteId,
        decision: &DirectProcDecision,
        declines: &mut Declines,
    ) -> Option<WasmNativeI64AddSelection> {
        let direct = match decision {
            DirectProcDecision::Selected(direct) => direct,
            DirectProcDecision::Declined(decline) => {
                declines.reject(
                    NativePremise::DirectCall,
                    NativeDeclineReason::DirectCall(decline.clone()),
                    Some(site),
                );
                return None;
            }
        };
        let body = body_selected(site, direct, declines);
        let covered = self.covered(site, direct, declines);
        let frame = frame_elidable(site, direct, declines);
        let actuals = self.actuals(site, direct, declines);
        let boundary = self.boundary(site, declines);
        let ranges = self.integer(site, direct, declines);
        let operands = operands(site, covered.as_ref(), ranges, declines);
        if !(body && frame && actuals) {
            return None;
        }
        let ((left, right), covered, boundary_operation) = (operands?, covered?, boundary?);
        Some(WasmNativeI64AddSelection {
            callee: direct.callee.clone(),
            left,
            right,
            boundary_operation,
            frame_elided: true,
            closed_program_statements: covered.statements,
        })
    }

    /// The closed program is the procedure definition, the two constants the
    /// call passes, and the boundary that consumes its result.
    fn covered(
        &self,
        site: &DirectCallSiteId,
        direct: &DirectProcEvidence,
        declines: &mut Declines,
    ) -> Option<Covered> {
        let covered = covered_shape(self.coverage?, site, direct);
        if covered.is_none() {
            declines.reject(
                NativePremise::ClosedProgram,
                NativeDeclineReason::ClosedProgramShape,
                Some(site),
            );
        }
        covered
    }

    /// Both actuals are exact caller values in materialisable integer slots.
    fn actuals(
        &self,
        site: &DirectCallSiteId,
        direct: &DirectProcEvidence,
        declines: &mut Declines,
    ) -> bool {
        if direct.formals.len() != 2 || direct.actual_values.len() != 2 {
            declines.reject(
                NativePremise::Actuals,
                NativeDeclineReason::OperandCount,
                Some(site),
            );
            return false;
        }
        let mut admitted = true;
        for actual in &direct.actual_values {
            let rejection = match actual {
                DirectActualValue::Unproven => Some(NativeDeclineReason::ActualUnproven),
                DirectActualValue::Ssa(value) => self.slot_rejection(value),
            };
            if let Some(reason) = rejection {
                declines.reject(NativePremise::Actuals, reason, Some(site));
                admitted = false;
            }
        }
        admitted
    }

    fn slot_rejection(&self, value: &SsaValueIdentity) -> Option<NativeDeclineReason> {
        match self
            .common
            .materialisable_slots()
            .find(|(identity, _)| *identity == value)
        {
            Some((_, MaterialisableSlotDecision::Selected(slot)))
                if slot.shape == TypeShape::Int =>
            {
                None
            }
            Some((_, MaterialisableSlotDecision::Selected(_))) => {
                Some(NativeDeclineReason::SlotNotInteger)
            }
            Some((_, MaterialisableSlotDecision::Declined(decline))) => {
                Some(NativeDeclineReason::Slot(decline.clone()))
            }
            None => Some(NativeDeclineReason::NoSlotDecision),
        }
    }

    /// The registry boundary that consumes the call's boxed result.
    fn boundary(
        &self,
        site: &DirectCallSiteId,
        declines: &mut Declines,
    ) -> Option<SemanticOperationId> {
        let boundary = selected_boundary(self.common, site);
        if boundary.is_none() {
            declines.reject(
                NativePremise::Boundary,
                NativeDeclineReason::NoSelectedBoundary,
                Some(site),
            );
        }
        boundary
    }

    /// The operand ranges of the callee's one addition, when the integer proof
    /// accepts it as a non-overflowing return that only this call reaches.
    fn integer(
        &mut self,
        site: &DirectCallSiteId,
        direct: &DirectProcEvidence,
        declines: &mut Declines,
    ) -> Option<(Interval, Interval)> {
        let callee = direct.callee.qualified_name.as_str();
        let (unit, registry, config, common) = (self.unit, self.registry, self.config, self.common);
        let proof = self.proofs.entry(callee.to_owned()).or_insert_with(|| {
            prove_native_integer_adds(
                unit,
                callee,
                registry,
                config,
                NativeIntegerPolicy::default(),
                common,
            )
        });
        let mut note = |reason| {
            declines.reject(NativePremise::NativeInteger, reason, Some(site));
        };
        let decisions = match proof {
            NativeIntegerProof::Analysed(decisions) => decisions,
            NativeIntegerProof::Disabled => {
                note(NativeDeclineReason::PassDisabled);
                return None;
            }
            NativeIntegerProof::FunctionUnavailable => {
                note(NativeDeclineReason::ProofFunctionUnavailable);
                return None;
            }
            NativeIntegerProof::ComplexityGuarded => {
                note(NativeDeclineReason::ProofComplexityGuarded);
                return None;
            }
        };
        let native = match decisions.as_slice() {
            [NativeAddDecision::Proven(native)] => native,
            [] => {
                note(NativeDeclineReason::NoAddCandidate);
                return None;
            }
            [NativeAddDecision::Declined { reason, .. }] => {
                note(NativeDeclineReason::Integer(*reason));
                return None;
            }
            _ => {
                note(NativeDeclineReason::AmbiguousAdd);
                return None;
            }
        };
        let mut accepted = true;
        if native.composition.direct_calls.as_slice() != std::slice::from_ref(site) {
            note(NativeDeclineReason::CallerSetDiffers);
            accepted = false;
        }
        let slots_are_integers = [native.left.value, native.right.value]
            .into_iter()
            .all(|value| selected_integer_slot(common, callee, value));
        if native.site.result != NativeAddResult::FunctionReturn
            || native.execution != NativeAddExecution::OverflowImpossible
            || !native.composition.requires_frame_plan
            || !native
                .composition
                .requires_internal_operation_guard_or_sealed_policy
            || !slots_are_integers
        {
            note(NativeDeclineReason::AddShape);
            accepted = false;
        }
        accepted.then_some((native.left.range, native.right.range))
    }
}

/// The body of the called procedure may run specialised.
fn body_selected(
    site: &DirectCallSiteId,
    direct: &DirectProcEvidence,
    declines: &mut Declines,
) -> bool {
    match &direct.body {
        DirectProcBodyDecision::Selected(_) => true,
        DirectProcBodyDecision::Declined(decline) => {
            declines.reject(
                NativePremise::DirectBody,
                NativeDeclineReason::DirectBody(*decline),
                Some(site),
            );
            false
        }
    }
}

/// The callee's frame is private to it and the frame pass authorises omitting it.
fn frame_elidable(
    site: &DirectCallSiteId,
    direct: &DirectProcEvidence,
    declines: &mut Declines,
) -> bool {
    let mut admitted = true;
    if !direct.frame_escape_private {
        declines.reject(
            NativePremise::Frame,
            NativeDeclineReason::FrameEscapes,
            Some(site),
        );
        admitted = false;
    }
    if !direct.frame_elidable {
        declines.reject(
            NativePremise::Frame,
            NativeDeclineReason::FrameNotElidable,
            Some(site),
        );
        admitted = false;
    }
    admitted
}

/// The proved operands are exact, and are the constants the program defines.
fn operands(
    site: &DirectCallSiteId,
    covered: Option<&Covered>,
    ranges: Option<(Interval, Interval)>,
    declines: &mut Declines,
) -> Option<(i64, i64)> {
    let (covered, (left, right)) = (covered?, ranges?);
    match (exact_i64(left), exact_i64(right)) {
        (Some(left), Some(right)) if (left, right) == (covered.left, covered.right) => {
            Some((left, right))
        }
        (Some(_), Some(_)) => {
            declines.reject(
                NativePremise::Operands,
                NativeDeclineReason::OperandsDiffer,
                Some(site),
            );
            None
        }
        _ => {
            declines.reject(
                NativePremise::Operands,
                NativeDeclineReason::OperandNotExact,
                Some(site),
            );
            None
        }
    }
}

fn selected_integer_slot(
    common: &CommonAotProofPlan,
    function: &str,
    value: crate::ssa::ValueKey,
) -> bool {
    common.materialisable_slots().any(|(identity, decision)| {
        identity.function == function
            && (identity.symbol, identity.version) == value
            && matches!(
                decision,
                MaterialisableSlotDecision::Selected(evidence) if evidence.shape == TypeShape::Int
            )
    })
}

/// Read the four accounted statements as the addition consumes them: the
/// definition of the called procedure, the two constants it is called with,
/// and the boundary that consumes the call.
fn covered_shape(
    coverage: &ClosedProgramCoverageEvidence,
    direct_site: &DirectCallSiteId,
    direct: &DirectProcEvidence,
) -> Option<Covered> {
    let [
        ClosedProgramStatementEvidence::DirectProcedureDefinition { procedure, .. },
        ClosedProgramStatementEvidence::DirectActualConstant {
            value: first,
            constant: left,
            operation: left_operation,
            ..
        },
        ClosedProgramStatementEvidence::DirectActualConstant {
            value: second,
            constant: right,
            operation: right_operation,
            ..
        },
        ClosedProgramStatementEvidence::SemanticBoundary { call, operation },
    ] = coverage.statements.as_slice()
    else {
        return None;
    };
    if !(procedure == &direct.callee
        && matches!(
            direct.actual_values.as_slice(),
            [DirectActualValue::Ssa(left), DirectActualValue::Ssa(right)] if left == first && right == second
        )
        && call.function == direct_site.function
        && call.block == direct_site.block
        && call.statement_index == direct_site.statement_index
        && call.nested_argument.is_none()
        && *operation == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
        && *left_operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && *right_operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set))
    {
        return None;
    }
    Some(Covered {
        left: lattice_i64(left)?,
        right: lattice_i64(right)?,
        statements: u32::try_from(coverage.statements.len()).ok()?,
    })
}

fn lattice_i64(value: &LatticeValue) -> Option<i64> {
    match value {
        LatticeValue::Const(ConstValue::Int(value)) => Some(*value),
        LatticeValue::Unknown
        | LatticeValue::Const(ConstValue::Float(_) | ConstValue::Bool(_) | ConstValue::String(_))
        | LatticeValue::ConstSet(_)
        | LatticeValue::Overdefined => None,
    }
}

fn selected_boundary(
    common: &CommonAotProofPlan,
    direct_site: &DirectCallSiteId,
) -> Option<SemanticOperationId> {
    common.semantic_calls().find_map(|(_, decision)| {
        let SemanticCallDecision::Selected(evidence) = decision else {
            return None;
        };
        (evidence.operation == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
            && matches!(
                evidence.arguments.as_slice(),
                [SemanticCallArgument::NestedDirect { outer_argument: 0, call }] if call == direct_site
            ))
        .then_some(evidence.operation)
    })
}

fn exact_i64(interval: Interval) -> Option<i64> {
    match (interval.lo, interval.hi) {
        (Some(value), Some(same)) if value == same => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{WasmCodegenPlan, compile_wasm};
    use super::*;

    const SEALED: &str =
        "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n";

    fn options(passes: &[SemanticOptimisationPassId]) -> WasmCompileOptions {
        passes.iter().fold(
            WasmCompileOptions::hosted().for_sealed_program(),
            |options, pass| options.with_semantic_optimisation(*pass),
        )
    }

    fn without(pass: SemanticOptimisationPassId) -> Vec<SemanticOptimisationPassId> {
        REQUIRED_PASSES
            .into_iter()
            .filter(|required| *required != pass)
            .collect()
    }

    fn plan_of(source: &str, options: WasmCompileOptions) -> WasmCodegenPlan {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
        compile_wasm(&unit, &registry, options).plan
    }

    /// One line per rejected premise: the premise (and its pass), the reason,
    /// and how many call sites it was rejected at.
    fn summary(plan: &WasmCodegenPlan) -> Vec<String> {
        plan.native_declines()
            .iter()
            .map(|decline| {
                let premise = match decline.premise {
                    NativePremise::Pass(pass) => format!("pass {}", pass.as_str()),
                    other => other.as_str().to_owned(),
                };
                let sites = match decline.sites.len() {
                    0 => String::new(),
                    count => format!(" x{count}"),
                };
                format!("{premise}: {}{sites}", decline.reason.as_str())
            })
            .collect()
    }

    #[test]
    fn a_selected_addition_records_no_decline() {
        let plan = plan_of(SEALED, options(&REQUIRED_PASSES));
        assert!(matches!(plan, WasmCodegenPlan::NativeI64Add { .. }));
        assert!(plan.native_declines().is_empty());
    }

    #[test]
    fn an_addition_nobody_asked_for_records_the_options_premises_alone() {
        let plan = plan_of(SEALED, WasmCompileOptions::hosted());
        assert_eq!(
            summary(&plan),
            [
                "sealed-program: hosted-environment",
                "pass direct-proc: pass-disabled",
                "pass materialisable-slot: pass-disabled",
                "pass frame-elision: pass-disabled",
                "pass native-integer: pass-disabled",
                "pass semantic-operation-specialisation: pass-disabled",
            ]
        );
        assert!(plan.native_declines().iter().all(|d| d.sites.is_empty()));
    }

    #[test]
    fn a_hosted_compile_names_the_environment_the_passes_and_what_they_take_down() {
        let requested = [
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ];
        let plan = plan_of(
            SEALED,
            WasmCompileOptions::hosted().with_semantic_optimisations({
                let mut config = SemanticOptimisationConfig::new();
                for pass in requested {
                    config.enable(pass);
                }
                config
            }),
        );
        assert_eq!(
            summary(&plan),
            [
                "sealed-program: hosted-environment",
                "pass direct-proc: pass-disabled",
                "pass native-integer: pass-disabled",
                "closed-program: hosted-environment",
                "direct-call: pass-disabled x1",
            ]
        );
    }

    #[test]
    fn a_rejected_body_records_each_premise_it_takes_down() {
        let plan = plan_of(
            SEALED,
            options(&without(SemanticOptimisationPassId::NativeInteger)),
        );
        assert_eq!(
            summary(&plan),
            [
                "pass native-integer: pass-disabled",
                "direct-body: native-integer-pass-disabled x1",
                "frame: frame-not-elidable x1",
                "native-integer: pass-disabled x1",
            ]
        );
        let site = DirectCallSiteId {
            function: "::top".to_owned(),
            block: crate::cfg::BlockId(0),
            statement_index: 3,
            nested_argument: Some(0),
        };
        assert!(
            plan.native_declines()[1..]
                .iter()
                .all(|decline| decline.sites == [site.clone()])
        );
    }

    #[test]
    fn a_surviving_statement_rejects_the_closed_program_and_the_actuals_it_hides() {
        let source = format!("puts before\n{SEALED}");
        let plan = plan_of(&source, options(&REQUIRED_PASSES));
        assert!(matches!(plan, WasmCodegenPlan::General { .. }));
        assert_eq!(
            summary(&plan),
            [
                "closed-program: uncovered-statement",
                // Two actuals, one call: the site is named once.
                "actuals: no-local-slot x1",
            ]
        );
    }

    #[test]
    fn a_program_without_a_procedure_call_names_the_missing_call() {
        let plan = plan_of("puts before\n", options(&REQUIRED_PASSES));
        assert_eq!(
            summary(&plan),
            [
                "closed-program: uncovered-statement",
                "direct-call: no-direct-call",
            ]
        );
    }

    #[test]
    fn a_second_call_to_the_callee_is_recorded_at_both_sites() {
        let source = format!("{SEALED}puts [add $e $d]\n");
        let plan = plan_of(&source, options(&REQUIRED_PASSES));
        assert_eq!(
            summary(&plan),
            [
                "closed-program: closed-program-shape x2",
                "native-integer: caller-set-differs x2",
            ]
        );
    }

    #[test]
    fn the_target_and_its_packaging_are_premises_too() {
        let options = REQUIRED_PASSES.iter().fold(
            WasmCompileOptions::standalone(false)
                .for_sealed_program()
                .for_eval_only_test_host(),
            |options, pass| options.with_semantic_optimisation(*pass),
        );
        let plan = plan_of(SEALED, options);
        assert_eq!(
            summary(&plan),
            [
                "semantic-plans: eval-only-test-host",
                "packaging: standalone-bootstrap",
            ]
        );
    }
}
