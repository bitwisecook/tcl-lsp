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

#[test]
// Implementation contract: naming.variable.aot-original-slot-purpose
// docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
fn original_native_entry_keeps_cells_and_runtime_trace_guards_despite_escape_labels() {
    use super::*;
    let registry = CommandRegistry::build_default();
    let function = crate::execution_region::evaluated_region_test_fixture();
    let mutations = ModuleCommandMutations::default();
    let hints = BTreeMap::new();
    let mut summary = ProcEscapeSummary::default();
    summary.local_slots.insert("x".into(), 11);
    let config = SemanticOptimisationConfig::new()
        .with_enabled(SemanticOptimisationPassId::CellDemotion)
        .with_enabled(SemanticOptimisationPassId::TraceBarrierElision);
    let authored = Module::default();
    let input = LoweringInput {
        registry: &registry,
        context: None,
        function: &function,
        source: "",
        module: &authored,
        mutations: &mutations,
        config,
        escape: Some(&summary),
        top_level: false,
        line_origin: 0,
        entry_assumption: DispatchEntryAssumption::UnknownWorld,
        type_hints: &hints,
    };
    let lowerer = Lowerer::new(&input);
    let place = super::super::cells::CellPlace::Named { name: "x".into() };
    assert_eq!(
        lowerer.demotion.decide("x").storage,
        super::super::cells::CellStorage::Slot(11)
    );
    assert_eq!(
        lowerer.ledger.incr_guard(&place),
        super::super::elide::IncrGuard::Unguarded
    );
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    entry.frame = tcl_runtime_api::native_compilation::NativeCompilationFrame::Procedure;
    let mut native = authored.clone();
    native.source_entry.native_entry = Some(std::sync::Arc::new(entry.clone()));
    let native_input = LoweringInput {
        module: &native,
        ..input
    };
    let lowerer = Lowerer::new(&native_input);
    assert_eq!(
        lowerer.demotion.decide("x").storage,
        super::super::cells::CellStorage::Cell
    );
    assert_eq!(
        lowerer.ledger.incr_guard(&place),
        super::super::elide::IncrGuard::RuntimeTraceBit
    );
    let mut traced = native.clone();
    traced.traced_variables.insert("x".into());
    let traced_input = LoweringInput {
        module: &traced,
        ..native_input
    };
    let lowerer = Lowerer::new(&traced_input);
    assert_eq!(
        lowerer.ledger.incr_guard(&place),
        super::super::elide::IncrGuard::RuntimeTraceBit
    );
}
