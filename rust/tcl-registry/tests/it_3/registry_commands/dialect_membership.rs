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

//! Per-dialect membership: lazy dialect loading, signatures per dialect, dialect names, version gates and the tclsh subcommand sets.

use super::*;

// Lazy dialect loading

/// A freshly built default registry has core commands but not iRules / non-Tk
/// dialect commands.
///
/// Tk is folded into the base registry by design (a `.tcl` may `package
/// require Tk`), so `button` IS present — a deliberate design choice here:
/// Tk is not gated behind a separate lazy loader.
#[test]
fn default_registry_has_core_but_not_irules() {
    let reg = CommandRegistry::build_default();
    assert!(reg.get("set").is_some(), "set is in the base registry");
    assert!(
        reg.get("HTTP::header").is_none(),
        "iRules commands are not in the default registry"
    );
    // Rust folds Tk into the base registry (documented divergence).
    assert!(
        reg.get("button").is_some(),
        "Tk is part of the base registry"
    );
}

/// Tk widget/window commands state `SpecSurface::TK_AND_TCL` — they must
/// resolve in a `wish` / `package require Tk` `.tcl` file but never in the
/// restricted embedded environments (F5 iRules / iApps), where Tk cannot be
/// loaded at all.  (The *loaded* gating — only offered once `package require
/// Tk` ran — is layered on in the LSP completion path, not here.)
#[test]
fn tk_commands_are_gated_to_tcl_and_tk_not_irules_or_iapps() {
    let reg = CommandRegistry::build_default();
    for name in ["button", "pack", "wm", "winfo", "ttk::treeview", "tkwait"] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert_eq!(spec.required_package, Some("Tk"), "{name} requires Tk");
        // Available in standard Tcl (a `.tcl` that loads Tk) and the `tk`
        // dialect.
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            "{name} available under tcl8.6 (wish / package require Tk)"
        );
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            "{name} available under tcl9.0"
        );
        assert!(
            spec.supports_dialect(Some(
                SurfaceQuery::any_release(Family::Tcl).with_packages(&[PackageFloor::named("Tk")])
            )),
            "{name} available under the tk dialect"
        );
        // NOT available in the F5 embedded environments.  Tk's rows admit
        // core Tcl, and iApps *is* an 8.4 core — so the exclusion is a fact
        // about the environment (Tk cannot be loaded there), not about the
        // point.  Asking the context is asking the honest question.
        for environment in ["f5-irules", "f5-iapps"] {
            assert!(
                !static_document_context_for(environment).spec_available(spec),
                "{name} must NOT be offered in {environment}"
            );
        }
    }
}

/// `load_surface` makes iRules commands resolvable and listed.
///
/// f5-dialect.
#[test]
fn load_dialect_makes_irules_commands_visible() {
    let mut reg = CommandRegistry::build_default();
    assert!(reg.get("HTTP::header").is_none());
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    assert!(reg.get("HTTP::header").is_some());
    let names: std::collections::HashSet<&str> = reg.command_names().collect();
    assert!(names.contains("HTTP::header"));
}

/// `load_surface` is idempotent.
///
/// f5-dialect / registry behaviour.
#[test]
fn load_dialect_is_idempotent() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let after_first = reg.len();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, "")); // second load is a no-op
    assert_eq!(
        reg.len(),
        after_first,
        "double-load must not duplicate specs"
    );
}

// Signature membership per dialect

/// `dict`/`try`/`tailcall` are absent in 8.4.
///
/// tclsh: `dict` (8.5), `try`/`tailcall` (8.6) — all post-date 8.4. We can't
/// run tclsh8.4 here (not installed), but these introduction versions are
/// well-known Tcl history and match the 8.5/8.6 listings observed above.
#[test]
fn tcl84_excludes_newer_commands() {
    let (reg, ds) = reg_and_set("tcl8.4");
    assert!(reg.get_for_surface("dict", ds).is_none(), "dict is 8.5+");
    assert!(reg.get_for_surface("try", ds).is_none(), "try is 8.6+");
    assert!(
        reg.get_for_surface("tailcall", ds).is_none(),
        "tailcall is 8.6+"
    );
}

/// `dict` present in 8.5, `try` absent.
///
/// tclsh: `dict` exists in 8.5+ (present in both 8.6 & 9.0 subcommand probes);
/// `try` is 8.6+.
#[test]
fn tcl85_has_dict_not_try() {
    let (reg, ds) = reg_and_set("tcl8.5");
    assert!(reg.get_for_surface("dict", ds).is_some());
    assert!(reg.get_for_surface("try", ds).is_none());
}

/// `TclOO` (`oo::class`, `oo::define`) is 8.6+.
///
/// tclsh: `oo::class` is present on 8.6 (`info commands oo::class` non-empty);
/// `TclOO` landed in 8.6.
#[test]
fn tcloo_is_tcl86_plus() {
    let (r86, d86) = reg_and_set("tcl8.6");
    assert!(r86.get_for_surface("oo::class", d86).is_some());
    assert!(r86.get_for_surface("oo::define", d86).is_some());

    let (r85, d85) = reg_and_set("tcl8.5");
    assert!(r85.get_for_surface("oo::class", d85).is_none());
    assert!(r85.get_for_surface("oo::define", d85).is_none());
}

/// The iRules profile pulls in `when`, the HTTP family, the seeded AAA catalogue,
/// the `class`/`table` generated commands, and deprecated profile commands.
///
/// f5-dialect: none of these are real tclsh commands.
#[test]
fn f5_irules_profile_membership() {
    let (reg, ds) = reg_and_set("f5-irules");
    for name in [
        "when",
        "HTTP::header",
        "AAA::acct_result",
        "class",
        "table",
        "PROFILE::list",
        "SSL::cert",
        "TCP::collect",
        "UDP::payload",
        "HTTP::path",
    ] {
        assert!(
            reg.get_for_surface(name, ds).is_some(),
            "{name} should be in the f5-irules profile"
        );
    }
}

/// `proc` keeps its core arity (exactly 3) and body role even under the iRules
/// profile.
///
/// tclsh: `proc name args body` is exactly 3 args. registry-metadata: the
/// body role.
#[test]
fn f5_irules_keeps_base_proc_signature() {
    let (reg, ds) = reg_and_set("f5-irules");
    let proc = reg.get_for_surface("proc", ds).expect("proc registered");
    assert_eq!(proc.arity, Arity::exact(3));
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body));
}

/// Curated iRules commands have concrete arity and subcommands.
///
/// f5-dialect.
#[test]
fn f5_irules_curated_signatures_are_concrete() {
    let (reg, ds) = reg_and_set("f5-irules");
    let header = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert!(header.subcommand("value").is_some());
    assert!(header.subcommand("insert").is_some());

    let when = reg.get_for_surface("when", ds).expect("when registered");
    assert_eq!(when.arity.max, 6);
    assert_eq!(reg.get_for_surface("pool", ds).expect("pool").arity.min, 1);
    assert_eq!(reg.get_for_surface("node", ds).expect("node").arity.min, 1);
    assert_eq!(
        reg.get_for_surface("HTTP::respond", ds)
            .expect("HTTP::respond")
            .arity
            .min,
        1
    );
}

/// The f5-iapps profile has `iapp::template` / `iapp::conf` but NOT the iRules
/// catalogue.
///
/// f5-dialect.
#[test]
fn f5_iapps_profile_membership() {
    let (reg, ds) = reg_and_set("f5-iapps");
    assert!(reg.get_for_surface("iapp::template", ds).is_some());
    assert!(reg.get_for_surface("iapp::conf", ds).is_some());
    // f5-iapps is a separate catalogue from f5-irules.
    assert!(
        reg.get_for_surface("AAA::acct_result", ds).is_none(),
        "iRules catalog must not leak into f5-iapps"
    );
}

/// The expect profile adds `spawn`/`expect`/… on top of base Tcl and excludes
/// iRules.
///
/// expect is a real Tcl extension; `spawn`/`expect`/`send`/`interact` are its
/// commands (not in bare tclsh, but part of the `expect` interpreter). The
/// dialect partitioning is registry behaviour.
#[test]
fn expect_profile_membership() {
    let (reg, ds) = reg_and_set("expect");
    for name in ["spawn", "expect", "send", "interact", "log_user"] {
        assert!(
            reg.get_for_surface(name, ds).is_some(),
            "{name} should be in the expect profile"
        );
    }
    // Base Tcl is still present.
    for name in ["set", "proc", "if"] {
        assert!(reg.get_for_surface(name, ds).is_some(), "{name} (base tcl)");
    }
    // iRules commands are not.
    assert!(reg.get_for_surface("when", ds).is_none());
    assert!(reg.get_for_surface("HTTP::header", ds).is_none());
}

/// `when EVENT … { body }` marks the trailing body argument.
///
/// f5-dialect + registry-metadata (the BODY role).
#[test]
fn when_marks_trailing_body_argument() {
    let (reg, _) = reg_and_set("f5-irules");
    // when EVENT { body }  → body at index 1
    let simple = reg.arg_indices_for_role("when", &["HTTP_REQUEST", "{ puts ok }"], ArgRole::Body);
    assert_eq!(simple, vec![1]);
    // when EVENT priority N { body } → body at index 3
    let extended = reg.arg_indices_for_role(
        "when",
        &["CLIENT_ACCEPTED", "priority", "500", "{ puts ok }"],
        ArgRole::Body,
    );
    assert_eq!(extended, vec![3]);
}

/// The `TclOO` definition bodies resolve to the BODY role.
///
/// registry-metadata: arg-role classification. (`TclOO` is real 8.6 Tcl.)
#[test]
fn tcloo_definition_bodies_marked_body() {
    let (reg, _) = reg_and_set("tcl8.6");
    // oo::class create Dog { defScript } → body at index 2
    let create = reg.arg_indices_for_role(
        "oo::class",
        &["create", "Dog", "{ method bark {} { puts ok } }"],
        ArgRole::Body,
    );
    assert!(create.contains(&2), "create body: {create:?}");

    // oo::define Dog method bark {} { body } → body at index 4
    let method = reg.arg_indices_for_role(
        "oo::define",
        &["Dog", "method", "bark", "{}", "{ puts ok }"],
        ArgRole::Body,
    );
    assert!(method.contains(&4), "method body: {method:?}");

    // oo::define Dog { defScript } → body at index 1
    let script = reg.arg_indices_for_role(
        "oo::define",
        &["Dog", "{ method bark {} { puts ok } }"],
        ArgRole::Body,
    );
    assert!(script.contains(&1), "script body: {script:?}");
}

/// The BODY role on a proc helper — `proc helper x { body }` marks the body
/// argument.
///
/// registry-metadata.
#[test]
fn proc_marks_body_argument() {
    let (reg, _) = reg_and_set("tcl8.6");
    let bodies = reg.arg_indices_for_role("proc", &["helper", "x", "{ return $x }"], ArgRole::Body);
    assert_eq!(bodies, vec![2]);
}

// `is_irules_dialect` predicate

/// The iRules predicate accepts `f5-irules` and the legacy `irules` alias,
/// rejects everything else and `None`.
///
/// registry-metadata: dialect-name predicate.
#[test]
fn is_irules_dialect_predicate() {
    assert!(tcl_dialect::DialectProfile::name_is_irules(Some(
        "f5-irules"
    )));
    assert!(tcl_dialect::DialectProfile::name_is_irules(Some("irules")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some("tcl8.6")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some(
        "f5-iapps"
    )));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some(
        "f5-bigip"
    )));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(None));
}

// Dialect-name surface (the part that lives in this crate)
//
// `detect_dialect_from_source` itself is implemented in tcl-compiler /
// tcl-lsp-core, not tcl-registry, so the source-scanning behaviour is out of
// scope here. The dialect *vocabulary* it resolves into — and which this
// crate owns — is covered instead.

/// The detection targets (`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `f5-irules`,
/// `expect`) that dialect detection produces as outputs are all
/// parseable dialect names in this crate.
///
/// registry-metadata: dialect vocabulary.
#[test]
fn detection_target_dialects_are_known() {
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "expect",
    ] {
        assert!(
            tcl_dialect::DialectProfile::find(d)
                .map(tcl_dialect::DialectProfile::surface_query)
                .is_some(),
            "{d} should parse"
        );
    }
    // The detection targets are catalogued names.
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "expect",
    ] {
        assert!(
            tcl_dialect::DialectProfile::find(d).is_some(),
            "{d} should be a known dialect"
        );
    }
    // `tcl-dialect: unknown` has no parse — the analogue of returning `None`.
    assert!(
        tcl_dialect::DialectProfile::find("unknown")
            .map(tcl_dialect::DialectProfile::surface_query)
            .is_none()
    );
}

/// `DialectProfile::find` round-trips the canonical names and rejects junk —
/// underpinning the cache key normalisation a typo'd dialect collapses to.
///
/// registry-metadata.
#[test]
fn dialect_name_resolves_to_its_point() {
    assert_eq!(
        tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::core(Family::Tcl, "8.6"))
    );
    assert_eq!(
        tcl_dialect::DialectProfile::find("tcl9.0").map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::core(Family::Tcl, "9.0"))
    );
    assert_eq!(
        tcl_dialect::DialectProfile::find("f5-irules")
            .map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::any_release(Family::F5Irules))
    );
    assert_eq!(
        tcl_dialect::DialectProfile::find("expect").map(tcl_dialect::DialectProfile::surface_query),
        Some(
            SurfaceQuery::core(Family::Tcl, "8.6").with_packages(&[PackageFloor::named("expect")])
        )
    );
    assert_eq!(
        tcl_dialect::DialectProfile::find("definitely-not-a-dialect")
            .map(tcl_dialect::DialectProfile::surface_query),
        None
    );
}

// C-Tcl ground-truth: subcommand sets of string / dict / info match tclsh.
//
// These are direct tclsh facts. The registry must register at least the
// subcommands common to tclsh8.6 AND tclsh9.0 (the version-specific extras —
// 8.6's `string bytelength`, 9.0's `string insert` / `dict getdef` /
// `info cmdtype` — are registry-superset entries and are not required here).

/// `string` subcommands.
///
/// tclsh8.6: `string` subcommands = bytelength, cat, compare, equal, first,
/// index, is, last, length, map, match, range, repeat, replace, reverse,
/// tolower, totitle, toupper, trim, trimleft, trimright, wordend, wordstart.
/// tclsh9.0: same minus `bytelength`, plus `insert`.
/// The set below is the 8.6∩9.0 intersection.
#[test]
fn string_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "cat",
        "compare",
        "equal",
        "first",
        "index",
        "is",
        "last",
        "length",
        "map",
        "match",
        "range",
        "repeat",
        "replace",
        "reverse",
        "tolower",
        "totitle",
        "toupper",
        "trim",
        "trimleft",
        "trimright",
        "wordend",
        "wordstart",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("string").expect("string registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "string missing subcommands: {missing:?}"
    );
}

/// `dict` subcommands.
///
/// tclsh8.6: dict subcommands = append, create, exists, filter, for, get,
/// incr, info, keys, lappend, map, merge, remove, replace, set, size, unset,
/// update, values, with. tclsh9.0: same plus getdef / getwithdefault. The set
/// below is the 8.6∩9.0 intersection.
#[test]
fn dict_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "append", "create", "exists", "filter", "for", "get", "incr", "info", "keys", "lappend",
        "map", "merge", "remove", "replace", "set", "size", "unset", "update", "values", "with",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("dict").expect("dict registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "dict missing subcommands: {missing:?}");
}

/// `info` subcommands.
///
/// tclsh8.6: info subcommands = args, body, class, cmdcount, commands,
/// complete, coroutine, default, errorstack, exists, frame, functions,
/// globals, hostname, level, library, loaded, locals, nameofexecutable,
/// object, patchlevel, procs, script, sharedlibextension, tclversion, vars.
/// tclsh9.0 adds cmdtype / constant / consts. The set below is the 8.6∩9.0
/// intersection (i.e. the 8.6 list).
#[test]
fn info_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "args",
        "body",
        "class",
        "cmdcount",
        "commands",
        "complete",
        "coroutine",
        "default",
        "errorstack",
        "exists",
        "frame",
        "functions",
        "globals",
        "hostname",
        "level",
        "library",
        "loaded",
        "locals",
        "nameofexecutable",
        "object",
        "patchlevel",
        "procs",
        "script",
        "sharedlibextension",
        "tclversion",
        "vars",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("info").expect("info registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "info missing subcommands: {missing:?}");
}

/// C-Tcl: `regsub` switch set is dialect-versioned.
///
/// tclsh8.6: `catch {regsub -bogus …}` → "must be -all, -nocase, -expanded,
/// -line, -linestop, -lineanchor, -start, or --". tclsh9.0 adds `-command`.
/// So `-command` must be 9.0-only and `-all`/`-nocase` must be in both.
#[test]
fn regsub_switches_are_version_gated() {
    let reg = CommandRegistry::build_default();
    let regsub = reg.get("regsub").expect("regsub registered");
    let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(in_86.contains(&"-all"), "8.6: {in_86:?}");
    assert!(in_86.contains(&"-nocase"), "8.6: {in_86:?}");
    assert!(
        !in_86.contains(&"-command"),
        "9.0-only -command leaked into 8.6: {in_86:?}"
    );
    let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert!(in_90.contains(&"-command"), "9.0: {in_90:?}");
}

/// C-Tcl: `source -nopkg` is a Tcl 9.0-only flag.
///
/// tclsh8.6: `catch {source -nopkg missing.tcl} message` reports a wrong
/// number of arguments. tclsh9.0 accepts the flag, then reports that the file
/// does not exist. The different errors establish both the parser contract and
/// the version boundary without needing a source file.
#[test]
fn source_nopkg_is_version_gated() {
    let reg = CommandRegistry::build_default();
    // Select the release's descriptor before reading its options: the default
    // catalogue also contains Jim's independently registered source command.
    let switches = |family, release| {
        let query = Some(SurfaceQuery::core(family, release));
        reg.get_for_surface("source", query)
            .expect("source registered for the selected release")
            .switch_names(query)
    };
    let in_86 = switches(Family::Tcl, "8.6");
    assert!(
        !in_86.contains(&"-nopkg"),
        "Tcl 9-only -nopkg leaked into 8.6: {in_86:?}"
    );
    let in_90 = switches(Family::Tcl, "9.0");
    assert!(in_90.contains(&"-nopkg"), "9.0: {in_90:?}");
    let in_91 = switches(Family::Tcl, "9.1");
    assert!(in_91.contains(&"-nopkg"), "9.1: {in_91:?}");
    let in_jim = switches(Family::Jim, "0.84");
    assert!(in_jim.is_empty(), "Jim source takes no options: {in_jim:?}");
}

/// C-Tcl: `const` is a Tcl 9.0 builtin, present in 9.0.
///
/// tclsh8.6: `info commands const` → empty. tclsh9.0: → `const`.
///
/// `const` (Tcl 9.0, TIP 677) is present from 9.0 and — after the
/// registry-wide explicit-dialect sweep — correctly ABSENT before it. It used
/// to be declared `surface: None` (universal) as a workaround to reach iRules
/// events, which wrongly made `get_for_surface("const", TCL86)` return `Some`
/// even though real tclsh8.6 lacks the command. It is now `TCL90_PLUS`, so the
/// registry matches C-Tcl on both axes: present in 9.0/9.1, absent in 8.x (and
/// absent from iRules, whose embedded 8.4.6 also predates it).
#[test]
fn const_is_present_in_tcl90_and_absent_before() {
    assert!(
        present_in("tcl9.0", "const"),
        "const must be present in tcl9.0"
    );
    assert!(
        present_in("tcl9.1", "const"),
        "const must be present in tcl9.1"
    );
    let (reg86, ds86) = reg_and_set("tcl8.6");
    assert!(
        reg86.get_for_surface("const", ds86).is_none(),
        "const is a Tcl 9.0 command; it must not resolve under tcl8.6"
    );
}

// Registry construction basics (the `build_default` checks)

/// The cached per-dialect registries are non-empty and round-trip a lookup.
///
/// registry behaviour.
#[test]
fn cached_dialect_registries_are_populated() {
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "f5-iapps",
        "expect",
    ] {
        let reg = static_context_for(d).commands();
        assert!(!reg.is_empty(), "{d} registry is empty");
        assert!(reg.get("set").is_some(), "{d} should have `set`");
    }
    // An unparseable dialect collapses to the plain-Tcl entry (still usable).
    let junk = static_context_for("not-a-real-dialect").commands();
    assert!(junk.get("set").is_some());
    assert!(
        junk.get("HTTP::header").is_none(),
        "the fallback registry is plain Tcl"
    );
}

/// The report package requires Tcl 8.5+, so its commands are gated out of the
/// tcl8.4 dialect (version restriction across the 8.4→9.1 range).
#[test]
fn report_commands_gated_out_of_tcl84() {
    for cmd in ["report::report", "report::defstyle", "report::styles"] {
        assert!(!present_in("tcl8.4", cmd), "{cmd} unavailable under tcl8.4");
        for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert!(present_in(d, cmd), "{cmd} available under {d}");
        }
    }
}

/// The `::tcl::` namespace does not exist at all before Tcl 8.5 (TIP 174
/// added `::tcl::mathop`; there is no `::tcl` namespace in a real Tcl 8.4).
/// Every dated addition under it must be gated to its real introduction
/// release, not left universally available (`surface: None`), which would
/// wrongly resolve under `tcl8.4` and every pre-that-version profile.
#[test]
fn tcl_namespace_commands_gated_to_their_real_introduction_version() {
    // (command, dialects it must resolve under, dialects it must not)
    let cases: &[(&str, &[&str], &[&str])] = &[
        // Tcl Modules (TIP 189) — 8.5+.
        (
            "tcl::tm::path",
            &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4"],
        ),
        (
            "tcl::tm::roots",
            &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4"],
        ),
        // Coroutines (TIP 396) and their `corotype` introspection — 8.6+;
        // empirically confirmed present on a real tclsh 8.6.14, not 9.0
        // only as an earlier (incorrect) version of this data claimed.
        (
            "::tcl::unsupported::corotype",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        // `cookiejar` package's IDNA helpers — bundled since Tcl 8.6.
        (
            "tcl::idna::decode",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        (
            "tcl::idna::encode",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        // `::tcl::build-info` — Tcl 9.0+ (both spellings carry their own spec).
        (
            "tcl::build-info",
            &["tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5", "tcl8.6"],
        ),
        (
            "::tcl::build-info",
            &["tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5", "tcl8.6"],
        ),
    ];
    for &(cmd, present, absent) in cases {
        for d in present {
            assert!(present_in(d, cmd), "{cmd} must be available under {d}");
        }
        for d in absent {
            assert!(!present_in(d, cmd), "{cmd} must be unavailable under {d}");
        }
    }
}
