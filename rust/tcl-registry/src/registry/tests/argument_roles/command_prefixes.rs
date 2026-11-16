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

//! Command-prefix arguments and their arities, and which Tk options and bindings take a prefix or a script body.

use super::*;

#[test]
fn command_prefix_option_is_captured_but_never_a_body() {
    // `lsort -command cmp {a b}` — the `-command` value is a CommandPrefix,
    // so it is captured under that role but never returned for a
    // Body query, i.e. a bareword prefix is not recursed as a script.
    let reg = CommandRegistry::build_default();
    let args = ["-command", "cmp", "{a b}"];
    assert!(
        reg.arg_indices_for_role("lsort", &args, ArgRole::Body)
            .is_empty(),
        "a command prefix must not be recursed as a body",
    );
    assert_eq!(
        reg.arg_indices_for_role("lsort", &args, ArgRole::CommandPrefix),
        vec![1],
        "the -command value should carry the CommandPrefix role",
    );
}

#[test]
fn command_prefixes_carry_verified_arities() {
    // Ground-truthed vs real tclsh (stable 8.6→9.0). `command_prefixes`
    // is the single source of truth for both the position and the
    // appended arity.
    let reg = CommandRegistry::build_default();
    // `lsort -command cmp {a b}` → cmp invoked as `cmp x y` (2 appended).
    assert_eq!(
        reg.command_prefixes("lsort", &["-command", "cmp", "{a b}"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // `socket -server accept 9000` → accept invoked as `accept ch a p` (3).
    assert_eq!(
        reg.command_prefixes("socket", &["-server", "accept", "9000"]),
        vec![(1, AppendedArity::Exactly(3))],
    );
    // Positional: `tcltest::customMatch mode cmd`
    // → `cmd expected actual` (2).
    assert_eq!(
        reg.command_prefixes("tcltest::customMatch", &["exact", "cmp"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // Dynamic resolver: `selection handle window cmd` → the
    // last arg, invoked as `cmd offset maxChars` (2).
    assert_eq!(
        reg.command_prefixes("selection", &["handle", ".w", "getData"]),
        vec![(2, AppendedArity::Exactly(2))],
    );
}

#[test]
fn command_prefixes_cover_core_callback_commands() {
    // Ground-truthed vs real tclsh 8.6/9.0 (stable). Locks in the
    // coverage of trace / interp / tcllib callbacks.
    let reg = CommandRegistry::build_default();
    // `trace add variable v w cb` → cb(name1 name2 op) = 3.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "variable", "v", "write", "cb"]),
        vec![(4, AppendedArity::Exactly(3))],
    );
    // `trace add command c ops cb` → cb(old new op) = 3.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "command", "c", "rename", "cb"]),
        vec![(4, AppendedArity::Exactly(3))],
    );
    // Execution operations select exact callback contracts.
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "enter", "cb"]),
        vec![(4, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "leave", "cb"]),
        vec![(4, AppendedArity::Exactly(4))],
    );
    assert_eq!(
        reg.command_prefixes("trace", &["add", "execution", "c", "enter leave", "cb"]),
        vec![(
            4,
            AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 4]))
        )],
    );
    // Deprecated `trace variable v ops cb` → cb at index 3, 3 appended args.
    assert_eq!(
        reg.command_prefixes("trace", &["variable", "v", "write", "cb"]),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `interp alias {} a {} target x` (create form) → target at index 3,
    // variadic. The 2-arg query form has no target.
    assert_eq!(
        reg.command_prefixes("interp", &["alias", "{}", "a", "{}", "target"]),
        vec![(4, AppendedArity::Unknown)],
    );
    assert!(
        reg.command_prefixes("interp", &["alias", "{}", "a"])
            .is_empty(),
        "the interp-alias query form has no command prefix",
    );
    // tcllib: `struct::list filter seq cb` (1), `map seq cb` (1),
    // `fold seq init cb` (2).
    assert_eq!(
        reg.command_prefixes("struct::list", &["filter", "$s", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("struct::list", &["fold", "$s", "0", "cb"]),
        vec![(3, AppendedArity::Exactly(2))],
    );
}

#[test]
fn command_prefixes_cover_deferred_core_commands() {
    // Version-gated / resolver-driven callback tails wired in the deferred
    // pass. Ground-truthed vs tclsh 9.0 (all are 8.7/9.0-era surfaces).
    let reg = CommandRegistry::build_default();

    // `regsub -command re str cmdPrefix ?var?` (TIP 463, 9.0): with
    // `-command` the subSpec slot is a prefix called per match with the
    // whole match + capture groups appended (variadic ⇒ AtLeast(1)). The
    // slot index tracks the leading-switch shift.
    assert_eq!(
        reg.command_prefixes("regsub", &["-command", "re", "s", "cb"]),
        vec![(3, AppendedArity::AtLeast(1))],
    );
    assert_eq!(
        reg.command_prefixes("regsub", &["-all", "-command", "re", "s", "cb"]),
        vec![(4, AppendedArity::AtLeast(1))],
    );
    // `-c` is NOT an abbreviation of `-command`: regsub's switch table
    // resolves with Tcl_GetIndexFromObj(..., TCL_EXACT, ...), confirmed
    // live (tclsh 8.6.14: `regsub -c {a} aaa X y` -> `bad option "-c"`)
    // and against the real `Tcl_RegsubObjCmd` C source. Only the exact
    // spelling `-command` enables command-prefix mode.
    assert_eq!(
        reg.command_prefixes("regsub", &["-c", "re", "s", "cb"]),
        Vec::new(),
    );
    // Without `-command`, subSpec is a replacement template, not a prefix.
    assert!(
        reg.command_prefixes("regsub", &["re", "s", "template"])
            .is_empty(),
        "plain regsub subSpec is not a command prefix",
    );
    // `--` terminates switches, so a following `-command`-looking word is a
    // pattern, not the flag.
    assert!(
        reg.command_prefixes("regsub", &["--", "-command", "s", "template"])
            .is_empty(),
        "`--` disables -command detection",
    );

    // `namespace unknown handler` → handler(cmd ?arg...?) = AtLeast(1). The
    // zero-arg query form carries no prefix.
    assert_eq!(
        reg.command_prefixes("namespace", &["unknown", "handler"]),
        vec![(1, AppendedArity::AtLeast(1))],
    );
    assert!(
        reg.command_prefixes("namespace", &["unknown"]).is_empty(),
        "the namespace-unknown query form has no command prefix",
    );

    // `package unknown handler` → handler(name requirement ?requirement
    // ...?) = AtLeast(2): verified empirically on tclsh 8.6.14 that Tcl
    // always appends the package name *plus* at least one requirement
    // word, synthesising a "0-" placeholder when `package require` was
    // given none itself — never just the bare name, so AtLeast(2), not
    // AtLeast(1) (see the fuller note on `package_.rs`'s `unknown`
    // subcommand).
    assert_eq!(
        reg.command_prefixes("package", &["unknown", "handler"]),
        vec![(1, AppendedArity::AtLeast(2))],
    );
    assert!(
        reg.command_prefixes("package", &["unknown"]).is_empty(),
        "the package-unknown query form has no command prefix",
    );

    // `coroinject coroName command ?arg...?` — per the Tcl 9.0/9.1
    // coroutine(n) manpage, exactly two more words are appended when the
    // injected command runs: the name of the command that suspended the
    // coroutine (yield or yieldto) and its current resumption value.
    assert_eq!(
        reg.command_prefixes("coroinject", &["myCoro", "cb", "x"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    // `coroprobe coroName command ?arg...?` runs command immediately
    // inside the suspended coroutine (not deferred through a yield/
    // yieldto resumption), so no fixed extra-argument count is
    // documented -- it stays a reference-only prefix (Unknown).
    assert_eq!(
        reg.command_prefixes("coroprobe", &["myCoro", "cb"]),
        vec![(1, AppendedArity::Unknown)],
    );

    // `chan create mode cmdPrefix` / `chan push channelId cmdPrefix` — the
    // reflected-channel/transform handler is invoked as `cmdPrefix
    // subcommand handle ?args...?` ⇒ AtLeast(2).
    assert_eq!(
        reg.command_prefixes("chan", &["create", "rw", "handler"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );
    assert_eq!(
        reg.command_prefixes("chan", &["push", "$ch", "xform"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );
}

#[test]
fn command_prefixes_cover_tcllib_callbacks() {
    // tcllib callback tails wired in the deferred pass.  Fixed arities are
    // ground-truthed against the package man pages; ambiguous/variadic tails
    // are Unknown (reference-only).
    let reg = CommandRegistry::build_default();

    // `struct::list split seq cmdprefix` — the twin of filter: cmdprefix(el).
    assert_eq!(
        reg.command_prefixes("struct::list", &["split", "$s", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    // `fileutil::find basedir filtercmd` — filtercmd(name).  Shorter forms
    // carry no prefix (idx≥argc dropped).
    assert_eq!(
        reg.command_prefixes("fileutil::find", &["/base", "flt"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert!(
        reg.command_prefixes("fileutil::find", &["/base"])
            .is_empty(),
        "the basedir-only fileutil::find form has no filter prefix",
    );

    // generator functional ops: reference-only (multi-value yield ⇒ Unknown).
    assert_eq!(
        reg.command_prefixes("generator", &["map", "fn", "$g"]),
        vec![(1, AppendedArity::Unknown)],
    );
    assert_eq!(
        reg.command_prefixes("generator", &["filter", "pred", "$g"]),
        vec![(1, AppendedArity::Unknown)],
    );

    // math::calculus func callbacks (man-page-pinned fixed arities).
    assert_eq!(
        reg.command_prefixes("math::calculus::integral", &["0", "1", "100", "f"]),
        vec![(3, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("math::calculus::integral3D", &["xi", "yi", "zi", "f"]),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `newtonRaphson func deriv initval` — two prefixes, each f(x).
    assert_eq!(
        reg.command_prefixes("math::calculus::newtonRaphson", &["f", "d", "0.5"]),
        vec![
            (0, AppendedArity::Exactly(1)),
            (1, AppendedArity::Exactly(1)),
        ],
    );
    // `math::probopt::pso function bounds ?args?` — objective(coordVec).
    assert_eq!(
        reg.command_prefixes("math::probopt::pso", &["obj", "bnds", "-iter", "50"]),
        vec![(0, AppendedArity::Exactly(1))],
    );

    // log message-writer callbacks — `cmd level text` (Exactly(2)).
    assert_eq!(
        reg.command_prefixes("log::lvCmd", &["debug", "writer"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("log::lvCmdForall", &["writer"]),
        vec![(0, AppendedArity::Exactly(2))],
    );
    // `uevent::bind tag event command` — command(tag event ?details?).
    assert_eq!(
        reg.command_prefixes("uevent::bind", &["tag", "ev", "cb"]),
        vec![(2, AppendedArity::AtLeast(2))],
    );

    // `hook bind subject hook observer binding` — the 4-word set form names
    // a command prefix (Unknown appended: the count is whatever the matching
    // `hook call` passes); the shorter query forms name none.
    assert_eq!(
        reg.command_prefixes("hook", &["bind", "sub", "hk", "obs", "cb"]),
        vec![(4, AppendedArity::Unknown)],
    );
    assert!(
        reg.command_prefixes("hook", &["bind", "sub", "hk", "obs"])
            .is_empty(),
        "the 3-arg `hook bind` query form has no callback prefix",
    );

    // `processman::onexit id cmd` — `cmd` is a deferred *script* (`eval $cmd`,
    // 0 appended), NOT a command prefix: it must declare none.
    assert!(
        reg.command_prefixes("processman::onexit", &["$pid", "cb"])
            .is_empty(),
        "processman::onexit cmd is a script body, not a command prefix",
    );
    assert_eq!(
        reg.get("processman::onexit")
            .and_then(|s| s.arg_roles.iter().find(|(i, _)| *i == 1))
            .map(|(_, r)| *r),
        Some(ArgRole::Body),
        "processman::onexit cmd (index 1) must carry the Body script role",
    );
}

#[test]
fn command_prefixes_cover_option_value_callbacks() {
    // Option-value callbacks — the prefix is the value of a named `-flag`,
    // resolved through each command's `OptionSpec` array.  Arities are
    // ground-truthed against tcllib source (verified by re-running the exact
    // `uplevel`/`eval`/`{*}` idioms in tclsh — `uplevel`/`eval` re-split a
    // `[list …]` word, `[list {*}pfx {*}args]` keeps one word per element).
    let reg = CommandRegistry::build_default();

    // `mime::getbody token -command cb` — async body callback.  uplevel
    // re-splits `[list end]` → 1 word, `[list data $c]` → 2 ⇒ AtLeast(1).
    assert_eq!(
        reg.command_prefixes("mime::getbody", &["tok", "-command", "cb"]),
        vec![(2, AppendedArity::AtLeast(1))],
    );
    // `-decode` before `-command` is a value-less flag: the scan skips it
    // without swallowing the callback.
    assert_eq!(
        reg.command_prefixes("mime::getbody", &["tok", "-decode", "-command", "cb"]),
        vec![(3, AppendedArity::AtLeast(1))],
    );

    // `smtp::sendmessage tok -tlspolicy pol` — `eval $pol [list $code]
    // [list $diag]` ⇒ Exactly(2).  A preceding value-taking option's value
    // is skipped, not mistaken for the callback.
    assert_eq!(
        reg.command_prefixes("smtp::sendmessage", &["tok", "-tlspolicy", "pol"]),
        vec![(2, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes(
            "smtp::sendmessage",
            &["tok", "-servers", "mail.x", "-tlspolicy", "pol"],
        ),
        vec![(4, AppendedArity::Exactly(2))],
    );

    // `comm::comm send -command cb id cmd` — 7 `-key value` reply pairs ⇒
    // Exactly(14).  Subcommand-relative option scan offsets by 1.
    assert_eq!(
        reg.command_prefixes("comm::comm", &["send", "-command", "cb", "id", "cmd"]),
        vec![(2, AppendedArity::Exactly(14))],
    );

    // `bibtex::parse` — every callback prepends the parser token, then its
    // own payload words: `-command`/`-*command` ⇒ Exactly(2), except
    // `-recordcommand` ⇒ Exactly(4) (token type key recdata).
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-recordcommand", "cb", "text"]),
        vec![(1, AppendedArity::Exactly(4))],
    );
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-command", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("bibtex::parse", &["-progresscommand", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );

    // `tcl::chan::halfpipe` — clean `[list {*}pfx {*}args]` idiom: write
    // appends (chan bytes) ⇒ 2, empty/close append (chan) ⇒ 1.  No
    // `-read-command` exists.
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-write-command", "cb"]),
        vec![(1, AppendedArity::Exactly(2))],
    );
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-close-command", "cb"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-empty-command", "cb"]),
        vec![(1, AppendedArity::Exactly(1))],
    );
    assert!(
        reg.command_prefixes("tcl::chan::halfpipe", &["-read-command", "cb"])
            .is_empty(),
        "halfpipe has no -read-command",
    );
}

#[test]
fn instance_method_command_prefixes_cover_struct_graph_and_tree() {
    // Object-instance method callbacks — the prefix is on a method of a
    // created object command (`$g walk … -command cb`, `$t walkproc … cb`),
    // resolved through the class's ObjectClassSpec.  Indices are relative to
    // the words after the method name.
    let reg = CommandRegistry::build_default();

    // struct::graph `walk node … -command cb` — option-value prefix,
    // Exactly(3) (action graphName node).
    assert_eq!(
        reg.instance_method_command_prefixes(
            "struct::graph",
            "walk",
            &["root", "-order", "pre", "-command", "cb"],
        ),
        vec![(4, AppendedArity::Exactly(3))],
    );
    assert_eq!(
        reg.instance_method_command_prefixes("struct::graph", "walk", &["root", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(3))],
    );

    // struct::tree `walkproc node … cmdprefix` — trailing positional prefix
    // (resolver), Exactly(3) (tree node action).  The prefix is the final
    // word regardless of intervening `-order`/`-type` options.
    assert_eq!(
        reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root", "cb"]),
        vec![(1, AppendedArity::Exactly(3))],
    );
    assert_eq!(
        reg.instance_method_command_prefixes(
            "struct::tree",
            "walkproc",
            &["root", "-type", "dfs", "cb"],
        ),
        vec![(3, AppendedArity::Exactly(3))],
    );
    // `walkproc node` with no prefix word yet names none.
    assert!(
        reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root"])
            .is_empty(),
        "a walkproc with only the node names no prefix",
    );
    // An unmodelled method / class resolves to nothing.
    assert!(
        reg.instance_method_command_prefixes("struct::graph", "get", &["x"])
            .is_empty(),
        "an unmodelled instance method has no command prefix",
    );
}

#[test]
fn tk_command_options_classified_prefix_vs_script() {
    // The Tk `script()→command_prefix` conversion (separate commit, highest
    // risk).  Locks in the classification: appended-arg callbacks are
    // prefixes (references / call-graph / W123 / arity); verbatim scripts and
    // percent-substitution callbacks stay `script()` (Body, not recorded as a
    // reference).  Ground truth: Tk 8.6/9.0 man pages.
    let reg = CommandRegistry::build_default();

    // PREFIXES — scroll callbacks append `first last` (2).
    for widget in ["listbox", "text", "canvas", "entry", "spinbox"] {
        assert_eq!(
            reg.command_prefixes(widget, &[".w", "-xscrollcommand", "cb"]),
            vec![(2, AppendedArity::Exactly(2))],
            "{widget} -xscrollcommand must be a prefix appending 2",
        );
    }
    // scale / ttk::scale append the new value (1).
    assert_eq!(
        reg.command_prefixes("scale", &[".s", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    assert_eq!(
        reg.command_prefixes("ttk::scale", &[".s", "-command", "cb"]),
        vec![(2, AppendedArity::Exactly(1))],
    );
    // scrollbar -command: `moveto frac` (2) or `scroll n units` (3).
    assert_eq!(
        reg.command_prefixes("scrollbar", &[".sb", "-command", "cb"]),
        vec![(
            2,
            AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(&[2, 3]))
        )],
    );
    // menu -tearoffcommand appends the parent + torn-off menu paths (2).
    assert_eq!(
        reg.command_prefixes("menu", &[".m", "-tearoffcommand", "cb"]),
        vec![(2, AppendedArity::Exactly(2))],
    );

    // NOT prefixes — verbatim action scripts and percent-substitution
    // callbacks are `script()`, never recorded as a command reference.
    for (widget, opt) in [
        ("button", "-command"),
        ("checkbutton", "-command"),
        ("radiobutton", "-command"),
        ("menu", "-command"),
        ("menu", "-postcommand"),
        ("ttk::combobox", "-postcommand"),
        ("spinbox", "-command"),         // percent-substitution (%W %s %d)
        ("spinbox", "-validatecommand"), // percent-substitution
        ("entry", "-validatecommand"),   // percent-substitution
        ("entry", "-invalidcommand"),    // percent-substitution
    ] {
        assert!(
            reg.command_prefixes(widget, &[".w", opt, "cb"]).is_empty(),
            "{widget} {opt} must stay a script, not a command prefix",
        );
    }
}

#[test]
fn commands_naming_a_cmdprefix_declare_a_command_prefix() {
    // Drift guard: any command whose synopsis literally names a `cmdprefix`
    // argument must declare a `CommandPrefix` (static table, resolver, or
    // command-prefix option) so callbacks light up references / call-graph /
    // W123 / arity.  The allowlist holds genuinely-deferred callbacks that
    // still need modelling; it is empty now that the option-value
    // callbacks (`mime::getbody -command`, …) carry `OptionSpec` arrays.
    const DEFERRED_OPTION_PREFIX: &[&str] = &[];
    let reg = CommandRegistry::build_default();
    let mut gaps = Vec::new();
    for name in reg.command_names() {
        if DEFERRED_OPTION_PREFIX.contains(&name) {
            continue;
        }
        let spec = reg.get(name).expect("registered");
        let mut synopses: Vec<&str> = Vec::new();
        if let Some(h) = &spec.hover {
            synopses.extend(h.synopsis.iter().copied());
        }
        synopses.extend(spec.forms.iter().map(|f| f.synopsis));
        synopses.extend(spec.subcommands.iter().map(|s| s.synopsis));
        for syn in synopses {
            if !syn.to_ascii_lowercase().contains("cmdprefix") {
                continue;
            }
            // Probe with the synopsis words after the command name; a
            // declared prefix yields a non-empty result.  Strip the `?…?`
            // optionality markers so an option-value prefix
            // (`?-command cmdprefix?`) presents its bare `-command` /
            // `cmdprefix` words to the option scanner.
            let args: Vec<&str> = syn
                .split_whitespace()
                .skip(1)
                .map(|w| w.trim_matches('?'))
                .collect();
            if reg.command_prefixes(name, &args).is_empty() {
                gaps.push(format!("{name}: {syn}"));
            }
        }
    }
    gaps.sort();
    assert!(
        gaps.is_empty(),
        "commands whose synopsis names a cmdprefix but declare no CommandPrefix:\n{}",
        gaps.join("\n"),
    );
}

#[test]
fn arg_indices_for_role_command_prefix_matches_command_prefixes() {
    // The delegation invariant: `arg_indices_for_role(CommandPrefix)` is
    // exactly the positions `command_prefixes` reports — so highlighting,
    // param-traits, and the call-reference extractor never drift.
    let reg = CommandRegistry::build_default();
    for (name, args) in [
        ("lsort", &["-command", "cmp", "{a b}"][..]),
        ("socket", &["-server", "accept", "9000"][..]),
        ("tcltest::customMatch", &["exact", "cmp"][..]),
        ("selection", &["handle", ".w", "getData"][..]),
        ("regsub", &["-command", "re", "s", "cb"][..]),
        ("namespace", &["unknown", "handler"][..]),
        ("package", &["unknown", "handler"][..]),
        ("coroinject", &["myCoro", "cb", "x"][..]),
    ] {
        let via_role = reg.arg_indices_for_role(name, args, ArgRole::CommandPrefix);
        let via_prefixes: Vec<usize> = reg
            .command_prefixes(name, args)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        assert_eq!(via_role, via_prefixes, "delegation mismatch for {name}");
    }
}

#[test]
fn bind_script_form_recurses_only_the_trailing_script() {
    // `bind $w <KeyPress> {…}` binds a script: the third
    // argument is a deferred event-handler body and must be recursed for
    // highlighting.  The `bind tag` / `bind tag sequence` query forms carry
    // no script and must not surface a Body.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role("bind", &["$w", "<KeyPress>", "{…}"], ArgRole::Body),
        vec![2],
        "the trailing script of the three-argument form is a body",
    );
    // The `+script` append form is still the trailing argument.
    assert_eq!(
        reg.arg_indices_for_role("bind", &[".b", "<Enter>", "+{puts hi}"], ArgRole::Body),
        vec![2],
    );
    assert!(
        reg.arg_indices_for_role("bind", &["$w"], ArgRole::Body)
            .is_empty(),
        "the single-tag query form has no script",
    );
    assert!(
        reg.arg_indices_for_role("bind", &["$w", "<KeyPress>"], ArgRole::Body)
            .is_empty(),
        "the tag+sequence query form has no script",
    );
}

#[test]
fn wm_protocol_handler_is_a_body() {
    // `wm protocol . WM_DELETE_WINDOW {script}` registers a deferred
    // handler script as its third argument; the query forms carry none.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role(
            "wm",
            &["protocol", ".", "WM_DELETE_WINDOW", "{exit}"],
            ArgRole::Body,
        ),
        // subcommand path offsets by +1 for the `protocol` word.
        vec![3],
        "the wm protocol handler command is a script body",
    );
    assert!(
        reg.arg_indices_for_role("wm", &["protocol", ".", "WM_DELETE_WINDOW"], ArgRole::Body)
            .is_empty(),
        "the two-argument `wm protocol window name` query form has no script",
    );
    assert!(
        reg.arg_indices_for_role("wm", &["protocol", "."], ArgRole::Body)
            .is_empty(),
        "the one-argument `wm protocol window` query form has no script",
    );
}

#[test]
fn canvas_bind_subcommand_script_is_a_body() {
    let reg = callback_metadata_registry();
    for (arguments, expected) in [
        (&["bind", "item", "<Button>", "{p}"][..], vec![3]),
        (&["bind", "item", "<Button>"][..], Vec::new()),
    ] {
        let invocation = reg
            .resolve_instance_invocation("canvas", ".canvas", arguments, None)
            .expect("provided canvas instance method");
        let roles = invocation
            .argument_roles()
            .0
            .into_iter()
            .filter_map(|(at, role)| {
                (role == ArgRole::Body)
                    .then_some(usize::from(at) + invocation.semantics.argument_offset)
            })
            .collect::<Vec<_>>();
        assert_eq!(roles, expected, "{arguments:?}");
    }
}

#[test]
fn text_tag_bind_script_is_a_body() {
    // `pathName tag bind tagName sequence script` binds a deferred
    // event-handler script as its trailing word.
    let reg = crate::model::semantic::SemanticContext::for_environment("tk").commands();
    let roles = |arguments: &[&str]| {
        let invocation = reg
            .resolve_instance_invocation("text", ".t", arguments, reg.own_surface_query())
            .expect("provided text instance method");
        invocation
            .argument_roles()
            .0
            .into_iter()
            .filter_map(|(index, role)| {
                (role == ArgRole::Body)
                    .then_some(usize::from(index) + invocation.semantics.argument_offset)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(roles(&["tag", "bind", "sel", "<Key>", "{p}"]), vec![4]);
    assert_eq!(roles(&["tag", "add", "sel", "1.0", "end"]).len(), 0);
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(DialectProfile::find("tcl8.6").unwrap());
    assert!(
        unprovided
            .resolve_instance_invocation(
                "text",
                ".t",
                &["tag", "bind", "sel", "<Key>", "{p}"],
                unprovided.own_surface_query()
            )
            .is_none()
    );
}

#[test]
fn selection_handle_command_is_a_command_prefix() {
    // `selection handle window command` — the trailing command is a
    // prefix Tk appends offset/maxChars to, not a recursed script body.
    let reg = CommandRegistry::build_default();
    assert_eq!(
        reg.arg_indices_for_role(
            "selection",
            &["handle", ".w", "getData"],
            ArgRole::CommandPrefix
        ),
        vec![2],
        "the selection handle command prefix is captured",
    );
    assert!(
        reg.arg_indices_for_role("selection", &["handle", ".w", "getData"], ArgRole::Body)
            .is_empty(),
        "a command prefix is not recursed as a script body",
    );
    // With leading option/value pairs the command is still the last arg.
    assert_eq!(
        reg.arg_indices_for_role(
            "selection",
            &["handle", "-format", "STRING", ".w", "getData"],
            ArgRole::CommandPrefix,
        ),
        vec![4],
    );
}
