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

//! Representative commands of each family: control flow, string, dict and list operations, I/O, namespaces, arrays, clock and iRules.

use super::*;

// TARGETED — representative commands across families (distinct spec shapes).

/// Control flow: `if` / `while` / `for` / `foreach` / `switch` carry the
/// control-flow + boolean-condition + loop-body trait shapes.
///
/// tclsh: these are all real commands (`info commands` non-empty on 8.6 & 9.0).
/// registry-metadata: the trait classification.
#[test]
fn family_control_flow_shapes() {
    let reg = registry_for_dialect("tcl8.6");
    for c in ["if", "while", "for", "foreach", "switch"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(
            spec.traits.contains(Traits::CONTROL_FLOW),
            "{c} CONTROL_FLOW"
        );
    }
    // Loop constructs carry a loop body; `if`/`switch` do not.
    for c in ["while", "for", "foreach"] {
        assert!(
            reg.get(c).unwrap().traits.contains(Traits::HAS_LOOP_BODY),
            "{c} HAS_LOOP_BODY"
        );
    }
    // The two derived answers that replaced retired command-level flags. Both
    // read a descriptor a consumer needs anyway, so neither can disagree with
    // it the way the flags did — `expect` declared a clause list without ever
    // carrying `HAS_SWITCH_BODY`, and `array` had a destructive subcommand
    // without `HAS_DESTRUCTIVE_OPS`.
    assert!(reg.get("switch").unwrap().has_switch_body());
    assert!(!reg.get("if").unwrap().has_switch_body());
    assert!(
        reg.get("file").unwrap().has_destructive_ops(),
        "`file delete` is irreversible"
    );
    assert!(
        reg.get("array").unwrap().has_destructive_ops(),
        "`array unset` is irreversible, and the retired flag missed it"
    );
    assert!(!reg.get("string").unwrap().has_destructive_ops());
    // tclsh: `switch --` terminator is real — `switch -- abc {abc {...}}` runs.
    // The registry models the terminator via resolve_option_terminator.
    let term = reg.resolve_option_terminator(
        "switch",
        &["--", "abc"],
        Some(SurfaceQuery::core(Family::Tcl, "8.6")),
    );
    if let Some(t) = term {
        assert!(
            t.options.iter().any(|o| o.name == "--"),
            "switch declares a -- option"
        );
    }
}

/// `resolve_option_terminator` must accept a unique subcommand prefix the same
/// way ensemble dispatch (and `arg_indices_for_role`) does, so an abbreviated
/// subcommand keeps its subcommand-scoped `--` terminator profile.
#[test]
fn terminator_resolves_subcommand_prefix() {
    let reg = registry_for_dialect("f5-irules");
    // `class match` (F5 iRules) declares a subcommand-scoped `--` terminator.
    let exact = reg
        .resolve_option_terminator(
            "class",
            &["match", "-nocase", "--"],
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .expect("class match declares a -- terminator");
    assert_eq!(exact.subcommand, Some("match"));
    assert!(exact.options.iter().any(|o| o.name == "--"));

    // The unique prefix `ma` must resolve to the same `match` profile.
    let abbrev = reg
        .resolve_option_terminator(
            "class",
            &["ma", "-nocase", "--"],
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .expect("abbreviated `class ma` must keep the -- terminator");
    assert_eq!(abbrev.subcommand, Some("match"));
    assert_eq!(abbrev.scan_start, exact.scan_start);
    assert!(abbrev.options.iter().any(|o| o.name == "--"));
}

/// String / dict / list operations: subcommand-bearing ensemble shape.
///
/// tclsh: the `string` / `dict` subcommand sets, and the `string is` class set,
/// are exactly what tclsh reports (probed via `catch {string is bogus}` etc.).
#[test]
fn family_string_dict_list_ops() {
    // tclsh8.6 & 9.0: `string is bogus` lists these classes (8.6∩9.0).
    const CLASSES: &[&str] = &[
        "alnum", "alpha", "ascii", "boolean", "control", "digit", "double", "false", "integer",
        "list", "lower", "space", "true", "upper", "xdigit",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let string = reg.get("string").expect("string");
    let is = string.subcommand("is").expect("string is");
    let allowed: BTreeSet<&str> = is.arg_values_at(0).iter().map(|v| v.value).collect();
    let missing: Vec<&str> = CLASSES
        .iter()
        .copied()
        .filter(|c| !allowed.contains(c))
        .collect();
    assert!(
        missing.is_empty(),
        "string is missing classes from its arg-0 value set: {missing:?} (have {allowed:?})"
    );

    // tclsh: `dict for` / `dict map` are real subcommands and declare their CFG
    // rewrite names (registry-metadata for the rewrite name).
    let dict = reg.get("dict").expect("dict");
    for sub in ["for", "map", "get", "set", "keys", "values"] {
        assert!(dict.subcommand(sub).is_some(), "dict {sub}");
    }

    // List ops: `lsort` carries the switch set tclsh reports.
    // tclsh8.6 & 9.0: `lsort -bogus` lists -ascii … -unique (8.6∩9.0 below).
    let lsort = reg.get("lsort").expect("lsort");
    let sw = lsort.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    for opt in [
        "-ascii",
        "-decreasing",
        "-increasing",
        "-integer",
        "-nocase",
        "-unique",
    ] {
        assert!(sw.contains(&opt), "lsort missing {opt}: {sw:?}");
    }
}

/// `lsearch` option version-gating.
///
/// tclsh9.0 (9.0.3): `lsearch -bogus` lists `… -start, -stride, -subindices` —
/// i.e. `-stride` IS accepted.
/// tclsh8.6 (8.6.14): the same probe lists `… -start, or -subindices` — NO
/// `-stride`; a functional `lsearch -stride 2 …` is hard-rejected with
/// "bad option \"-stride\"". So in *real* Tcl, `lsearch -stride` is **9.0-only**
/// (distinct from `lsort -stride`, which is genuinely 8.6+ / TIP 351).
///
/// `commands/tcl/lsearch_.rs` therefore gates `-stride` to `TCL90`, not to
/// `TCL86_PLUS`: the 8.6+ switch of that name is `lsort`'s TIP 351 `-stride`,
/// a different command's option. So `switch_names(TCL86)` does not report it —
/// matching tclsh8.6.14.
#[test]
fn lsearch_stride_version_gating() {
    let reg = CommandRegistry::build_default();
    let lsearch = reg.get("lsearch").expect("lsearch");
    let in_86 = lsearch.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    let in_90 = lsearch.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    // Common options present in both (these match tclsh on 8.6 & 9.0).
    for opt in ["-exact", "-glob", "-regexp", "-all", "-inline"] {
        assert!(in_86.contains(&opt), "8.6 lsearch missing {opt}: {in_86:?}");
    }
    // 9.0 accepts -stride — matches tclsh9.0.3.
    assert!(
        in_90.contains(&"-stride"),
        "9.0 lsearch should accept -stride: {in_90:?}"
    );
    // 8.6 must NOT — real tclsh8.6.14 hard-rejects `lsearch -stride`.
    assert!(
        !in_86.contains(&"-stride"),
        "8.6 lsearch must not accept -stride (real tclsh8.6.14 rejects it): {in_86:?}"
    );
}

/// I/O family: `socket` / `open` / `close` / `puts` / `gets`.
///
/// tclsh: `socket -server` is a real switch (`catch {socket -server}` reports
/// "no argument given for -server option" on 8.6 & 9.0). `open` opens a
/// channel; `close` is file I/O in plain Tcl.
#[test]
fn family_io_shapes() {
    let reg = registry_for_dialect("tcl8.6");

    // tclsh: socket -server / -myaddr.
    let socket = reg.get("socket").expect("socket");
    let sw = socket.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(sw.contains(&"-server"), "socket -server: {sw:?}");
    // socket needs host+port at minimum.
    assert_eq!(socket.arity.min, 2, "socket min arity");
    assert!(
        socket.arity.is_unlimited(),
        "socket accepts trailing options"
    );

    // open opens a channel (registry-metadata trait).
    assert!(
        reg.get("open")
            .unwrap()
            .traits
            .contains(Traits::OPENS_CHANNEL),
        "open OPENS_CHANNEL"
    );

    // close is file I/O in plain Tcl (registry-metadata classification of a real
    // command).
    let close = reg.get("close").expect("close");
    assert!(
        close
            .side_effects
            .lower_bound()
            .iter()
            .any(|e| e.target == SideEffectTarget::FileIo),
        "plain-Tcl close is file I/O: {:?}",
        close.side_effects
    );
}

/// Namespace / proc family: scope-alias + procedure-definition shapes.
///
/// tclsh: `namespace` subcommands match tclsh (`catch {namespace bogus}` lists
/// children…which on 8.6 & 9.0). `proc` defines a procedure.
#[test]
fn family_namespace_proc_shapes() {
    // tclsh8.6 & 9.0: namespace subcommand set (8.6∩9.0 — identical on both).
    const NS_SUBS: &[&str] = &[
        "children",
        "code",
        "current",
        "delete",
        "ensemble",
        "eval",
        "exists",
        "export",
        "forget",
        "import",
        "inscope",
        "origin",
        "parent",
        "path",
        "qualifiers",
        "tail",
        "unknown",
        "upvar",
        "which",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let ns = reg.get("namespace").expect("namespace");
    let missing: Vec<&str> = NS_SUBS
        .iter()
        .copied()
        .filter(|s| ns.subcommand(s).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "namespace missing subcommands: {missing:?}"
    );

    // proc defines a procedure and marks its body argument structural.
    let proc = reg.get("proc").expect("proc");
    assert!(
        proc.traits.contains(Traits::DEFINES_PROCEDURE),
        "proc DEFINES_PROCEDURE"
    );
    assert_eq!(proc.arity, Arity::exact(3), "proc name args body");
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body), "proc body role");

    // upvar / global / variable create scope aliases.
    for c in ["upvar", "global", "variable"] {
        assert!(
            reg.get(c)
                .unwrap()
                .traits
                .contains(Traits::CREATES_SCOPE_ALIAS),
            "{c} CREATES_SCOPE_ALIAS"
        );
    }
}

/// `array` / `clock`: subcommand sets that match tclsh exactly (intersection
/// of 8.6 and 9.0, which differ slightly for `array`).
///
/// tclsh8.6: `array bogus` lists anymore, donesearch, exists, get, names,
/// nextelement, set, size, startsearch, statistics, unset.
/// tclsh9.0: same plus `default` / `for`.  The set below is the 8.6∩9.0
/// intersection (the 8.6 list).
/// tclsh (both): `clock bogus` lists add, clicks, format, microseconds,
/// milliseconds, scan, seconds.
#[test]
fn family_array_clock_subcommands() {
    const ARRAY_SUBS: &[&str] = &[
        "anymore",
        "donesearch",
        "exists",
        "get",
        "names",
        "nextelement",
        "set",
        "size",
        "startsearch",
        "statistics",
        "unset",
    ];
    const CLOCK_SUBS: &[&str] = &[
        "add",
        "clicks",
        "format",
        "microseconds",
        "milliseconds",
        "scan",
        "seconds",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let array = reg.get("array").expect("array");
    let missing: Vec<&str> = ARRAY_SUBS
        .iter()
        .copied()
        .filter(|s| array.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "array missing subcommands: {missing:?}");

    let clock = reg.get("clock").expect("clock");
    let missing: Vec<&str> = CLOCK_SUBS
        .iter()
        .copied()
        .filter(|s| clock.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "clock missing subcommands: {missing:?}");
}

/// F5 iRules family: the distinct iRules spec shapes (getter/setter forms,
/// side switches, event handler, closed value args, excluded events).
///
/// f5-dialect: none of these are real tclsh commands; all assertions are
/// registry metadata.
#[test]
fn family_f5_irules_shapes() {
    let reg = registry_for_dialect("f5-irules");
    let ds = Some(SurfaceQuery::any_release(Family::F5Irules));

    // when is the event handler with a structural body.
    let when = reg.get_for_surface("when", ds).expect("when");
    assert!(
        when.traits.contains(Traits::IS_EVENT_HANDLER),
        "when IS_EVENT_HANDLER"
    );

    // HTTP::uri getter (no args, PURE source) vs setter (one arg) forms.
    let getter = reg
        .resolve_call("HTTP::uri", &[], ds)
        .expect("HTTP::uri getter");
    assert!(
        getter.spec.traits.contains(Traits::PURE),
        "HTTP::uri getter is pure"
    );
    let setter = reg
        .resolve_call("HTTP::uri", &["/p"], ds)
        .expect("HTTP::uri setter");
    assert_eq!(setter.spec.name, "HTTP::uri");

    // Side switches.
    for name in ["clientside", "serverside", "peer"] {
        assert!(reg.is_side_switch(name), "{name} side switch");
    }

    // Closed value arg: HTTP::version arg 0 is a closed version set.
    let ver = reg
        .get_for_surface("HTTP::version", ds)
        .expect("HTTP::version");
    assert!(
        ver.closed_value_args.contains(&0),
        "HTTP::version arg0 closed"
    );
    let versions: BTreeSet<&str> = ver.arg_values_at(0).iter().map(|v| v.value).collect();
    assert!(
        versions.contains("1.1"),
        "HTTP::version closed set has 1.1: {versions:?}"
    );

    // Excluded events: TCP::rcv_scale declares at least one excluded event, and
    // is illegal there.
    let rcv = reg
        .get_for_surface("TCP::rcv_scale", ds)
        .expect("TCP::rcv_scale");
    assert!(
        !rcv.excluded_events.is_empty(),
        "TCP::rcv_scale excluded_events"
    );
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let ex = rcv.excluded_events[0];
    assert!(
        !reg.is_irules_command_legal_in_event("TCP::rcv_scale", ex, &events, &profiles),
        "TCP::rcv_scale must be illegal in excluded event {ex}"
    );
}
