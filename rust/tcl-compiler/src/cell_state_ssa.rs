// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cell-content SSA over program-point bound accesses. Versions follow CFG
//! predecessors and overlapping writes, never block enumeration order.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use tcl_registry::CommandRegistry;

use crate::cfg::{BlockId, Function};
use crate::place::{Place, overlap};
use crate::place_bridge::{read_places_at, terminator_read_places_at};
use crate::state_ssa::{
    CfgStatePosition, StateClobber, StateDef, StateOp, StatePhi, StateSite, StateSsa,
    StateSsaError, StateUse, StateVersion,
};
use crate::variable_bindings::PointResolveContexts;

#[derive(Debug, Clone)]
struct Access {
    site: StateSite,
    location: Place,
    writes: bool,
    clobber: bool,
    version: StateVersion,
}

/// Build real reaching cell-state versions for every bound access. A returned
/// error preserves the absence of facts rather than manufacturing a graph.
///
/// # Errors
/// Returns a version-exhaustion error when the graph cannot represent another state version.
pub fn build_cell_state_ssa(
    cfg: &Function,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Result<StateSsa<Place>, StateSsaError> {
    let accesses = collect_accesses(cfg, points, registry)?;
    let mut locations = Vec::new();
    for access in accesses.values().flatten() {
        if !locations.contains(&access.location) {
            locations.push(access.location.clone());
        }
    }
    let predecessors = predecessors(cfg);
    let mut next_version = accesses
        .values()
        .flatten()
        .map(|access| access.version.raw())
        .max()
        .unwrap_or(0);
    let mut abrupt = BTreeMap::new();
    for &edge in &cfg.exception_edges {
        abrupt.insert(edge, allocate(&mut next_version)?);
    }
    let phis = allocate_phis(&predecessors, cfg.entry, locations.len(), &mut next_version)?;
    let exits = solve_versions(cfg, &accesses, &locations, &predecessors, &phis, &abrupt);
    materialise(
        cfg,
        &accesses,
        &locations,
        &predecessors,
        &phis,
        &exits,
        &abrupt,
    )
}

fn address(mut place: Place) -> Place {
    place.observed = false;
    place.name_reads.clear();
    if let Some(index) = &mut place.index {
        index.read_places.clear();
    }
    for key in &mut place.keys {
        key.read_places.clear();
    }
    place
}

fn allocate(next: &mut u32) -> Result<StateVersion, StateSsaError> {
    *next = next.checked_add(1).ok_or(StateSsaError::VersionExhausted)?;
    Ok(StateVersion::new(*next))
}

fn collect_accesses(
    cfg: &Function,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Result<BTreeMap<BlockId, Vec<Access>>, StateSsaError> {
    let mut accesses = BTreeMap::new();
    let mut next = 0;
    let represented_writes: std::collections::HashSet<_> = cfg
        .blocks
        .values()
        .flat_map(|block| {
            block
                .statements
                .iter()
                .map(|statement| statement.span().start())
        })
        .collect();
    let ids: BTreeSet<_> = cfg.blocks.keys().copied().collect();
    for id in ids {
        let block = &cfg.blocks[&id];
        let mut block_accesses = Vec::new();
        for (index, statement) in block.statements.iter().enumerate() {
            block_accesses.extend(collect_statement_accesses(
                statement,
                (id, index),
                points,
                registry,
                &represented_writes,
                &mut next,
            )?);
        }
        if let Some(terminator) = &block.terminator {
            block_accesses.extend(collect_terminator_accesses(
                terminator,
                id,
                points,
                registry,
                &represented_writes,
                &mut next,
            )?);
        }
        accesses.insert(id, block_accesses);
    }
    Ok(accesses)
}

fn collect_terminator_accesses(
    terminator: &crate::cfg::Terminator,
    block: BlockId,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
    represented_writes: &std::collections::HashSet<u32>,
    next: &mut u32,
) -> Result<Vec<Access>, StateSsaError> {
    let reads = terminator_read_places_at(terminator, block, points, registry);
    let mut clobbers =
        unrepresented_read_clobbers(points, (block, usize::MAX), registry, represented_writes);
    if reads.iter().chain(&clobbers).any(|place| place.observed) {
        clobbers.push(crate::place::unknown_top());
    }
    clobbers
        .into_iter()
        .map(|place| (place, true))
        .chain(reads.into_iter().map(|place| (place, false)))
        .enumerate()
        .map(|(ordinal, (location, clobber))| {
            Ok(Access {
                site: StateSite::terminator(
                    block,
                    u32::try_from(ordinal).map_err(|_| StateSsaError::VersionExhausted)?,
                ),
                location: address(location),
                writes: clobber,
                clobber,
                version: if clobber {
                    allocate(next)?
                } else {
                    StateVersion::INITIAL
                },
            })
        })
        .collect()
}

fn collect_statement_accesses(
    statement: &crate::ir::Statement,
    point: (BlockId, usize),
    points: &PointResolveContexts,
    registry: &CommandRegistry,
    represented_writes: &std::collections::HashSet<u32>,
    next: &mut u32,
) -> Result<Vec<Access>, StateSsaError> {
    let (block, index) = point;
    let index_u32 = u32::try_from(index).map_err(|_| StateSsaError::VersionExhausted)?;
    let before = points.before_statement(block, index);
    let after = points.after_statement(block, index);
    let definitions: Vec<_> =
        crate::place_bridge::def_places_with_continuation(statement, before, after, registry)
            .into_iter()
            .collect();
    let destructive = crate::place_bridge::statement_destruction_places_with_continuation(
        statement, before, after, registry,
    )
    .into_iter()
    .filter(|location| !definitions.contains(location));
    let scoped_clobbers =
        crate::dictionary_bindings::scope_marker_effects(statement, before, registry)
            .map_or_else(Vec::new, |effects| effects.clobbers);
    let nested = unrepresented_read_clobbers(points, point, registry, represented_writes);
    let reads = read_places_at(statement, block, index, points, registry);
    let before_read = reads
        .iter()
        .chain(&nested)
        .any(|place| place.observed)
        .then(crate::place::unknown_top);
    let mut selected: Vec<_> = nested
        .into_iter()
        .chain(scoped_clobbers)
        .chain(before_read)
        .map(|location| (location, true, true))
        .chain(reads.into_iter().map(|location| (location, false, false)))
        .chain(definitions.iter().cloned().map(|location| {
            let clobber = location.dynamic;
            (location, true, clobber)
        }))
        .chain(destructive.map(|location| (location, true, true)))
        .collect();
    if statement.has_opaque_native_accesses()
        || selected
            .iter()
            .any(|(location, writes, _)| *writes && location.observed)
        || (crate::memory_ssa::is_clobber(
            statement,
            registry,
            registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        ) && after.dynamic_bindings)
    {
        selected.push((crate::place::unknown_top(), true, true));
    }
    selected
        .into_iter()
        .enumerate()
        .map(|(ordinal, (location, writes, clobber))| {
            Ok(Access {
                site: StateSite::statement(
                    block,
                    index_u32,
                    u32::try_from(ordinal).map_err(|_| StateSsaError::VersionExhausted)?,
                ),
                location: address(location),
                writes,
                clobber,
                version: if writes {
                    allocate(next)?
                } else {
                    StateVersion::INITIAL
                },
            })
        })
        .collect()
}

// Read inventories are lexical, so an unrepresented nested store forgets the
// affected contents before this read group without inventing execution order.
fn unrepresented_read_clobbers(
    points: &PointResolveContexts,
    point: (BlockId, usize),
    registry: &CommandRegistry,
    represented_writes: &std::collections::HashSet<u32>,
) -> Vec<Place> {
    let mut nested_clobbers = Vec::new();
    let lexical = points
        .source_reads_at(point.0, point.1)
        .iter()
        .map(|access| {
            (
                access.place_in_context(&access.variable_context, registry),
                access.variable_context.as_ref(),
            )
        });
    let executed = points
        .source_tokens_at(point.0, point.1)
        .into_iter()
        .flat_map(|tokens| {
            crate::place_bridge::invocation_execution_read_contexts(tokens, registry)
        });
    collect_unrepresented_reads(
        lexical.chain(executed),
        represented_writes,
        &mut nested_clobbers,
    );
    if let Some(tokens) = points.source_tokens_at(point.0, point.1) {
        let grammar = crate::place_bridge::invocation_read_grammar(tokens, registry);
        for nested in crate::word_subst::lifted_calls(Some(tokens), grammar) {
            if let Some(tokens) = nested.tokens {
                collect_unrepresented_reads(
                    crate::place_bridge::invocation_execution_read_contexts(&tokens, registry)
                        .into_iter(),
                    represented_writes,
                    &mut nested_clobbers,
                );
            }
        }
    }
    nested_clobbers
}

fn collect_unrepresented_reads<'a>(
    reads: impl Iterator<Item = (Place, &'a crate::var_resolve::ResolveContext)>,
    represented_writes: &std::collections::HashSet<u32>,
    nested_clobbers: &mut Vec<Place>,
) {
    for (location, context) in reads {
        let origin = context.contents_origin(&location);
        let represented = match origin {
            crate::var_resolve::ContentsOrigin::Incoming => true,
            crate::var_resolve::ContentsOrigin::WrittenAt(source) => {
                represented_writes.contains(&source)
            }
            crate::var_resolve::ContentsOrigin::Alternatives { writes, .. } => writes
                .iter()
                .all(|source| represented_writes.contains(source)),
            crate::var_resolve::ContentsOrigin::Unknown => false,
        };
        if (!represented || location.observed) && !nested_clobbers.contains(&location) {
            nested_clobbers.push(location);
        }
    }
}

fn predecessors(cfg: &Function) -> BTreeMap<BlockId, Vec<BlockId>> {
    let mut result: BTreeMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|&id| (id, Vec::new())).collect();
    for &id in cfg.blocks.keys() {
        for successor in cfg.block_successors(id) {
            result.entry(successor).or_default().push(id);
        }
    }
    for incoming in result.values_mut() {
        incoming.sort_unstable();
        incoming.dedup();
    }
    result
}

fn allocate_phis(
    predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    entry: BlockId,
    location_count: usize,
    next: &mut u32,
) -> Result<BTreeMap<(BlockId, usize), StateVersion>, StateSsaError> {
    let mut phis = BTreeMap::new();
    for (&id, incoming) in predecessors {
        if incoming.len() > 1 || (id == entry && !incoming.is_empty()) {
            for location in 0..location_count {
                phis.insert((id, location), allocate(next)?);
            }
        }
    }
    Ok(phis)
}

fn entry_versions(
    id: BlockId,
    count: usize,
    predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    phis: &BTreeMap<(BlockId, usize), StateVersion>,
    exits: &HashMap<BlockId, Vec<StateVersion>>,
    abrupt: &BTreeMap<(BlockId, BlockId), StateVersion>,
) -> Vec<StateVersion> {
    (0..count)
        .map(|location| {
            phis.get(&(id, location)).copied().unwrap_or_else(|| {
                predecessors
                    .get(&id)
                    .and_then(|incoming| incoming.first())
                    .map_or(StateVersion::INITIAL, |parent| {
                        edge_version(*parent, id, location, exits, abrupt)
                    })
            })
        })
        .collect()
}

fn edge_version(
    source: BlockId,
    target: BlockId,
    location: usize,
    exits: &HashMap<BlockId, Vec<StateVersion>>,
    abrupt: &BTreeMap<(BlockId, BlockId), StateVersion>,
) -> StateVersion {
    abrupt.get(&(source, target)).copied().unwrap_or_else(|| {
        exits
            .get(&source)
            .map_or(StateVersion::INITIAL, |versions| versions[location])
    })
}

fn apply_write(versions: &mut [StateVersion], locations: &[Place], access: &Access) {
    if access.writes {
        for (index, location) in locations.iter().enumerate() {
            if overlap(&access.location, location) {
                versions[index] = access.version;
            }
        }
    }
}

fn solve_versions(
    cfg: &Function,
    accesses: &BTreeMap<BlockId, Vec<Access>>,
    locations: &[Place],
    predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    phis: &BTreeMap<(BlockId, usize), StateVersion>,
    abrupt: &BTreeMap<(BlockId, BlockId), StateVersion>,
) -> HashMap<BlockId, Vec<StateVersion>> {
    let mut exits = HashMap::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (&id, block_accesses) in accesses {
            let mut state = entry_versions(id, locations.len(), predecessors, phis, &exits, abrupt);
            if id == cfg.entry && predecessors[&id].is_empty() {
                state.fill(StateVersion::INITIAL);
            }
            for access in block_accesses {
                apply_write(&mut state, locations, access);
            }
            if exits.get(&id) != Some(&state) {
                exits.insert(id, state);
                changed = true;
            }
        }
    }
    exits
}

fn materialise(
    cfg: &Function,
    accesses: &BTreeMap<BlockId, Vec<Access>>,
    locations: &[Place],
    predecessors: &BTreeMap<BlockId, Vec<BlockId>>,
    phis: &BTreeMap<(BlockId, usize), StateVersion>,
    exits: &HashMap<BlockId, Vec<StateVersion>>,
    abrupt: &BTreeMap<(BlockId, BlockId), StateVersion>,
) -> Result<StateSsa<Place>, StateSsaError> {
    let mut operations = Vec::new();
    for (&(source, target), &version) in abrupt {
        operations.push(StateOp::Clobber(StateClobber {
            location: crate::place::unknown_top(),
            version,
            site: StateSite::edge(source, target, CfgStatePosition::Terminator { ordinal: 0 }),
        }));
    }
    for (&(id, location), &version) in phis {
        operations.push(StateOp::Phi(StatePhi {
            location: locations[location].clone(),
            version,
            incoming: predecessors[&id]
                .iter()
                .map(|&parent| (parent, edge_version(parent, id, location, exits, abrupt)))
                .collect(),
            includes_initial: id == cfg.entry,
            site: StateSite::phi(
                id,
                u32::try_from(location).map_err(|_| StateSsaError::VersionExhausted)?,
            ),
        }));
    }
    for (&id, block_accesses) in accesses {
        let mut state = entry_versions(id, locations.len(), predecessors, phis, exits, abrupt);
        for access in block_accesses {
            let location = access.location.clone();
            let site = access.site.clone();
            operations.push(if access.writes {
                if access.clobber {
                    StateOp::Clobber(StateClobber {
                        location,
                        site,
                        version: access.version,
                    })
                } else {
                    StateOp::Def(StateDef {
                        location,
                        site,
                        version: access.version,
                    })
                }
            } else {
                let index = locations
                    .iter()
                    .position(|location| location == &access.location)
                    .expect("access location collected");
                StateOp::Use(StateUse {
                    location,
                    site,
                    reaching_version: state[index],
                })
            });
            apply_write(&mut state, locations, access);
        }
    }
    StateSsa::new(operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Terminator};
    use crate::ir::Statement;
    use crate::variable_bindings::build_point_resolve_contexts;

    #[test]
    fn opaque_native_invocation_reads_then_clobbers_untracked_cells() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut cfg = Function::new("::f", "entry");
        let entry = cfg.entry;
        cfg.blocks.insert(
            entry,
            Block {
                name: "entry".into(),
                statements: vec![crate::ir::native_call_for_test(b"opaque \xff")],
                terminator: None,
            },
        );
        assert!(cfg.has_opaque_native_accesses());
        let points = build_point_resolve_contexts(&cfg, "::f", registry);
        let mut next = 0;
        let accesses = collect_statement_accesses(
            &cfg.blocks[&entry].statements[0],
            (entry, 0),
            &points,
            registry,
            &std::collections::HashSet::new(),
            &mut next,
        )
        .unwrap();
        assert_eq!(accesses.len(), 2);
        assert_eq!(accesses[0].location, crate::place::unknown_top());
        assert!(!accesses[0].writes);
        assert!(!accesses[0].clobber);
        assert_eq!(accesses[1].location, crate::place::unknown_top());
        assert!(accesses[1].writes);
        assert!(accesses[1].clobber);
        let cells = build_cell_state_ssa(&cfg, &points, registry).unwrap();
        assert!(
            cells
                .operations()
                .iter()
                .any(|operation| matches!(operation, StateOp::Clobber(_)))
        );
        assert!(
            !cells
                .operations()
                .iter()
                .any(|operation| matches!(operation, StateOp::Def(_)))
        );
    }

    #[test]
    fn proved_unset_clobbers_only_its_selected_cell_without_a_value_definition() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} {set x OLD; set untouched KEPT; unset x; set x NEW; list $x $untouched}",
            registry,
            false,
        );
        let function = unit.function("::p").unwrap();
        let points = function.ssa.point_contexts.as_ref().unwrap();
        let cells = build_cell_state_ssa(&function.cfg, points, registry).unwrap();
        let (block, index, statement) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, contents)| {
                contents
                    .statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        (!crate::place_bridge::statement_destruction_places_with_continuation(
                            statement,
                            points.before_statement(block, index),
                            points.after_statement(block, index),
                            registry,
                        )
                        .is_empty())
                        .then_some((block, index, statement))
                    })
            })
            .unwrap();
        let before = points.before_statement(block, index);
        let selected = crate::var_resolve::resolve_literal_access(
            "x",
            before,
            false,
            registry,
            tcl_registry::TraceOperation::Unset,
        );
        let unrelated = crate::var_resolve::resolve_literal_access(
            "untouched",
            before,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let at_unset = |site: &StateSite| {
            matches!(site, StateSite::Cfg(anchor)
            if anchor.block == block && matches!(anchor.position, CfgStatePosition::Statement {index: ordinal, ..} if usize::try_from(ordinal).ok() == Some(index)))
        };
        let clobbers: Vec<_> = cells
            .operations()
            .iter()
            .filter_map(|operation| {
                if let StateOp::Clobber(clobber) = operation
                    && at_unset(&clobber.site)
                {
                    Some(clobber)
                } else {
                    None
                }
            })
            .collect();
        assert_ne!(clobbers, [] as [&StateClobber<Place>; 0]);
        assert!(
            clobbers
                .iter()
                .all(|clobber| overlap(&clobber.location, &selected)
                    && !overlap(&clobber.location, &unrelated))
        );
        assert!(!cells.operations().iter().any(
            |operation| matches!(operation, StateOp::Def(definition) if at_unset(&definition.site))
        ));
        assert_eq!(
            crate::place_bridge::def_places_with_continuation(
                statement,
                before,
                points.after_statement(block, index),
                registry
            ),
            [] as [Place; 0]
        );
    }

    #[test]
    fn whole_array_destruction_clobbers_retained_element_alias_without_touching_other_roots() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} {set a(k) OLD; set untouched KEPT; upvar 0 a(k) alias; unset a; set a(k) NEW; list $a(k) $untouched}",
            registry,
            false,
        );
        let function = unit.function("::p").unwrap();
        let points = function.ssa.point_contexts.as_ref().unwrap();
        let cells = build_cell_state_ssa(&function.cfg, points, registry).unwrap();
        let (block, index) = function
            .cfg
            .blocks
            .iter()
            .find_map(|(&block, contents)| {
                contents
                    .statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        crate::place_bridge::statement_destruction_places_with_continuation(
                            statement,
                            points.before_statement(block, index),
                            points.after_statement(block, index),
                            registry,
                        )
                        .iter()
                        .any(|place| place.name == "a")
                        .then_some((block, index))
                    })
            })
            .unwrap();
        let before = points.before_statement(block, index);
        let retired = crate::var_resolve::resolve_literal_access(
            "alias",
            before,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let unrelated = crate::var_resolve::resolve_literal_access(
            "untouched",
            before,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(cells.operations().iter().any(|operation| matches!(operation,
            StateOp::Clobber(clobber) if matches!(&clobber.site, StateSite::Cfg(site)
                if site.block == block && matches!(site.position, CfgStatePosition::Statement {index: ordinal, ..} if usize::try_from(ordinal).ok() == Some(index)))
                && overlap(&clobber.location, &retired) && !overlap(&clobber.location, &unrelated))));
        let recreated = crate::var_resolve::resolve_literal_access(
            "a(k)",
            points.after_statement(block, index),
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        assert!(
            !overlap(&retired, &recreated),
            "retired={retired:?} recreated={recreated:?} before={before:?} after={:?}",
            points.after_statement(block, index)
        );
    }

    #[test]
    fn observed_reads_clobber_callback_world_before_observer_metadata_is_removed() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut cfg = Function::new("::f", "entry");
        let mut entry = Block::new("entry");
        entry.statements.push(Statement::AssignValue {
            span: tcl_lexer::Span::new(1, 10),
            name: "result".into(),
            name_braced: false,
            value: "$x".into(),
            value_needs_backsubst: false,
            tokens: None,
        });
        cfg.blocks.insert(cfg.entry, entry);
        let mut context = crate::var_resolve::ResolveContext::for_function("::f");
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        context.define_literal("x", "OLD", registry);
        context.define_literal("unrelated", "KEPT", registry);
        let traced_read = crate::var_resolve::resolve_literal_access(
            "x",
            &context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        context
            .traced
            .insert(crate::var_resolve::trace_key(&traced_read));
        let unrelated = crate::var_resolve::resolve_literal_access(
            "unrelated",
            &context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let points = crate::variable_bindings::build_point_resolve_contexts_with_entry(
            &cfg, context, registry,
        );
        let cells = build_cell_state_ssa(&cfg, &points, registry).unwrap();
        assert!(cells.operations().iter().any(|operation|matches!(operation,StateOp::Clobber(clobber) if clobber.location.kind==crate::place::PlaceKind::Unknown && overlap(&clobber.location,&unrelated))));
        assert!(
            cells
                .operations()
                .iter()
                .all(|operation| !operation.location().observed)
        );
        let clobber_index = cells
            .operations()
            .iter()
            .position(|operation| {
                matches!(operation, StateOp::Clobber(clobber)
                if clobber.location.kind == crate::place::PlaceKind::Unknown)
            })
            .unwrap();
        let read_index = cells.operations().iter().position(|operation| {
            matches!(operation, StateOp::Use(read) if overlap(&read.location, &traced_read))
        }).unwrap();
        assert!(clobber_index < read_index);
        let mut terminator_cfg = Function::new("::f", "entry");
        read(
            terminator_cfg
                .blocks
                .get_mut(&terminator_cfg.entry)
                .unwrap(),
        );
        let terminator_points = crate::variable_bindings::build_point_resolve_contexts_with_entry(
            &terminator_cfg,
            points.before_statement(cfg.entry, 0).clone(),
            registry,
        );
        let terminator_cells =
            build_cell_state_ssa(&terminator_cfg, &terminator_points, registry).unwrap();
        assert!(matches!(
            terminator_cells.operations().first(),
            Some(StateOp::Clobber(_))
        ));
        assert!(terminator_cells.operations().iter().any(|operation| matches!(operation,
            StateOp::Clobber(clobber) if clobber.location.kind == crate::place::PlaceKind::Unknown && overlap(&clobber.location, &unrelated))));
    }

    fn assign(value: &str) -> Statement {
        Statement::AssignConst {
            span: tcl_lexer::Span::new(0, 1),
            name: "x".to_owned(),
            name_braced: false,
            value: value.to_owned(),
            value_span: None,
        }
    }

    fn read(block: &mut Block) {
        block.terminator = Some(Terminator::Return {
            tokens: None,
            value: Some("$x".to_owned()),
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            braced: false,
        });
    }

    fn graph(cfg: &Function) -> StateSsa<Place> {
        let registry = CommandRegistry::build_default();
        let points = build_point_resolve_contexts(cfg, &cfg.name, &registry);
        build_cell_state_ssa(cfg, &points, &registry).expect("valid reaching graph")
    }

    #[test]
    fn sibling_blocks_do_not_share_sequential_memory_versions() {
        let mut cfg = Function::new("::f", "entry");
        let written = cfg.intern_block("written");
        let untouched = cfg.intern_block("untouched");
        let join = cfg.intern_block("join");
        let mut left = Block::new("written");
        left.statements.push(assign("1"));
        left.terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        let mut right = Block::new("untouched");
        read(&mut right);
        right.terminator = Some(Terminator::Goto {
            target: join,
            span: None,
        });
        let mut merge = Block::new("join");
        read(&mut merge);
        cfg.blocks
            .extend([(written, left), (untouched, right), (join, merge)]);
        cfg.analysis_edges
            .extend([(cfg.entry, written), (cfg.entry, untouched)]);
        let cells = graph(&cfg);
        let phi = cells
            .operations()
            .iter()
            .find_map(|operation| match operation {
                StateOp::Phi(phi) if phi.site == StateSite::phi(join, 0) => Some(phi),
                _ => None,
            })
            .expect("join phi");
        assert!(
            phi.incoming
                .iter()
                .any(|(&block, &version)| block == untouched && version.is_initial())
        );
        assert!(
            phi.incoming
                .iter()
                .any(|(&block, &version)| block == written && !version.is_initial())
        );
        assert!(cells.operations().iter().any(|operation| matches!(operation,
            StateOp::Use(usage) if usage.site == StateSite::terminator(join, 0) && usage.reaching_version == phi.version)));
    }

    #[test]
    fn entry_backedge_keeps_initial_state_in_phi() {
        let mut cfg = Function::new("::f", "entry");
        cfg.blocks
            .get_mut(&cfg.entry)
            .expect("entry")
            .statements
            .push(assign("1"));
        cfg.analysis_edges.push((cfg.entry, cfg.entry));
        let cells = graph(&cfg);
        assert!(
            cells
                .operations()
                .iter()
                .any(|operation| matches!(operation,
            StateOp::Phi(phi) if phi.includes_initial && phi.incoming.len() == 1))
        );
    }

    #[test]
    fn exception_handler_does_not_assume_last_store_committed() {
        let mut cfg = Function::new("::f", "entry");
        let handler = cfg.intern_block("handler");
        cfg.blocks
            .get_mut(&cfg.entry)
            .expect("entry")
            .statements
            .push(assign("1"));
        let mut block = Block::new("handler");
        read(&mut block);
        cfg.blocks.insert(handler, block);
        cfg.exception_edges.push((cfg.entry, handler));
        let cells = graph(&cfg);
        let use_version = cells
            .operations()
            .iter()
            .find_map(|operation| match operation {
                StateOp::Use(usage) if usage.site == StateSite::terminator(handler, 0) => {
                    Some(usage.reaching_version)
                }
                _ => None,
            })
            .expect("handler read");
        assert!(
            matches!(cells.definition(use_version), Some(StateOp::Clobber(clobber)) if matches!(clobber.site, StateSite::Edge(_)))
        );
    }
}
