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

//! Unresolved commands: when the analyser reports one and when it abstains.

// TestW123UnresolvedCommand — unknown-command detection.
mod unresolved_command {
    use super::super::*;

    fn w123(src: &str) -> Vec<String> {
        analyser_diags(src, D)
            .into_iter()
            .filter(|(c, _, _)| c == "W123")
            .map(|(_, m, _)| m)
            .collect()
    }

    #[test]
    fn unknown_command_emits_w123() {
        let d = w123("mycommand a b c");
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("mycommand"));
    }

    #[test]
    fn known_builtin_and_user_proc_no_w123() {
        assert_eq!(w123("set x 1"), [] as [std::string::String; 0]);
        assert_eq!(
            w123("proc mycommand {x} { puts $x }\nmycommand hello"),
            [] as [std::string::String; 0]
        );
        // A call written before the proc is defined runs first: tclsh 8.4-9.1
        // raise `invalid command name "mycommand"`, so W123 is warranted.
        let forward = w123("mycommand hello\nproc mycommand {x} { puts $x }");
        assert_eq!(forward.len(), 1, "{forward:?}");
        assert!(forward[0].contains("mycommand"));
    }

    #[test]
    fn namespace_qualified_and_variable_command_skipped() {
        assert_eq!(w123("::myns::cmd arg1"), [] as [std::string::String; 0]);
        // `$cmd hello` — the head starts with `$`, not a literal command name.
        assert!(
            w123("set cmd puts\n$cmd hello")
                .iter()
                .all(|m| !m.contains("$cmd"))
        );
    }

    #[test]
    fn package_require_suppresses_w123() {
        // A `package require` may register the missing command at runtime →
        // suppressed (e.g. `package require Tk` provides `button`).
        assert_eq!(
            w123("package require Tk\nbutton .b -text hi"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn dynamic_providers_suppress_w123() {
        // `has_dynamic_providers` gates W123 exactly as it gates W120: a
        // `load` / `auto_path` mutation / dynamic import may register
        // commands at runtime the analyser can't see, so the resolvable
        // command set is unknowable and W123 must abstain file-wide.
        for src in [
            "load mylib.so\nmycommand arg1",
            // An undefined `[computed]` would raise first (tclsh8.6.18/9.0.4:
            // invalid command name "computed"); read the part instead.
            "namespace import ::foo::[gets stdin]::*\nbar arg",
        ] {
            assert!(w123(src).is_empty(), "expected no W123 for {src:?}");
        }
        // tclsh8.6.18/9.0.4: an auto_path entry no index in the document
        // backs provides nothing: `mycmd arg` raises invalid command name.
        assert_eq!(w123("lappend auto_path /opt/mylib\nmycmd arg").len(), 1);
    }

    #[test]
    fn glob_namespace_import_suppresses_matching_tail_only() {
        // A literal glob import provides every source command matching the
        // pattern's tail — `render_box` matches `render_*` (silent), while an
        // unrelated name matches nothing and still fires. The namespace must
        // exist: tclsh 8.4.20, 8.6.18 and 9.0.4 raise `unknown namespace in
        // import pattern` otherwise, and with it run `render_box 10 20` and
        // raise `invalid command name "frobnicate"`.
        let src = "namespace eval ::acme::widgets {proc render_box {x y} {}; namespace export *}\nnamespace import ::acme::widgets::render_*\nrender_box 10 20\nfrobnicate 1\n";
        let d = w123(src);
        assert_eq!(d.len(), 1, "only the unrelated name fires; got {d:?}");
        assert!(d[0].contains("frobnicate"));
        // A full `*` tail conservatively resolves every bare name (the
        // imported namespace's export set is unknowable single-file).
        assert_eq!(
            w123("namespace import ::foo::*\nbar arg"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn literal_namespace_import_suppresses_exactly_that_name() {
        // tclsh 8.4.20, 8.6.18 and 9.0.4 run `render_box 10 20` and raise
        // `invalid command name "render_circle"`.
        let src = "namespace eval ::acme::widgets {proc render_box {x y} {}; proc render_circle {r} {}; namespace export *}\nnamespace import ::acme::widgets::render_box\nrender_box 10 20\nrender_circle 5\n";
        let d = w123(src);
        assert_eq!(d.len(), 1, "only the un-imported name fires; got {d:?}");
        assert!(d[0].contains("render_circle"));
    }

    #[test]
    fn bare_dict_ensemble_builtin_resolves_from_inside_tcl_dict_namespace() {
        // FP: `exists`/`get` are real,
        // separately-callable commands (`::tcl::dict::exists`,
        // `::tcl::dict::get`), backing the `dict` ensemble's own
        // subcommands (confirmed against tclsh9.0.4/8.6.14: `info commands
        // ::tcl::dict::*` lists them). A proc lexically defined *inside*
        // `::tcl::dict` (the real tcllib `dicttool.tcl` idiom) resolves a
        // bare call to them via ordinary current-namespace-then-global
        // lookup, so it must not fire W123 — isolated from any
        // `namespace ensemble configure` patching (tested separately) to
        // prove this half is a pure namespace-resolution fact.
        let src = "proc ::tcl::dict::myhelper {d k} {\n\
             if {[exists $d $k]} { return [get $d $k] }\n\
             return MISSING\n\
             }\n\
             ::tcl::dict::myhelper [dict create a 1 b 2] a\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn bare_dict_ensemble_builtin_outside_tcl_dict_namespace_still_fires_w123() {
        // TP — regression guard: `exists`/`get` called bare *outside*
        // `::tcl::dict` must still fire (tclsh-confirmed real error:
        // "invalid command name \"exists\"") — proves the fix is properly
        // namespace-scoped, not a blanket allow-list for these two names.
        let d = w123("proc foo {} { return [exists bar] }\n");
        assert_eq!(d.len(), 1, "got {d:?}");
        assert!(d[0].contains("exists"));
    }

    #[test]
    fn fully_qualified_tcl_dict_builtin_calls_are_unaffected() {
        // TN — regression guard: the qualified spelling already worked via
        // the pre-existing `name.contains("::")` conservative skip; the new
        // resolution-candidate check must not change that.
        let src = "proc demo {} {\n\
             set d [dict create a 1 b 2]\n\
             if {[::tcl::dict::exists $d a]} { return [::tcl::dict::get $d a] }\n\
             }\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn ordinary_namespace_bare_name_resolution_is_unaffected_by_dict_fix() {
        // TN — regression guard: the common case (a bare call resolving to
        // an ordinary user proc via its own namespace) must be unaffected by
        // the new resolution-candidate check, which is a no-op there (the
        // bare-name check above it already resolves it).
        assert_eq!(
            w123(
                "namespace eval ::foo {\n\
                 proc helper {} { return 42 }\n\
                 proc bar {} { return [helper] }\n\
                 }\n"
            ),
            [] as [std::string::String; 0]
        );
    }

    // tcllib `textutil::adjust` submodule commands must be registered
    // under their real 3-segment name, not the umbrella-only 2-segment
    // name, so calls through the common `package require textutil::adjust;
    // namespace import textutil::adjust::*` idiom (georgtree_argparse's
    // `argparse.tcl:380-382`) resolve. Ground truth (tclsh 9.0.4 + real
    // tcllib-2.0): `package require
    // textutil::adjust` creates `::textutil::adjust::adjust` /
    // `::textutil::adjust::indent`, never a bare `::textutil::adjust`.
    mod textutil_adjust_idx3_idx4 {
        use super::*;

        #[test]
        fn qualified_submodule_calls_resolve_after_package_require() {
            // A call to the real, canonical name resolves once the
            // submodule package is required. Note `package require` (any
            // package, this file's
            // included) blanket-suppresses W123 file-wide (`emit_unresolved_
            // command_diagnostics`'s conservative "package may define
            // anything at runtime" gate) — so this pins that the call
            // resolves under the real-world idiom, while the two tests below
            // isolate the registry-data fact itself (no `package require` in
            // scope, so the file-wide suppression gate does not mask it).
            let src = "package require textutil::adjust\n\
                       namespace import textutil::adjust::*\n\
                       set a [adjust hello -length 20]\n\
                       set b [indent $a \"  \"]\n";
            assert!(w123(src).is_empty(), "got {:?}", w123(src));
        }

        #[test]
        fn qualified_submodule_names_are_directly_registry_known() {
            // TP, isolating the registry-data fact with no `package require`
            // in scope (so the blanket suppression gate above cannot mask
            // it): the literal, fully-qualified 3-segment names must be
            // directly known to W123's registry-name set.
            let src = "textutil::adjust::adjust hello -length 20\n\
                       textutil::adjust::indent hello \"  \"\n";
            assert!(w123(src).is_empty(), "got {:?}", w123(src));
        }

        #[test]
        fn a_bare_unimported_flat_name_still_unresolved() {
            // TN/FN contrast: with no `package require`, no `namespace
            // import`, and no umbrella alias in play, the *bare* flattened
            // names (`adjust`/`indent` with no qualification at all) are
            // correctly still unknown — the fix adds the submodule's real
            // names, it does not fabricate a global bare alias that only
            // the `textutil` umbrella actually provides.
            let d = w123("adjust hello -length 20\n");
            assert_eq!(d.len(), 1, "got {d:?}");
            assert!(d[0].contains("adjust"));
        }
    }

    #[test]
    fn coroutine_name_is_a_known_command() {
        // `coroutine NAME cmd ?arg …?` creates the command NAME
        // (`TclNRCoroutineObjCmd`) — calling it is not an unknown command,
        // while a typo'd name in the same file still fires.
        let src = "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNum\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
        let typo = "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNun\n";
        let d = w123(typo);
        assert_eq!(d.len(), 1, "the typo'd name fires; got {d:?}");
        assert!(d[0].contains("nextNun"));
    }

    #[test]
    fn interp_create_name_is_a_known_command() {
        // `interp create NAME` binds NAME as the child interpreter's
        // command — `NAME eval {…}` dispatches on it.
        assert_eq!(
            w123("interp create child\nchild eval { puts hi }\n"),
            [] as [std::string::String; 0]
        );
        // The `-safe` flag at the name index is an option, never a name; a
        // missing name is auto-generated — nothing to record, nothing fires.
        assert_eq!(
            w123("interp create -safe\n"),
            [] as [std::string::String; 0]
        );
        let r = Analyser::new().analyse("interp create -safe\n", D);
        assert!(
            !r.created_instance_commands.contains("-safe"),
            "an option flag must not be recorded as a created command"
        );
    }

    #[test]
    fn renamed_away_builtin_call_fires_w123() {
        // `rename puts {}` (or a rename-away) deletes the builtin — a later
        // call fails "invalid command name" (tclsh 9.0.4), so W123 fires.
        let d = w123("rename puts {}\nputs x\n");
        assert_eq!(d.len(), 1, "deleted builtin fires; got {d:?}");
        assert!(d[0].contains("'puts'"));
        assert_eq!(w123("rename puts myputs\nputs x\n").len(), 1);
        // The rename target stays known, and calls lexically BEFORE the
        // rename still resolve to the builtin.
        assert_eq!(
            w123("rename puts myputs\nmyputs x\n"),
            [] as [std::string::String; 0]
        );
        assert_eq!(
            w123("puts before\nrename puts myputs\n"),
            [] as [std::string::String; 0]
        );
        // A re-binding after the deletion makes the name callable again.
        assert_eq!(
            w123("rename puts {}\nproc puts {args} {}\nputs x\n"),
            [] as [std::string::String; 0]
        );
        // A rename buried in a proc body is conditional — it runs only if
        // the proc is ever called — so the builtin stays known at top level.
        assert_eq!(
            w123("proc hook {} { rename puts _p }\nputs hi\n"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn namespace_unknown_handler_suppresses_w123() {
        // Installing a per-namespace unknown handler (TIP 181) makes command
        // resolution unknowable — file-wide suppression, like a dynamic
        // `proc unknown`.
        let src = "proc handler {args} { puts $args }\nnamespace unknown handler\nmystery_cmd 1\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
        // The bare query form installs nothing; an empty handler resets to
        // the default — neither suppresses.
        assert_eq!(w123("namespace unknown\nmystery_cmd 1\n").len(), 1);
        assert_eq!(w123("namespace unknown {}\nmystery_cmd 1\n").len(), 1);
    }

    #[test]
    fn list_wrapped_namespace_unknown_installer_covers_only_its_namespace() {
        // FP: `namespace eval $ns [list namespace
        // unknown $handler]` (tcllib's `namespacex::hook::Set` idiom) is
        // a `Cmd`-kind body — `analyse_body`'s literal-`{...}`-only gate
        // never walks it, and the generic nested-substitution scan
        // resolves the head to `list`, never `namespace unknown` — so
        // this installer shape must be recognised separately, or it stays
        // invisible to every path. The handler covers only commands
        // resolved from ::target: tclsh8.6.18/9.0.4/9.1.0 `mystery_cmd 1`
        // from :: still raises invalid command name "mystery_cmd".
        let src = "proc handler {args} { puts $args }\nnamespace eval ::target [list namespace unknown handler]\nmystery_cmd 1\n";
        assert_eq!(w123(src).len(), 1, "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_full_tcllib_idiom_suppresses_w123() {
        // FP — the full attested shape (mirrors tcllib
        // modules/namespacex/namespacex.tcl:157-162's `hook::Set`, and
        // the finding's own repro): the installer is itself wrapped in
        // another proc, and the handler is installed from inside a
        // namespace body.
        let src = "proc ::hooklib::Set {ns handler} {\n    namespace eval $ns [list namespace unknown $handler]\n}\nnamespace eval ::target {\n    proc fallbackHandler {args} { return \"handled:$args\" }\n    ::hooklib::Set ::target [namespace code fallbackHandler]\n    proc run {} { return [mystery arg1 arg2] }\n}\n";
        assert!(w123(src).is_empty(), "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_via_namespace_inscope_covers_only_its_namespace() {
        // `namespace inscope` shares the `namespace eval` hook; the handler
        // it installs in ::target does not cover a lookup from ::
        // (tclsh8.6.18/9.0.4/9.1.0: invalid command name "mystery_cmd").
        let src = "proc handler {args} { puts $args }\nnamespace inscope ::target [list namespace unknown handler]\nmystery_cmd 1\n";
        assert_eq!(w123(src).len(), 1, "got {:?}", w123(src));
    }

    #[test]
    fn list_wrapped_namespace_unknown_requires_the_exact_shape() {
        // TP — over-suppression guards: each of these must still fire
        // W123, proving the recogniser requires the literal `list
        // namespace unknown` head with a non-empty handler, not "any
        // list-wrapped namespace eval body".
        assert_eq!(
            w123("namespace eval ::target [list puts hello]\nmystery_cmd 1\n").len(),
            1,
            "an unrelated list-wrapped body must not suppress W123"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace export foo]\nmystery_cmd 1\n").len(),
            1,
            "a different namespace subcommand must not suppress W123"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace unknown]\nmystery_cmd 1\n").len(),
            1,
            "the bare query form installs nothing"
        );
        assert_eq!(
            w123("namespace eval ::target [list namespace unknown {}]\nmystery_cmd 1\n").len(),
            1,
            "an empty handler resets to the default, installs nothing"
        );
    }

    #[test]
    fn list_wrapped_namespace_unknown_does_not_blanket_suppress_the_namespace() {
        // TP — negative control: with no `namespace unknown` installer
        // anywhere, a genuinely unknown command inside the namespace
        // body must still be flagged.
        let src = "namespace eval ::target {\n    proc run {} { return [mystery arg1 arg2] }\n}\n";
        assert_eq!(w123(src).len(), 1);
    }

    #[test]
    fn list_wrapped_namespace_unknown_via_concat_is_a_known_remaining_gap() {
        // FP (documented, intentionally not covered) — the same idiom
        // built via `concat` instead of a literal `list` call is out of
        // scope: the recogniser is narrow to the exact attested
        // `list namespace unknown` shape. Pinned so nobody mistakes the
        // narrow fix for a full generalisation.
        let src = "proc handler {args} { puts $args }\nnamespace eval ::target [concat namespace unknown handler]\nmystery_cmd 1\n";
        assert_eq!(
            w123(src).len(),
            1,
            "concat-built installers are a documented, out-of-scope gap"
        );
    }

    #[test]
    fn static_rename_target_is_a_known_command() {
        // A static `rename OLD NEW` *defines* NEW (confirmed against tclsh
        // 9.0.4: `rename puts myputs; info commands myputs` lists it), so a
        // call to the renamed name must not draw W123 — while a genuinely
        // unknown name in the same file still does.
        assert_eq!(
            w123("rename puts myputs\nmyputs hello"),
            [] as [std::string::String; 0]
        );
        assert_eq!(w123("rename puts myputs\nmyputz hello").len(), 1);
    }

    #[test]
    fn stub_directive_suppresses_w123() {
        let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub mycommand {arg1 arg2}\n# tcl-lsp: stubs-end\nmycommand x y\n";
        assert_eq!(w123(src), [] as [std::string::String; 0]);
    }

    #[test]
    fn stub_directive_outside_block_reports_its_own_line_2365() {
        let line = "# tcl-lsp: stub db_eval {sql script:body}";
        let body = "proc p {} {\n    db_eval {select 1} {puts row}\n}\np\n";
        let unbounded = format!("{line}\n{body}");
        let result = Analyser::new().analyse(&unbounded, "tcl8.6");
        let notices = result
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W153")
            .collect::<Vec<_>>();
        assert_eq!(notices.len(), 1, "{:?}", result.diagnostics);
        assert_eq!(notices[0].span.start(), 0);
        assert_eq!(notices[0].span.end(), u32::try_from(line.len()).unwrap());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.to_string() == "W123")
        );

        let bounded = format!("# tcl-lsp: stubs-begin\n{line}\n# tcl-lsp: stubs-end\n{body}");
        let result = Analyser::new().analyse(&bounded, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| matches!(d.code.to_string().as_str(), "W123" | "W153")),
            "{:?}",
            result.diagnostics
        );

        let embedded = "set x {\n# tcl-lsp: stub db_eval {sql script:body}\n}\n";
        let result = Analyser::new().analyse(embedded, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.to_string() == "W153")
        );

        let expression = "# tcl-lsp: stub expr-func pick 1\n# tcl-lsp: stub expr-op ::pick 2\n# tcl-lsp: stub incomplete\n";
        let result = Analyser::new().analyse(expression, "tcl8.6");
        let outside = result
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W153")
            .collect::<Vec<_>>();
        assert_eq!(outside.len(), 2, "{:?}", result.diagnostics);
        assert_eq!(outside[0].span.start(), 0);
        assert_eq!(outside[1].span.start(), 33);
    }

    #[test]
    fn alias_command_no_w123_and_alias_in_did_you_mean() {
        assert_eq!(
            w123("interp alias {} = {} expr\n= {1 + 2}"),
            [] as [std::string::String; 0]
        );
        let d = w123("interp alias {} myput {} puts stdout\nmyptu hello");
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("myput"));
    }

    // Did you mean?

    #[test]
    fn did_you_mean_suggestion_for_near_name() {
        // `putz` is one edit from `puts` (a real builtin).
        let r = Analyser::new().analyse("putz hello", D);
        let d: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W123")
            .collect();
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("puts"));
        assert_ne!(d[0].fixes, [] as [tcl_compiler::analyser::CodeFix; 0]);
        assert_eq!(d[0].fixes[0].new_text, "puts");
    }

    #[test]
    fn no_suggestion_for_distant_name() {
        let d = w123("xyzzyplugh arg1");
        assert_eq!(d.len(), 1);
        assert!(!d[0].contains("did you mean"));
    }

    // Unknown-proc dispatch analysis.

    #[test]
    fn unknown_proc_switch_dispatch_covers_targets() {
        let handled = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n        bar { puts bar }\n    }\n}\nfoo x\nbar y\n";
        assert_eq!(w123(handled), [] as [std::string::String; 0]);
        let unhandled = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\nfoo x\nbaz y\n";
        // tclsh8.6.18/9.0.4: `baz y` runs the handler, no arm matches and
        // it returns; nothing raises.
        assert_eq!(w123(unhandled), [] as [std::string::String; 0]);
    }

    #[test]
    fn unknown_proc_empty_stub_handles_the_call() {
        // tclsh8.6.18/9.0.4: `proc unknown {args} {}; mycommand arg1;
        // puts done` prints done: the empty handler returns "".
        assert_eq!(
            w123("proc unknown {args} {}\nmycommand arg1"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn unknown_proc_opaque_dispatch_suppresses() {
        // chains-original / auto_load / exec / glob-switch make dispatch opaque.
        // tclsh8.6.18/9.0.4: with the glob-switch handler, `foobar x` prints
        // foo and raises nothing.
        for src in [
            "proc unknown {cmd args} {\n    _original_unknown $cmd {*}$args\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    auto_load $cmd\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    exec $cmd {*}$args\n}\nmycommand arg1\n",
            "proc unknown {cmd args} {\n    switch -glob $cmd {\n        fo* { puts foo }\n    }\n}\nfoobar x\n",
        ] {
            assert!(w123(src).is_empty(), "unexpected W123 for {src:?}");
        }
    }

    #[test]
    fn unknown_if_else_chains_original_suppresses() {
        let src = "proc unknown {cmd args} {\n    if {$cmd eq \"foo\"} {\n        puts foo\n    } else {\n        _original_unknown $cmd {*}$args\n    }\n}\nbaz x\n";
        // tclsh8.6.18/9.0.4: `baz x` runs the handler; a global `proc
        // unknown` handles every unknown head (kcs-diagnostic-w123).
        assert_eq!(w123(src), [] as [std::string::String; 0]);
    }

    #[test]
    fn multiple_unknown_procs_last_wins() {
        let src = "proc unknown {cmd args} {\n    _original_unknown $cmd {*}$args\n}\nproc unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\nfoo x\nbaz y\n";
        let r = Analyser::new().analyse(src, D);
        let upi = r.unknown_proc_info.as_ref().expect("upi");
        assert!(!upi.chains_original);
        assert!(upi.dispatch_targets.contains("foo"));
        // tclsh8.6.18/9.0.4: `baz y` runs the last handler, which matches
        // no arm and returns; nothing raises, so no W123.
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code.to_string() == "W123" && d.message.contains("baz"))
        );
    }

    // Unknown_proc_info population.

    #[test]
    fn unknown_proc_info_populated_for_switch_dispatch() {
        // Coordinator ruling 17 Nov 21:14: under a proved document handler,
        // a name its switch handles is a known call and a name falling to a
        // raising arm draws W123. tclsh8.6.18/9.0.4: `foo x` -> foo,
        // `bar y` -> bar, `baz z` -> invalid command name "baz".
        let src = "proc unknown {cmd args} {\n    switch $cmd {\n        foo { return foo }\n        bar { return bar }\n        default { error \"invalid command name \\\"$cmd\\\"\" }\n    }\n}\nfoo x\nbar y\nbaz z\n";
        let d = w123(src);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].contains("baz"), "{d:?}");
    }

    #[test]
    fn unknown_proc_info_empty_stub_flag() {
        let r = Analyser::new().analyse("proc unknown {args} {}", D);
        assert!(r.unknown_proc_info.as_ref().expect("upi").empty_stub);
    }

    #[test]
    fn no_unknown_proc_info_by_default() {
        assert!(
            Analyser::new()
                .analyse("set x 1", D)
                .unknown_proc_info
                .is_none()
        );
    }

    #[test]
    fn tcl_unknown_qualified_recognised() {
        // tclsh8.6.18/9.0.4: Tcl consults `::unknown`, not `::tcl::unknown`;
        // `foo x` raises `invalid command name "foo"`.
        let src = "proc ::tcl::unknown {cmd args} {\n    switch $cmd {\n        foo { puts foo }\n    }\n}\nfoo x\n";
        let r = Analyser::new().analyse(src, D);
        assert!(r.unknown_proc_info.is_none());
        assert_eq!(w123(src).len(), 1);
    }

    // Dead/live short-circuit arms.
    // The analyser surfaces W123 for an unknown command like `[missingCommand]`
    // even inside a provably-dead `&&`/`||`/`?:` arm (which tclsh never
    // executes). The *live*-arm control — where tclsh genuinely errors — is
    // below.

    #[test]
    fn live_short_circuit_arm_command_still_w123() {
        // tclsh: `expr {1 && [missingCommand]}` runs the right arm and errors.
        assert_eq!(w123("expr {1 && [missingCommand]}").len(), 1);
        assert_eq!(w123("expr {[missingCommand] && 1}").len(), 1);
        assert_eq!(
            w123("proc f {c} { expr {$c && [missingCommand]} }").len(),
            1
        );
    }
}
