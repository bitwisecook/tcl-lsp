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

//! The general diagnostics: arity, control flow, dialect gating of `when`, numeral releases and a leading byte-order mark.

use super::*;

#[test]
fn numeral_release_diagnostic_tracks_profile() {
    assert!(fires("set x 0d5; set y 1_000", "tcl8.4", "W148"));
    assert!(fires("set x 0d5; set y 1_000", "tcl8.6", "W148"));
    assert!(!fires("set x 0d5; set y 1_000", "tcl9.0", "W148"));
    assert!(!fires("set x 010", "tcl8.6", "W148"));
    assert!(!fires("set x 010", "tcl9.0", "W148"));
}

// Arity (E001/E002/E003), unknown subcommand (W001),
// read-before-set (W210), and the constant-branch family (I230).
pub(super) mod diagnostics {
    use super::super::*;

    // Builtin arity (E001/E002/E003), all tclsh-observable `wrong # args`.

    #[test]
    fn too_few_args_set_fires_e002() {
        // tclsh8.6/9.0: `set` → wrong # args: should be "set varName ?newValue?".
        let ds = analyser_diags("set", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert_ne!(
            errs,
            [] as [&(
                std::string::String,
                std::string::String,
                tcl_compiler::analyser::Severity
            ); 0]
        );
        assert!(errs.iter().any(|(_, m, _)| m.contains("Too few")));
    }

    #[test]
    fn too_many_args_break_fires_e003() {
        // tclsh8.6/9.0: `break extra` → wrong # args: should be "break".
        let ds = analyser_diags("break extra", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert_ne!(
            errs,
            [] as [&(
                std::string::String,
                std::string::String,
                tcl_compiler::analyser::Severity
            ); 0]
        );
        assert!(errs.iter().any(|(_, m, _)| m.contains("Too many")));
    }

    #[test]
    fn correct_arity_no_error() {
        // `set x 42` is valid in tclsh; no Error-severity diagnostic.
        let ds = analyser_diags("set x 42", D);
        assert!(!ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn puts_arity_options_not_counted() {
        // `puts -nonewline stderr hello` is valid (the switch is not a
        // positional); `puts a b c` is over-arity. tclsh agrees.
        assert!(!fires("puts -nonewline stderr hello", D, "E003"));
        assert!(fires("puts a b c", D, "E003"));
    }

    #[test]
    fn declared_switches_not_counted_as_positional() {
        // Declared option flags are skipped before counting.
        // These regsub switches exist in every supported dialect.
        for snippet in [
            "regsub -all -line {\\n} $args {} str",
            "regsub -all {a} $b {} c",
            "regsub -nocase -all -- $pat $s {} out",
        ] {
            assert!(
                !fires(snippet, D, "E003"),
                "unexpected E003 for {snippet:?}"
            );
        }
        // Genuine over-arity (5 positional, max 4) still fires.
        // tclsh: `regsub a b c d e` → wrong # args.
        assert!(fires("regsub a b c d e", D, "E003"));
    }

    #[test]
    fn while_too_few_args_is_error() {
        let ds = analyser_diags("while {1}", D);
        assert!(ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn for_too_few_args_is_error() {
        let ds = analyser_diags("for {set i 0} {$i < 10}", D);
        assert!(ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    #[test]
    fn missing_subcommand_namespace_fires_e001() {
        // tclsh: bare `namespace` → wrong # args: should be "namespace subcommand …".
        let ds = analyser_diags("namespace", D);
        let errs: Vec<_> = ds
            .iter()
            .filter(|(_, _, s)| *s == Severity::Error)
            .collect();
        assert_ne!(
            errs,
            [] as [&(
                std::string::String,
                std::string::String,
                tcl_compiler::analyser::Severity
            ); 0]
        );
        assert!(
            errs.iter()
                .any(|(_, m, _)| m.to_lowercase().contains("subcommand"))
        );
    }

    #[test]
    fn unknown_command_not_arity_checked() {
        // Unknown user commands get no arity error (W123 only, not E00x).
        let ds = analyser_diags("mycommand a b c d e", D);
        assert!(!ds.iter().any(|(_, _, s)| *s == Severity::Error));
    }

    // Unknown subcommand (W001), tclsh-observable `unknown … subcommand`.

    #[test]
    fn unknown_subcommand_warns_w001() {
        // tclsh: `string bogus hello` → unknown or ambiguous subcommand "bogus".
        let ds = analyser_diags("string bogus hello", D);
        assert!(
            ds.iter()
                .any(|(c, m, _)| c == "W001" && m.contains("Unknown subcommand"))
        );
    }

    #[test]
    fn widget_creation_pathname_is_not_a_subcommand_no_w001() {
        // A widget-creation command's first word is a window pathname
        // (`.e`), not a subcommand — even though the creator carries an
        // instance-command `subcommands` table. It must never trip W001.
        for src in [
            "entry .e -textvariable v",
            "canvas .c -background white",
            "menu .m -tearoff 0",
            "text .t -width 40",
            "listbox .lb -listvariable items",
            "ttk::treeview .tv",
            "ttk::notebook .nb",
        ] {
            assert!(
                !fires(src, D, "W001"),
                "widget creation `{src}` must not fire W001",
            );
        }
        // A genuine mistyped subcommand (no leading `.`) still fires.
        assert!(fires("string bogus hello", D, "W001"));
    }

    #[test]
    fn package_prefer_is_a_real_subcommand_no_w001() {
        // `package prefer` is real in every supported dialect — tclsh returns
        // "stable" and it must not be flagged "Unknown subcommand".
        let ds = analyser_diags("package prefer", D);
        assert!(!ds.iter().any(|(_, m, _)| m.contains("Unknown subcommand")));
    }

    #[test]
    fn package_files_is_disabled_under_tcl86_per_tclsh() {
        // Under tcl8.6 `package files` is NOT a subcommand — tclsh8.6 errors
        //   `bad option "files": must be forget, ifneeded, names, …`
        // (tclsh9.0 added `files`; this test pins 8.6.)  Because `files` *is* a
        // real subcommand in 9.0, this is W002 ("disabled in the active dialect
        // profile"), not W001 ("Unknown subcommand") which is reserved for a
        // name that exists in no dialect.
        assert!(fires("package files mypackage", D, "W002"));
        assert!(!fires("package files mypackage", D, "W001"));
    }

    #[test]
    fn zipfs_is_a_9_0_ensemble_gated_out_of_earlier_dialects() {
        // `zipfs` ships in Tcl 9.0 (TIP 430); the 8.x profiles have no such
        // command.  Under 9.0 a real subcommand resolves cleanly, and a bogus
        // one is W001 (unknown subcommand) — proving the subcommand set is
        // modelled.  Under 8.6 the whole command is dialect-gated, so it is
        // W002 ("disabled in the active dialect profile"), which points the
        // user at the version rather than a bare "unknown command".
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W123"));
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W001"));
        assert!(!fires("zipfs mount archive.zip /mnt", "tcl9.0", "W002"));
        assert!(fires("zipfs bogus x", "tcl9.0", "W001"));
        assert!(fires("zipfs mount archive.zip /mnt", "tcl8.6", "W002"));
    }

    #[test]
    fn namespace_which_command_probe_does_not_flag_unknown() {
        // `namespace which -command foo` is an existence PROBE — it returns ""
        // for an unknown command rather than failing, so probing a name no
        // command defines must NOT draw W123.  (Navigation to a command that
        // *does* exist still works; that's a separate reference.)
        let src = "namespace which -command no_such_command_xyz\n";
        assert!(
            !fires(src, D, "W123"),
            "namespace which probe must not W123: {:?}",
            analyser_diags(src, D),
        );
    }

    #[test]
    fn proc_shadowing_ensemble_command_suppresses_w001() {
        // tclsh8.6: `proc string {op args} {...}` completely replaces the
        // builtin `string` ensemble at the call site — `string reverse x`
        // dispatches to the user proc, not the registry subcommand set, so
        // it must not be flagged "Unknown subcommand".  See FP-STY-17.
        let src = "proc string {op args} { return $op }\nstring reverse x\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn ensemble_shadowing_does_not_hide_a_genuinely_unknown_subcommand() {
        // The proc-shadow suppression above is scoped to `string` only — a
        // different, unshadowed ensemble in the same file still fires.
        let src = "proc string {op args} { return $op }\ninfo bogus\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dynamically_mapped_dict_ensemble_subcommand_suppresses_w001() {
        // FP: the real tcllib `dicttool.tcl` idiom patches the `dict`
        // ensemble's `-map` at
        // runtime to add `getnull` (`namespace ensemble configure dict -map
        // [dict replace [namespace ensemble configure dict -map] getnull
        // ::tcl::dict::getnull]`), which a static `SUBCOMMANDS` table can't
        // reflect. Must not fire "Unknown subcommand 'getnull' for 'dict'"
        // once the backing proc exists at `::tcl::dict::getnull` — hover
        // and definition already resolve the same call site correctly, and
        // W001 must agree with them.
        let src = "proc ::tcl::dict::getnull {dictionary args} {\n\
             if {[exists $dictionary {*}$args]} { get $dictionary {*}$args }\n\
             }\n\
             namespace ensemble configure dict -map \
             [dict replace [namespace ensemble configure dict -map] \
             getnull ::tcl::dict::getnull]\n\
             proc demo {} {\n\
             set clay [dict create a 1 b 2]\n\
             return [dict getnull $clay a]\n\
             }\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dynamically_mapped_dict_ensemble_fix_is_not_overfit_to_getnull() {
        // FP — a second, differently-named sibling from the same real
        // tcllib idiom (`is_dict`), proving the fix isn't hardcoded to one
        // subcommand name. This block never calls bare `exists`/`get`, so it
        // exercises Part B in isolation from Part A.
        let src = "proc ::tcl::dict::is_dict {d} {\n\
             if {[catch {dict size $d} err]} { return 0 }\n\
             return 1\n\
             }\n\
             namespace ensemble configure dict -map \
             [dict replace [namespace ensemble configure dict -map] \
             is_dict ::tcl::dict::is_dict]\n\
             dict is_dict [dict create a 1]\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn genuinely_unknown_dict_subcommand_still_fires_w001() {
        // TP — regression guard: a real, non-existent `dict` subcommand
        // (no proc named `zzzznotreal` anywhere, no ensemble patch) must
        // still fire — proves `dynamic_ensemble_subcommand_known` doesn't
        // over-suppress.
        let src = "proc demo {} {\n\
             set d [dict create a 1 b 2]\n\
             return [dict zzzznotreal $d a]\n\
             }\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn dict_proc_at_conventional_location_without_ensemble_patch_is_an_accepted_false_negative() {
        // FN — deliberately ACCEPTED gap (the primary/simple design, chosen
        // over the more precise "observed an actual
        // `namespace ensemble configure -map` call" variant): a proc
        // defined at `::tcl::dict::<name>` is treated as "this ensemble
        // subcommand is known" even when the ensemble's `-map` was never
        // actually reconfigured to include it. Real tclsh would still error
        // `unknown or ambiguous subcommand "stray"` here. If this gap is
        // ever closed (tracking whether `-map` was truly reconfigured),
        // this test's assertion flips and documents the improvement.
        // The gap is closed: tclsh 8.5.19 / 8.6.18 / 9.0.4 raise `unknown or
        // ambiguous subcommand "stray"`, and W001 now says so.
        let src = "proc ::tcl::dict::stray {d} { return $d }\n\
             dict stray [dict create a 1]\n";
        assert!(
            fires(src, D, "W001"),
            "a proc beside the ensemble is not one of its subcommands; got {:?}",
            analyser_diags(src, D),
        );
    }

    #[test]
    fn namespace_ensemble_configure_on_tk_suppresses_w001_for_systray_and_sysnotify() {
        // FP: the real `tk/library/systray.tcl` (and `print.tcl`, `fileicon.tcl`,
        // `accessibility.tcl`) idiom splices `systray`/`sysnotify` into the
        // *pre-existing, registry-builtin* `tk` ensemble via `namespace
        // ensemble configure tk -map [dict merge [namespace ensemble
        // configure tk -map] {systray ::tk::systray sysnotify
        // ::tk::sysnotify::sysnotify}]` — a `CONFIGURE`, not `CREATE`, on an
        // ensemble this file never itself created. tclsh9.0/8.6 both
        // confirm `tk systray create`/`tk sysnotify ...` are correct,
        // documented calls; must not fire "Unknown subcommand".
        let src = "proc ::tk::systray {args} {}\n\
             proc ::tk::sysnotify::sysnotify {a b} {}\n\
             namespace ensemble configure tk -map \
             [dict merge [namespace ensemble configure tk -map] \
             {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]\n\
             tk systray create -image book\n\
             tk sysnotify Alert message\n";
        assert!(!fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn namespace_ensemble_configure_genuinely_unknown_tk_subcommand_still_fires_w001() {
        // TP — regression guard: a real, non-existent `tk` subcommand (no
        // splice recorded for it) must still fire — proves
        // `statically_mapped_ensemble_subcommand_known` doesn't over-suppress
        // the whole `tk` ensemble once *any* subcommand has been spliced in.
        let src = "namespace ensemble configure tk -map \
             [dict merge [namespace ensemble configure tk -map] \
             {systray ::tk::systray}]\n\
             tk zzznotreal\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn ensemble_without_implementation_namespace_is_unaffected_by_dynamic_map_check() {
        // TN — regression guard: `dynamic_ensemble_subcommand_known` is
        // inert for every ensemble whose `CommandSpec` doesn't set
        // `implementation_namespace` (only `dict`, currently) — a
        // same-named proc existing elsewhere must not suppress an unrelated
        // ensemble's genuinely unknown subcommand.
        let src = "proc bogus {} {}\nstring bogus hello\n";
        assert!(fires(src, D, "W001"), "got {:?}", analyser_diags(src, D));
    }

    #[test]
    fn error_diagnostics_have_a_span() {
        // The error anchors at the start of the script. The diagnostic carries
        // a byte `span`; assert it starts at offset 0 (the `set` token).
        let r = Analyser::new().analyse("set", D);
        assert_ne!(r.diagnostics, [] as [tcl_compiler::analyser::Diagnostic; 0]);
        assert_eq!(r.diagnostics[0].span.start(), 0);
    }

    #[test]
    fn multiline_diagnostic_anchors_on_offending_line() {
        // `break extra` sits on line 1; its span starts after the first line's
        // newline. We check the byte offset lands within the second line.
        let src = "set x 1\nbreak extra\nset y 2";
        let r = Analyser::new().analyse(src, D);
        let err = r
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error)
            .expect("an error");
        let line_start = src.find("break").unwrap();
        assert!(
            (err.span.start() as usize) >= line_start,
            "error span should be on the `break` line"
        );
    }

    // Read-before-set (W210), tclsh `can't read "x"`.

    #[test]
    fn read_before_set_warns_w210() {
        // tclsh8.6/9.0: `puts $x` → can't read "x": no such variable.
        assert_eq!(count("puts $x", D, "W210"), 1);
        let ds = analyser_diags("puts $x", D);
        assert!(
            ds.iter()
                .any(|(c, _, s)| c == "W210" && *s == Severity::Warning)
        );
    }

    #[test]
    fn read_before_set_in_expr_warns_w210() {
        // tclsh: `if {$x > 0} …` with x unset → can't read "x".
        assert_eq!(count("if {$x > 0} { puts yes }", D, "W210"), 1);
    }

    #[test]
    fn read_before_set_cleared_after_assignment() {
        assert!(!fires("set x 1\nputs $x", D, "W210"));
    }

    #[test]
    fn array_read_uses_base_variable_no_w210() {
        assert!(!fires("set arr(key) 1\nputs $arr(key)", D, "W210"));
    }

    // Output-var writers suppress W210 on the written var.
    // (regexp/scan/lassign write their target vars; reading them is safe.)

    #[test]
    fn regexp_capture_vars_not_read_before_set() {
        // The `email` read still fires (it is genuinely unset), but the capture
        // targets `user`/`domain` do not.
        assert_eq!(
            w210_for(
                "regexp {^(\\w+)@(\\w+)$} $email -> user domain\nputs $user",
                "user"
            ),
            0
        );
    }

    #[test]
    fn regexp_match_var_not_read_before_set() {
        assert_eq!(
            w210_for("regexp {\\d+} $text match\nputs $match", "match"),
            0
        );
    }

    #[test]
    fn scan_capture_vars_not_read_before_set() {
        assert_eq!(
            w210_for("scan \"42 hello\" \"%d %s\" num word\nputs $num", "num"),
            0
        );
    }

    #[test]
    fn lassign_vars_not_read_before_set() {
        assert_eq!(w210_for("lassign {a b c} x y z\nputs \"$x $y $z\"", "x"), 0);
    }

    // `regsub … result` writes its trailing `result` variable (recognised via
    // the registry's `VarWrite` arg-role, like `set`/`lassign`/`scan`), so a
    // later `$result` read is NOT read-before-set.
    // The `$text` argument is still unset and fires its own W210 (tclsh would
    // error on that first at runtime).
    #[test]
    fn regsub_result_var_suppresses_w210_at_top_level() {
        assert_eq!(
            w210_for("regsub {old} $text new result\nputs $result", "result"),
            0
        );
        // The unset input `$text` still reports read-before-set.
        assert_eq!(
            w210_for("regsub {old} $text new result\nputs $result", "text"),
            1
        );
    }

    // Unused / dead-store / paste hints (pure static heuristics).

    #[test]
    fn unused_assigned_variable_hint_w211() {
        // Heuristic (no direct tclsh analogue): `set x 1` never read.
        let ds = analyser_diags("proc foo {} { set x 1 }", D);
        assert!(
            ds.iter()
                .any(|(c, _, s)| c == "W211" && *s == Severity::Hint)
        );
    }

    #[test]
    fn used_variable_no_unused_hint() {
        assert!(!fires("proc foo {} { set x 1; puts $x }", D, "W211"));
    }

    #[test]
    fn dead_assignment_detected_w220() {
        // Heuristic: `set x 1; set x 2; puts $x` — first store is dead.
        let ds = analyser_diags("proc foo {} { set x 1; set x 2; puts $x }", D);
        let dead: Vec<_> = ds.iter().filter(|(c, _, _)| c == "W220").collect();
        assert_eq!(dead.len(), 1);
        assert!(dead[0].1.contains('x'));
    }

    // The trace names `::g`, the top-level store is spelled
    // `g`, and both name the same global variable; the write trace observes
    // every store (tclsh prints `trace` on each `set`), so W220 must stay
    // suppressed for the unqualified spelling exactly as it already is for
    // `set ::g …`.
    #[test]
    fn traced_global_unqualified_store_suppresses_w220() {
        let unqualified = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset g 1\nset g 2\nputs $g";
        assert!(!fires(unqualified, D, "W220"));
        let qualified = "proc onw {a b c} { puts trace }\ntrace add variable ::g write ::onw\nset ::g 1\nset ::g 2\nputs $::g";
        assert!(!fires(qualified, D, "W220"));
    }

    #[test]
    fn paste_error_hint_for_duplicate_static_assignment_h300() {
        // Heuristic: repeated assignment of the *same* literal value.
        let ds = analyser_diags("proc foo {} { set x 0; set x 0; puts $x }", D);
        let h: Vec<_> = ds.iter().filter(|(c, _, _)| c == "H300").collect();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].2, Severity::Hint);
        assert!(h[0].1.contains('x'));
    }

    #[test]
    fn paste_error_not_emitted_when_value_changes_or_dynamic() {
        assert!(!fires(
            "proc foo {} { set x 0; set x 1; puts $x }",
            D,
            "H300"
        ));
        assert!(!fires(
            "proc foo {} { set x $y; set x $y; puts $x }",
            D,
            "H300"
        ));
    }

    // Constant-branch family (I230).

    #[test]
    fn constant_if_unreachable_branch_i230() {
        assert!(fires("if {1} { set x 1 } else { set y 2 }", D, "I230"));
    }

    #[test]
    fn constant_string_eq_and_double_equals_fold_i230() {
        // With x provably "foo", `$x == "foo"` / `$x eq "foo"` is always true →
        // alternate branch unreachable. tclsh: expr {"foo" == "foo"} → 1.
        for op in ["==", "eq"] {
            let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}");
            assert_eq!(count(&src, D, "I230"), 1, "expected I230 for {op:?}");
        }
    }

    #[test]
    fn constant_string_ne_and_bang_equals_fold_i230() {
        // `$x != "foo"` with x=="foo" is always false → then-branch unreachable.
        for op in ["!=", "ne"] {
            let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}");
            assert_eq!(count(&src, D, "I230"), 1, "expected I230 for {op:?}");
        }
    }

    #[test]
    fn infinite_loop_idiom_not_flagged_i230() {
        // `while 1` / `for … 1 …` are intentional infinite loops (exit via
        // break/return); a constant-true loop condition must not be I230.
        for src in [
            "proc f {} { while 1 { if {[g]} break } }",
            "proc f {} { while true { if {[g]} break } }",
            "proc f {} { for {set i 0} 1 {incr i} { if {$i > 9} break } }",
        ] {
            assert!(!fires(src, D, "I230"), "unexpected I230 for {src:?}");
        }
    }

    #[test]
    fn dead_while_zero_still_flagged() {
        // A constant-*false* loop condition means the body never runs.
        // This is reported as W240 (loop-never-executes) rather than I230;
        // the dead-loop fact is still surfaced.
        assert!(fires("proc f {} { while 0 { puts dead } }", D, "W240"));
    }

    // NOTE — no I231 for a constant switch: `switch 1 {1 {…} …}` does not fire
    // I231. The analyser + run_all_checks surface does not fold a constant
    // switch subject into an unreachable-arm diagnostic.
}

// `when` body recursion is dialect-gated (iRules-only builtin).
mod when_dialect_gating {
    use super::super::*;

    #[test]
    fn when_body_not_analysed_under_plain_tcl() {
        // Under tcl8.6 `when` is an unknown command; its braced arg is opaque
        // data, NOT a handler script — so the body command `boguscmd` must not
        // get its own W123, and no spurious W210 on the body read.
        let src = "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n";
        let ds = analyser_diags(src, D);
        assert!(
            !ds.iter()
                .any(|(c, m, _)| c == "W123" && m.contains("boguscmd")),
            "when body must not be analysed under tcl8.6"
        );
        assert!(
            !ds.iter().any(|(c, _, _)| c == "W210"),
            "no spurious W210 in opaque body"
        );
    }

    #[test]
    fn when_body_analysed_under_irules() {
        // Under f5-irules `when` IS a handler; the body is analysed and the
        // bogus inner command earns its own W123.
        let src = "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n";
        let ds = analyser_diags(src, "f5-irules");
        assert_eq!(
            ds.iter()
                .filter(|(c, m, _)| c == "W123" && m.contains("boguscmd"))
                .count(),
            1,
            "when body IS analysed under f5-irules"
        );
    }
}

// Bodies of if/while/for/foreach/dict-for are walked.
mod control_flow {
    use super::super::*;

    fn vars(src: &str) -> std::collections::HashSet<String> {
        Analyser::new()
            .analyse(src, D)
            .global_scope
            .variables
            .keys()
            .cloned()
            .collect()
    }

    #[test]
    fn if_while_for_bodies_analysed() {
        assert!(vars("if {1} { set x 42 }").contains("x"));
        assert!(vars("while {1} { set x 42; break }").contains("x"));
        let v = vars("for {set i 0} {$i < 10} {incr i} { set x $i }");
        assert!(v.contains("i") && v.contains("x"));
    }

    #[test]
    fn if_expr_records_var_reference() {
        let r = Analyser::new().analyse("set x 1\nif {$x > 0} { set y 1 }", D);
        let xv = r.global_scope.variables.get("x").expect("x defined");
        assert_ne!(xv.references, [] as [tcl_lexer::Span; 0]);
    }

    #[test]
    fn foreach_defines_loop_var() {
        assert!(vars("foreach item {a b c} { puts $item }").contains("item"));
    }

    #[test]
    fn foreach_multi_var_each_defined_with_distinct_ranges() {
        let r = Analyser::new().analyse("foreach {a b c} {1 2 3} { puts $a }", D);
        for v in ["a", "b", "c"] {
            assert!(r.global_scope.variables.contains_key(v), "{v} missing");
        }
        // Each binding gets its own span, not the whole varList token's.
        let ra = r.global_scope.variables["a"].definition_span;
        let rb = r.global_scope.variables["b"].definition_span;
        let rc = r.global_scope.variables["c"].definition_span;
        assert_ne!(ra.start(), rb.start());
        assert_ne!(rb.start(), rc.start());
    }

    #[test]
    fn if_elseif_else_all_bodies_analysed() {
        let v = vars("if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }");
        assert!(v.contains("a") && v.contains("b") && v.contains("c"));
    }

    #[test]
    fn dict_for_body_analysed() {
        // `$d` is bound first: tclsh errors on reading an unset variable.
        assert!(vars("set d [gets stdin]\ndict for {k v} $d { set seen 1 }").contains("seen"));
    }

    #[test]
    fn command_subst_inside_expr_records_outer_var() {
        // The analyser records the outer `n` but does not hoist the nested
        // `[set y 1]` write (inside the `[expr {…}]` command-substitution) into
        // `global_scope.variables`.
        let v = vars("set n [expr {[set y 1] + 2}]");
        assert!(v.contains("n"), "outer assignment `n` is recorded");
    }

    #[test]
    fn nested_proc_and_if_records_proc() {
        let src = "proc check {x} {\n    if {== $x 0} {\n        set result zero\n    } else {\n        set result nonzero\n    }\n    return $result\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.global_scope.procs.contains_key("check"));
    }

    #[test]
    fn tcloo_method_body_in_method_scope() {
        let src =
            "oo::class create Dog {\n    method bark {} {\n        set message woof\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.all_classes.contains_key("::Dog"));
        // The method scope is named `::Dog::bark`.
        let scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name.ends_with("Dog::bark"))
            .expect("bark method scope");
        assert!(scope.variables.contains_key("message"));
    }
}

// Leading byte-order mark.
mod leading_bom {
    use super::super::{Analyser, analyser_diags, fires};

    /// A file whose first byte is a UTF-8 BOM: Tcl 9's `source` strips it
    /// before evaluating, so the first command is an ordinary `set` and must
    /// draw no unresolved-command diagnostic.
    #[test]
    fn tcl9_skips_a_leading_bom() {
        let src = "\u{FEFF}set x 1\nputs $x\n";
        assert!(
            !fires(src, "tcl9.0", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl9.0"),
        );
    }

    /// Tcl 8.x's `source` does **not** strip it — such a file really does fail
    /// with `invalid command name "<BOM>set"`, so the diagnostic stays.
    #[test]
    fn tcl86_still_reports_a_leading_bom() {
        let src = "\u{FEFF}set x 1\nputs $x\n";
        assert!(
            fires(src, "tcl8.6", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl8.6"),
        );
    }

    /// The mark is skipped by starting the scan past it, never by rewriting
    /// the buffer — so every span keeps its true byte offset and the first
    /// command's own span still points at the real name word.  This is the
    /// position-maths guarantee: a client that did send the BOM sees line 0
    /// columns that still land on the text it holds.
    #[test]
    fn skipping_the_bom_leaves_byte_offsets_untouched() {
        let src = "\u{FEFF}proc greet {} { return 1 }\n";
        let r = Analyser::new().analyse(src, "tcl9.0");
        let proc = r
            .global_scope
            .procs
            .values()
            .next()
            .expect("the proc must be recorded once the BOM is skipped");
        assert_eq!(
            proc.name_span.start() as usize,
            "\u{FEFF}proc ".len(),
            "name span must be in original-source offsets",
        );
        assert_eq!(
            &src[proc.name_span.start() as usize..proc.name_span.end() as usize],
            "greet",
        );
    }

    /// A BOM that is not at offset 0 is ordinary data in every dialect — the
    /// skip is a file-prologue rule, not a "U+FEFF is whitespace" rule.
    #[test]
    fn a_bom_mid_file_is_not_skipped() {
        let src = "set x 1\n\u{FEFF}set y 2\n";
        assert!(
            fires(src, "tcl9.0", "W123"),
            "diagnostics: {:?}",
            analyser_diags(src, "tcl9.0"),
        );
    }
}
