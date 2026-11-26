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

//! Option, subcommand and type words resolve as `Tcl_GetIndexFromObj` resolves them: abbreviations, ambiguity and the error wording, per command.

use super::*;

/// `subst`'s option words resolve through the one shared
/// `tcl-cmd-core::prefix` matcher, so they word every miss exactly as
/// `Tcl_GetIndexFromObj` at flags `0` does (`TclSubstOptions`,
/// `tclCmdMZ.c:3341`). The **empty** word is the case a naive matcher gets
/// wrong: it prefixes all three entries, so C calls it `ambiguous`, not `bad`.
#[test]
fn subst_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -nobackslashes, -nocommands, or -novariables";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    // The empty option word abbreviates every entry ⇒ `ambiguous` (tclsh
    // 8.6.16 and 9.0.4 agree).
    assert_eq!(
        msg("subst {} abc\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    // A word that prefixes more than one entry is likewise ambiguous.
    assert_eq!(
        msg("subst -no abc\n"),
        format!("ambiguous option \"-no\": {MUST}")
    );
    // A word that prefixes nothing is `bad`.
    assert_eq!(msg("subst -q abc\n"), format!("bad option \"-q\": {MUST}"));
    // A unique prefix still resolves (subst-7.7).
    out_eq("puts [subst -nov {$x}]\n", "$x\n");
    out_eq("puts [subst -nob {a\\tb}]\n", "a\\tb\n");
    out_eq("puts [subst -noc {[cmd]}]\n", "[cmd]\n");
}

/// A hand-rolled `starts_with` filter can only ever say `bad option`: the
/// empty word — a prefix of *every* entry — reports `bad option ""` where C
/// reports `ambiguous option ""`. Both engines route `interp limit` through
/// `prefix::OptionTable::abbreviating`, which owns the verdict and the
/// `", or"` enumeration alike.
///
/// Byte-checked against tclsh 8.6.16 and 9.0.4, which agree on every row.
#[test]
fn interp_limit_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -command, -granularity, -milliseconds, or -seconds";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp limit i time {}\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i time -\n"),
        format!("ambiguous option \"-\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i time -zz\n"),
        format!("bad option \"-zz\": {MUST}")
    );
    // Unique prefixes still resolve.
    let (ok, _r, _) = run("interp create i\ninterp limit i time -sec 5\n");
    assert!(ok, "a unique prefix must still resolve");
    let (ok, _r, _) = run("interp create i\ninterp limit i time -com {}\n");
    assert!(ok);
}

/// `interp debug`'s option word is a `Tcl_GetIndexFromObj` table
/// whose noun is `debug option` (`debugTypes[]`, `tclInterp.c`), so `-f`/`-fr`
/// abbreviate and the one-entry table never says `ambiguous`. The arity check
/// runs first, as it does in C.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp debug i {}      -> bad debug option "": must be -frame
///   interp debug i -x      -> bad debug option "-x": must be -frame
///   interp debug i -f      -> 0
///   interp debug i -fr 1   -> 1
///   i debug -f             -> 1   (after the latch)
///   interp debug i -x 1 2  -> wrong # args: should be "interp debug path ?-frame ?bool??"
///   i debug -x 1 2         -> wrong # args: should be "i debug ?-frame ?bool??"
#[test]
fn interp_debug_option_uses_c_noun_and_abbreviates() {
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp debug i {}\n"),
        "bad debug option \"\": must be -frame"
    );
    assert_eq!(
        msg("interp create i\ninterp debug i -x\n"),
        "bad debug option \"-x\": must be -frame"
    );
    assert_eq!(run("interp create i\ninterp debug i -f\n").1, "0");
    assert_eq!(run("interp create i\ninterp debug i -fr 1\n").1, "1");
    // The child-as-command spelling reaches the same switch.
    assert_eq!(
        run("interp create i\ninterp debug i -fr 1\ni debug -f\n").1,
        "1"
    );
    assert_eq!(
        msg("interp create i\ninterp debug i -x 1 2\n"),
        "wrong # args: should be \"interp debug path ?-frame ?bool??\""
    );
    assert_eq!(
        msg("interp create i\ni debug -x 1 2\n"),
        "wrong # args: should be \"i debug ?-frame ?bool??\""
    );
}

/// `binary`, `binary encode`/`decode`, `encoding` and `namespace`
/// are `TclMakeEnsemble` commands. `binary encode`/`decode` run with
/// **`-prefixes` off**, so nothing abbreviates there and the miss is worded
/// `unknown subcommand`, never `unknown or ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   binary e hex a       -> 61     ;  binary en hex a -> 61
///   binary {}            -> unknown or ambiguous subcommand "": must be
///                           decode, encode, format, or scan
///   binary encode h a    -> unknown subcommand "h": must be base64, hex, or uuencode
///   binary encode {} a   -> unknown subcommand "": must be <same>
///   binary decode b YQ== -> unknown subcommand "b": must be <same>
///   encoding s           -> the system encoding
///   encoding c           -> unknown or ambiguous subcommand "c": must be …
///   namespace cu         -> ::
#[test]
fn ensemble_subcommand_words_resolve_like_tclsh() {
    const CLOCK_MUST: &str = "must be add, clicks, format, microseconds, milliseconds, \
                              scan, or seconds";
    const FORMATS: &str = "must be base64, hex, or uuencode";
    const ENC_MUST: &str = "must be convertfrom, convertto, dirs, names, profiles, system, or user";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    // `binary` abbreviates; its encode/decode format tables do not.
    assert_eq!(run("binary e hex a").1, "61");
    assert_eq!(run("binary en hex a").1, "61");
    assert_eq!(
        msg("binary {}"),
        "unknown or ambiguous subcommand \"\": must be decode, encode, format, or scan"
    );
    assert_eq!(
        msg("binary encode h a"),
        format!("unknown subcommand \"h\": {FORMATS}")
    );
    assert_eq!(
        msg("binary encode {} a"),
        format!("unknown subcommand \"\": {FORMATS}")
    );
    assert_eq!(
        msg("binary decode b YQ=="),
        format!("unknown subcommand \"b\": {FORMATS}")
    );
    // `encoding`: 9.0 adds `profiles` and `user` to the table.
    // The default system encoding is locale-dependent; pin it so the prefix
    // lookup is what is measured.
    assert_eq!(run("encoding system utf-8; encoding s").1, "utf-8");
    assert_eq!(
        msg("encoding c"),
        format!("unknown or ambiguous subcommand \"c\": {ENC_MUST}")
    );
    assert_eq!(
        msg("encoding {}"),
        format!("unknown or ambiguous subcommand \"\": {ENC_MUST}")
    );
    // `clock` is an ensemble too: `se` resolves, `m`/`mi` are ambiguous.
    assert!(
        run("clock se").0,
        "`clock se` must resolve to `clock seconds`"
    );
    assert_eq!(
        msg("clock m"),
        format!("unknown or ambiguous subcommand \"m\": {CLOCK_MUST}")
    );
    assert_eq!(
        msg("clock {}"),
        format!("unknown or ambiguous subcommand \"\": {CLOCK_MUST}")
    );
    // `namespace`'s miss sentence now comes from the same owner.
    assert_eq!(run("namespace cu").1, "::");
    assert_eq!(
        msg("namespace {}"),
        "unknown or ambiguous subcommand \"\": must be children, code, current, delete, \
         ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, \
         qualifiers, tail, unknown, upvar, or which"
    );
}

/// `package`'s subcommand word is a `Tcl_GetIndexFromObj(…,
/// "option", 0)` table (`pkgOptions[]`, `tclPkg.c`) — both engines said
/// `unknown or ambiguous subcommand "x"` with no list, which is the *ensemble*
/// wording; `package` is not an ensemble. `package prefer`'s word is a second
/// table with the noun `preference`.
///
/// tclsh 9.0.4 (the VM's default release):
///   package x  -> bad option "x": must be files, forget, ifneeded, names,
///                 prefer, present, provide, require, unknown, vcompare,
///                 versions, or vsatisfies       [TCL LOOKUP INDEX option x]
///   package {} -> ambiguous option "": must be <same>
///   package pr -> ambiguous option "pr": must be <same>   (prefer/present/provide)
///   package v  -> ambiguous option "v": must be <same>    (vcompare/versions/vsatisfies)
///   package n  -> the names list
///   package prefer {} -> ambiguous preference "": must be latest or stable
///   package prefer x  -> bad preference "x": must be latest or stable
///   package prefer s  -> stable
#[test]
fn package_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be files, forget, ifneeded, names, prefer, present, provide, \
                        require, unknown, vcompare, versions, or vsatisfies";
    const PREFER_MUST: &str = "must be latest or stable";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(msg("package x\n"), format!("bad option \"x\": {MUST}"));
    assert_eq!(
        msg("package {}\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    assert_eq!(
        msg("package pr\n"),
        format!("ambiguous option \"pr\": {MUST}")
    );
    assert_eq!(
        msg("package v\n"),
        format!("ambiguous option \"v\": {MUST}")
    );
    assert_eq!(
        run("package provide foo 1.0\nllength [lsearch -all -exact [package n] foo]\n").1,
        "1"
    );
    // C's lookup error code travels with the message.
    assert_eq!(
        run("catch {package x} e opts\nreturn [dict get $opts -errorcode]\n").1,
        "TCL LOOKUP INDEX option x"
    );
    // `package prefer`'s own table.
    assert_eq!(
        msg("package prefer {}\n"),
        format!("ambiguous preference \"\": {PREFER_MUST}")
    );
    assert_eq!(
        msg("package prefer x\n"),
        format!("bad preference \"x\": {PREFER_MUST}")
    );
    assert_eq!(run("package prefer s\n").1, "stable");
    assert_eq!(run("package prefer l\n").1, "latest");
    // 9.0's `files`: nothing here loads through a package loader, so the
    // answer is the empty list, as it is in tclsh for a script-provided
    // package (`package provide foo 1.0; package files foo` → {}).
    assert_eq!(run("package provide foo 1.0\npackage files foo\n").1, "");
}

/// The `interp` ensemble and the child-as-command dispatch are
/// `Tcl_GetIndexFromObj(…, "option", 0)` tables (`options[]` in `Tcl_InterpObjCmd`
/// and `NRChildCmd`, `tclInterp.c`), so subcommands abbreviate and the empty
/// word — a prefix of every entry — is `ambiguous option ""`.
///
/// Both lists are tclsh 9.0.4's:
///   interp x  -> bad option "x": must be alias, aliases, bgerror, cancel,
///                children, create, debug, delete, eval, exists, expose, hide,
///                hidden, issafe, invokehidden, limit, marktrusted,
///                recursionlimit, share, target, or transfer
///   i x       -> bad option "x": must be alias, aliases, bgerror, debug, eval,
///                expose, hide, hidden, issafe, invokehidden, limit,
///                marktrusted, or recursionlimit
///
/// The abbreviation verdicts are tclsh's exactly (8.6.16 / 9.0.4 agree):
///   interp cr j        -> j
///   interp c j         -> ambiguous option "c"
///   interp ev {set x 1} -> 1   ;  interp e {set x 1} -> ambiguous option "e"
///   i ev {set x 1}     -> 1    ;  i h / i hi -> ambiguous option "h" / "hi"
#[test]
fn interp_subcommand_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be alias, aliases, bgerror, cancel, children, create, debug, \
                        delete, eval, exists, expose, hide, hidden, issafe, invokehidden, \
                        limit, marktrusted, recursionlimit, share, target, or transfer";
    const CHILD_MUST: &str = "must be alias, aliases, bgerror, debug, eval, expose, hide, \
                              hidden, issafe, invokehidden, limit, marktrusted, or \
                              recursionlimit";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(msg("interp x\n"), format!("bad option \"x\": {MUST}"));
    assert_eq!(msg("interp {}\n"), format!("ambiguous option \"\": {MUST}"));
    assert_eq!(
        msg("interp c j\n"),
        format!("ambiguous option \"c\": {MUST}")
    );
    assert_eq!(run("interp cr j\n").1, "j");
    assert_eq!(
        msg("interp e {set x 1}\n"),
        format!("ambiguous option \"e\": {MUST}")
    );
    assert_eq!(run("interp ev {} {set x 1}\n").1, "1");
    // The 8.x-only `slaves` spelling still resolves, and still dispatches.
    assert_eq!(run("interp create i\nllength [interp sl]\n").1, "1");
    // The child-as-command table.
    assert_eq!(
        msg("interp create i\ni x\n"),
        format!("bad option \"x\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni {}\n"),
        format!("ambiguous option \"\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni e {set x 1}\n"),
        format!("ambiguous option \"e\": {CHILD_MUST}")
    );
    assert_eq!(run("interp create i\ni ev {set x 1}\n").1, "1");
    assert_eq!(
        msg("interp create i\ni h\n"),
        format!("ambiguous option \"h\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni hi\n"),
        format!("ambiguous option \"hi\": {CHILD_MUST}")
    );
}

/// `interp create`'s and `interp invokehidden`'s leading options
/// are `Tcl_GetIndexFromObj(…, "option", 0)` tables (`createOptions[]` /
/// `hiddenOptions[]`, `tclInterp.c`), so they abbreviate and the lone `-` —
/// a prefix of every entry — is `ambiguous`, not `bad`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp create -x k          -> bad option "-x": must be -safe or --
///   interp create - k           -> ambiguous option "-": must be -safe or --
///   interp create -s k          -> k        ;  interp create -- k -> k
///   interp invokehidden i -x f  -> bad option "-x": must be -global, -namespace, or --
///   interp invokehidden i - f   -> ambiguous option "-": must be -global, -namespace, or --
///   i invokehidden -x f         -> bad option "-x": must be -global, -namespace, or --
#[test]
fn interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj() {
    const CREATE_MUST: &str = "must be -safe or --";
    const HIDDEN_MUST: &str = "must be -global, -namespace, or --";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create -x k\n"),
        format!("bad option \"-x\": {CREATE_MUST}")
    );
    assert_eq!(
        msg("interp create - k\n"),
        format!("ambiguous option \"-\": {CREATE_MUST}")
    );
    assert_eq!(run("interp create -s k\n").1, "k");
    assert_eq!(run("interp create -- k\n").1, "k");
    assert_eq!(
        msg("interp create i\ninterp invokehidden i -x foo\n"),
        format!("bad option \"-x\": {HIDDEN_MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp invokehidden i - foo\n"),
        format!("ambiguous option \"-\": {HIDDEN_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni invokehidden -x foo\n"),
        format!("bad option \"-x\": {HIDDEN_MUST}")
    );
}

/// `interp limit`'s type word is `Tcl_GetIndexFromObj(…,
/// "limit type", 0)` (`limitTypes[]`, `tclInterp.c`), so `c`/`t` abbreviate
/// and the empty word — a prefix of both entries — is `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp limit i {}  -> ambiguous limit type "": must be commands or time
///   interp limit i x   -> bad limit type "x": must be commands or time
///   interp limit i c   -> -command {} -granularity 1 -value {}
///   interp limit i t   -> -command {} -granularity 10 -milliseconds {} -seconds {}
///   i limit {}         -> ambiguous limit type "": must be commands or time
#[test]
fn interp_limit_type_word_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be commands or time";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp limit i {}\n"),
        format!("ambiguous limit type \"\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i x\n"),
        format!("bad limit type \"x\": {MUST}")
    );
    assert_eq!(
        run("interp create i\ninterp limit i c\n").1,
        "-command {} -granularity 1 -value {}"
    );
    assert_eq!(
        run("interp create i\ninterp limit i t\n").1,
        "-command {} -granularity 10 -milliseconds {} -seconds {}"
    );
    // The child-as-command spelling shares the resolver.
    assert_eq!(
        msg("interp create i\ni limit {}\n"),
        format!("ambiguous limit type \"\": {MUST}")
    );
    assert_eq!(
        run("interp create i\ni limit c\n").1,
        "-command {} -granularity 1 -value {}"
    );
}

/// `glob`'s option words are a
/// `Tcl_GetIndexFromObj(…, "option", 0)` table (`globOptions[]`,
/// `tclFileName.c`). Silently *skip*ping an unrecognised `-word` would let
/// `glob -x a` run, and `-types d` leak its value into the
/// pattern list. Rejecting an unknown option is a deliberate behaviour choice,
/// pinned byte for byte.
///
/// tclsh 8.6.16 and 9.0.4 agree on every row (no dialect split):
///   glob -x a  -> bad option "-x": must be -directory, -join, -nocomplain,
///                 -path, -tails, -types, or --
///   glob - a   -> ambiguous option "-": must be <same>
///   glob -t d a-> ambiguous option "-t": must be <same>   (-tails/-types)
///   glob -tails <pat>
///              -> "-tails" must be used with either "-directory" or "-path"
///   glob {}    -> .    (the empty word is a pattern, not an option)
///   -n/-d/-j/-ta/-ty all resolve as unique prefixes.
#[test]
fn glob_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -directory, -join, -nocomplain, -path, -tails, -types, or --";
    const SETUP: &str = "set d /tmp/zz_tcltest_glob\n\
                         file delete -force $d\n\
                         file mkdir $d/sub\n\
                         set f [open $d/a.tcl w]\nclose $f\n";
    let msg = |tail: &str| {
        let (ok, result, _) = run(&format!("{SETUP}catch {{{tail}}} e\nset e\n"));
        assert!(ok, "expected the catch to succeed for {tail}");
        result
    };
    // Unique prefixes resolve, and each advertised name is honoured.
    assert_eq!(run(&format!("{SETUP}glob -d $d -ta *.tcl")).1, "a.tcl");
    assert_eq!(run(&format!("{SETUP}glob -n -d $d -ta -ty f *")).1, "a.tcl");
    assert_eq!(run(&format!("{SETUP}glob -n -d $d -ta -ty d *")).1, "sub");
    // The new rejection.
    assert_eq!(msg("glob -x a"), format!("bad option \"-x\": {MUST}"));
    assert_eq!(msg("glob - a"), format!("ambiguous option \"-\": {MUST}"));
    assert_eq!(
        msg("glob -t d a"),
        format!("ambiguous option \"-t\": {MUST}")
    );
    assert_eq!(
        msg("glob -tails *.tcl"),
        "\"-tails\" must be used with either \"-directory\" or \"-path\""
    );
    assert_eq!(
        run(&format!(
            "{SETUP}set r [glob -j -d $d -- sub]\nfile delete -force $d\nset r"
        ))
        .1,
        "/tmp/zz_tcltest_glob/sub"
    );
}

/// `seek`'s origin word is a `Tcl_GetIndexFromObj(…, "origin", 0)`
/// table (`originOptions[]`, `tclIOCmd.c`). This engine silently treated any
/// unknown origin as `start`; C rejects it, abbreviates `s`/`c`/`e`, and words
/// the empty origin — a prefix of all three — `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   seek $f 0 x  -> bad origin "x": must be start, current, or end
///   seek $f 0 {} -> ambiguous origin "": must be start, current, or end
///   seek $f 0 s / c / e -> {}
#[test]
fn seek_origin_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be start, current, or end";
    const SETUP: &str = "set p /tmp/zz_tcltest_seek_test.txt\n\
                         set f [open $p w]\nputs $f abcdef\nclose $f\n\
                         set r [open $p r]\n";
    let msg = |tail: &str| {
        let (ok, result, _) = run(&format!("{SETUP}catch {{{tail}}} e\nclose $r\nset e\n"));
        assert!(ok, "expected the catch to succeed for {tail}");
        result
    };
    assert_eq!(msg("seek $r 0 x"), format!("bad origin \"x\": {MUST}"));
    assert_eq!(
        msg("seek $r 0 {}"),
        format!("ambiguous origin \"\": {MUST}")
    );
    // Abbreviations resolve, and `e` seeks to the end.
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 0 e\nset n [tell $r]\nclose $r\nfile delete $p\nset n\n"
        ))
        .1,
        "7"
    );
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 2 s\nset n [tell $r]\nclose $r\nset n\n"
        ))
        .1,
        "2"
    );
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 2 s\nseek $r 1 c\nset n [tell $r]\nclose $r\nfile delete $p\nset n\n"
        ))
        .1,
        "3"
    );
}
