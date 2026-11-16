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

//! Command checks: unknown methods, shadowed builtins, the private `::tcl` namespace, `package require` and declared calls.

// W308 — TclOO unknown-method validation.
//
// `$obj nonexistent` where the class is statically known and has no such
// method → W308. Built-in methods (`destroy`), classes with a `method unknown`,
// `oo::objdefine`-added methods, and external/unknown classes suppress it.
//
// NOTE — `W308` here is solely the TclOO method check; the analyser does not
// emit a `subst`-without-`-nocommands` HINT on this surface, so those cases are
// out-of-surface.
mod tcloo_unknown_method {
    use super::super::*;

    #[test]
    fn unknown_method_fires() {
        let src = "oo::class create MyClass {\n    method greet {name} {\n        puts \"hello $name\"\n    }\n}\nset obj [MyClass new]\n$obj nonexistent_method\n";
        let ds = of_code(src, D, "W308");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("nonexistent_method"));
    }

    #[test]
    fn builtin_method_clean() {
        let src = "oo::class create MyClass {\n    method greet {} { puts hello }\n}\nset obj [MyClass new]\n$obj destroy\n";
        assert!(!fires(src, D, "W308"));
    }

    #[test]
    fn chained_constructor_validates_method() {
        let src =
            "oo::class create Dog {\n    method bark {} { puts woof }\n}\n[Dog new] nonexistent\n";
        let ds = of_code(src, D, "W308");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("nonexistent"));
    }

    #[test]
    fn method_unknown_and_objdefine_suppress() {
        // A `method unknown` catch-all means any method may exist.
        let mu = "oo::class create Proxy {\n    method unknown {name args} { puts $name }\n}\nset p [Proxy new]\n$p anyMethod\n";
        assert!(!fires(mu, D, "W308"));
        // oo::objdefine adds methods at runtime → conservative, suppress.
        let od = "oo::class create Base {\n    method existing {} { puts hello }\n}\nset obj [Base new]\noo::objdefine $obj method custom {} { puts custom }\n$obj custom\n";
        assert!(!fires(od, D, "W308"));
    }

    #[test]
    fn external_class_no_w308() {
        // Unknown `[Circuit new …]` is not in all_classes → no method validation.
        let src = "set circuit [Circuit new {Monte-Carlo}]\n$circuit runAndRead\n";
        assert!(!fires(src, D, "W308"));
    }
}

// W113 — proc shadows a core built-in.
//
// Redefining an unqualified core builtin (`set`, `exit`) is a real shadow. A
// namespace-qualified library command (`::snit::type`, `::base64::encode`) is
// not a builtin → not flagged.
mod builtin_shadow {
    use super::super::*;

    #[test]
    fn core_builtin_shadow_fires() {
        let ds = of_code("proc set {a b} {}", D, "W113");
        assert_eq!(ds.len(), 1);
        assert!(ds[0].0.contains("set"));
        assert_eq!(count("proc exit {} {}", D, "W113"), 1);
    }

    #[test]
    fn namespaced_library_command_not_flagged() {
        assert!(!fires(
            "namespace eval base64 { proc encode {s} {return $s} }",
            D,
            "W113"
        ));
        assert!(!fires("proc ::snit::type {name def} {}", D, "W113"));
    }

    // A bare-name `proc` whose only registry match is a
    // `required_package`-gated third-party command (argparse, a tcllib
    // package, …) must not fire W113 — that command does not exist in a
    // stock interpreter until its package is loaded, so defining a proc of
    // the same name is not shadowing a built-in; it is (often) that
    // package's own implementation. Ground truth (tclsh 9.0.4 / 8.6.14):
    // `info commands argparse` is empty and calling it errors
    // `invalid command name "argparse"` before the package is sourced.
    #[test]
    fn package_gated_command_name_not_flagged() {
        // TP (repro, argparse.tcl:17's exact shape): defining the
        // `argparse` package's own entry point must not warn that it
        // shadows a built-in — `argparse` is required_package-gated
        // (rust/tcl-registry/src/commands/argparse/command.rs), not a core
        // command.
        assert!(!fires("proc ::argparse {args} {}", D, "W113"));
        // A tcllib package command name hits the same gate.
        assert!(!fires("proc ::textutil::adjust {s} {}", D, "W113"));
    }

    // TN: a genuine core built-in (no `required_package`) still fires,
    // across dialects — the package-gating filter must not blunt the
    // original check.
    #[test]
    fn ungated_builtin_shadow_still_fires_across_dialects() {
        assert_eq!(count("proc set {a b} {}", "tcl8.4", "W113"), 1);
        assert_eq!(count("proc set {a b} {}", "tcl9.0", "W113"), 1);
    }

    // TN: iRules' `pool` is ambient (part of the profile's real, always-
    // present command surface) and carries no `required_package` at all —
    // it must keep firing W113 exactly as before this fix, proving the new
    // filter does not touch commands that were never package-gated.
    #[test]
    fn ambient_irules_command_still_flagged() {
        assert_eq!(count("proc pool {a} {}", IR, "W113"), 1);
    }
}

// W143 — direct call into a private `::tcl::` implementation namespace.
//
// tclsh ground truth: `::tcl::dict::create a 1` runs and returns `a 1` under
// both tclsh8.6 and tclsh9.0 (confirmed live) — the call works, so this is
// advisory (Warning), not an error. Prefix-level only: no per-subcommand or
// per-version modelling.
mod private_tcl_namespace {
    use super::super::*;

    #[test]
    fn direct_call_is_flagged_with_concrete_suggestion() {
        let ds = of_code("::tcl::dict::create a 1", D, "W143");
        assert_eq!(ds.len(), 1);
        assert!(
            ds[0].0.contains("dict create"),
            "expected the concrete 'dict create' suggestion in the message; got {:?}",
            ds[0].0
        );
        // The bare (non-`::`-rooted) spelling is equally flagged.
        assert!(fires("tcl::dict::create a 1", D, "W143"));
    }

    #[test]
    fn quick_fix_replaces_head_with_public_ensemble_call() {
        let ds = of_code("::tcl::string::totitle abc", D, "W143");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["string totitle".to_string()]);
    }

    // The fix's description must name the *whole* replacement, not just the
    // ensemble command — "Replace with 'dict'" alongside a `dict create`
    // edit told the user one thing and did another.
    #[test]
    fn quick_fix_description_matches_its_replacement_text() {
        let src = "::tcl::dict::create a 1";
        let ds = Analyser::new()
            .analyse(src, D)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W143")
            .flat_map(|d| d.fixes.clone())
            .collect::<Vec<_>>();
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].new_text, "dict create");
        assert_eq!(ds[0].description, "Replace with 'dict create'");
    }

    #[test]
    fn every_whole_private_namespace_fires() {
        use tcl_registry::private_tcl_namespaces::{PRIVATE_TCL_NAMESPACES, TailRule};
        for ns in PRIVATE_TCL_NAMESPACES
            .iter()
            .filter(|n| n.tail_rule == TailRule::Whole)
        {
            let src = format!("::tcl::{}::sub arg", ns.namespace);
            assert!(fires(&src, D, "W143"), "expected W143 for {src:?}");
        }
    }

    // (a) `::tcl::chan` is shared with tcllib's public `virtchannel_base`
    // packages, so only a genuine `chan` ensemble subcommand is private
    // there.  `tcl::chan::memchan` and friends are real, documented,
    // registry-known commands.
    #[test]
    fn public_tcllib_channel_packages_not_flagged() {
        for src in [
            "package require tcl::chan::memchan\nset c [tcl::chan::memchan]",
            "set c [::tcl::chan::memchan]",
            "set c [tcl::chan::fifo]",
            "set c [tcl::chan::cat $a $b]",
            "set c [tcl::transform::base64 $chan]",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // W120 and W143 must not contradict each other: the memchan
    // repro asks for a `package require` (W120) without also claiming the
    // command is Tcl-private (W143).
    #[test]
    fn missing_package_require_does_not_also_draw_w143() {
        let src = "set c [tcl::chan::memchan]";
        assert!(
            fires(src, D, "W120"),
            "expected W120; got {:?}",
            codes(src, D)
        );
        assert!(!fires(src, D, "W143"));
    }

    // A genuine `chan` ensemble-backing call under the shared namespace is
    // still flagged, with the fix.
    #[test]
    fn genuine_chan_ensemble_backing_still_fires() {
        // `$c` is bound first: tclsh errors on reading an unset variable.
        let ds = of_code("set c [gets stdin]\n::tcl::chan::truncate $c 0", D, "W143");
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].2, vec!["chan truncate".to_string()]);
    }

    // (b) A file that defines the command itself owns that name — a user's
    // own `proc ::tcl::dict::mine` is not a call into Tcl's internals.
    #[test]
    fn file_defining_the_command_suppresses_the_warning() {
        for src in [
            "proc ::tcl::dict::mine {d} { return $d }\n::tcl::dict::mine {a 1}",
            "namespace eval ::tcl::dict { proc mine {d} { return $d } }\n::tcl::dict::mine {a 1}",
            "proc ::tcl::dict::mine {d} { return $d }\nproc caller {} { ::tcl::dict::mine {a 1} }",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // …but only for the name it actually defines.
    #[test]
    fn defining_one_private_name_does_not_licence_another() {
        let src = "proc ::tcl::dict::mine {d} { return $d }\n::tcl::dict::create a 1";
        assert!(fires(src, D, "W143"));
    }

    // (c) A `package require` covering the qualified name (or any namespace
    // prefix of it) means the name belongs to that package.
    #[test]
    fn package_require_in_scope_suppresses_the_warning() {
        for src in [
            // The package *is* the command.
            "package require tcl::dict::mycustom\n::tcl::dict::mycustom {a 1}",
            // …or owns the namespace the command sits in.
            "package require tcl::dict\n::tcl::dict::mycustom {a 1}",
            // Order-independent: the analyser's whole-file view sees the
            // requirement wherever it sits.
            "::tcl::dict::mycustom {a 1}\npackage require tcl::dict::mycustom",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
    }

    // (d) A tail that is not a subcommand of the public ensemble still
    // warns, but must NOT offer a code-corrupting rewrite.
    #[test]
    fn non_subcommand_tail_warns_without_a_fix() {
        for (src, needle) in [
            ("::tcl::clock::GetSystemTimeZone", "GetSystemTimeZone"),
            ("::tcl::dict::mycustom {a 1}", "mycustom"),
            ("::tcl::dict::a::b {a 1}", "a::b"),
        ] {
            let ds = of_code(src, D, "W143");
            assert_eq!(ds.len(), 1, "expected exactly one W143 for {src:?}");
            assert!(
                ds[0].2.is_empty(),
                "{src:?} must offer no quick fix; got {:?}",
                ds[0].2
            );
            assert!(
                ds[0].0.contains(needle),
                "message should name the tail {needle:?}; got {:?}",
                ds[0].0
            );
        }
    }

    // (e) A delimited command head — `"::tcl::dict::create"` or
    // `{::tcl::dict::create}` — still warns, but the head token's span does
    // not cover its delimiters, so no fix is offered rather than one that
    // leaves an orphan `"`/`}` behind.
    #[test]
    fn delimited_head_warns_without_a_fix() {
        for src in ["\"::tcl::dict::create\" a 1", "{::tcl::dict::create} a 1"] {
            let ds = of_code(src, D, "W143");
            assert_eq!(ds.len(), 1, "expected exactly one W143 for {src:?}");
            assert!(
                ds[0].2.is_empty(),
                "{src:?} must offer no quick fix; got {:?}",
                ds[0].2
            );
        }
    }

    #[test]
    fn public_ensemble_call_not_flagged() {
        assert!(!fires("dict create a 1", D, "W143"));
        assert!(!fires("string totitle abc", D, "W143"));
    }

    #[test]
    fn users_own_tcl_namespace_not_flagged() {
        assert!(!fires(
            "namespace eval ::tcl::mycustom { proc foo {} {} }\n::tcl::mycustom::foo",
            D,
            "W143"
        ));
    }

    #[test]
    fn public_documented_tcl_rooted_namespaces_not_flagged() {
        // `tcl::mathop`/`tcl::mathfunc` are public, documented namespaces —
        // must never be confused with the private implementation ones.
        // Both the `::`-rooted and the bare spellings, at the integration
        // layer (the registry classifier pins them too).
        for src in [
            "::tcl::mathop::+ 1 2",
            "tcl::mathop::+ 1 2",
            "::tcl::mathfunc::sin 0",
            "tcl::mathfunc::sin 0",
            "set x [tcl::mathop::* 2 3]",
            "set y [tcl::mathfunc::max 1 2]",
        ] {
            assert!(!fires(src, D, "W143"), "unexpected W143 for {src:?}");
        }
        // `tcl::prefix` is likewise a real, public, documented command
        // (Tcl 8.6+) — not a private namespace.
        assert!(!fires("tcl::prefix match {apple banana} app", D, "W143"));
    }

    #[test]
    fn near_miss_namespace_not_flagged() {
        // Shares the `dict` prefix textually but is not the exact segment.
        assert!(!fires("::tcl::dictionary::foo", D, "W143"));
    }
}

// Requiring a package the interpreter provides at start-up only checks its
// version; no loader runs, so the command world stays proved.
//   tclsh8.6: `package names` ⇒ `zlib TclOO tcl::tommath Tcl`
//   tclsh9.0: `package names` ⇒ `zlib tcl::zlib TclOO tcl::oo tcl::tommath Tcl tcl`
mod interpreter_provided_package_require {
    use super::super::*;

    const BODY: &str = "proc f {a} { puts $nope }\nf 1\n";

    #[test]
    fn requiring_tcl_keeps_procedure_diagnostics() {
        for (dialect, require) in [
            ("tcl8.4", "package require Tcl 8.4"),
            ("tcl8.5", "package require Tcl 8.5"),
            ("tcl8.6", "package require Tcl 8.6"),
            ("tcl9.0", "package require Tcl 9.0"),
            ("tcl9.0", "package require tcl 9.0"),
            ("tcl9.1", "package require Tcl 9.1"),
        ] {
            let src = format!("{require}\n{BODY}");
            let found = codes(&src, dialect);
            for code in ["W210", "W214"] {
                assert!(
                    found.iter().any(|c| c == code),
                    "{dialect} `{require}` lost {code}: {found:?}"
                );
            }
        }
    }

    #[test]
    fn requiring_tcloo_where_the_core_provides_it_keeps_procedure_diagnostics() {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let src = format!("package require TclOO\n{BODY}");
            assert!(fires(&src, dialect, "W210"), "{dialect}");
        }
    }

    #[test]
    fn the_diagnostics_match_the_unrequired_script() {
        let mut plain = codes(BODY, D);
        let mut required = codes(&format!("package require Tcl 8.6\n{BODY}"), D);
        plain.sort();
        required.sort();
        assert_eq!(plain, required);
    }
}

// A package whose commands the registry declares installs exactly that
// surface, so the rest of the command world stays proved after requiring it.
// A package nothing declares remains an unknown loader.
mod declared_package_require {
    use super::super::*;

    const BODY: &str = "proc f {a} { puts $nope }\nf 1\n";

    #[test]
    fn requiring_a_declared_package_keeps_procedure_diagnostics() {
        for require in [
            "package require http",
            "package require http 2.9",
            "package require struct::set",
        ] {
            let found = codes(&format!("{require}\n{BODY}"), D);
            for code in ["W210", "W214"] {
                assert!(
                    found.iter().any(|c| c == code),
                    "`{require}` lost {code}: {found:?}"
                );
            }
        }
    }

    #[test]
    fn the_required_packages_own_commands_resolve() {
        assert!(fires("package require http\nhttp::geturl\n", D, "E002"));
        assert!(!fires("package require http\nhttp::geturl\n", D, "W120"));
    }

    #[test]
    fn requiring_an_undeclared_package_stays_an_unknown_loader() {
        let found = codes(
            &format!("package require no_such_package_anywhere\n{BODY}"),
            D,
        );
        assert!(!found.iter().any(|c| c == "W214"), "{found:?}");
    }
}

// A call to a recursive procedure keeps the command world proved when the body
// changes no command, namespace or object for any argument; otherwise the
// world after the call is unknown.
//   tclsh: `proc g {n} {if {$n} {g 0} else {rename puts p2}}; g 1; puts x`
//          ⇒ invalid command name "puts"
mod recursive_call_command_world {
    use super::super::*;

    const LATER: &str = "proc f {a} { puts $nope }\nf 1\n";

    #[test]
    fn a_world_preserving_recursive_call_keeps_later_diagnostics() {
        for callee in [
            "proc g {n} { if {$n} { g 0 } }\ng 1\n",
            "proc g {n} { h }\nproc h {} { g 0 }\ng 1\n",
            "proc fib {n} {\n    if {$n < 2} { return $n }\n    return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]\n}\nfib 10\n",
        ] {
            let src = format!("{callee}{LATER}");
            assert!(fires(&src, D, "W210"), "{callee}: {:?}", codes(&src, D));
        }
    }

    #[test]
    fn a_world_changing_recursive_body_still_widens() {
        for callee in [
            "proc g {n} { if {$n} { proc puts {a} {}; g 0 } }\ng 1\n",
            "proc g {n} { if {$n} { g 0 } else { rename puts p2 } }\ng 1\n",
            "proc g {n {m 0}} { if {$n} { g 0 1 } elseif {$m} { rename puts p2 } }\ng 1\n",
        ] {
            let src = format!("{callee}{LATER}");
            assert!(!fires(&src, D, "W210"), "{callee}: {:?}", codes(&src, D));
        }
    }
}

// A call to a declared command (stub) leaves the command world intact; its
// stated frame effect bounds what it does to the caller's variables. An
// undeclared command still widens everything after it.
mod declared_command_call {
    use super::super::*;

    fn stubbed(flags: &str, body: &str) -> String {
        format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub putz {{a}} {flags}\n# tcl-lsp: stubs-end\n{body}"
        )
    }

    #[test]
    fn a_declared_call_keeps_later_unknown_commands_reported() {
        let src = stubbed("-frame own", "putz hi\nputa hi\n");
        let found = codes(&src, D);
        assert!(found.iter().any(|c| c == "W123"), "{found:?}");
    }

    #[test]
    fn a_stated_own_frame_keeps_local_reads_proved() {
        let body = "proc f {b} { putz hi; puts $nope }\nf 1\n";
        assert!(fires(&stubbed("-frame own", body), D, "W210"));
        assert!(!fires(&stubbed("", body), D, "W210"));
    }

    #[test]
    fn an_undeclared_call_still_widens() {
        // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
        let found = codes(
            "set ::auto_index(putz) {source lib.tcl}\nputz hi\nputa hi\nproc f {b} { puts $nope }\nf 1\n",
            D,
        );
        assert!(!found.iter().any(|c| c == "W210"), "{found:?}");
    }
}
