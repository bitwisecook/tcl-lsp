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

use tcl_dialect::model::{Family, SurfaceQuery};

use tcl_lexer::Span;
use tcl_registry::model::semantic::SemanticContext;

use tcl_registry::{CommandRegistry, DispatchDependencies};

use super::*;
use crate::executable_ir::{
    CompletionCase, CompletionId, ExecutableArgvId, ExecutableBlock, ExecutableBlockId,
    ExecutableFunctionId, GenericInvoke,
};
use crate::ir::{NodeId, SourceSite};
use crate::registry_invocation::resolve_word_exprs;
use tcl_core_types::Code as CompletionCode;

fn source() -> SourceSite {
    SourceSite::source(Span::new(0, 0))
}

fn literal(text: &str) -> WordExpr {
    WordExpr::Literal {
        text: text.to_owned(),
        source: source(),
    }
}

fn registry() -> &'static CommandRegistry {
    static REGISTRY: std::sync::OnceLock<CommandRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(CommandRegistry::build_default)
}

fn invoke(
    id: ExecutableFunctionId,
    completion: CompletionId,
    words: &[WordExpr],
) -> ExecutableInstruction {
    let resolution = resolve_word_exprs(
        registry(),
        Some(SemanticContext::for_environment("tcl9.0")),
        words,
    )
    .expect("test command resolves through the registry");
    ExecutableInstruction::Invoke(GenericInvoke {
        completion,
        argv: ExecutableArgvId::new(id, completion.index()),
        resolution,
        original_words: words.to_vec(),
        source_binding: None,
        effective_words: None,
        node: NodeId::from_path(vec![
            u32::try_from(completion.index()).expect("completion index fits"),
        ]),
        source: source(),
    })
}

fn literal_invoke(
    id: ExecutableFunctionId,
    completion: CompletionId,
    words: &[&str],
) -> ExecutableInstruction {
    let words: Vec<_> = words.iter().map(|word| literal(word)).collect();
    invoke(id, completion, &words)
}

fn ok_switch(
    completion: CompletionId,
    ok: ExecutableBlockId,
    abrupt: ExecutableBlockId,
) -> ExecutableTerminator {
    ExecutableTerminator::CompletionSwitch {
        completion,
        cases: vec![CompletionCase {
            code: CompletionCode::Ok,
            target: ok,
        }],
        default: abrupt,
    }
}

fn llength_dependencies() -> DispatchDependencies {
    registry()
        .resolve_invocation(
            "llength",
            &["a b"],
            Some(SurfaceQuery::core(Family::Tcl, "9.0")),
        )
        .expect("llength resolves")
        .facts()
        .dispatch_dependencies
}

/// One invoke per block, each block's switch dispatching that invoke's
/// completion: `targets[i]` is `(ok, abrupt)` for block `i`, and any
/// index past the word list becomes a `ReturnCompletion` sink block.
fn chain_function(
    commands: &[&[&str]],
    targets: &[(usize, usize)],
    sink_count: usize,
) -> ExecutableFunction {
    let id = ExecutableFunctionId::new(93);
    let block_ids: Vec<_> = (0..commands.len() + sink_count)
        .map(|index| ExecutableBlockId::new(id, index))
        .collect();
    let mut blocks = Vec::new();
    for (index, words) in commands.iter().enumerate() {
        let completion = CompletionId::new(id, index);
        let (ok, abrupt) = targets[index];
        blocks.push(ExecutableBlock {
            id: block_ids[index],
            instructions: vec![literal_invoke(id, completion, words)],
            terminator: Some(ok_switch(completion, block_ids[ok], block_ids[abrupt])),
        });
    }
    for sink in &block_ids[commands.len()..] {
        blocks.push(ExecutableBlock {
            id: *sink,
            instructions: Vec::new(),
            terminator: Some(ExecutableTerminator::ReturnCompletion(CompletionId::new(
                id, 0,
            ))),
        });
    }
    ExecutableFunction::new(id, block_ids[0], blocks)
}

fn covered(analysis: &DispatchProofAnalysis, block: u32) -> bool {
    analysis
        .site(block, 0)
        .is_some_and(|proof| proof.satisfies(llength_dependencies()))
}

#[test]
fn long_straight_line_functions_analyse_with_shared_states() {
    // A linear CFG converges in one worklist pass with `Arc`-shared edge
    // states; this guards the pass's linear cost contract functionally.
    let commands: Vec<&[&str]> = (0..2000).map(|_| &["llength", "a b"] as &[&str]).collect();
    let targets: Vec<(usize, usize)> = (0..2000).map(|index| (index + 1, 2000)).collect();
    let function = chain_function(&commands, &targets, 1);
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    for block in 0..2000_u32 {
        assert!(covered(&analysis, block), "site {block} must stay proven");
    }
}

#[test]
fn count_ranges_join_and_saturate_soundly() {
    let one = CountRange::ZERO.add_one();
    assert!(one.may_be_live());
    assert!(!one.remove_exact().may_be_live());
    // A removal that may not have matched keeps the upper bound.
    assert!(one.remove_maybe().may_be_live());
    assert!(one.join(CountRange::ZERO).may_be_live());
    assert!(!CountRange::ZERO.join(CountRange::ZERO).may_be_live());
    let mut saturated = CountRange::ZERO;
    for _ in 0..=COUNT_SATURATION {
        saturated = saturated.add_one();
    }
    assert_eq!(saturated.hi, None);
    // A saturated count can never prove absence again.
    let mut removed = saturated;
    for _ in 0..=COUNT_SATURATION {
        removed = removed.remove_exact();
    }
    assert!(removed.may_be_live());
}

#[test]
fn straight_line_trace_add_blocks_and_literal_remove_restores() {
    let function = chain_function(
        &[
            &["llength", "a b"],
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &["llength", "a b"],
            &[
                "trace",
                "remove",
                "execution",
                "llength",
                "enter",
                "observe",
            ],
            &["llength", "a b"],
        ],
        &[(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(
        covered(&analysis, 0),
        "pristine entry proves the first site"
    );
    assert!(
        !covered(&analysis, 2),
        "a live execution trace blocks reuse"
    );
    assert!(
        covered(&analysis, 4),
        "literal removal of the last applicable trace restores eligibility on the success edge"
    );
}

#[test]
fn duplicate_trace_add_needs_a_removal_per_registration() {
    let function = chain_function(
        &[
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &[
                "trace",
                "remove",
                "execution",
                "llength",
                "enter",
                "observe",
            ],
            &["llength", "a b"],
            &[
                "trace",
                "remove",
                "execution",
                "llength",
                "enter",
                "observe",
            ],
            &["llength", "a b"],
        ],
        &[(1, 6), (2, 6), (3, 6), (4, 6), (5, 6), (6, 6)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(
        !covered(&analysis, 3),
        "C Tcl registers duplicate traces; one removal leaves one live"
    );
    assert!(covered(&analysis, 5), "the second removal restores absence");
}

#[test]
fn on_ok_only_trace_add_leaves_the_abrupt_edge_unchanged() {
    // Block 0 dispatches the trace add: OK to block 1, abrupt to block 2.
    let function = chain_function(
        &[
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &["llength", "a b"],
            &["llength", "a b"],
        ],
        &[(1, 2), (3, 3), (3, 3)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(
        !covered(&analysis, 1),
        "the OK edge carries the registration"
    );
    assert!(
        covered(&analysis, 2),
        "an OnOkOnly transition must not commit on the abrupt edge"
    );
}

#[test]
fn may_commit_rename_blocks_both_completion_edges() {
    let function = chain_function(
        &[
            &["rename", "llength", "llen"],
            &["llength", "a b"],
            &["llength", "a b"],
        ],
        &[(1, 2), (3, 3), (3, 3)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(!covered(&analysis, 1));
    assert!(
        !covered(&analysis, 2),
        "rename may commit before an abrupt completion, so the abrupt edge joins the change"
    );
}

#[test]
fn rename_of_an_unrelated_command_preserves_the_candidate_proof() {
    let function = chain_function(
        &[&["rename", "lreverse", "lrev"], &["llength", "a b"]],
        &[(1, 2), (2, 2)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    let proof = analysis.site(1, 0).expect("site reached");
    assert!(
        proof.satisfies(llength_dependencies()),
        "binding invalidation is per-subject"
    );
    assert!(
        !proof
            .covers
            .contains(tcl_registry::DispatchDependencyDomain::UnknownHandling),
        "a binding change still invalidates unknown-handling precision"
    );
}

#[test]
fn a_live_trace_keeps_the_trace_only_callback_barrier() {
    // With a command trace live anywhere, a rename may fire its handler,
    // so the registry's trace-only barrier must stay and widen the world.
    let function = chain_function(
        &[
            &["trace", "add", "command", "observe", "delete", "observe"],
            &["rename", "lreverse", "lrev"],
            &["llength", "a b"],
        ],
        &[(1, 3), (2, 3), (3, 3)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    let proof = analysis.site(2, 0).expect("site reached");
    assert!(
        proof.covers.is_empty(),
        "an undischarged trace-only barrier is a full world barrier"
    );
}

#[test]
fn a_trace_on_either_join_predecessor_blocks_after_the_join() {
    // Block 0 branches: OK path installs a trace (block 1, both edges to
    // block 3), abrupt path is clean (block 2).  Block 3 joins.
    let function = chain_function(
        &[
            &["llength", "a b"],
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &["llength", "a b"],
            &["llength", "a b"],
        ],
        &[(1, 2), (3, 3), (3, 3), (4, 4)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(covered(&analysis, 0));
    assert!(
        covered(&analysis, 2),
        "the clean predecessor stays provable"
    );
    assert!(
        !covered(&analysis, 3),
        "a trace present on either predecessor prevents reuse after the join"
    );
}

#[test]
fn loop_back_edges_reach_a_fixpoint_and_block_the_looped_site() {
    // Block 1 is a loop header: entered from block 0 and from block 2's
    // back edge; the body installs a trace each iteration.
    let function = chain_function(
        &[
            &["llength", "a b"],
            &["trace", "add", "execution", "llength", "enter", "observe"],
            &["llength", "a b"],
        ],
        &[(1, 3), (2, 3), (1, 3)],
        1,
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(covered(&analysis, 0), "the pre-loop site stays provable");
    assert!(
        !covered(&analysis, 2),
        "the looped site always follows a registration"
    );
}

#[test]
fn unknown_world_entry_fails_every_site_closed() {
    let function = chain_function(&[&["llength", "a b"]], &[(1, 1)], 1);
    let analysis = analyse_dispatch_stability(&function, DispatchEntryAssumption::UnknownWorld);
    // An unknown world starts at the lattice top and widening is
    // absorbing, so the analysis records no proof at all rather than
    // walking the CFG to produce one that could only fail. An absent
    // proof and a failing proof are the same answer at the use site:
    // `gvn_site_eligibility` declines both.
    assert!(analysis.site(0, 0).is_none());

    // The same function under a pristine entry world does prove its site,
    // so the emptiness above is the entry contract talking, not an
    // unreachable site or a malformed fixture.
    let pristine =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    assert!(
        pristine
            .site(0, 0)
            .expect("site reached")
            .satisfies(llength_dependencies())
    );
}

#[test]
fn an_unresolved_head_widens_every_domain() {
    let id = ExecutableFunctionId::new(94);
    let b0 = ExecutableBlockId::new(id, 0);
    let b1 = ExecutableBlockId::new(id, 1);
    let b2 = ExecutableBlockId::new(id, 2);
    let dynamic_head = invoke(
        id,
        CompletionId::new(id, 0),
        &[WordExpr::Variable {
            spelling: "$command".to_owned(),
            source: source(),
        }],
    );
    let function = ExecutableFunction::new(
        id,
        b0,
        vec![
            ExecutableBlock {
                id: b0,
                instructions: vec![dynamic_head],
                terminator: Some(ok_switch(CompletionId::new(id, 0), b1, b2)),
            },
            ExecutableBlock {
                id: b1,
                instructions: vec![literal_invoke(
                    id,
                    CompletionId::new(id, 1),
                    &["llength", "a b"],
                )],
                terminator: Some(ExecutableTerminator::ReturnCompletion(CompletionId::new(
                    id, 1,
                ))),
            },
            ExecutableBlock {
                id: b2,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(CompletionId::new(
                    id, 0,
                ))),
            },
        ],
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    let proof = analysis.site(1, 0).expect("site reached");
    assert!(
        proof.covers.is_empty(),
        "a computed head runs the resolver, `unknown`, aliases, and traces"
    );
}

#[test]
fn operand_words_gate_admissibility_without_blocking_domain_proofs() {
    let id = ExecutableFunctionId::new(95);
    let b0 = ExecutableBlockId::new(id, 0);
    let b1 = ExecutableBlockId::new(id, 1);
    let with_substitution = invoke(
        id,
        CompletionId::new(id, 0),
        &[
            literal("llength"),
            WordExpr::CommandSubstitution {
                spelling: "[list a b]".to_owned(),
                source: source(),
            },
        ],
    );
    let function = ExecutableFunction::new(
        id,
        b0,
        vec![
            ExecutableBlock {
                id: b0,
                instructions: vec![with_substitution],
                terminator: Some(ok_switch(CompletionId::new(id, 0), b1, b1)),
            },
            ExecutableBlock {
                id: b1,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(CompletionId::new(
                    id, 0,
                ))),
            },
        ],
    );
    let analysis =
        analyse_dispatch_stability(&function, DispatchEntryAssumption::PristineRegistryWorld);
    let proof = analysis.site(0, 0).expect("site reached");
    assert!(
        !proof.operand_words_admissible,
        "a command-substitution operand cannot be proven inert"
    );
    assert!(!proof.satisfies(llength_dependencies()));
}

#[test]
fn namespace_alias_targets_qualify_each_written_word() {
    // tclsh 8.6/9.0/9.1: with `set ::v glob; namespace eval : {variable v colon}`,
    // `namespace upvar : v l` reads `colon` and `namespace upvar ns ::v m` reads `glob`.
    assert_eq!(namespace_alias_target("ns", "v"), "ns::v");
    assert_eq!(namespace_alias_target("::a:::b", "v"), "a::b::v");
    assert_eq!(namespace_alias_target("ns", "::v"), "v");
    assert_eq!(namespace_alias_target(":", "v"), ":::v");
    assert_ne!(namespace_alias_target(":", "v"), normalise_name("v"));
}

#[test]
fn an_alias_on_one_predecessor_only_loses_its_target_either_way_round() {
    // A local aliased on one path and left alone on the other may or may
    // not be an alias after the join, so its target must become unknown.
    // Keeping the concrete target would check the target cell's traces
    // while a trace on the unaliased local cell went unnoticed.
    let with_link = |local: &str, target: Option<&str>| {
        let mut ledger = AliasLedger::default();
        ledger.record(local.to_owned(), target.map(str::to_owned));
        ledger
    };
    let aliased = with_link("counter", Some("shared"));
    let empty = AliasLedger::default();

    let mut forward = aliased.clone();
    forward.join(&empty);
    let mut reverse = empty.clone();
    reverse.join(&aliased);

    assert_eq!(
        forward, reverse,
        "the join must not depend on predecessor visit order"
    );
    assert_eq!(forward.links, vec![("counter".to_owned(), None)]);

    // Agreeing predecessors keep the precise target.
    let mut agreed = aliased.clone();
    agreed.join(&aliased);
    assert_eq!(
        agreed.links,
        vec![("counter".to_owned(), Some("shared".to_owned()))]
    );

    // Disagreeing targets weaken to unknown.
    let mut disagreed = aliased.clone();
    disagreed.join(&with_link("counter", Some("other")));
    assert_eq!(disagreed.links, vec![("counter".to_owned(), None)]);
}

#[test]
fn removing_the_last_step_registration_clears_the_step_hazard() {
    // The step hazard is derived, not latched: a stored flag could only
    // ever be set, so removing the last `enterstep` registration would
    // suppress reuse forever with no observer live.
    let mut ledger = TraceLedger::default();
    let key = TraceKey {
        operations: vec![TraceOperation::EnterStep],
        prefix: Some("observe".to_owned()),
    };
    assert!(
        !ledger.step_hazard(),
        "an empty ledger has no step observer"
    );

    ledger.cell_mut("llength").expect("cell").add(key.clone());
    assert!(ledger.step_hazard());

    ledger.cell_mut("llength").expect("cell").remove(&key);
    assert!(
        !ledger.step_hazard(),
        "the last step registration was proven removed"
    );

    // A non-step registration never raises the hazard, and an unknown
    // cell always does.
    ledger.cell_mut("llength").expect("cell").add(TraceKey {
        operations: vec![TraceOperation::Enter],
        prefix: Some("observe".to_owned()),
    });
    assert!(!ledger.step_hazard());
    ledger.cell_mut("llength").expect("cell").unknown = true;
    assert!(ledger.step_hazard());
}

#[test]
fn normalisation_equates_global_spellings_only() {
    assert_eq!(normalise_name("::llength"), "llength");
    assert_eq!(normalise_name("llength"), "llength");
    assert_eq!(normalise_name("::::llength"), "llength");
    assert_eq!(normalise_name("foo::llength"), "foo::llength");
    assert_eq!(normalise_name("::foo::::bar"), "foo::bar");
    assert!(matches!(normalise_name("plain"), Cow::Borrowed(_)));
}

#[test]
fn variable_references_parse_scalars_elements_and_dynamic_elements() {
    let config = tcl_lexer::LexerConfig::for_profile(Some(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    ));
    let scalar = parse_variable_reference("$items", config).expect("scalar parses");
    assert_eq!(scalar.base, "items");
    assert!(matches!(scalar.element, ElementAccess::Scalar));
    let braced = parse_variable_reference("${a(1)}", config).expect("braced element parses");
    assert_eq!(braced.base, "a");
    assert!(matches!(braced.element, ElementAccess::Known("1")));
    let dynamic = parse_variable_reference("$a($i)", config).expect("dynamic element parses");
    assert!(matches!(dynamic.element, ElementAccess::Dynamic));
    assert!(
        parse_variable_reference("$a([cmd])", config).is_none() || {
            // A bracketed element must never be treated as a literal.
            matches!(
                parse_variable_reference("$a([cmd])", config)
                    .expect("parsed")
                    .element,
                ElementAccess::Dynamic
            )
        }
    );
    assert!(parse_variable_reference("$", config).is_none());
    assert!(parse_variable_reference("${unclosed", config).is_none());
}

#[test]
fn source_reference_grammar_keeps_literal_names_and_unknown_indexes() {
    let c8 = tcl_lexer::LexerConfig::for_profile(Some(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    ));
    let jim = tcl_lexer::LexerConfig::for_profile(Some(
        tcl_registry::model::ingress::resolve_known_environment("jimtcl")
            .unwrap()
            .unit_profile(),
    ));
    for name in ["", "$literal", "[literal]", " with space "] {
        let spelling = format!("${{{name}}}");
        let selected = parse_variable_reference(&spelling, c8).unwrap();
        assert_eq!(selected.base, name);
        assert!(matches!(selected.element, ElementAccess::Scalar));
    }
    assert!(parse_variable_reference("$café", c8).is_none());
    assert_eq!(parse_variable_reference("$café", jim).unwrap().base, "café");
    assert!(parse_variable_reference("items", c8).is_none());
    assert!(parse_variable_reference("$items suffix", c8).is_none());
    let escaped = parse_variable_reference(r"$a(\x31)", c8).unwrap();
    assert!(matches!(escaped.element, ElementAccess::Dynamic));
    let literal = parse_variable_reference(r"${a(\x31)}", c8).unwrap();
    assert!(matches!(literal.element, ElementAccess::Known(r"\x31")));
    let mut state = WorldContents::entry(DispatchEntryAssumption::PristineRegistryWorld);
    state
        .variable_traces
        .cell_mut("other")
        .unwrap()
        .add(TraceKey {
            operations: vec![TraceOperation::Read],
            prefix: Some("observe".to_owned()),
        });
    assert!(state.variable_access_may_observe("$untraced"));
    state.variable_grammar = Some(c8);
    assert!(!state.variable_access_may_observe("$untraced"));
    assert!(state.variable_access_may_observe("$other"));
    let mut quiet = WorldContents::entry(DispatchEntryAssumption::PristineRegistryWorld);
    quiet.variable_grammar = Some(c8);
    assert!(!quiet.variable_access_may_observe("$ordinary"));
    assert!(quiet.variable_access_may_observe("$a([arbitrary_command])"));
    assert!(quiet.variable_access_may_observe("$a($index)"));
}

#[test]
fn variable_trace_observation_respects_element_relations() {
    let mut ledger = TraceLedger::default();
    let key = || TraceKey {
        operations: vec![TraceOperation::Read],
        prefix: Some("observe".to_owned()),
    };
    ledger.cell_mut("a(1)").expect("cell").add(key());
    assert!(ledger.variable_access_may_observe("a", &ElementAccess::Known("1")));
    assert!(ledger.variable_access_may_observe("a", &ElementAccess::Dynamic));
    assert!(!ledger.variable_access_may_observe("a", &ElementAccess::Scalar));
    assert!(!ledger.variable_access_may_observe("a", &ElementAccess::Known("2")));
    ledger.cell_mut("b").expect("cell").add(key());
    assert!(ledger.variable_access_may_observe("b", &ElementAccess::Scalar));
    assert!(ledger.variable_access_may_observe("b", &ElementAccess::Known("9")));
    assert!(!ledger.variable_access_may_observe("c", &ElementAccess::Scalar));
}

#[test]
fn observer_names_and_alias_targets_keep_literal_sigils_and_keys() {
    let mut state = WorldContents::entry(DispatchEntryAssumption::PristineRegistryWorld);
    state.variable_grammar = Some(tcl_lexer::LexerConfig::for_profile(Some(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    )));
    let key = || TraceKey {
        operations: vec![TraceOperation::Read],
        prefix: Some("observe".to_owned()),
    };
    state.variable_traces.cell_mut("$b").unwrap().add(key());
    assert!(state.variable_name_may_observe("$b"));
    assert!(!state.variable_name_may_observe("b"));
    assert!(state.variable_access_may_observe("${$b}"));
    state.aliases.links.push(("view".into(), Some("$b".into())));
    assert!(state.variable_access_may_observe("$view"));

    state
        .variable_traces
        .cell_mut("a($key)")
        .unwrap()
        .add(key());
    state
        .aliases
        .links
        .push(("element".into(), Some("a($key)".into())));
    assert!(state.variable_access_may_observe("$element"));
    assert!(!state.variable_name_may_observe("a(other)"));
}
