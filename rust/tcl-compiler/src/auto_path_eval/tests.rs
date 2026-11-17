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

use super::*;

#[test]
fn written_inventory_names_preserve_constructed_namespace_segments() {
    assert_eq!(rooted_variable_key("a:::b"), "::a::b");
    assert_eq!(qualified_key("::a:::", "b:::c"), "::a:::::b::c");
    let writes = constant_path_assignments(
        "namespace eval : {variable dir /one; namespace eval x {variable dir /two}}",
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].name, ":::::dir");
    assert_eq!(writes[1].name, ":::::x::dir");
}

// Absolute `info_script` keeps the fold rooted; a rootless fold result
// is returned relative (see the module docs), never anchored on the
// analysing process's working directory.

/// `file` resolves in the document's dialect: iRules has no `file`,
/// so `file tail`, which only the route answers, folds in a plain Tcl
/// document and not in an iRule.
#[test]
fn a_file_route_is_the_documents_dialects() {
    let irules = tcl_dialect::DialectProfile::find("f5-irules").expect("f5-irules");
    let fold = |dialect| {
        evaluate_auto_path_expr_with_constants("[file tail /a/b]", None, &HashMap::new(), dialect)
    };
    assert_eq!(fold(None).as_deref(), Some("b"));
    assert_eq!(fold(Some(irules)), None);
}

#[test]
fn dirname_of_info_script() {
    assert_eq!(
        evaluate_auto_path_expr("[file dirname [info script]]", Some("/proj/lib/init.tcl"))
            .as_deref(),
        Some("/proj/lib"),
    );
}

#[test]
fn parent_dir_join_idiom_collapses_dotdot() {
    // [file join [file dirname [info script]] ..] → parent of the
    // script's directory.
    assert_eq!(
        evaluate_auto_path_expr(
            "[file join [file dirname [info script]] ..]",
            Some("/proj/lib/pkg/init.tcl"),
        )
        .as_deref(),
        Some("/proj/lib"),
    );
}

#[test]
fn nested_dirname() {
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [file dirname [info script]]]",
            Some("/proj/lib/init.tcl"),
        )
        .as_deref(),
        Some("/proj"),
    );
}

/// TP — the idiom `pix` writes: three nested
/// `file dirname`s around `[info script]`, one level deeper than the
/// two-level form the other tests use.  `examples/user.tcl` reaching the
/// repo root two directories up is what makes the package resolvable
/// when the workspace root is scoped below it.
#[test]
fn triple_nested_dirname_is_the_pix_idiom() {
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [file dirname [file dirname [info script]]]]",
            Some("/proj/lib/examples/user.tcl"),
        )
        .as_deref(),
        Some("/proj"),
    );
}

/// The analysing process's working directory must never influence a
/// fold.  A rootless expression stays rootless (its caller anchors it);
/// running the same fold from two different directories must not change
/// the answer.
///
/// On tclsh 8.6.16 / 9.0.4: `set v helper.tcl; source $v` inside
/// `sub/main.tcl` run as `cd other && tclsh ../sub/main.tcl` loads
/// `other/helper.tcl` — resolution is against the interpreter's *run
/// time* working directory, which a language server cannot know and must
/// not substitute its own launch directory for.
#[test]
fn a_rootless_fold_stays_relative_and_ignores_the_process_cwd() {
    assert_eq!(
        evaluate_auto_path_expr("helper.tcl", None).as_deref(),
        Some("helper.tcl"),
    );
    assert_eq!(
        evaluate_auto_path_expr("[file join lib pkg]", None).as_deref(),
        Some("lib/pkg"),
    );
    // `..` still collapses lexically without a root to ascend past.
    // (tclsh returns the uncollapsed text `lib/../other`; this evaluator
    // normalises deliberately — see `normalise_path_value` — because its
    // consumers compare and scan directories rather than print them, and
    // the two name the same directory.)
    assert_eq!(
        evaluate_auto_path_expr("[file join lib .. other]", None).as_deref(),
        Some("other"),
    );
    // `file dirname` of a single-component relative path is `.` in tclsh
    // 8.6/9.0 — not the process's working directory.
    assert_eq!(
        evaluate_auto_path_expr("[file dirname helper.tcl]", None).as_deref(),
        Some("."),
    );
}

#[test]
fn literal_absolute_path() {
    assert_eq!(
        evaluate_auto_path_expr("/usr/local/lib", None).as_deref(),
        Some("/usr/local/lib"),
    );
}

#[test]
fn variable_substitution_is_unsupported() {
    assert_eq!(evaluate_auto_path_expr("$auto_path", None), None);
    assert_eq!(
        evaluate_auto_path_expr("[file dirname $base]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn info_script_without_path_is_none() {
    assert_eq!(
        evaluate_auto_path_expr("[file dirname [info script]]", None),
        None
    );
}

// `file normalize` — the wrapper the corpus writes around the
// `[info script]` idiom; without it every `source [file join $dir …]`
// in a file spelled that way abstains.

/// Both nestings of the corpus idiom fold, and to the same directory.
///
/// On tclsh 8.6.16 / 9.0.4: with `[info script]` reporting
/// `/proj/test/arbitaryTest.tcl`, both spellings answer `/proj/test`.
#[test]
fn file_normalize_folds_the_info_script_idiom() {
    for expr in [
        "[file normalize [file dirname [info script]]]",
        "[file dirname [file normalize [info script]]]",
    ] {
        assert_eq!(
            evaluate_auto_path_expr(expr, Some("/proj/test/arbitaryTest.tcl")).as_deref(),
            Some("/proj/test"),
            "{expr} must fold",
        );
    }
}

/// `file normalize` collapses `.` and `..` like the rest of the module.
#[test]
fn file_normalize_collapses_dot_segments() {
    assert_eq!(
        evaluate_auto_path_expr("[file normalize /proj/test/../lib/.]", None).as_deref(),
        Some("/proj/lib"),
    );
}

/// A relative argument does not fold: `file normalize` would resolve it
/// against the interpreter's working directory, which is not knowable
/// statically — and inventing the analysing process's own cwd is the one
/// answer that is always wrong.
#[test]
fn file_normalize_of_a_relative_path_is_none() {
    for expr in [
        "[file normalize lib]",
        "[file normalize ../lib]",
        "[file normalize [file dirname helper.tcl]]",
        "[file normalize C:lib]",
    ] {
        assert_eq!(
            evaluate_auto_path_expr(expr, Some("/proj/test/caller.tcl")),
            None,
            "{expr} must abstain",
        );
    }
}

/// Anchored by `~` rather than a literal root, expanded the same way the
/// top-level fold expands it.
#[test]
fn file_normalize_expands_tilde() {
    temp_env::with_var("HOME", Some("/home/tester"), || {
        assert_eq!(
            evaluate_auto_path_expr("[file normalize ~/lib/..]", None).as_deref(),
            Some("/home/tester"),
        );
    });
}

// Chained single-assignment constants.

/// A directory reached through an intermediate folds, so the whole chain
/// resolves.
///
/// On tclsh 8.6.16 / 9.0.4: running `/proj/SpiceGenTcl.tcl`, `dir`
/// is `/proj` and `sourceDir` is `/proj/src`.
#[test]
fn constants_fold_through_a_chain() {
    let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set libDir [file join $dir lib]\n\
                   set sourceDir [file join $dir src]\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/proj/SpiceGenTcl.tcl"),
    );
    assert_eq!(constants.get("dir").map(String::as_str), Some("/proj"));
    assert_eq!(
        constants.get("libDir").map(String::as_str),
        Some("/proj/lib")
    );
    assert_eq!(
        constants.get("sourceDir").map(String::as_str),
        Some("/proj/src"),
    );
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "[file join $sourceDir ngspice x.tcl]",
            Some("/proj/SpiceGenTcl.tcl"),
            &constants,
            None,
        )
        .as_deref(),
        Some("/proj/src/ngspice/x.tcl"),
    );
}

/// Order matters: a value naming a constant established *later* does not
/// fold, because at that point in the file the name has no value.
#[test]
fn a_constant_used_before_it_is_assigned_does_not_fold() {
    let src = "set early [file join $late x]\n\
                   set late /opt\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(constants.get("late").map(String::as_str), Some("/opt"));
    assert!(!constants.contains_key("early"), "{constants:?}");
}

/// A re-assigned name contributes nothing — and nothing computed from it
/// contributes either, since the poison is counted in a pre-pass rather
/// than as the walk goes.  Which value a later `source` sees is a flow
/// question this map does not answer.
#[test]
fn a_reassigned_name_poisons_itself_and_its_dependents() {
    let src = "set dir /opt/a\n\
                   set sub [file join $dir sub]\n\
                   set dir /opt/b\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("dir"), "{constants:?}");
    assert!(
        !constants.contains_key("sub"),
        "a value computed from a poisoned name must not survive: {constants:?}",
    );
}

/// A resolved value containing spaces is **one** path element, exactly as
/// `file join {my dir} x` treats its arguments (filename(n): join takes
/// arguments, not words — a space is just a character).  On tclsh
/// 8.6.14: `set d "my dir"; file join $d x` → `my dir/x`, and a `source`
/// through such a join really loads from the spaced directory.  This is
/// the case that makes node-based resolution obligatory: splicing
/// `my dir` into the raw text and re-lexing splits it into two words and
/// answers `my/dir/x`.
#[test]
fn a_constant_with_spaces_folds_as_one_join_element() {
    let src = "set d {my dir}\nset sub [file join $d x]\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(constants.get("d").map(String::as_str), Some("my dir"));
    assert_eq!(constants.get("sub").map(String::as_str), Some("my dir/x"));
    assert_eq!(
        evaluate_auto_path_expr_with_constants("[file join $d y.tcl]", None, &constants, None)
            .as_deref(),
        Some("my dir/y.tcl"),
    );
}

/// A mixed word — literal text around the reference — folds through the
/// same interpolation grammar, so `source $base/lib/x.tcl` resolves.
#[test]
fn a_mixed_word_resolves_through_the_resolver() {
    let constants: HashMap<String, String> = [("base".to_owned(), "/opt".to_owned())]
        .into_iter()
        .collect();
    assert_eq!(
        evaluate_auto_path_expr_with_constants("$base/lib/x.tcl", None, &constants, None)
            .as_deref(),
        Some("/opt/lib/x.tcl"),
    );
    assert_eq!(
        evaluate_auto_path_expr_with_constants("${base}2/x.tcl", None, &constants, None).as_deref(),
        Some("/opt2/x.tcl"),
    );
}

/// Braces suppress substitution: `{$d}` names a file with a literal `$`
/// in it, and the fold abstains on it even when the resolver knows `d` —
/// resolving there would be inventing a substitution Tcl never performs.
#[test]
fn a_braced_dollar_word_is_never_resolved() {
    let constants: HashMap<String, String> =
        [("d".to_owned(), "/opt".to_owned())].into_iter().collect();
    assert_eq!(
        evaluate_auto_path_expr_with_constants("[file join {$d} x]", None, &constants, None),
        None,
    );
}

/// The resolver entry point is the lower-layer contract: any caller with
/// its own variable facts — the analyser's constant-string lattice, an
/// SCCP value table — plugs in here without this module knowing it.
#[test]
fn a_custom_resolver_supplies_variable_provenance() {
    let folded = evaluate_auto_path_expr_with_resolver(
        "[file join $root pkg]",
        None,
        &|name| (name == "root").then(|| "/from/lattice".to_owned()),
        None,
    );
    assert_eq!(folded.as_deref(), Some("/from/lattice/pkg"));
    // An unresolved name abstains the whole fold — never a guess.
    assert_eq!(
        evaluate_auto_path_expr_with_resolver("[file join $other pkg]", None, &|_| None, None),
        None,
    );
}

/// Assignments inside a body are not top-level and never enter the map:
/// the segmenter sees the `proc` body as one braced word.
#[test]
fn a_set_inside_a_body_is_not_a_top_level_constant() {
    let src = "proc p {} {\n    set inner /opt\n}\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("inner"), "{constants:?}");
}

// Namespace-variable path constants — each test is a corpus shape, named
// for the project that writes it.

/// tcllib snit's shape, verbatim: `variable` with a value inside a
/// `namespace eval ::snit:: { … }` (trailing colons and all), consumed
/// by a fully-qualified reference in the same file.
#[test]
fn snit_variable_with_value_in_namespace_eval_folds() {
    let src = "namespace eval ::snit:: {\n\
                   \x20   variable library [file dirname [info script]]\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/lib/snit/snit.tcl"),
    );
    assert_eq!(
        constants.get("::snit::library").map(String::as_str),
        Some("/lib/snit"),
    );
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "[file join $::snit::library main1.tcl]",
            Some("/lib/snit/snit.tcl"),
            &constants,
            None,
        )
        .as_deref(),
        Some("/lib/snit/main1.tcl"),
    );
}

/// ruff's shape: a *nested relative* `namespace eval private` inside
/// `namespace eval ruff`, with a value-less `variable` declaration
/// followed by an unconditional `set` — the declaration must not count
/// as a write, or the set would look like a second one and poison.
#[test]
fn ruff_declaration_then_set_in_nested_namespace_folds() {
    let src = "namespace eval ruff {\n\
                   \x20   namespace eval private {\n\
                   \x20       variable ruff_dir\n\
                   \x20       set ruff_dir [file dirname [info script]]\n\
                   \x20       variable names\n\
                   \x20       set names(display) \"Ruff!\"\n\
                   \x20   }\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/proj/src/ruff.tcl"),
    );
    assert_eq!(
        constants
            .get("::ruff::private::ruff_dir")
            .map(String::as_str),
        Some("/proj/src"),
        "{constants:?}",
    );
    // The array-element set is neither recorded nor allowed to disturb
    // anything else.
    assert!(!constants.contains_key("::ruff::private::names(display)"));
}

/// alited's shape: a multi-hop bare chain *inside* the namespace body —
/// each `variable` value reads the previous one by bare name, which
/// resolves namespace-first exactly as Tcl reads it — consumed by
/// qualified references.
#[test]
fn alited_bare_chain_inside_namespace_body_folds() {
    let src = "namespace eval alited {\n\
                   \x20   variable DIR [file normalize [file dirname [info script]]]\n\
                   \x20   variable LIBDIR [file join $DIR lib]\n\
                   \x20   variable HLDIR [file join $LIBDIR hl_tcl]\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/app/src/alited.tcl"),
    );
    assert_eq!(
        constants.get("::alited::HLDIR").map(String::as_str),
        Some("/app/src/lib/hl_tcl"),
        "{constants:?}",
    );
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "[file join $::alited::HLDIR hl_c.tcl]",
            Some("/app/src/alited.tcl"),
            &constants,
            None,
        )
        .as_deref(),
        Some("/app/src/lib/hl_tcl/hl_c.tcl"),
    );
}

// Cross-file imports — the fold-level half: how one
// document's fold consumes values its source-graph ancestors provide.
// The graph half lives with the workspace index.

/// OSVVM's `StartUpShared.tcl` shape: the reader's only writes of the
/// imported name are if-guarded (invisible), an unguarded `variable` in
/// the same namespace chains off the imported value by bare reference,
/// and a `source` folds through the qualified one.
#[test]
fn osvvm_reader_chains_off_an_imported_namespace_constant() {
    let imported: HashMap<String, String> = [(
        "::osvvm::OsvvmScriptDirectory".to_owned(),
        "/scripts".to_owned(),
    )]
    .into_iter()
    .collect();
    let src = "namespace eval ::osvvm {\n\
                   \x20   if {![info exists OsvvmScriptDirectory]} {\n\
                   \x20       variable OsvvmScriptDirectory ${SCRIPT_DIR}\n\
                   \x20   }\n\
                   \x20   variable OsvvmHomeDirectory [file normalize ${OsvvmScriptDirectory}/..]\n\
                   }\n";
    let merged = fold_constant_assignments_with_imports(
        &constant_path_assignments(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        ),
        Some("/scripts/StartUpShared.tcl"),
        &imported,
    );
    assert_eq!(
        merged
            .get("::osvvm::OsvvmHomeDirectory")
            .map(String::as_str),
        Some("/"),
        "the in-namespace bare read must chain off the import: {merged:?}",
    );
    // The import itself survives into the merged view for the reader's
    // own `source ${::osvvm::OsvvmScriptDirectory}/x.tcl` lines.
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "${::osvvm::OsvvmScriptDirectory}/OsvvmScriptsCore.tcl",
            Some("/scripts/StartUpShared.tcl"),
            &merged,
            None,
        )
        .as_deref(),
        Some("/scripts/OsvvmScriptsCore.tcl"),
    );
}

/// A name the reader itself writes never answers from the import: a
/// foldable single write replaces it, an unfoldable one removes it —
/// the runtime overwrote the inherited value with something this tier
/// cannot know.
#[test]
fn an_own_write_shadows_or_removes_the_import() {
    let imported: HashMap<String, String> = [("dir".to_owned(), "/inherited".to_owned())]
        .into_iter()
        .collect();
    let replaced = fold_constant_assignments_with_imports(
        &constant_path_assignments(
            "set dir /own\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        ),
        None,
        &imported,
    );
    assert_eq!(replaced.get("dir").map(String::as_str), Some("/own"));
    let removed = fold_constant_assignments_with_imports(
        &constant_path_assignments(
            "set dir [pwd]\n",
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        ),
        None,
        &imported,
    );
    assert!(
        !removed.contains_key("dir"),
        "an unfoldable own write must remove the import, not defer to it: {removed:?}",
    );
}

/// alited's cross-target write: `set ::e_menu_dir [file join $LIBDIR
/// e_menu]` **targets** the global but **executes** inside `namespace
/// eval alited`, so its `$LIBDIR` reads `::alited::LIBDIR` — the write's
/// execution namespace is a recorded fact of its own, not derivable from
/// the target key.  A later `variable PAVEDIR [file join $::e_menu_dir
/// src]` then reads the global back by qualified name.
#[test]
fn alited_global_target_written_from_namespace_body_folds() {
    let src = "namespace eval alited {\n\
                   \x20   variable LIBDIR [file dirname [info script]]\n\
                   \x20   set ::e_menu_dir [file join $LIBDIR e_menu]\n\
                   \x20   variable PAVEDIR [file join $::e_menu_dir src]\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/app/alited.tcl"),
    );
    assert_eq!(
        constants
            .lookup_at("::e_menu_dir", u32::try_from(src.len()).unwrap())
            .as_deref(),
        Some("/app/e_menu"),
        "{constants:?}",
    );
    assert_eq!(
        constants.get("::alited::PAVEDIR").map(String::as_str),
        Some("/app/e_menu/src"),
        "{constants:?}",
    );
}

/// A read-modify-write (`append`) poisons its target: the value after it
/// is not the `set`'s.  Recognised through the registry's
/// [`tcl_registry::ArgRole::VarWrite`] roles, so every mutating command
/// counts, not a hand-listed few.
#[test]
fn a_read_modify_write_poisons_the_constant() {
    for src in [
        "set dir /a\nappend dir /b\n",
        "set dir /a\nlappend dir x\n",
        "set dir /a\nset other [file join $dir y]\nincr dir\n",
    ] {
        let constants = constant_path_vars(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            None,
        );
        assert!(
            !constants.contains_key("dir"),
            "a mutated name must not fold to its first value: {src:?} -> {constants:?}",
        );
    }
    // And nothing computed *from* it folds either.
    let constants = constant_path_vars(
        "set dir /a\nset sub [file join $dir s]\nappend dir /b\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("sub"), "{constants:?}");
}

#[test]
fn a_scope_alias_poisons_every_bound_name() {
    let constants = constant_path_vars(
        "set dir /a\nupvar 1 elsewhere dir\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("dir"), "{constants:?}");
}

/// A command the scan cannot read substitutes its words before it runs, so
/// its own operand still reads the constant; a nested command substitution
/// runs first and may write it.
///
/// tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0 print `source /s/x` for
/// `set dir /s; puts [list source $dir/x]`.
#[test]
fn an_unread_command_reads_the_constant_in_its_own_words() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let source = "set dir /s\n$loader $dir/shared.tcl\n";
    let constants = constant_path_vars(source, profile, None);
    let operand = u32::try_from(source.find("$dir").unwrap()).unwrap();
    assert_eq!(constants.lookup_at("dir", operand).as_deref(), Some("/s"));
    assert_eq!(
        constants.lookup_at("dir", u32::try_from(source.len()).unwrap()),
        None
    );
    let source = "set dir /s\n$loader [unknown_writer]$dir/shared.tcl\n";
    let constants = constant_path_vars(source, profile, None);
    let operand = u32::try_from(source.find("$dir").unwrap()).unwrap();
    assert_eq!(constants.lookup_at("dir", operand), None);
}

#[test]
fn a_scope_alias_withdraws_subsequent_lookups_and_exports() {
    let source = "set dir /a\nupvar 1 elsewhere dir\n";
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let constants = constant_path_vars(source, profile, None);
    let before_alias = u32::try_from(source.find("upvar").unwrap() - 1).unwrap();
    assert_eq!(
        constants.lookup_at("dir", before_alias).as_deref(),
        Some("/a")
    );
    assert_eq!(
        constants.lookup_at("dir", u32::try_from(source.len()).unwrap()),
        None
    );
    assert!(!constants.contains_key("dir"), "{constants:?}");
    assert!(constants.get("dir").is_none());
    assert!(constants.is_empty());
    assert_eq!(constants.len(), 0);
    assert_eq!(constants.iter().count(), 0);
    assert_eq!((&constants).into_iter().count(), 0);
    assert!(std::panic::catch_unwind(|| &constants["dir"]).is_err());
    assert!(constants.exported().is_empty());
    let next = constant_path_assignments("set inherited $dir", profile);
    let imported = fold_constant_assignments_with_imports(&next, None, &constants);
    assert!(imported.is_empty(), "{imported:?}");
}

/// A braced value is data: Tcl suppresses substitution when it is
/// assigned, so `$root` inside it is five characters, not a reference.
#[test]
fn a_braced_value_is_literal_data() {
    let constants = constant_path_vars(
        "set root /a\nset dir {$root}\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(constants.get("dir").map(String::as_str), Some("$root"));
    assert_eq!(
        evaluate_auto_path_expr_with_constants("[file join $dir x.tcl]", None, &constants, None,)
            .as_deref(),
        Some("$root/x.tcl"),
        "the folded target is the literal directory Tcl would use",
    );
    // The quoted twin *does* substitute — the distinction is real.
    let quoted = constant_path_vars(
        "set root /a\nset dir \"$root\"\n",
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(quoted.get("dir").map(String::as_str), Some("/a"));
}

/// A relative qualified reference (`$demo::dir`, no leading `::`) read
/// from the global context reaches the same key.
#[test]
fn a_relative_qualified_reference_resolves() {
    let src = "namespace eval demo {\n    variable dir [file dirname [info script]]\n}\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/ex/config.tcl"),
    );
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "[file join $demo::dir config.tcl]",
            Some("/ex/config.tcl"),
            &constants,
            None,
        )
        .as_deref(),
        Some("/ex/config.tcl"),
    );
}

/// Tk ttk's shape must keep abstaining: the declaration marks the name,
/// but the only `set` hides behind `if {![info exists library]}` — a
/// braced body this tier deliberately does not see — so the value is
/// statically unknown and `$::ttk::library` folds to nothing.
#[test]
fn ttk_if_guarded_set_still_abstains() {
    let src = "namespace eval ttk {\n\
                   \x20   variable library\n\
                   \x20   if {![info exists library]} {\n\
                   \x20       set library [file dirname [info script]]\n\
                   \x20   }\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/tk/ttk/ttk.tcl"),
    );
    assert!(!constants.contains_key("::ttk::library"), "{constants:?}");
    assert_eq!(
        evaluate_auto_path_expr_with_constants(
            "[file join $::ttk::library fonts.tcl]",
            Some("/tk/ttk/ttk.tcl"),
            &constants,
            None,
        ),
        None,
    );
}

/// A declared namespace name also *blocks* a bare in-body read from
/// falling through to an unrelated global of the same name — Tcl reads
/// the namespace's variable, whose value we do not know.
#[test]
fn a_declared_namespace_name_shadows_the_global_for_body_reads() {
    let src = "set library /wrong/global\n\
                   namespace eval ttk {\n\
                   \x20   variable library\n\
                   \x20   variable sub [file join $library themes]\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(
        !constants.contains_key("::ttk::sub"),
        "a bare read of a declared-but-valueless namespace name must \
             abstain, not bind the global: {constants:?}",
    );
}

/// An unattributable body `set` — no prior declaration in the namespace,
/// no prior top-level global — poisons both candidate names: Tcl's own
/// answer depends on whether some other file created the global first.
#[test]
fn an_unattributable_body_set_poisons_both_candidates() {
    let src = "namespace eval foo {\n    set dir /opt/foo\n}\n\
                   set dir /opt/global\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("::foo::dir"), "{constants:?}");
    assert!(
        !constants.contains_key("dir"),
        "the later top-level set is the global's second possible write: {constants:?}",
    );
}

/// A body `set` of a name the top level already wrote goes to the
/// existing global (Tcl's write fallback) — making it a second write,
/// which poisons the global.
#[test]
fn a_body_set_of_an_existing_global_counts_as_its_write() {
    let src = "set dir /opt/first\n\
                   namespace eval foo {\n    set dir /opt/second\n}\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert!(!constants.contains_key("dir"), "{constants:?}");
    assert!(!constants.contains_key("::foo::dir"), "{constants:?}");
}

/// `variable` pairs: value'd names assign, the trailing lone name only
/// declares.
#[test]
fn variable_pairs_assign_and_a_trailing_name_declares() {
    let src = "namespace eval cfg {\n\
                   \x20   variable base /opt/app sub lib pending\n\
                   }\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(
        constants.get("::cfg::base").map(String::as_str),
        Some("/opt/app")
    );
    assert_eq!(constants.get("::cfg::sub").map(String::as_str), Some("lib"));
    assert!(!constants.contains_key("::cfg::pending"));
}

/// An absolute-qualified `set ::ns::name` at the top level (ntext's
/// shape) records under its canonical key.
#[test]
fn an_absolute_qualified_top_level_set_records() {
    let src = "set ::mypkg::dir /opt/mypkg\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        None,
    );
    assert_eq!(
        constants.get("::mypkg::dir").map(String::as_str),
        Some("/opt/mypkg"),
    );
}

#[test]
fn unsupported_command_is_none() {
    assert_eq!(
        evaluate_auto_path_expr("[lsort {a b}]", Some("/a/b.tcl")),
        None,
    );
}

#[test]
fn tilde_expands_to_home() {
    // `temp_env` scopes the `$HOME` mutation so this crate stays unsafe-free
    // (`set_var` is `unsafe` on edition 2024).
    temp_env::with_var("HOME", Some("/home/tester"), || {
        assert_eq!(
            evaluate_auto_path_expr("~/lib", None).as_deref(),
            Some("/home/tester/lib"),
        );
    });
}

#[test]
fn empty_is_none() {
    assert_eq!(evaluate_auto_path_expr("   ", None), None);
}

/// TP — the raw text the analyser records for a
/// `lappend auto_path …` argument is exactly what this evaluator folds, so
/// the package database can consume `AnalysisResult::auto_path_entries`
/// An `auto_path` entry naming a chained constant contributes its
/// directory — a package's own registration shape:
///
/// ```tcl
/// set dir [file dirname [file normalize [info script]]]
/// set libDir [file join $dir lib]
/// lappend auto_path $libDir
/// ```
#[test]
fn an_auto_path_entry_through_a_chained_constant_contributes_its_directory() {
    let src = "set dir [file dirname [file normalize [info script]]]\n\
                   set libDir [file join $dir lib]\n\
                   lappend auto_path $libDir\n";
    let constants = constant_path_vars(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        Some("/proj/SpiceGenTcl.tcl"),
    );
    let entry = AutoPathEntry {
        raw_path: "$libDir".to_owned(),
        range: tcl_lexer::Span::new(0, 0),
        form: AutoPathForm::Append,
    };
    assert_eq!(
        evaluate_auto_path_entry_with_constants(
            &entry,
            Some("/proj/SpiceGenTcl.tcl"),
            &constants,
            None,
        ),
        vec!["/proj/lib".to_owned()],
    );
    // The constant-less form still contributes nothing — the wiring is
    // opt-in per consumer, never a change to the bare entry point.
    assert_eq!(
        evaluate_auto_path_entry(&entry, Some("/proj/SpiceGenTcl.tcl"), None),
        [] as [std::string::String; 0]
    );
}

/// directly.  Pins the *contract between the two*, which is what the
/// workspace scan's `extend_resolver_with_document_auto_paths` relies on.
#[test]
fn recorded_auto_path_entries_round_trip_through_the_evaluator() {
    let src = "lappend auto_path [file dirname [file dirname [info script]]]\n\
                   lappend auto_path $someVar\n";
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    let folded: Vec<Option<String>> = r
        .auto_path_entries
        .iter()
        .map(|e| evaluate_auto_path_expr(&e.raw_path, Some("/a/b/c/user.tcl")))
        .collect();
    assert_eq!(
        folded,
        vec![Some("/a/b".to_owned()), None],
        "the computed idiom folds; a `$var` entry abstains: {:?}",
        r.auto_path_entries,
    );
}

// Slash-form path arithmetic.
//
// Pure string math, so the Windows shapes are exercised on any host — the
// evaluator has no `cfg(windows)` branch to skip.

#[test]
fn dirname_join_normalise_are_root_aware() {
    // POSIX, unchanged.
    assert_eq!(path_dirname("/a/b/c"), "/a/b");
    assert_eq!(path_dirname("/a"), "/");
    assert_eq!(path_dirname("/"), "/");
    // A rootless single component: `.`, exactly as tclsh 8.6.16 / 9.0.4
    // answer `file dirname a`.  Returning the empty string instead would
    // have to be rescued by absolutising against the process's own working
    // directory, which is not something a language server may assume.
    assert_eq!(path_dirname("a"), ".");
    assert_eq!(path_join(&["/a/b".into(), "..".into()]), "/a/b/..");
    assert_eq!(path_join(&["/a".into(), "/b".into()]), "/b");
    assert_eq!(normpath("/a/b/../c"), "/a/c");
    assert_eq!(normpath("/a/./b"), "/a/b");
    // `..` cannot climb out of the POSIX root.
    assert_eq!(normpath("/a/../.."), "/");

    // Windows drive.  `file dirname C:/x` is `C:/`, never the
    // drive-*relative* `C:`.
    assert_eq!(path_dirname("C:/x/y"), "C:/x");
    assert_eq!(path_dirname("C:/x"), "C:/");
    assert_eq!(path_dirname("C:/"), "C:/");
    assert_eq!(path_join(&["C:/a".into(), "b".into()]), "C:/a/b");
    assert_eq!(path_join(&["/a".into(), "C:/b".into()]), "C:/b");
    assert_eq!(normpath("C:/a/../b"), "C:/b");
    assert_eq!(normpath("C:/a/../.."), "C:/");

    // UNC: the root spans host *and* share, and `..` stops there.
    assert_eq!(path_dirname("//server/share/dir"), "//server/share");
    assert_eq!(path_dirname("//server/share"), "//server/share");
    assert_eq!(
        path_join(&["//server/share".into(), "d".into()]),
        "//server/share/d"
    );
    assert_eq!(normpath("//server/share/a/../b"), "//server/share/b");
    assert_eq!(normpath("//server/share/a/../.."), "//server/share");
}

/// `filename(n)`: on Windows a leading `/` is *volumerelative*, not
/// absolute, so joining it onto a volume-rooted path keeps the volume.
#[test]
fn join_treats_a_leading_slash_as_volume_relative() {
    assert_eq!(path_join(&["C:/a/b".into(), "/x".into()]), "C:/x");
    assert_eq!(
        path_join(&["//server/share/a".into(), "/x".into()]),
        "//server/share/x",
    );
    // Between two POSIX paths the ordinary "absolute component wins" rule
    // still applies.
    assert_eq!(path_join(&["/a/b".into(), "/x".into()]), "/x");
}

#[test]
fn native_windows_separators_become_slash_form() {
    assert_eq!(to_tcl_slash_form("C:\\repo\\user.tcl"), "C:/repo/user.tcl");
    assert_eq!(
        to_tcl_slash_form("\\\\server\\share\\lib"),
        "//server/share/lib",
    );
    // TN: on POSIX a backslash is an ordinary filename character and must
    // survive untouched.
    assert_eq!(to_tcl_slash_form("/tmp/a\\b"), "/tmp/a\\b");
    assert_eq!(to_tcl_slash_form("/proj/lib"), "/proj/lib");
}

/// Path conversion: `Uri::to_file_path` hands back a native Windows path, and
/// the `[file dirname [info script]]` idiom must resolve against *it*, not
/// silently fall through to the process working directory.
#[test]
fn info_script_folds_a_native_windows_path() {
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [info script]]",
            Some("C:\\repo\\pkg\\init.tcl"),
        )
        .as_deref(),
        Some("C:/repo/pkg"),
    );
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [file dirname [info script]]]",
            Some("C:\\repo\\pkg\\init.tcl"),
        )
        .as_deref(),
        Some("C:/repo"),
    );
    assert_eq!(
        evaluate_auto_path_expr(
            "[file join [file dirname [info script]] ..]",
            Some("C:/repo/pkg/init.tcl"),
        )
        .as_deref(),
        Some("C:/repo"),
    );
    assert_eq!(
        evaluate_auto_path_expr(
            "[file dirname [info script]]",
            Some("\\\\server\\share\\pkg\\init.tcl"),
        )
        .as_deref(),
        Some("//server/share/pkg"),
    );
    // A literal written with native separators folds too.
    assert_eq!(
        evaluate_auto_path_expr("C:\\lib\\tcl", None).as_deref(),
        Some("C:/lib/tcl"),
    );
}

/// TN: a drive-*relative* path names a per-drive working directory a static
/// analysis cannot know, so it abstains rather than resolving against the
/// process cwd.
#[test]
fn drive_relative_paths_abstain() {
    assert_eq!(evaluate_auto_path_expr("C:lib", None), None);
    assert_eq!(evaluate_auto_path_expr("C:", None), None);
    assert_eq!(
        evaluate_auto_path_expr("[file dirname [info script]]", Some("C:user.tcl")),
        None,
    );
}

// `set` vs `lappend` arity.

/// Fold every entry a source records, in order.
fn fold_all(src: &str, info_script: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse(src, "tcl9.0");
    r.auto_path_entries
        .iter()
        .flat_map(|e| evaluate_auto_path_entry(e, Some(info_script), None))
        .collect()
}

/// TP — `set auto_path {A B}` assigns a **list**, so both directories are
/// searched.  On `tclsh8.6` / `tclsh9.0`: `set auto_path {/opt/p1
/// /opt/p2}; llength $auto_path` → 2, and `package require` finds a package
/// under either.
#[test]
fn set_auto_path_yields_one_dir_per_list_element() {
    assert_eq!(
        fold_all("set auto_path {/opt/p1 /opt/p2}\n", "/w/user.tcl"),
        vec!["/opt/p1".to_owned(), "/opt/p2".to_owned()],
    );
    assert_eq!(
        fold_all("set auto_path \"/opt/x /opt/y\"\n", "/w/user.tcl"),
        vec!["/opt/x".to_owned(), "/opt/y".to_owned()],
    );
}

/// TN — a brace-quoted element containing a space is **one** directory
/// (`llength {/opt/a {/opt/with space} /opt/b}` → 3 on tclsh), and
/// so is every `lappend` word however many spaces it holds.
#[test]
fn braced_elements_and_lappend_words_stay_one_directory_each() {
    assert_eq!(
        fold_all(
            "set auto_path {/opt/a {/opt/with space} /opt/b}\n",
            "/w/user.tcl",
        ),
        vec![
            "/opt/a".to_owned(),
            "/opt/with space".to_owned(),
            "/opt/b".to_owned(),
        ],
    );
    // `lappend auto_path {p q}` appends ONE element — never list-split.
    assert_eq!(
        fold_all("lappend auto_path {/opt/p q}\n", "/w/user.tcl"),
        vec!["/opt/p q".to_owned()],
    );
    // …while `lappend auto_path a b` is already two words, hence two dirs.
    assert_eq!(
        fold_all("lappend auto_path /opt/a /opt/b\n", "/w/user.tcl"),
        vec!["/opt/a".to_owned(), "/opt/b".to_owned()],
    );
}

/// A `set` right-hand side carrying a substitution is *not* a literal list:
/// `set auto_path [file dirname [info script]]` is one computed directory,
/// and `[list …]` is outside the subset entirely.
#[test]
fn a_computed_set_right_hand_side_is_not_list_split() {
    assert_eq!(
        fold_all(
            "set auto_path [file dirname [info script]]\n",
            "/proj/lib/user.tcl",
        ),
        vec!["/proj/lib".to_owned()],
    );
    assert_eq!(
        fold_all("set auto_path [list /a /b]\n", "/w/user.tcl").len(),
        0,
        "`[list …]` is outside the supported subset — abstain, never \
             split its text as if it were a list literal",
    );
    assert_eq!(
        fold_all("set auto_path $env(TCLLIBPATH)\n", "/w/user.tcl"),
        [] as [std::string::String; 0]
    );
}
