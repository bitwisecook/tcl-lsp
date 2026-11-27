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

//! Grammar follows the head's effective command identity: qualified names, aliases, renames and rebinding.

use super::*;

// Semantic tokens read command grammar from the registry,
// so the explicitly global spellings C Tcl resolves to the same commands
// (`namespace which -command ::format` → `::format`) are classified
// identically to their bare forms, and a same-named user proc is not.

/// The decoded token *kinds* of `src`, positions discarded.
fn kinds_only(src: &str, registry: &CommandRegistry) -> Vec<u32> {
    decode_full(src, tcl(), registry)
        .into_iter()
        .map(|(_, _, _, kind, _)| kind)
        .collect()
}

/// Assert that qualifying a command head with a leading `::` changes
/// nothing but the head itself: the qualified stream must *end with* the
/// bare stream's kinds (the qualified head contributes one extra
/// namespace-separator token at the front), so every argument is
/// classified identically.
fn assert_qualified_matches_bare(
    bare: &str,
    head: &str,
    qualified: &str,
    registry: &CommandRegistry,
) {
    let bare_kinds = kinds_only(bare, registry);
    let qualified_kinds = kinds_only(&bare.replacen(head, qualified, 1), registry);
    assert!(
        qualified_kinds.len() >= bare_kinds.len(),
        "{qualified} produced fewer tokens than {head}: \
             {qualified_kinds:?} vs {bare_kinds:?}"
    );
    let tail = &qualified_kinds[qualified_kinds.len() - bare_kinds.len()..];
    assert_eq!(
        tail,
        &bare_kinds[..],
        "{qualified} classified its arguments differently from {head}"
    );
}

/// The loop-variable-list scan is registry-driven,
/// so a `::`-qualified head classifies exactly like the bare one.
///
/// tclsh-proof (9.0.4): `::foreach {a b} {1 2} { puts $a }` and
/// `::dict for {k v} {a 1} { puts $v }` run identically to their bare forms —
/// `namespace which -command ::foreach` is `::foreach`.
#[test]
fn qualified_loop_headers_classify_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
        ("lmap x {1 2} { expr {$x} }\n", "lmap", "::lmap"),
        ("dict for {k v} {a 1} { puts $v }\n", "dict", "::dict"),
        ("dict map {k v} {a 1} { set v }\n", "dict", "::dict"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

/// The loop-variable binding pass reads the registry's own
/// `ArgRole::LoopVarList` indices, so every loop header it understands is
/// one the registry declares — not a `foreach` / `lmap` / `dict for`
/// spelling list in the walker.
#[test]
fn loop_var_list_roles_come_from_the_registry() {
    let r = reg();
    let roles = |cmd: &str, args: &[&str]| {
        r.arg_indices_for_role(cmd, args, tcl_registry::ArgRole::LoopVarList)
    };
    // `foreach VARS LIST ?VARS LIST …? BODY`: the repeated (start 0,
    // stride 2) layout marks every variable-list word.
    assert_eq!(roles("foreach", &["{a b}", "$l", "{}"]), vec![0]);
    assert_eq!(
        roles("foreach", &["{a}", "$l", "{b}", "$m", "{}"]),
        vec![0, 2]
    );
    assert_eq!(roles("lmap", &["x", "$l", "{}"]), vec![0]);
    // `dict for {k v} $d body` marks its pair at index 1 (after the
    // subcommand word), and the collection at index 2 is declared a Dict —
    // which is what tells the binder this is a key/value pair rather than
    // a free variable list.
    assert_eq!(roles("dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
    assert_eq!(roles("dict", &["map", "{k v}", "$d", "{}"]), vec![1]);
    assert_eq!(
        r.arg_type_hint("dict", &["for", "{k v}", "$d", "{}"], 2)
            .and_then(|h| h.expected),
        Some(tcl_registry::types::TclType::Dict)
    );
    // A `::`-qualified head resolves to the same spec, so it reports the
    // same roles.
    assert_eq!(roles("::foreach", &["{a b}", "$l", "{}"]), vec![0]);
    assert_eq!(roles("::dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
    // A command with no loop-variable list reports none.
    assert_eq!(roles("puts", &["hi"]), [] as [usize; 0]);
}

#[test]
fn loop_var_binding_uses_tcl_list_grammar_and_abstains_when_dynamic() {
    use tcl_source::segment::segment_commands;

    let names = |src: &str| {
        let command = segment_commands(src)
            .into_iter()
            .next()
            .expect("one command");
        static_loop_var_names(
            src,
            &command,
            1,
            tcl_syntax::word_rules::WordValueRules::default(),
        )
    };

    assert_eq!(
        names("foreach {a b} $coll {}\n"),
        Some(vec![String::from("a"), String::from("b")])
    );
    assert_eq!(
        names("foreach \"a b\" $coll {}\n"),
        Some(vec![String::from("a"), String::from("b")])
    );
    assert_eq!(
        names("foreach a\\ b $coll {}\n"),
        Some(vec![String::from("a b")])
    );
    assert_eq!(names("foreach {} $coll {}\n"), Some(Vec::new()));
    assert_eq!(names("foreach $vars $coll {}\n"), None);
    assert_eq!(names("foreach [makeVars] $coll {}\n"), None);
    assert_eq!(names("foreach {a b $coll {}\n"), None);
}

#[test]
fn qualified_format_family_classifies_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("format {%08x} 42\n", "format", "::format"),
        ("scan $s {%d %d} a b\n", "scan", "::scan"),
        ("binary format c3 {1 2 3}\n", "binary", "::binary"),
        ("binary scan $v c3 out\n", "binary", "::binary"),
        ("clock format $t -format {%Y-%m-%d}\n", "clock", "::clock"),
        ("clock scan $s -format {%Y}\n", "clock", "::clock"),
        ("regsub -all e $s {[&]} out\n", "regsub", "::regsub"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

#[test]
fn qualified_grammar_commands_classify_like_the_bare_form() {
    let r = reg();
    for (bare, head, qualified) in [
        ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
        ("lmap x $l { expr {$x} }\n", "lmap", "::lmap"),
        ("upvar 1 src local\n", "upvar", "::upvar"),
        ("upvar src local\n", "upvar", "::upvar"),
        ("global aa bb cc\n", "global", "::global"),
        ("variable x 1 y 2\n", "variable", "::variable"),
        (
            "oo::define C method m {args} { return $args }\n",
            "oo::define",
            "::oo::define",
        ),
        (
            "oo::objdefine $o method m {} { return }\n",
            "oo::objdefine",
            "::oo::objdefine",
        ),
        (
            "namespace upvar ::ns o1 l1 o2 l2\n",
            "namespace",
            "::namespace",
        ),
        ("dict update d k1 v1 k2 v2 { puts $v1 }\n", "dict", "::dict"),
    ] {
        assert_qualified_matches_bare(bare, head, qualified, &r);
    }
}

/// FP guard — a user proc in another namespace that happens to share a
/// built-in's tail name does not inherit its grammar, and data words that
/// merely look like format strings are not painted as ones.
#[test]
fn same_named_user_command_does_not_inherit_grammar() {
    let r = reg();
    // `ns::format` is a different command; its argument stays a plain
    // string, so no format-specifier sub-tokens appear.
    let user = decode_full("ns::format {%08x} 42\n", tcl(), &r);
    let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
    assert!(
        builtin.len() > user.len(),
        "the built-in must produce specifier sub-tokens the user proc does not:\n\
             builtin={builtin:?}\nuser={user:?}"
    );
    // A data word that spells a format string is untouched.
    assert_eq!(
        kinds_only("puts {%08x}\n", &r),
        kinds_only("puts {plain}\n", &r)
    );
}

// A head's *effective command identity* (a static
// `interp alias`, a `rename`, a shadowing top-level `proc`) drives the
// grammar, so calling a built-in through a proven alias classifies exactly
// like calling it directly, and calling a name whose binding was taken
// over does not.

/// TP — `interp alias {} myfmt {} format` makes `myfmt %08x 42` classify
/// exactly like `format %08x 42`.
///
/// tclsh-proof (9.0.4 and 8.6.16, byte-identical): `interp alias {} myfmt
/// {} format; myfmt %08x 42` → `0000002a`.
#[test]
fn a_static_interp_alias_inherits_the_targets_grammar() {
    let r = reg();
    let bind = "interp alias {} myfmt {} format\n";
    // The argument kinds of a direct `format` call — its head token is the
    // one thing an aliased or qualified spelling legitimately differs in.
    let direct = kinds_only("format {%08x} 42\n", &r);
    let direct_args = &direct[1..];
    for head in ["myfmt", "::myfmt"] {
        let call = kinds_only(&format!("{bind}{head} {{%08x}} 42\n"), &r);
        assert_eq!(
            &call[call.len() - direct_args.len()..],
            direct_args,
            "`{head}` must classify its arguments like a direct `format` call"
        );
    }
    // Without the alias the same call is an ordinary unknown command whose
    // argument stays a plain string.
    let unaliased = kinds_only("myfmt {%08x} 42\n", &r);
    assert!(
        unaliased.len() < direct.len(),
        "an unbound name must not get format sub-tokens: {unaliased:?}"
    );
}

/// TP — `rename foreach myforeach` moves the loop grammar (and the
/// keyword classification) onto the new name.
#[test]
fn a_static_rename_moves_the_grammar_to_the_new_name() {
    let r = reg();
    let bind = "rename upvar myupvar\n";
    let direct = kinds_only("upvar 1 src local\n", &r);
    let renamed = kinds_only(&format!("{bind}myupvar 1 src local\n"), &r);
    let baseline = kinds_only(bind, &r);
    assert_eq!(
        &renamed[baseline.len()..],
        &direct[..],
        "a renamed call must classify like the original"
    );
}

/// FP — a name whose binding was taken over gets **no** registry grammar:
/// after `rename format origfmt` the bare `format` is gone, and a top-level
/// `proc format` shadows the built-in outright.
///
/// tclsh-proof (9.0.4 and 8.6.16): `rename format origfmt; proc format
/// {args} {return USER}; format x` → `USER`; `origfmt %d 7` → `7`.
#[test]
fn a_rebound_builtin_loses_its_grammar() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    for bind in [
        "rename format origfmt\n",
        "rename format {}\n",
        "proc format {args} { return USER }\n",
        "interp alias {} format {} myformatter\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
        assert!(
            after.len() - baseline.len() < direct.len(),
            "`{}` must strip format's specifier sub-tokens, got {after:?}",
            bind.trim()
        );
    }
}

/// FN guard — a binding only applies from its own statement onwards, so a
/// call *before* the `rename` still classifies as the built-in.
#[test]
fn a_binding_does_not_retroactively_retag_earlier_calls() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    let before = kinds_only("format {%08x} 42\nrename format origfmt\n", &r);
    let rename_only = kinds_only("rename format origfmt\n", &r);
    assert_eq!(
        &before[..direct.len()],
        &direct[..],
        "the call before the rename must keep the built-in's grammar"
    );
    assert_eq!(&before[direct.len()..], &rename_only[..]);
}

/// TN — a binding the analyser cannot prove states nothing, so the head
/// keeps its literal identity rather than gaining a wrong one.
#[test]
fn unprovable_bindings_abstain() {
    let r = reg();
    let plain = kinds_only("myfmt {%08x} 42\n", &r);
    for bind in [
        // Dynamic alias target / name, dynamic rename source.
        "interp alias {} myfmt {} $target\n",
        "interp alias {} $n {} format\n",
        "rename $old myfmt\n",
        // Pre-bound arguments shift every index.
        "interp alias {} myfmt {} format %08x\n",
        // A child interpreter's command table is not this one's.
        "interp alias slave myfmt {} format\n",
        // Not an unconditional top-level statement.
        "if {$x} { interp alias {} myfmt {} format }\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let after = kinds_only(&format!("{bind}myfmt {{%08x}} 42\n"), &r);
        assert_eq!(
            &after[baseline.len()..],
            &plain[..],
            "`{}` must not give `myfmt` a grammar",
            bind.trim()
        );
    }
}

/// FP — a `proc` in another namespace, or one that shadows nothing, states
/// no fact: only a global-namespace redefinition of a built-in takes a name
/// over (tclsh 9.0.4: inside `namespace eval ::n`, `proc format` defines
/// `::n::format` and the global `format` is untouched).
#[test]
fn a_namespaced_proc_does_not_shadow_the_global_builtin() {
    let r = reg();
    let direct = kinds_only("format {%08x} 42\n", &r);
    let bind = "namespace eval ::n { proc format {a} { return 1 } }\n";
    let baseline = kinds_only(bind, &r);
    let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
    assert_eq!(&after[baseline.len()..], &direct[..]);
}

/// TP — `apply`'s `ArgRole::LambdaLiteral` reaches the renamed and aliased
/// spellings too, closing the failure mode written up in the
/// apply-lambda-body KCS note: without them a `[list …]`-quoted or
/// directly-called lambda under `rename apply myapply` collapses into one
/// opaque `string` token.
///
/// tclsh-proof (9.0.4 / 8.6.16): `rename apply myapply; myapply {x {puts
/// $x}} 5` prints `5`, exactly as the literal call does.
#[test]
fn apply_lambda_literals_survive_a_rename_or_alias() {
    let r = reg();
    let direct = kinds_only("apply {x {puts $x}} 5\n", &r);
    for bind in [
        "rename apply myapply\n",
        "interp alias {} myapply {} apply\n",
    ] {
        let baseline = kinds_only(bind, &r);
        let bound = kinds_only(&format!("{bind}myapply {{x {{puts $x}}}} 5\n"), &r);
        assert_eq!(
            &bound[baseline.len()..],
            &direct[..],
            "`{}` must keep apply's lambda-literal split",
            bind.trim()
        );
    }
}

/// TN — a `{*}`-expanded (dynamic) head has no resolvable identity, so no
/// grammar is applied to its arguments.
#[test]
fn dynamic_head_gets_no_format_grammar() {
    let r = reg();
    let dynamic = decode_full("{*}$cmd {%08x} 42\n", tcl(), &r);
    let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
    assert!(
        dynamic.len() < builtin.len(),
        "a dynamic head must not get format sub-tokens: {dynamic:?}"
    );
}
