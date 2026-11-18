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

//! `interp alias`, alias targets flowing through values, and `rename`.

// interp alias records in `command_aliases`.
mod interp_alias {
    use super::super::*;

    fn alias(src: &str, name: &str) -> Option<(String, Vec<String>)> {
        Analyser::new()
            .analyse(src, D)
            .command_aliases
            .get(name)
            .map(|a| (a.target.as_str().to_owned(), a.extras.clone()))
    }

    #[test]
    fn alias_recorded_with_target() {
        assert_eq!(
            alias("interp alias {} = {} expr", "::="),
            Some(("expr".to_string(), vec![]))
        );
    }

    #[test]
    fn alias_with_prepended_args() {
        assert_eq!(
            alias("interp alias {} myput {} puts stdout", "::myput"),
            Some(("puts".to_string(), vec!["stdout".to_string()]))
        );
    }

    #[test]
    fn alias_non_current_interp_not_recorded() {
        assert!(alias("interp alias child = {} expr", "::=").is_none());
    }

    #[test]
    fn alias_qualified_name_normalised() {
        assert!(
            Analyser::new()
                .analyse("interp alias {} ::myexpr {} expr", D)
                .command_aliases
                .contains_key("::myexpr")
        );
    }

    #[test]
    fn alias_chain_not_resolved_transitively() {
        let src = "interp alias {} a {} b\ninterp alias {} b {} expr\n";
        assert_eq!(alias(src, "::a"), Some(("b".to_string(), vec![])));
        assert_eq!(alias(src, "::b"), Some(("expr".to_string(), vec![])));
    }

    #[test]
    fn alias_dynamic_name_not_recorded() {
        assert!(alias("set n \"=\"\ninterp alias {} $n {} expr", "::=").is_none());
    }

    #[test]
    fn alias_redefinition_overwrites() {
        let src = "interp alias {} myop {} expr\ninterp alias {} myop {} puts\n";
        assert_eq!(alias(src, "::myop"), Some(("puts".to_string(), vec![])));
    }

    #[test]
    fn alias_too_few_args_no_crash() {
        assert!(alias("interp alias {} = {}", "::=").is_none());
    }

    #[test]
    fn alias_target_in_child_interp_is_domain_qualified_945() {
        // A cross-domain alias deliberately crosses interpreter domains:
        // the current-interp `myexpr` runs the *child's* `expr`, so the
        // recorded target is qualified into the child's `@interp@` domain
        // — never treated as the parent's own `expr` — since skipping the
        // record entirely would lose the cross-domain link.
        assert_eq!(
            alias("interp alias {} myexpr child expr", "::myexpr"),
            Some(("::@interp@child::expr".to_string(), vec![])),
        );
    }

    #[test]
    fn alias_query_form_no_crash() {
        assert!(alias("interp alias {} myexpr", "::myexpr").is_none());
    }

    #[test]
    fn alias_for_expr_and_body_taking_commands_no_w214() {
        // Aliases for expr / eval / foreach route the role analysis through the
        // alias target, so params used via the alias are not W214.
        let cases = [
            "interp alias {} = {} expr\nproc foo {x y} {\n    set result [= {$x + $y}]\n    return $result\n}\n",
            "interp alias {} myeval {} eval\nproc foo {x} {\n    myeval { set y 1 }\n    return $x\n}\n",
            "interp alias {} myforeach {} foreach\nproc foo {items} {\n    myforeach item $items { puts $item }\n    return $item\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn alias_resolved_from_namespace_and_global_fallback_no_w214() {
        let cases = [
            "interp alias {} ::math::= {} expr\nnamespace eval math {\n    proc calc {x y} {\n        set result [= {$x + $y}]\n        return $result\n    }\n}\n",
            "interp alias {} = {} expr\nnamespace eval utils {\n    proc calc {x y} {\n        set result [= {$x + $y}]\n        return $result\n    }\n}\n",
            "interp alias {} = {} expr\nproc calc {x y} {\n    set result [::= {$x + $y}]\n    return $result\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn alias_standalone_expr_call_has_no_error() {
        let src = "interp alias {} = {} expr\n= {1 + 2}\n";
        assert!(
            !analyser_diags(src, D)
                .iter()
                .any(|(_, _, s)| *s == Severity::Error)
        );
    }

    #[test]
    fn real_world_file_shortcuts_record_extras() {
        let src = "interp alias {} cp {} file copy -force\ninterp alias {} mkdir {} file mkdir\ninterp alias {} rm {} file delete -force\n";
        assert_eq!(
            alias(src, "::cp"),
            Some((
                "file".to_string(),
                vec!["copy".to_string(), "-force".to_string()]
            ))
        );
        assert_eq!(
            alias(src, "::mkdir"),
            Some(("file".to_string(), vec!["mkdir".to_string()]))
        );
        assert_eq!(
            alias(src, "::rm"),
            Some((
                "file".to_string(),
                vec!["delete".to_string(), "-force".to_string()]
            ))
        );
    }

    #[test]
    fn real_world_safe_and_cross_interp_aliases_945() {
        // A dynamic source path bound by a tracked `set i [interp create
        // ...]` resolves through that binding rather
        // than aborting: `add` is defined *inside* `$i`'s domain (calling
        // `::api::add` back in the parent), so it homes under `$i`'s
        // synthetic `@interp@@autoname@<offset>` domain — never under the
        // bare `::add` a plain top-level alias would use.
        let safe = "set i [interp create -safe]\ninterp alias $i add {} ::api::add\nproc ::api::add {a b} {\n    return [expr {$a + $b}]\n}\n";
        assert!(alias(safe, "::add").is_none());
        assert_eq!(
            alias(safe, "::@interp@@autoname@6::add"),
            Some(("::api::add".to_string(), vec![])),
        );
        // A literal parent-side alias into a live child is tracked with a
        // domain-qualified target: `localGreet` runs
        // the child's `greet`, so navigation follows the alias link into
        // the child's `@interp@` domain, where `interp eval` homed the
        // proc's definition.
        let cross = "interp create child\ninterp eval child { proc greet {} { return hello } }\ninterp alias {} localGreet child greet\nlocalGreet\ninterp delete child\n";
        assert_eq!(
            alias(cross, "::localGreet"),
            Some(("::@interp@child::greet".to_string(), vec![])),
        );
    }

    #[test]
    fn real_world_dynamic_html_tag_aliases_not_tracked() {
        let src = "proc html_tag {tag args} { return \"<$tag>$args</$tag>\" }\nforeach tag {h1 h2 h3 p div span} {\n    interp alias {} $tag {} html_tag $tag\n}\n";
        assert!(alias(src, "::h1").is_none());
    }

    #[test]
    fn alias_deletion_form_keeps_prior_definition() {
        // `interp alias {} name {}` (4 args) deletes; not recorded as a new
        // alias, and the prior definition survives (we don't track deletions).
        let src = "interp alias {} myalias {} list\ninterp alias {} myalias {}\n";
        assert_eq!(alias(src, "::myalias"), Some(("list".to_string(), vec![])));
    }

    #[test]
    fn return_expr_alias_and_braced_no_w214() {
        let cases = [
            "proc foo {x} { return [expr {$x + 1}] }",
            "interp alias {} = {} expr\nproc pdPsCalc {width length} {\n    return [= {2*$width+2*$length}]\n}\n",
        ];
        for src in cases {
            assert!(!fires(src, D, "W214"), "unexpected W214 for {src:?}");
        }
    }

    #[test]
    fn issue_42_examples_no_w214() {
        let original = "interp alias {} = {} expr\nproc moveKeyInDict {dict key relPos} {\n    set keys [dkeys $dict]\n    set index [lsearch -exact $keys $key]\n    set newIndex [= {$index+$relPos}]\n    set prevKey [@ $keys $newIndex]\n    set newKeysOrder [lreplace [lreplace $keys $index $index $prevKey] $newIndex $newIndex $key]\n    foreach key $newKeysOrder {\n        dict append newDict $key [dget $dict $key]\n    }\n    return $newDict\n}\n";
        assert!(!fires(original, D, "W214"));
        let reopened = "interp alias {} = {} expr\nset scriptPath [file dirname [file normalize [info script]]]\nproc pdPsCalc {width length} {\n    return [= {2*$width+2*$length}]\n}\nset vSupply 2.0\nset inpFreq 850e6\n";
        assert!(!fires(reopened, D, "W214"));
    }
}

// interp_value_flow — `set VAR [interp create ...]`
// binds VAR to an interpreter-domain key, so a later dynamic `$VAR` operand
// to `interp alias` / `interp eval` / the handle's own object command can
// resolve through the tracked binding instead of abstaining outright.
mod interp_value_flow {
    use super::super::*;

    fn command_alias_targets(src: &str) -> std::collections::HashMap<String, String> {
        Analyser::new()
            .analyse(src, D)
            .command_aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.target.as_str().to_owned()))
            .collect()
    }

    #[test]
    fn primary_repro_cross_domain_alias_through_tracked_binding_resolves() {
        // TP: tclsh9.0-verified, prints 42. `set s [interp create -safe]`
        // binds `s`; `interp alias $s greet {} ::app::Helper` must resolve
        // through that binding rather than abstaining outright because the
        // source path is dynamic text — otherwise `greet` is left
        // unresolved inside the child's eval body (spurious W123 + 0
        // definition locations).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset s [interp create -safe]\ninterp alias $s greet {} ::app::Helper\ninterp eval $s { greet }\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn object_command_handle_eval_form_resolves_through_tracked_binding() {
        // TP — the object-command spelling (`$mpip eval { ... }`) doctools.tcl
        // actually uses, exercising `handle_interp_handle_eval_command` rather
        // than the literal `interp eval PATH` form. `$mpip eval {...}` is
        // itself a non-literal command dispatch — `mpip` is tracked only as
        // an interpreter handle, not a known TclOO object
        // (`var_command.rs`'s separate, unrelated dispatch-suppression
        // system), so it legitimately still draws W307 the same as any
        // other untracked `$var subcommand` call. What this fix controls is
        // narrower: `greet` must resolve *inside* the eval body — no W123
        // (unknown command) and no W140 (never-created interpreter).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset mpip [interp create -safe]\ninterp alias $mpip greet {} ::app::Helper\n$mpip eval { greet }\n";
        let diags = analyser_diags(src, D);
        assert!(
            diags.iter().all(|(c, _, _)| c != "W123" && c != "W140"),
            "{diags:?}"
        );
    }

    #[test]
    fn explicit_literal_name_variant_records_no_spurious_w140() {
        // TP — `set s [interp create -safe literalName]` must record
        // `literalName` in the interpreter map via the value-flow path, so a
        // later literal `interp eval literalName {...}` does not see an
        // apparently-uncreated interpreter, even though the path never
        // reaches `handle_interp_create_command` directly (it's nested
        // inside a `set`, not a bare top-level statement).
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset s [interp create -safe literalName]\ninterp alias literalName greet {} ::app::Helper\ninterp eval literalName { greet }\n";
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn two_procs_sharing_a_variable_name_never_collide() {
        // TP — cross-contamination guard: each proc's
        // `set s [interp create -safe]` must get its own per-call-site
        // `@autoname@<offset>` domain, not one shared domain keyed off the
        // raw variable text `s` — sharing one domain would let makeA's
        // `helper` call resolve into makeB's definition.
        let src = "proc makeA {} {\n    set s [interp create -safe]\n    interp eval $s {\n        proc helper {} { return A }\n        helper\n    }\n}\nproc makeB {} {\n    set s [interp create -safe]\n    interp eval $s {\n        proc helper {} { return B }\n        helper\n    }\n}\n";
        let r = Analyser::new().analyse(src, D);
        let helper_keys: Vec<&String> = r
            .all_procs
            .keys()
            .filter(|k| k.ends_with("::helper"))
            .collect();
        assert_eq!(
            helper_keys.len(),
            2,
            "expected 2 distinct helper domains: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert_ne!(helper_keys[0], helper_keys[1]);
    }

    #[test]
    fn untracked_dynamic_interp_path_stays_conservative() {
        // FP guard — a `$VAR` never bound by `set VAR [interp create ...]`
        // (here a bare proc parameter) must stay exactly as conservative as
        // before this fix: no crash, no spurious W140 (existence stays
        // unknowable for a genuinely untracked dynamic path).
        let src = "proc useEval {ip} {\n    interp eval $ip { someCmd }\n}\n";
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn reassigning_the_variable_clears_the_stale_interp_binding() {
        // FP guard — a later plain-string `set` of the same variable must
        // clear the interp binding, so a stale domain is never resolved and
        // the cross-domain alias falls through exactly as it does today for
        // a genuinely-unresolvable dynamic src_path.
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\nset mpip [interp create -safe]\nset mpip \"not an interpreter\"\ninterp alias $mpip greet {} ::app::Helper\n";
        assert!(
            command_alias_targets(src).is_empty(),
            "{:?}",
            command_alias_targets(src)
        );
    }

    #[test]
    fn existing_literal_top_level_behaviour_is_unchanged() {
        // TN — byte-for-byte unchanged control (scratchpad/audit/a): the
        // same shape as the primary repro, but with a literal path
        // throughout instead of a tracked variable binding.
        let src = "namespace eval ::app {}\nproc ::app::Helper {} { return 42 }\ninterp create -safe s\ninterp alias s greet {} ::app::Helper\ninterp eval s { greet }\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn builtins_only_dynamic_interp_eval_body_stays_silent() {
        // TN — a tracked dynamic-path eval body containing only builtins
        // must stay diagnostic-free, confirming the fix doesn't newly
        // over-fire (scratchpad/audit/c).
        let src = "set s [interp create -safe]\ninterp eval $s {\n    set x 1\n    expr {$x + 1}\n    puts \"hi\"\n}\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    #[test]
    fn builtins_only_literal_interp_eval_body_stays_silent() {
        // TN — the same false-fire (W210 on `x`) reproduced with a *literal*
        // interpreter path, no dynamic tracking involved at all: `interp
        // eval`'s script argument is a `Plain`-body-kind `ArgRole::Body`
        // (unlike an `if`/`while`/`catch` body, never flattened into its
        // own CFG since the target interpreter is opaque to static
        // analysis), so it was scanned as ordinary value text with no
        // notion of its own `set x 1` write before the later `$x` read.
        let src = "interp create child\ninterp eval child {\n    set x 1\n    expr {$x + 1}\n}\n";
        assert!(codes(src, D).is_empty(), "{:?}", codes(src, D));
    }

    /// The `@interp@<key>` synthetic namespaces a source's definitions
    /// land in — the observable projection of the interpreter-domain key.
    fn interp_domains(src: &str) -> Vec<String> {
        let mut keys: Vec<String> = Analyser::new()
            .analyse(src, D)
            .all_procs
            .keys()
            .filter_map(|k| k.split("::").find(|s| s.starts_with("@interp@")))
            .map(str::to_owned)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn braced_path_value_flow_key_matches_the_direct_handler_issue_1025() {
        // TP — `interp create {child}` names interpreter `child`
        // (tclsh8.6/9.0: `interp exists child` → 1, `interp slaves` →
        // `child`). The value-flow path must not `split_whitespace` the raw
        // substitution text: doing so would bind `$i` to `"{child}"`, a
        // domain the direct handler never records.
        let via_value = "set i [interp create {child}]\n$i eval { proc helper {} {} }\n";
        let direct = "interp create {child}\ninterp eval child { proc helper {} {} }\n";
        assert_eq!(
            interp_domains(via_value),
            interp_domains(direct),
            "value-flow and direct keys must agree"
        );
        assert_eq!(interp_domains(via_value), vec!["@interp@child".to_string()]);
    }

    #[test]
    fn nested_path_value_flow_key_matches_the_direct_handler_issue_1025() {
        // TP — `{parent child}` is one word: a descent path (tclsh8.6/9.0:
        // `interp create {parent child}` returns `parent child` and
        // `interp slaves parent` → `child`). `split_whitespace` must not
        // split it into `"{parent"` + `"child}"` and bind the first
        // fragment. A nested path is not a command in the parent
        // (`$i eval` raises `invalid command name "parent child"`), so the
        // value reaches the child through `interp eval $i`.
        let via_value = "interp create parent\nset i [interp create {parent child}]\ninterp eval $i { proc helper {} {} }\n";
        let direct = "interp create parent\ninterp create {parent child}\ninterp eval {parent child} { proc helper {} {} }\n";
        assert_eq!(interp_domains(via_value), interp_domains(direct));
        assert!(
            interp_domains(via_value).contains(&"@interp@parent child".to_string()),
            "{:?}",
            interp_domains(via_value)
        );
    }

    #[test]
    fn braced_path_binding_links_a_later_literal_eval_issue_1025() {
        // TP — the linkage the diverging key broke: a proc defined through
        // the tracked `$i eval` handle must resolve for a later literal
        // `interp eval child` call, with no W123.
        let src = "set i [interp create {child}]\n$i eval { proc helper {} { return 1 } }\ninterp eval child { helper }\n";
        assert!(!fires(src, D, "W123"), "{:?}", codes(src, D));
        assert!(!fires(src, D, "W140"), "{:?}", codes(src, D));
    }

    #[test]
    fn synthetic_autoname_key_used_for_pathless_interp_create() {
        // TP — pins the synthetic per-call-site key convention (mirrors
        // `handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name`'s
        // `@dynns@15` pin): a pathless `interp create` inside `set` gets an
        // `@autoname@<offset-of-the-substitution's-opening-bracket>` key,
        // not the variable's raw text, so `foo`'s definition homes under a
        // synthetic, call-site-unique domain rather than one keyed on `s`.
        let src = "set s [interp create -safe]\ninterp eval $s { proc foo {} {} }\n";
        let r = Analyser::new().analyse(src, D);
        assert!(
            r.all_procs.keys().any(|k| k.contains("@autoname@6")),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }
}

// rename — constant-folding a dynamic-but-resolvable
// `rename OLD NEW` argument instead of unconditionally giving up.
pub(super) mod rename {
    use super::super::*;

    fn renamed_commands(src: &str) -> std::collections::HashMap<String, String> {
        Analyser::new().analyse(src, D).renamed_commands.clone()
    }

    #[test]
    fn static_rename_recorded_unchanged() {
        // TN — control check: a fully literal rename never even reaches
        // the new constant-folding resolver and must keep working
        // byte-for-byte as before.
        let src = "proc ::foo_impl {} { return impl }\nrename ::foo_impl ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn single_var_old_name_resolves_via_constant_folding() {
        // TP — finding's own simplest repro #1: a single whole-word
        // `$var` holding a known-constant command name.
        let src = "proc ::foo_impl {} { return impl }\nset old ::foo_impl\nrename $old ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn concatenated_var_new_name_resolves_via_constant_folding() {
        // TP — finding's own simplest repro #2: a literal-plus-`$var`
        // concatenation.
        let src = "proc ::foo_impl {} { return impl }\nset key impl\nrename ::foo_$key ::foo\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn two_concatenated_vars_in_same_straight_line_scope_resolve() {
        // TP — both OLD and NEW built from resolvable variables in the
        // same straight-line proc scope, the closest Tier-1-fixable
        // approximation of the tcllib `json::SwitchTo` idiom (which
        // additionally needs a `foreach` var and a proc parameter — see
        // the two FN tests below, deliberately out of scope for Tier 1).
        let src = "namespace eval ::mypkg {}\n\
             proc ::mypkg::greet_tcl {name} { return \"hi $name\" }\n\
             proc ::mypkg::activate {} {\n    \
                 set c greet\n    \
                 set key tcl\n    \
                 rename ::mypkg::${c}_$key ::mypkg::${c}\n\
             }\n\
             ::mypkg::activate\n\
             ::mypkg::greet x\n";
        // tclsh8.6.18/9.0.4: `::mypkg::greet` exists only once `activate`
        // has run; the later call then answers `hi x`.
        assert_eq!(
            renamed_commands(src).get("::mypkg::greet"),
            Some(&"::mypkg::greet_tcl".to_string())
        );
        let r = Analyser::new().analyse(src, D);
        assert!(
            !r.diagnostics.iter().any(|d| d.code.to_string() == "W123"),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn genuinely_dynamic_value_stays_unresolved() {
        // TN — regression guard against over-eager folding: a value
        // that's never a compile-time constant (piped through `gets`)
        // must still be reported dynamic (no entry recorded).
        let src = "proc ::foo_impl {} { return impl }\nset old [gets stdin]\nrename $old ::foo\n";
        assert!(!renamed_commands(src).contains_key("::foo"));
    }

    #[test]
    fn foreach_loop_variable_over_a_literal_list_is_constant_folded() {
        // A `foreach VAR {literal list} { ... }` loop over a fully
        // literal list must bind `VAR` to each element in turn before
        // simulating the body's own `rename`/`proc` sub-commands (the two
        // constant-fold-sensitive callers go-to-definition/references/
        // rename care about), rather than leaving the loop variable out of
        // the constant-string lattice entirely. `renamed_commands` is
        // populated the same as if `c` had been a plain top-level `set`
        // constant.
        let src =
            "proc ::foo_impl {} { return impl }\nforeach c {foo} { rename ::${c}_impl ::${c} }\n";
        assert_eq!(
            renamed_commands(src).get("::foo"),
            Some(&"::foo_impl".to_string())
        );
    }

    #[test]
    fn foreach_over_a_braced_list_element_is_not_mis_split_on_whitespace() {
        // The literal-`foreach` simulation must parse its value as a real
        // Tcl list, not with `split_whitespace`: a braced element
        // `{bar baz}` in `foreach c {a {bar baz}} { proc ::$c {} {} }` is
        // exactly two elements — `a` and `bar baz` — never `{bar` + `baz}`,
        // so the re-dispatched `proc` never creates bogus commands
        // `::{bar` / `::baz}` and no brace-fragment proc is ever recorded.
        let procs = Analyser::new()
            .analyse("foreach c {a {bar baz}} { proc ::$c {} {} }\n", D)
            .all_procs;
        assert!(
            !procs.keys().any(|k| k.contains('{') || k.contains('}')),
            "no brace-fragment proc from a mis-split braced element: {:?}",
            procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn rename_target_set_only_in_a_conditional_branch_does_not_resolve() {
        // A `rename`/`source` target read from the last-write-wins const map
        // is unsound across an `if` join. Here `t`
        // is `::foo` straight-line but reassigned `::bar` inside an `if`
        // body, so at the `rename` it could be either — the analyser must
        // abstain (leave the rename dynamic), never pin it to the branch
        // value `::bar` the lexical map happens to hold last.
        let renamed = renamed_commands(
            "proc ::foo_impl {} {}\nset t ::foo\nif {$cond} { set t ::bar }\nrename ::foo_impl $t\n",
        );
        assert!(
            !renamed.contains_key("::bar") && !renamed.contains_key("::foo"),
            "a branch-dependent rename target must not resolve to either branch: {renamed:?}"
        );
    }

    #[test]
    fn rename_target_set_straight_line_still_resolves() {
        // Control for the above: without the conditional, the single
        // straight-line `set` dominates the `rename`, so it still resolves.
        let renamed =
            renamed_commands("proc ::foo_impl {} {}\nset t ::foo\nrename ::foo_impl $t\n");
        assert_eq!(renamed.get("::foo"), Some(&"::foo_impl".to_string()));
    }

    /// A name built from a parameter folds from the call that reaches the
    /// `rename`. tclsh 8.4.20 to 9.1.0 print `impl` for the first source
    /// followed by `puts [foo]`; a second call with another argument raises
    /// `can't rename "::foo_other": command doesn't exist` and leaves `foo`
    /// in place; with no call there is no `foo`.
    #[test]
    fn a_rename_through_a_parameter_folds_from_the_call_that_reaches_it() {
        let declared = "proc ::foo_impl {} { return impl }\n\
             proc activate {key} { rename ::foo_$key ::foo }\n";
        for (calls, renamed) in [
            ("activate impl\n", true),
            ("activate impl\nactivate other\n", true),
            ("", false),
        ] {
            let src = format!("{declared}{calls}");
            assert_eq!(
                renamed_commands(&src).get("::foo").map(String::as_str),
                renamed.then_some("::foo_impl"),
                "{calls:?}"
            );
        }
    }

    #[test]
    fn resolvable_dynamic_rename_no_longer_widens_has_dynamic_providers() {
        // TN: a dynamic-*looking* rename that resolves statically must not
        // set `has_dynamic_providers` — which otherwise blanket-suppresses
        // W123 for the whole file (diagnostics/unresolved.rs) — so W123
        // keeps firing on genuinely unknown commands elsewhere in the file.
        let src = "proc ::foo_impl {} { return impl }\nset old ::foo_impl\nrename $old ::foo\n";
        assert!(!Analyser::new().analyse(src, D).has_dynamic_providers);
    }

    #[test]
    fn unresolvable_rename_still_widens_has_dynamic_providers() {
        // TN — control check for the above: a genuinely unresolvable
        // dynamic rename must still widen `has_dynamic_providers`.
        let src = "proc ::foo_impl {} { return impl }\nset old [gets stdin]\nrename $old ::foo\n";
        assert!(Analyser::new().analyse(src, D).has_dynamic_providers);
    }
}
