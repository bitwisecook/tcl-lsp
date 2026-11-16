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

//! Source-may definitions and clobber receipts: source identity, changed maps and versions reaching post-join observers.

use super::*;

#[test]
fn original_source_may_definitions_keep_source_identity_separate_from_locations() {
    let (_, unit) = crate::interprocedural::logical_completion_unit(
        "proc p {} {global g; set g 1}\np",
        "tcl8.6",
    );
    let function = &unit.top_level;
    let sources: Vec<_> = function.ssa.source_variable_symbols().collect();
    assert!(
        !sources.is_empty(),
        "actual retained hidden write must issue source geometry"
    );
    assert_eq!(
        function.ssa.cell_keys().len() + sources.len(),
        function.ssa.var_names().len()
    );
    for (symbol, source) in sources {
        assert!(function.ssa.cell_key(symbol).is_none());
        assert!(function.ssa.cell_name(symbol).is_none());
        assert!(function.ssa.variable_symbols().any(|held| held == symbol));
        assert!(function.ssa.cell_symbols().all(|(held, _)| held != symbol));
        assert_eq!(
            function
                .def_use
                .source_variable_symbol(&function.ssa, symbol),
            Some(source)
        );
        assert!(function.ssa.source_variable_snapshot(symbol).is_some());
        let mut reached = false;
        for (&block, body) in &function.ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if let Some(&version) = statement.defs.get(&symbol) {
                    reached = true;
                    assert!(statement.may_defs.contains(&symbol));
                    assert!(function.ssa.source_may_requires_unknown_at(block, index));
                    assert_eq!(
                        function.sccp.values.get(&(symbol, version)),
                        Some(&crate::analyses::LatticeValue::Overdefined)
                    );
                    assert_eq!(
                        function.sccp.existence.get(&(symbol, version)),
                        Some(&tcl_registry::value_transfer::Existence::MayBound)
                    );
                    let mut stale = function.ssa.clone();
                    let statement =
                        &mut stale.blocks.get_mut(&block).unwrap().statements[index].statement;
                    let Statement::Call { command, .. } = statement else {
                        panic!("actual original call");
                    };
                    command.push_str("-report-changed");
                    assert!(stale.source_variable_snapshot(symbol).is_none());
                    assert!(
                        function
                            .def_use
                            .source_variable_symbol(&stale, symbol)
                            .is_none()
                    );
                    assert!(stale.source_may_requires_unknown_at(block, index));
                }
            }
        }
        assert!(
            reached,
            "source identity must retain its actual definition point/version"
        );
    }
}

#[test]
fn original_source_may_clobber_receipts_refuse_removed_or_changed_maps() {
    let (_, unit) = crate::interprocedural::logical_completion_unit(
        "proc p {} {global g; set g 1}\nset local before\np\nreturn [unknown_read]",
        "tcl8.6",
    );
    let ssa = &unit.top_level.ssa;
    let mut reached = false;
    for (&(block, index), effect) in &ssa.source_may_effects {
        if effect
            .original
            .as_ref()
            .is_none_or(|original| original.writes().is_empty())
        {
            continue;
        }
        reached = true;
        assert!(
            ssa.source_may_clobber_is_current_at(block, index),
            "actual live location must have a producer-issued marker/clobber relation"
        );
        let held = effect.clobber.as_ref().unwrap();
        assert!(!held.versions.is_empty());
        assert!(is_unseen_call_marker(
            &ssa.blocks[&block].statements[held.marker_index].statement
        ));
        let mut stale = ssa.clone();
        stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .remove(&held.marker_index);
        assert!(is_unseen_call_marker(
            &stale.blocks[&block].statements[held.marker_index].statement
        ));
        assert!(!stale.source_may_clobber_is_current_at(block, index));
        assert!(stale.source_may_requires_unknown_at(block, index));
        let mut stale = ssa.clone();
        stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .get_mut(&held.marker_index)
            .unwrap()
            .clear();
        assert!(
            !stale.source_may_clobber_is_current_at(block, index),
            "empty clobber inventory is not closed coverage"
        );
        let mut stale = ssa.clone();
        let versions = stale
            .value_clobbers
            .get_mut(&block)
            .unwrap()
            .get_mut(&held.marker_index)
            .unwrap();
        let (&symbol, &pair) = versions.iter().next().unwrap();
        versions.insert(symbol, (pair.0, pair.1 + 1));
        assert!(!stale.source_may_clobber_is_current_at(block, index));
    }
    assert!(
        reached,
        "genuine call source-MAY producer must be exercised"
    );
}

fn assert_source_may_identity(
    function: &crate::compilation_unit::FunctionUnit,
    symbol: Symbol,
    variable: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
) {
    assert!(function.ssa.cell_key(symbol).is_none());
    assert_eq!(
        function
            .def_use
            .source_variable_symbol(&function.ssa, symbol),
        Some(variable)
    );
}

fn source_may_cfg_has_cycle(function: &crate::compilation_unit::FunctionUnit) -> bool {
    let mut cyclic = false;
    for &start in function.cfg.blocks.keys() {
        let mut pending = function.cfg.block_successors(start);
        let mut seen = HashSet::new();
        while let Some(next) = pending.pop() {
            if next == start {
                cyclic = true;
                break;
            }
            if seen.insert(next) {
                pending.extend(function.cfg.block_successors(next));
            }
        }
    }
    cyclic
}

fn assert_source_may_stale_graphs(
    function: &crate::compilation_unit::FunctionUnit,
    symbol: Symbol,
    phis: &HashMap<Version, (BlockId, &Phi)>,
) {
    let (&version, &(block, _)) = phis.iter().next().unwrap();
    let mut stale = function.ssa.clone();
    let phi = stale
        .blocks
        .get_mut(&block)
        .unwrap()
        .phis
        .iter_mut()
        .find(|phi| phi.name == symbol && phi.version == version)
        .unwrap();
    phi.incoming.clear();
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "same-source changed phi graph cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    let (_, body) = stale
        .blocks
        .iter_mut()
        .find(|(_, body)| {
            body.statements
                .iter()
                .any(|statement| statement.uses.contains_key(&symbol))
        })
        .unwrap();
    let statement = body
        .statements
        .iter_mut()
        .find(|statement| statement.uses.contains_key(&symbol))
        .unwrap();
    statement.uses.remove(&symbol);
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "same-source changed source use cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    stale.source_topology.get_mut(&block).unwrap().terminator = None;
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "changed retained source topology cannot reuse old chains"
    );
    let mut stale = function.ssa.clone();
    stale.idom.clear();
    assert!(
        function
            .def_use
            .source_variable_symbol(&stale, symbol)
            .is_none(),
        "changed SSA topology cannot reuse old chains"
    );
}

fn assert_original_source_may_joins(source: &str, minimum_calls: usize, has_loop: bool) {
    let (_, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
    let function = &unit.top_level;
    let sources: Vec<_> = function.ssa.source_variable_symbols().collect();
    assert!(
        !sources.is_empty(),
        "actual hidden writes must issue source identities"
    );
    let cyclic = source_may_cfg_has_cycle(function);
    assert_eq!(
        cyclic, has_loop,
        "control must retain its actual CFG branch/backedge geometry"
    );
    for (symbol, variable) in sources {
        assert_source_may_identity(function, symbol, variable);
        let definitions: HashSet<_> = function
            .ssa
            .blocks
            .values()
            .flat_map(|body| {
                body.statements
                    .iter()
                    .filter_map(|statement| statement.defs.get(&symbol).copied())
            })
            .collect();
        assert!(
            definitions.len() >= minimum_calls,
            "each genuine call point must retain its own source MAY version"
        );
        let mut phis = HashMap::new();
        for (&block, body) in &function.ssa.blocks {
            for phi in body.phis.iter().filter(|phi| phi.name == symbol) {
                assert!(!phi.incoming.is_empty());
                for (&predecessor, &version) in &phi.incoming {
                    assert!(function.cfg.block_successors(predecessor).contains(&block));
                    assert!(
                        function
                            .def_use
                            .uses_of(symbol, version)
                            .iter()
                            .any(|use_site| {
                                use_site.kind == crate::def_use::UseKind::PhiIncoming
                                    && use_site.phi_version == phi.version
                                    && use_site.class == UseClass::Quoted
                            })
                    );
                }
                phis.insert(phi.version, (block, phi));
            }
        }
        assert!(
            !phis.is_empty(),
            "source definitions must use the existing SSA join algorithm"
        );
        let mut observed = false;
        for (&block, body) in &function.ssa.blocks {
            if !matches!(
                &function.cfg.blocks[&block].terminator,
                Some(cfg::Terminator::Return { .. })
            ) {
                continue;
            }
            let Some(&exit) = body.exit_versions.get(&symbol) else {
                continue;
            };
            if exit == 0 {
                continue;
            }
            let mut pending = vec![exit];
            let mut reaching = HashSet::new();
            while let Some(version) = pending.pop() {
                if !reaching.insert(version) {
                    continue;
                }
                if let Some((_, phi)) = phis.get(&version) {
                    pending.extend(phi.incoming.values().copied());
                }
            }
            if !definitions.is_subset(&reaching) {
                continue;
            }
            observed = true;
            assert!(
                function
                    .def_use
                    .uses_of(symbol, exit)
                    .iter()
                    .any(|use_site| {
                        use_site.kind == crate::def_use::UseKind::Terminator
                            && use_site.block == body.name
                            && use_site.class == UseClass::Quoted
                    })
            );
        }
        assert!(
            observed,
            "the actual post-join observer must retain every reaching source definition"
        );
        assert_source_may_stale_graphs(function, symbol, &phis);
    }
}

#[test]
fn original_source_may_branch_versions_reach_post_join_observers() {
    assert_original_source_may_joins(
        "proc p {} {global g; set g 1}\nif {$condition} {p} else {p}\nreturn [unknown_read]",
        2,
        false,
    );
}

#[test]
fn original_source_may_loop_versions_reach_post_join_observers() {
    assert_original_source_may_joins(
        "proc p {} {global g; set g 1}\nwhile {$condition} {p}\nreturn [unknown_read]",
        1,
        true,
    );
}
