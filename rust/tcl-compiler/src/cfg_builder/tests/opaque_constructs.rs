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

//! Constructs kept opaque: `catch` and `try` that stay one statement, their body effects and frozen loops.

use super::*;

#[test]
fn drop_word_keeps_structured_words_aligned() {
    let tokens = CommandTokens::from_lossy_parts(
        vec![Span::new(0, 4), Span::new(5, 8), Span::new(9, 13)],
        vec!["dict".into(), "for".into(), "body".into()],
        vec![TokenType::Esc, TokenType::Esc, TokenType::Str],
        vec![true, true, true],
        Vec::new(),
        None,
    );
    let dropped = drop_word(&tokens, 1);
    assert_eq!(dropped.argv_texts, vec!["dict", "body"]);
    assert!(dropped.words_align_with_argv_text());
    assert_eq!(dropped.words()[1].legacy_text(), "body");
}

/// A `catch` whose body is one straight-line block is lowered into real
/// blocks, so the ordinary emitters compile it: its variables reach the
/// LVT and the optimiser can see inside (#2207).
#[test]
fn straight_line_catch_lowers_to_real_blocks() {
    let func = build_test_cfg_function("::test", &straight_line_catch_script(), true);
    let body_id = func
        .blocks
        .keys()
        .find(|id| func.block_name(**id).starts_with("catch_body_"))
        .copied()
        .expect("expected a catch_body block");
    // The body's own statement is in the body block, not summarised onto
    // an opaque call.
    assert!(
        func.blocks[&body_id]
            .statements
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { name, .. } if name == "inner")),
        "the body's write belongs to the body block",
    );
    // The result variable is still defined for SSA, on the block both
    // paths reach.
    assert!(
            func.blocks.iter().any(|(id, blk)| {
                func.block_name(*id).starts_with("catch_end_")
                    && blk.statements.iter().any(|s| {
                        matches!(
                            s,
                            Statement::Call { tokens: Some(tokens), defs, .. }
                                if tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs)
                                    && defs.iter().any(|d| d == "result")
                        )
                    })
            }),
            "result var must be defined at the merge",
        );
}

/// A body that terminates its block — here an `error` — keeps the opaque
/// form: the split-off part would fall outside the exception range.
#[test]
fn catch_with_terminating_body_stays_opaque() {
    let script = Script::from_statements(vec![Statement::Catch {
        span: Span::new(0, 30),
        body: Script::from_statements(vec![Statement::Call {
            span: Span::new(7, 14),
            command: "error".into(),
            canonical_command: None,
            args: vec!["boom".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]),
        body_span: Span::new(6, 15),
        result_var: Some("result".into()),
        options_var: None,
        raw_args: vec!["{error boom}".into(), "result".into()],
        tokens: None,
    }]);
    let registry = native_registry();
    let func = CfgBuilder::new(true, &registry)
        .with_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()))
        .build_function("::test", &script);
    let entry = &func.blocks[&func.entry];
    assert!(
        entry.statements.iter().any(|s| matches!(
            s,
            Statement::Call { command, defs, .. } if command == "catch" && !defs.is_empty()
        )),
        "a terminating body keeps the opaque call with summarised defs",
    );
    assert!(
        !func
            .blocks
            .keys()
            .any(|id| func.block_name(*id).starts_with("catch_body_")),
        "and is not lowered into blocks",
    );
}

fn straight_line_catch_script() -> Script {
    Script::from_statements(vec![Statement::Catch {
        span: Span::new(0, 30),
        body: Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(7, 14),
            name: "inner".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]),
        body_span: Span::new(6, 15),
        result_var: Some("result".into()),
        options_var: None,
        raw_args: vec!["{set inner 1}".into(), "result".into()],
        tokens: None,
    }])
}

#[test]
fn dedup_preserves_order() {
    let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
    dedup_preserve_order(&mut v);
    assert_eq!(v, vec!["a", "b", "c"]);
}

/// An opaque `catch` keeps its body inside the statement and stops at the
/// body's first error, so what the body writes is a may-definition stated
/// ahead of the call, and only the result and options variables the call
/// assigns once the body has finished are its own definitions.
#[test]
fn an_opaque_catch_states_what_its_body_writes_as_may_definitions_ahead_of_the_call() {
    let module = lower_module("set g 5\ncatch { if {[gets stdin] eq {q}} { set g 0 } } msg opts\n");
    let definitions = crate::ir_helpers::defs_from_ir_script(&module.top_level);
    assert!(definitions.ends_with(&["msg".to_owned(), "opts".to_owned()]));
    let analysis = build_cfg(&module, false);
    assert!(analysis.top_level.blocks.values().any(|block| {
        block.statements.iter().any(|statement| matches!(
            statement,
            Statement::Call { defs, tokens: Some(tokens), .. }
                if defs == &["msg", "opts"]
                    && tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs)
        ))
    }));

    let cfg = build_cfg_codegen(&module, false);
    let calls = calls_in_order(&cfg.top_level);
    let marker = position_of(&calls, "<arm-writes>");
    let catch = position_of(&calls, "catch");
    assert!(marker < catch, "{calls:?}");
    assert_eq!(calls[marker].1, ["g"]);
    assert_eq!(calls[catch].1, ["msg", "opts"]);
}

/// A callee an opaque `catch` body calls writes the caller's name through
/// `upvar`, as it would at a call the graph lowers: the name joins the
/// may-definitions.
#[test]
fn a_callee_an_opaque_catch_body_calls_may_define_the_name_it_writes() {
    let module = lower_module(
        "proc zero {v} {upvar 1 $v x; set x 0}\n\
             proc p {} {\n set go 1\n catch { if {1} { zero go } }\n puts $go\n}\n",
    );
    let cfg = build_cfg(&module, false);
    let p = fixture_id_if_present(&cfg.procedures, "::p")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::p CFG");
    let calls = calls_in_order(p);
    assert_eq!(calls[position_of(&calls, "<arm-writes>")].1, ["go"]);
}

/// A `try` the codegen shape keeps as one call states what its body, its
/// handlers and its `finally` script write as may-definitions, and the
/// variables its handlers bind are among them: a handler runs only when it
/// is selected.
#[test]
fn a_deferred_try_states_what_its_scripts_write_as_may_definitions() {
    let module = lower_module("try { foo } on error {msg opts} { set a 1 } finally { set b 2 }\n");
    let cfg = build_cfg(&module, true);
    let calls = calls_in_order(&cfg.top_level);
    let marker = position_of(&calls, "<arm-writes>");
    let call = position_of(&calls, "try");
    assert!(marker < call, "{calls:?}");
    assert_eq!(calls[marker].1, ["a", "b", "msg", "opts"]);
    assert!(calls[call].1.is_empty(), "{calls:?}");
    assert!(calls.iter().any(|(command, _)| *command == "<unseen-call>"));
}

#[test]
fn frozen_while_condition_carries_upvar_proc_defs() {
    // TP — `while {[getopt ...]}`'s condition is purely a command
    // substitution, so `lower_while_or_frozen` freezes the whole loop
    // as an opaque `Statement::Barrier` (no `defs` field at all) rather
    // than calling `lower_while`. Without a separate synthetic `<cond>`
    // Call carrying the condition's defs, `getopt`'s upvar write to
    // `opt` would be invisible to the def-use graph even though the
    // barrier's own `uses_of` textually scans the (un-lowered) body
    // for `$opt` and attributes the read to the same statement.
    let module = lower_module(
        "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             while {[getopt opt]} { puts $opt }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"opt".to_string()),
        "expected opt in the frozen while's <cond> Call defs, got {defs:?}",
    );
    // The barrier itself must still be there too — freezing, not
    // ordinary `lower_while`, is exactly the shape under test.
    let has_barrier = cfg.top_level.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { command, .. } if command == "while"))
    });
    assert!(
        has_barrier,
        "expected the loop to still freeze as a Barrier"
    );
}

#[test]
fn frozen_for_condition_carries_upvar_proc_defs() {
    // TP — the `for` loop's identical frozen-barrier path: the same rule
    // applies to `for {...} [cond] {...} {...}`.
    let module = lower_module(
        "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             for {} {[getopt opt]} {} { puts $opt }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"opt".to_string()),
        "expected opt in the frozen for's <cond> Call defs, got {defs:?}",
    );
}

#[test]
fn frozen_while_condition_with_no_upvar_proc_omits_synthetic_cond() {
    // TN — a frozen loop whose condition calls an *ordinary* (non-
    // upvar, non-`catch`/`scan`/`gets`/`regexp`) command must not gain
    // a synthetic `<cond>` Call at all — nothing for it to carry.
    let module = lower_module("proc pureCheck {} { return 1 }\nwhile {[pureCheck]} { }");
    let cfg = build_cfg(&module, false);
    assert!(
        find_call_defs(&cfg.top_level, "<cond>").is_none(),
        "no synthetic <cond> Call expected when the condition defines nothing",
    );
}
