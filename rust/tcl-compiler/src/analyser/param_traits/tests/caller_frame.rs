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

//! The parameters and literal targets bound in the caller's frame.

use super::*;

/// The frame level the trait map cannot record.  Pinned against tclsh
/// 9.0.4 and 8.6.14 (byte-identical): with `proc q {n} {upvar L $n a; set
/// a 1}` called as `q y` from a proc, the caller's `y` exists afterwards
/// only for `L` = `1` (or an omitted level).  `0` aliases the callee's
/// own frame, `#0` the global one, `2` the caller's caller.
#[test]
fn caller_frame_upvar_params_accepts_only_the_caller_frame_level() {
    let params = ["n"];
    for (body, expected) in [
        ("upvar 1 $n a; set a 1", true),
        ("upvar $n a; set a 1", true),
        ("upvar +1 $n a", true),
        ("upvar 0x1 $n a", true),
        ("upvar 0 $n a; set a 1", false),
        ("upvar #0 $n a; set a 1", false),
        ("upvar 2 $n a; set a 1", false),
        ("upvar -1 $n a", false),
        ("upvar $lvl $n a", false),
        ("upvar bogus $n a", false),
        // `namespace upvar` aliases a namespace variable, never the
        // calling frame.
        ("namespace upvar ::cfg $n a", false),
        // The `myVar` slot names a callee-local alias, not a caller
        // variable.
        ("upvar 1 caller $n", false),
    ] {
        let got = with_receipt(&params, body, |env| {
            caller_frame_upvar_params(&params, body, env)
        });
        assert_eq!(
            got.contains("n"),
            expected,
            "{body:?} -> {got:?} (expected caller-frame param: {expected})"
        );
    }
}

/// The literal-target fact — `upvar 1 name name` binds the
/// caller's `name`, written through the alias when the body assigns it.
/// tclsh 9.0.4: `proc np {} {upvar name name; set name W1}` then
/// `np; puts $name` prints `W1` in the caller.
#[test]
fn caller_frame_literal_targets_records_written_and_read_only() {
    let targets =
        |body: &str| with_receipt(&[], body, |env| caller_frame_literal_targets(body, env));
    let got = targets("upvar name name\nset name W1");
    assert_eq!(got.get("name"), Some(&true), "written through the alias");
    // A body that only reads through the alias creates nothing.
    let got = targets("upvar name name\nreturn [string length $name]");
    assert_eq!(got.get("name"), Some(&false), "read-only alias");
    // Distinct local alias spelling still records the CALLER name.
    let got = targets("upvar 1 other mine\nset mine 1");
    assert_eq!(got.get("other"), Some(&true));
    assert!(!got.contains_key("mine"));
}

/// The exclusions: non-caller levels, `::`-qualified cells, array
/// elements, substituted sources, dynamic locals, `namespace upvar`.
#[test]
fn caller_frame_literal_targets_excludes_other_frames_and_cells() {
    let registry = CommandRegistry::build_default();
    for body in [
        "upvar 0 name name; set name 1",
        "upvar #0 name name; set name 1",
        "upvar 2 name name; set name 1",
        "upvar $lvl name name; set name 1",
        "upvar 1 ::tk::FocusGrab($i) data; set data 1",
        "upvar 1 ::ns::cell local; set local 1",
        "upvar 1 arr(k) local; set local 1",
        "upvar 1 $src local; set local 1",
        "upvar 1 name $dst",
        "namespace upvar ::cfg name local; set local 1",
    ] {
        let got = caller_frame_literal_targets(body, env_for(&registry, LexerConfig::default()));
        assert!(got.is_empty(), "{body:?} must record nothing, got {got:?}");
    }
}

/// The fact is strictly narrower than the trait: `upvar 0 $n a; set a 1`
/// still records `VarWrite` (the parameter's value *is* used as a
/// variable name through an `upvar`) while contributing no caller-frame
/// parameter — which is exactly why a call-site consumer must intersect
/// the two.
#[test]
fn caller_frame_params_is_narrower_than_the_var_traits() {
    let registry = CommandRegistry::build_default();
    let body = "upvar 0 $n a; set a 1";
    let traits = infer(&["n"], body);
    assert_trait(&traits, "n", ProcArgTrait::VarWrite);
    assert!(
        caller_frame_upvar_params(&["n"], body, env_for(&registry, LexerConfig::default()))
            .is_empty(),
        "the level fact must reject what the trait alone accepts"
    );
}

/// `upvar $lvl a b` has three words, so parity puts `$lvl` in the level
/// slot and `(a, b)` in the pair slot — the level is unplaceable, so no
/// caller-frame parameter is claimed, while the *pair* handling (and hence
/// the trait map) still applies.
#[test]
fn dynamic_level_word_keeps_parity_pairing_but_claims_no_frame() {
    let registry = CommandRegistry::build_default();
    let traits = infer(&["lvl", "v"], "upvar $lvl $v local\nset local 1");
    assert_trait(&traits, "v", ProcArgTrait::VarWrite);
    assert!(
        caller_frame_upvar_params(
            &["lvl", "v"],
            "upvar $lvl $v local\nset local 1",
            env_for(&registry, LexerConfig::default())
        )
        .is_empty(),
        "a computed level names no statically-known frame"
    );
}

/// The caller-frame scans read the resolved head too: `upvar`'s
/// `FrameArgLayout::AliasPairs` grammar belongs to the command a head is.
#[test]
fn caller_frame_scans_follow_the_resolved_head() {
    let registry = CommandRegistry::build_default();
    let params_under = |prelude: &str, body: &str| {
        let (identities, span) = realm_for_body(prelude, body);
        caller_frame_upvar_params(
            &["body", "v"],
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, None),
                config: LexerConfig::default(),
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
                identities: &identities,
            },
        )
    };
    let literals_under = |prelude: &str, body: &str| {
        let (identities, span) = realm_for_body(prelude, body);
        caller_frame_literal_targets(
            body,
            TraitScanEnv {
                surface: DocumentCommandSurface::new(&registry, None),
                config: LexerConfig::default(),
                executed_source: identities.executed_script_for_word(span),
                original_logical_body: None,
                identities: &identities,
            },
        )
    };
    let alias = "interp alias {} peek {} upvar\n";
    assert!(
        params_under(alias, "peek 1 $v alias").contains("v"),
        "an alias of upvar must select the caller frame"
    );
    assert!(literals_under(alias, "peek 1 name name\nset name W1").contains_key("name"));
    let shadow = "proc upvar {a b c} { return 1 }\n";
    assert!(params_under(shadow, "upvar 1 $v alias").is_empty());
    assert!(literals_under(shadow, "upvar 1 name name\nset name W1").is_empty());
    assert!(params_under("", "upvar 1 $v alias").contains("v"));
    assert!(literals_under("", "upvar 1 name name\nset name W1").contains_key("name"));
}

#[test]
fn caller_frame_parameter_templates_reach_navigation_without_entered_caller() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "proc setter {d} {upvar 1 $d local; set local VALUE}\nproc caller {} {setter target; puts $target}";
        let mut whole = crate::analyser::Analyser::new();
        let mut per_item = crate::analyser::Analyser::new();
        for analysis in [
            whole.analyse(source, dialect),
            per_item.analyse_per_item(source, dialect),
        ] {
            let setter = analysis.all_procs.get("::setter").expect("original setter");
            assert!(setter.caller_frame_params.contains("d"), "{dialect}");
            assert_trait(&setter.param_traits, "d", ProcArgTrait::VarWrite);
        }
        let replaced = "proc setter {d} {set d OTHER; upvar 1 $d local; set local VALUE}\nproc caller {} {setter target; puts $target}";
        let mut analyser = crate::analyser::Analyser::new();
        let analysis = analyser.analyse(replaced, dialect);
        let setter = analysis.all_procs.get("::setter").expect("original setter");
        assert!(!setter.caller_frame_params.contains("d"), "{dialect}");
        assert!(
            !setter
                .param_traits
                .get("d")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite)),
            "{dialect}"
        );
    }
}
