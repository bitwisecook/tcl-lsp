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

//! Every hook family's calling convention, end to end on the real VM.
//!
//! One test per family, each entered through the **registry's own function
//! pointer** rather than the host's API — that is the surface the analyser,
//! the optimiser, and `hover.rs` call, so testing anything else would prove
//! the wrong thing. Each asserts the family's emitter verb, its `ctx` keys,
//! and its silence.
//!
//! The tests share one process-wide host (installed per test thread), so each
//! builds its own pack and clears the host afterwards.

use std::rc::Rc;

use tcl_registry::arg_role::{AppendedArity, ArgRole};
use tcl_registry::clause_shape::ClauseShapeError;
use tcl_registry::hover::ScriptTiming;
use tcl_registry::invocation_words::{CommandPrefixArguments, InvocationArguments, InvocationWord};
use tcl_registry::literal_validation::{LiteralArgumentIssueReason, LiteralArgumentValidation};
use tcl_registry::pack_hooks::{self, HookFamily, HookInputs};
use tcl_spec_hooks::{HookProgram, PackPrograms, tclvm_host};

/// Install a one-hook pack and return its slot.
fn one_hook(program: HookProgram) -> pack_hooks::HookSlot {
    let host = Rc::new(tclvm_host());
    let installed = host.install_pack_hooks(PackPrograms::new("mylib").with(program));
    assert!(
        installed[0].declined.is_none(),
        "{:?}",
        installed[0].declined
    );
    pack_hooks::install_host(host);
    installed[0].slot.expect("a slot")
}

#[test]
fn arg_role_resolver_emits_the_complete_index_role_map() {
    let slot = one_hook(
        HookProgram::new(
            "mylib::with_var",
            HookFamily::ArgRoleResolver,
            "role 0 VarWrite\nrole 1 Body\n",
        )
        .with_inputs(HookInputs::parse(&["nwords", "kinds"])),
    );
    let resolver = pack_hooks::arg_role_resolver_fn(slot).expect("a resolver thunk");
    assert_eq!(
        resolver(&["v", "{puts hi}"]),
        vec![(0, ArgRole::VarWrite), (1, ArgRole::Body)]
    );
    // Shape-cacheable: the declared inputs exclude the words themselves, so a
    // second call of the same shape never re-enters the VM.
    let before = pack_hooks::cache_stats();
    let _ = resolver(&["other", "{other body}"]);
    let after = pack_hooks::cache_stats();
    assert_eq!(after.hits, before.hits + 1);
    pack_hooks::clear_host();
}

#[test]
fn a_silent_resolver_falls_back_to_the_declared_roles() {
    let slot = one_hook(HookProgram::new(
        "mylib::quiet",
        HookFamily::ArgRoleResolver,
        "if {[llength $words] < 99} { return }\nrole 0 VarWrite\n",
    ));
    let resolver = pack_hooks::arg_role_resolver_fn(slot).expect("a resolver thunk");
    assert!(resolver(&["v"]).is_empty(), "silence means no roles");
    pack_hooks::clear_host();
}

#[test]
fn command_prefix_resolver_emits_positions_and_appended_arity() {
    let slot = one_hook(HookProgram::new(
        "mylib::trace",
        HookFamily::CommandPrefixResolver,
        "prefix 2 {Exactly 4}\n",
    ));
    let resolver = pack_hooks::command_prefix_resolver_fn(slot).expect("a prefix thunk");
    let words = ["add", "execution", "cb"];
    assert_eq!(
        resolver(CommandPrefixArguments::literals(&words)),
        vec![(2, AppendedArity::Exactly(4))]
    );
    pack_hooks::clear_host();
}

#[test]
fn script_timing_resolver_preserves_the_exact_enum_spelling() {
    let slot = one_hook(HookProgram::new(
        "mylib::send",
        HookFamily::ScriptTimingResolver,
        "if {[lindex $words 0] eq \"-async\"} {\n\
             timing 2 Deferred\n\
         } else {\n\
             timing 1 SameInvocation\n\
         }",
    ));
    let resolver = pack_hooks::script_timing_resolver_fn(slot).expect("a timing thunk");
    assert_eq!(
        resolver(&["other", "work"]),
        vec![(1, ScriptTiming::SameInvocation)]
    );
    assert_eq!(
        resolver(&["-async", "other", "work"]),
        vec![(2, ScriptTiming::Deferred)]
    );
    pack_hooks::clear_host();
}

#[test]
fn script_timing_resolver_can_emit_reference_only() {
    let slot = one_hook(HookProgram::new(
        "mylib::trace",
        HookFamily::ScriptTimingResolver,
        "timing 3 ReferenceOnly",
    ));
    let resolver = pack_hooks::script_timing_resolver_fn(slot).expect("a timing thunk");
    assert_eq!(
        resolver(&["remove", "variable", "name", "callback"]),
        vec![(3, ScriptTiming::ReferenceOnly)]
    );
    pack_hooks::clear_host();
}

#[test]
fn a_versioned_fold_reads_the_tcl_version_from_ctx() {
    let slot = one_hook(HookProgram::new(
        "mylib::release",
        HookFamily::ConstFoldVersioned,
        "fold [dict get $ctx tcl-version]",
    ));
    let folder = pack_hooks::const_fold_versioned_fn(slot).expect("a versioned fold thunk");
    assert_eq!(
        folder(&["x"], Some(tcl_dialect::TclVersion::V9_0)),
        Some("9.0".to_owned())
    );
    // No release named: the key is present and empty, and a body that needs a
    // release can abstain on it.
    assert_eq!(folder(&["x"], None), Some(String::new()));
    pack_hooks::clear_host();
}

#[test]
fn a_taint_sink_gate_keeps_the_finding_alive_unless_it_says_otherwise() {
    let slot = one_hook(HookProgram::new(
        "mylib::log",
        HookFamily::TaintSinkGate,
        "if {[lindex $words 0] eq \"-safe\"} { sink-suppressed }\n",
    ));
    let gate = pack_hooks::taint_sink_gate_fn(slot).expect("a sink thunk");
    assert!(gate(&["untrusted"]), "silence means the sink applies");
    assert!(!gate(&["-safe"]), "an explicit suppression turns it off");
    pack_hooks::clear_host();
}

#[test]
fn a_context_gate_rejects_with_a_message_and_reads_in_event_body() {
    let slot = one_hook(HookProgram::new(
        "mylib::return",
        HookFamily::ContextGate,
        "if {[dict get $ctx in-event-body] && [llength $words] > 0} {\n\
         reject {only the bare form is allowed in an event body}\n}\n",
    ));
    let gate = pack_hooks::context_gate_fn(slot).expect("a gate thunk");
    assert_eq!(gate(&["value"], false), None, "silence allows the call");
    assert_eq!(
        gate(&["value"], true),
        Some("only the bare form is allowed in an event body")
    );
    pack_hooks::clear_host();
}

#[test]
fn a_literal_validator_sees_kinds_and_reports_a_typed_issue() {
    let slot = one_hook(HookProgram::new(
        "mylib::cipher",
        HookFamily::LiteralArgumentValidator,
        "if {[lindex [dict get $ctx kinds] 0] ne \"literal\"} {\n\
         abstain non-literal-argument\n return\n}\n\
         if {[lindex $words 0] eq \"NULL\"} {\n\
         invalid -index 0 -subject {cipher suite} -reason invalid-members \
         -members NULL -allowed {AES CHACHA20}\n}\n",
    ));
    let validator = pack_hooks::literal_argument_validator_fn(slot).expect("a validator thunk");

    assert_eq!(
        validator(InvocationArguments::literals(&["AES"])),
        LiteralArgumentValidation::Valid,
        "silence means valid"
    );

    let dynamic = [InvocationWord::Dynamic];
    assert!(
        matches!(
            validator(InvocationArguments::structured(&dynamic)),
            LiteralArgumentValidation::Abstain(_)
        ),
        "a dynamic word is visible in kinds and declines"
    );

    let LiteralArgumentValidation::Invalid(issue) =
        validator(InvocationArguments::literals(&["NULL"]))
    else {
        panic!("the body reports an invalid member");
    };
    assert_eq!(issue.argument_index, 0);
    assert_eq!(issue.subject, "cipher suite");
    assert_eq!(issue.allowed_values, ["AES", "CHACHA20"]);
    assert_eq!(
        issue.reason,
        LiteralArgumentIssueReason::InvalidMembers(vec!["NULL".to_owned()])
    );
    pack_hooks::clear_host();
}

#[test]
fn a_clause_shape_check_reports_the_first_structural_defect() {
    let slot = one_hook(HookProgram::new(
        "mylib::if",
        HookFamily::ClauseShapeCheck,
        "if {[llength $words] == 0} { missing-expr\n return }\n\
         if {[llength $words] == 1} { missing-body 0 }\n",
    ));
    let check = pack_hooks::clause_shape_check_fn(slot).expect("a shape thunk");
    assert_eq!(check(&["1", "{body}"]), None, "silence accepts the shape");
    assert_eq!(
        check(&[]),
        Some(ClauseShapeError::MissingExpr { after: None })
    );
    assert_eq!(
        check(&["1"]),
        Some(ClauseShapeError::MissingBody { after: 0 })
    );
    pack_hooks::clear_host();
}

#[test]
fn an_option_arity_hook_reads_its_option_keys_and_consumes() {
    let slot = one_hook(HookProgram::new(
        "mylib::return",
        HookFamily::OptionArity,
        "set start [dict get $ctx option-value-start]\n\
         set value [lindex $words $start]\n\
         if {$value eq \"\"} { consume 0\n return }\n\
         if {![string is list $value]} {\n\
         consume 1 -invalid \"bad -errorstack value\"\n return\n}\n\
         consume 1\n",
    ));
    let hook = pack_hooks::option_arity_fn(slot).expect("an option-arity thunk");

    let outcome = hook(&["-errorstack", "{CALL a}"], 1);
    assert_eq!(outcome.words, 1);
    assert_eq!(outcome.invalid, None);

    let missing = hook(&["-errorstack"], 1);
    assert_eq!(missing.words, 0, "consume 0 is a report, not an abstention");

    let bad = hook(&["-errorstack", "{unbalanced"], 1);
    assert_eq!(bad.words, 1);
    assert_eq!(bad.invalid, Some("bad -errorstack value"));
    pack_hooks::clear_host();
}

#[test]
fn a_body_that_reaches_for_a_denied_command_abstains() {
    let slot = one_hook(HookProgram::new(
        "mylib::sneaky",
        HookFamily::ConstFold,
        "set data [open /etc/passwd]\nfold $data",
    ));
    let folder = pack_hooks::const_fold_fn(slot).expect("a fold thunk");
    assert_eq!(
        folder(&["x"]),
        None,
        "the sandbox denies it, so it abstains"
    );
    pack_hooks::clear_host();
}

#[test]
fn foldlist_is_available_and_quotes_like_the_shipped_list_fold() {
    let slot = one_hook(HookProgram::new(
        "mylib::pair",
        HookFamily::ConstFold,
        "fold [foldlist [lindex $words 0] [lindex $words 1]]",
    ));
    let folder = pack_hooks::const_fold_fn(slot).expect("a fold thunk");
    assert_eq!(
        folder(&["a b", "c"]),
        tcl_registry::const_fold::fold_list(&["a b", "c"])
    );
    pack_hooks::clear_host();
}

/// **The E-R14 escape hatch, end to end** — a `constraints` body reading the
/// whole invocation through its four reading verbs and reporting through
/// `invalid SLOT MESSAGE ?-conflict?`.
///
/// The rule below is deliberately one no declarative relation can express: a
/// *numeric comparison* between two option values. That is what the hatch is
/// for; everything the vocabulary can say is checked natively instead.
#[test]
fn constraints_reads_the_invocation_and_reports_through_invalid() {
    let slot = one_hook(
        HookProgram::new(
            "mylib::window",
            HookFamily::Constraints,
            "if {![dict get $ctx complete]} { abstain }\n\
             if {![option-present -min] || ![option-present -max]} { return }\n\
             set lo [option-value -min]\n\
             set hi [option-value -max]\n\
             if {$lo eq {} || $hi eq {}} { abstain }\n\
             if {$lo > $hi} { invalid -min \"-min $lo exceeds -max $hi\" }\n",
        )
        .with_inputs(HookInputs::parse(&["invocation"])),
    );
    let hook = pack_hooks::constraints_fn(slot).expect("a constraints thunk");

    let facts = |min: &'static str, max: &'static str| {
        ([("-min", Some(min)), ("-max", Some(max))], [Some("body")])
    };

    let (options, positionals) = facts("9", "2");
    let bad = hook(&tcl_registry::RelationFacts {
        options: &options,
        positionals: &positionals,
        complete: true,
    });
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert_eq!(bad[0].slot, tcl_registry::ConstraintSlot::Option("-min"));
    assert_eq!(bad[0].message, "-min 9 exceeds -max 2");
    assert!(!bad[0].conflict, "the default report is W152, not W147");

    let (options, positionals) = facts("2", "9");
    assert!(
        hook(&tcl_registry::RelationFacts {
            options: &options,
            positionals: &positionals,
            complete: true,
        })
        .is_empty(),
        "an ordered window is silent"
    );

    // Abstention: an invocation the analyser could not read to its end is
    // judged by nobody, exactly as the `types` hook contract requires.
    let (options, positionals) = facts("9", "2");
    assert!(
        hook(&tcl_registry::RelationFacts {
            options: &options,
            positionals: &positionals,
            complete: false,
        })
        .is_empty(),
        "an incomplete invocation abstains rather than accusing"
    );
    pack_hooks::clear_host();
}

/// **The P-B cache discipline** (redesign §0.05): a `constraints` hook that
/// declared `-inputs {invocation}` is content-cached, so re-analysing a
/// document after an edit elsewhere never re-runs it for an unchanged call
/// site — while a call site whose own words changed does re-run.
#[test]
fn a_constraints_verdict_is_reused_when_the_call_site_did_not_change() {
    let slot = one_hook(
        HookProgram::new(
            "mylib::cached",
            HookFamily::Constraints,
            "if {[option-present -min]} { invalid -min {reported} }\n",
        )
        .with_inputs(HookInputs::parse(&["invocation"])),
    );
    assert_eq!(
        pack_hooks::cache_mode(slot),
        pack_hooks::CacheMode::Content,
        "declaring `invocation` earns content caching"
    );
    let hook = pack_hooks::constraints_fn(slot).expect("a constraints thunk");
    let judge = |min: &'static str| {
        let options = [("-min", Some(min))];
        let positionals = [Some("body")];
        hook(&tcl_registry::RelationFacts {
            options: &options,
            positionals: &positionals,
            complete: true,
        })
    };

    pack_hooks::clear_cache();
    let first = judge("1");
    assert_eq!(first.len(), 1);
    let cold = pack_hooks::cache_stats();
    assert_eq!((cold.hits, cold.misses), (0, 1), "the first call is cold");

    // The same call site, re-analysed after an edit somewhere else in the
    // document: identical content, so the verdict is reused and the VM is
    // never entered.
    for _ in 0..5 {
        assert_eq!(judge("1").len(), 1);
    }
    let warm = pack_hooks::cache_stats();
    assert_eq!(
        (warm.hits, warm.misses),
        (5, 1),
        "an unchanged call site is answered from the cache"
    );

    // Editing *this* call site changes its content hash, so it is re-judged.
    assert_eq!(judge("2").len(), 1);
    let edited = pack_hooks::cache_stats();
    assert_eq!(
        (edited.hits, edited.misses),
        (5, 2),
        "a changed call site re-runs, and only it"
    );
    pack_hooks::clear_host();
}

/// An **undeclared** `constraints` hook stays uncacheable: it made no claim
/// about what it reads, so nothing may be hashed on its behalf. The
/// abstention is conservative, not a caching bug.
#[test]
fn an_undeclared_constraints_hook_is_not_cached() {
    let slot = one_hook(HookProgram::new(
        "mylib::undeclared",
        HookFamily::Constraints,
        "invalid command {always}\n",
    ));
    assert_eq!(pack_hooks::cache_mode(slot), pack_hooks::CacheMode::None);
    let hook = pack_hooks::constraints_fn(slot).expect("a constraints thunk");
    pack_hooks::clear_cache();
    let options: [(&'static str, Option<&str>); 0] = [];
    let positionals: [Option<&str>; 0] = [];
    for _ in 0..3 {
        let reports = hook(&tcl_registry::RelationFacts {
            options: &options,
            positionals: &positionals,
            complete: true,
        });
        assert_eq!(reports[0].slot, tcl_registry::ConstraintSlot::Command);
    }
    let stats = pack_hooks::cache_stats();
    assert_eq!((stats.hits, stats.misses), (0, 0), "never cached at all");
    pack_hooks::clear_host();
}
