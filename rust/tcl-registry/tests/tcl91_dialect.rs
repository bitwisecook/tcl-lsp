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

//! Tcl 9.1 dialect sync — verified against the **C Tcl 9.1b0 source**
//! (`tmp/tcl9.1-src`, `TCL_VERSION "9.1"`): the built-in command table in
//! `generic/tclBasic.c` and `doc/{unicode,timer,subst,ledit,coroutine,divmod,
//! frexp,modf,remquo,lfilter}.n`.  These assert registry facts (dialect
//! membership, command availability, side effects) that are not directly
//! observable over LSP; the observable completion / W003 behaviour is covered
//! by the `lsp_e2e` suite.

use tcl_dialect::available_dialects;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::surface_admits;
use tcl_dialect::model::{Family, SurfaceQuery};
use tcl_registry::CommandRegistry;

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[test]
fn tcl91_is_a_known_catalogued_dialect() {
    assert_eq!(
        tcl_dialect::DialectProfile::find("tcl9.1").map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::core(Family::Tcl, "9.1"))
    );
    assert!(available_dialects().contains(&"tcl9.1"));
}

#[test]
fn tcl90_plus_admits_91_and_90_but_not_86() {
    let at = |release| SurfaceQuery::core(Family::Tcl, release);
    // A `.1` release is additive: 9.0 features persist in 9.1.
    assert!(surface_admits(SpecSurface::TCL90_PLUS, Some(&at("9.0"))));
    assert!(surface_admits(SpecSurface::TCL90_PLUS, Some(&at("9.1"))));
    assert!(!surface_admits(SpecSurface::TCL90_PLUS, Some(&at("8.6"))));
    // The 8.x "and later" windows absorb 9.1 too.
    assert!(surface_admits(SpecSurface::TCL86_PLUS, Some(&at("9.1"))));
    assert!(surface_admits(SpecSurface::ALL_TCL, Some(&at("9.1"))));
}

#[test]
fn unicode_is_91_only_with_normalization_subcommands() {
    // doc/unicode.n (9.1): `unicode function ?options? string`, functions
    // tonfc/tonfd/tonfkc/tonfkd, each `?-profile profile?`.  Absent in 9.0.
    let r = reg();
    let spec = r.get("unicode").expect("unicode registered");
    assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
    assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
    assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))));
    for name in ["tonfc", "tonfd", "tonfkc", "tonfkd"] {
        let sub = spec
            .subcommand(name)
            .unwrap_or_else(|| panic!("unicode {name}"));
        assert!(sub.pure, "unicode {name} is a pure normalization");
        assert!(sub.options.iter().any(|o| o.name == "-profile"));
    }
}

#[test]
fn timer_is_91_only_with_scheduler_side_effects() {
    use tcl_registry::side_effects::SideEffectTarget;
    // doc/timer.n (9.1): subcommands in/at/idle/sleep/cancel/info.  Scheduling
    // an event mutates interpreter state (same model as `after`); `info` is a
    // pure query.
    let r = reg();
    let spec = r.get("timer").expect("timer registered");
    assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
    assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
    for name in ["in", "at", "idle", "sleep", "cancel", "info"] {
        assert!(spec.subcommand(name).is_some(), "timer {name}");
    }
    assert!(spec.subcommand("info").unwrap().pure);
    assert!(spec.subcommand("in").unwrap().mutator);
    assert!(
        spec.side_effects
            .iter()
            .any(|e| e.target == SideEffectTarget::InterpState && e.writes),
        "timer schedules events → InterpState write (like `after`)",
    );
}

#[test]
fn subst_positive_forms_are_91_only() {
    // doc/subst.n (9.1): `subst ?-backslashes? ?-commands? ?-variables? string`
    // enable only the named substitution; the negated forms exist in all
    // versions.
    let r = reg();
    let spec = r.get("subst").expect("subst registered");
    // Negated forms: available everywhere (present in the unfiltered set and
    // under 9.0).
    let in_90 = spec.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    for name in ["-nobackslashes", "-nocommands", "-novariables"] {
        assert!(in_90.contains(&name), "{name} available in all dialects");
    }
    // Positive forms are gated to 9.1.
    let in_91 = spec.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.1")));
    for name in ["-backslashes", "-commands", "-variables"] {
        assert!(in_91.contains(&name), "subst {name} in 9.1");
        assert!(!in_90.contains(&name), "subst {name} NOT in 9.0");
    }
}

#[test]
fn tcl90_commands_persist_in_91() {
    // doc/ledit.n / lseq.n: 9.0 additions remain valid in 9.1.
    let r = reg();
    for name in ["lseq", "ledit", "lpop", "lremove", "readFile", "writeFile"] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))),
            "{name} available in 9.1"
        );
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            "{name} available in 9.0"
        );
        assert!(
            !spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            "{name} NOT in 8.6"
        );
    }
}

#[test]
fn tcl91_math_commands_are_pure_and_91_only() {
    // C Tcl 9.1b0 generic/tclBasic.c: DivModObjCmd / FracExpObjCmd / ModFObjCmd
    // / RemQuoObjCmd — pure numeric ops returning a two-element list.  The C
    // source is the authoritative reference for these.
    use tcl_registry::traits::Traits;
    use tcl_registry::types::TclType;
    let r = reg();
    for name in ["divmod", "frexp", "modf", "remquo"] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))),
            "{name} in 9.1"
        );
        assert!(
            !spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            "{name} NOT in 9.0"
        );
        assert!(spec.traits.contains(Traits::PURE), "{name} is pure");
        assert_eq!(spec.return_type, Some(TclType::List));
    }
    // `divmod x y` / `remquo x y` take 2 args; `frexp value` / `modf value` 1.
    assert!(r.get("divmod").unwrap().arity.accepts(2));
    assert!(!r.get("divmod").unwrap().arity.accepts(1));
    assert!(r.get("frexp").unwrap().arity.accepts(1));
    assert!(!r.get("frexp").unwrap().arity.accepts(2));
}

#[test]
fn lfilter_is_a_91_list_loop() {
    // C Tcl 9.1b0 generic/tclBasic.c: `Tcl_LfilterObjCmd` / `TclCompileLfilterCmd`
    // — a byte-compiled loop (like `lmap`) returning the filtered sublist.
    use tcl_registry::traits::Traits;
    use tcl_registry::types::TclType;
    let r = reg();
    let spec = r.get("lfilter").expect("lfilter registered");
    assert!(spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.1"))));
    assert!(!spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))));
    assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
    assert!(spec.traits.contains(Traits::LOOP_LIST_HEADER));
    assert_eq!(spec.return_type, Some(TclType::List));
    // `lfilter varname list body` — the last (body) arg carries `ArgRole::Body`.
    let roles = r.arg_indices_for_role(
        "lfilter",
        &["v", "{1 2 3}", "$v"],
        tcl_registry::arg_role::ArgRole::Body,
    );
    assert_eq!(roles, vec![2], "the predicate body is a Body arg");
}

#[test]
fn coroutine_probe_and_inject_run_arbitrary_code() {
    // doc/coroutine.n (9.1): `coroprobe` evaluates a command in the coroutine
    // now; `coroinject` schedules one for the next resume.  Both run arbitrary
    // code → an Unknown read+write effect (like `eval` / `uplevel`), which was
    // previously unencoded.
    use tcl_registry::side_effects::SideEffectTarget;
    let r = reg();
    for name in ["coroprobe", "coroinject"] {
        let spec = r.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            spec.side_effects
                .iter()
                .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
            "{name} runs arbitrary code → Unknown read+write effect",
        );
    }
}
