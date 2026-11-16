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

//! What follows the pinned release: the number grammar, the command surface, core packages, build info and the context an interpreter states.

use super::*;

/// Selecting the release selects the *numeral grammar* the whole runtime
/// reads with, because every numeral goes through one facility
/// (`tcl_syntax::number`) whose ambient dialect `set_runtime_version`
/// installs. Tcl 9.0 defines `KILL_OCTAL` in `tclStrToD.c`, so a bare
/// leading zero is decimal there (`0755` == 755) while 8.4/8.6 read it as
/// octal (`0755` == 493); `0b`/`0o` only exist from 8.5 and `0d` from 9.0.
/// Exercised through `expr`, which reads operands with
/// `ParseFlags::default()` — i.e. exactly the ambient this installs.
#[cfg(have_tommath)]
#[test]
fn expr_numerals_follow_the_release_selected_number_grammar() {
    use tcl_dialect::TclVersion;

    // TP: octal-by-leading-zero holds up to 8.6 …
    for version in [TclVersion::V8_4, TclVersion::V8_6] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(ok(i, b"expr {0755}"), b"493", "{version:?}");
            assert_eq!(ok(i, b"expr {010 + 1}"), b"9", "{version:?}");
            // TN: without a leading zero the release cannot matter.
            assert_eq!(ok(i, b"expr {755}"), b"755", "{version:?}");
            // A leading-zero run before `.`/`e` stays a decimal float in
            // every release (C backtracks out of its octal state).
            assert_eq!(ok(i, b"expr {07.5}"), b"7.5", "{version:?}");
        });
    }

    // FN: 9.0 must not keep reading it as octal — the direction of this
    // change is easy to invert, so pin both sides.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
        assert_eq!(ok(i, b"expr {010 + 1}"), b"11");
        assert_eq!(ok(i, b"expr {755}"), b"755");
        assert_eq!(ok(i, b"expr {07.5}"), b"7.5");
    });

    // FP: a prefix its release does not have is not a numeral at all, so
    // the word is a bareword rather than a silently-different number.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"expr {0o17}"), Code::Error);
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"expr {0o17}"), b"15");
        assert_eq!(i.eval_str(b"expr {1_0}"), Code::Error);
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"expr {0o17}"), b"15");
        assert_eq!(ok(i, b"expr {1_0}"), b"10");
    });
}

/// Selecting the release selects the *lexing grammar* scripts parse
/// under and the builtin command surface: under an 8.4 pin `{*}` does
/// not expand and the first-close `${…}`
/// rule applies, and `lassign` (8.5+) resolves to `invalid command
/// name` — while a user-defined proc of the same name stays callable
/// (the compat-polyfill pattern).
#[test]
fn grammar_and_command_surface_follow_the_selected_release() {
    use tcl_dialect::TclVersion;

    // The `{*}` expansion grammar (TIP 157, 8.5+).
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"llength [list {*}{a b}]"), b"2");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        // No expansion under 8.4 (TIP 157 is 8.5+), so `{*}{a b}` is a
        // braced word `{*}` with `{a b}` welded onto its close-brace —
        // which C rejects. Measured on tclsh8.4.20: `llength [list
        // {*}{a b}]` reports `extra characters after close-brace`, where
        // 8.5.19/8.6.16/9.0.4/9.1b0 all answer `2`. The boundary owner's
        // `welded_after_close` enforces this: a braced word directly
        // followed by another brace-delimited word is rejected rather
        // than merged into one word.
        assert_eq!(i.eval_str(b"llength [list {*}{a b}]"), Code::Error);
        assert_eq!(i.result_bytes(), b"extra characters after close-brace");
    });

    // The `${…}` delimiting rule: 8.x stops at the first `}`
    // (its `Tcl_ParseVarName` counts no braces), 9.x nests. Verified
    // against tclsh8.4.20 / tclsh9.0.4.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
        assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
        assert_eq!(ok(i, b"set r ${a{b}c}"), b"5c}");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
        assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
        assert_eq!(ok(i, b"set r ${a{b}c}"), b"9");
    });

    // The builtin surface: lassign is 8.5+, lpop is 9.0+.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"lassign {a b} x"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        assert_eq!(ok(i, b"llength [info commands lassign]"), b"0");
        // The polyfill pattern: a user proc wins over the hidden builtin.
        assert_eq!(
            i.eval_str(b"proc lassign {l args} { return polyfill }"),
            Code::Ok
        );
        assert_eq!(ok(i, b"lassign {a b} x"), b"polyfill");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"lassign {a b} x"), b"b");
        assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
        assert_eq!(i.eval_str(b"lpop l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lpop\"");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
        assert_eq!(ok(i, b"lpop l"), b"c");
    });
}

/// The availability gate is a property of the **final resolved
/// builtin**, not of the spelling the caller wrote. Without that, the
/// two resolve-then-`invoke` shapes — the alias trampoline and the
/// `namespace import` redirect — could reach the builtin through a
/// second, ungated resolution and make an 8.4-hidden `lassign` callable
/// as `interp alias {} la {} lassign; la {a b} x y`.
///
/// Error identity is oracled against real tclsh 8.6.16 / 9.0.4:
/// `interp alias {} la {} nosuchcmd; la a b` reports `invalid command
/// name "nosuchcmd"` — the *target* name, not the alias — so a
/// release-hidden target reports the same way a deleted one does.
#[test]
fn the_release_command_surface_gates_alias_and_import_dispatch() {
    use tcl_dialect::TclVersion;

    // The reviewer's reproducer: an 8.4 alias to the 8.5+ `lassign`.
    // (Creating the alias still succeeds — real Tcl binds alias targets
    // lazily, so an alias to a nonexistent command is legal.)
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(i.eval_str(b"la {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        // …and nothing was assigned through the alias.
        assert_eq!(i.eval_str(b"set x"), Code::Error);
        // An alias *chain* is gated at the final builtin too, not merely
        // at the first hop.
        assert_eq!(i.eval_str(b"interp alias {} lb {} la"), Code::Ok);
        assert_eq!(i.eval_str(b"lb {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
    });
    // The same alias is a working `lassign` on a release that carries it.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(ok(i, b"la {a b} x y; list $x $y"), b"a b");
    });

    // The `namespace import` redirect.  A leading `::` with no separator
    // before the command (`::lassign`) must retain its absolute-global
    // meaning while resolving the import source (the public `namespace
    // qualifiers` result intentionally returns an empty string here).
    // First pin the exact issue reproducer, then explicitly export the
    // builtin so the redirect and release-hidden dispatch halves remain
    // observable in the same test.
    const IMPORT: &[u8] = b"namespace eval n {namespace import ::lassign}";
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(IMPORT), Code::Ok);
        // This reproducer succeeds without an explicit export.  Global
        // builtins are not exported by default, so this
        // records no alias; the important contract is that it does not
        // resolve the absolute root as the destination namespace.
        assert_eq!(i.eval_str(b"namespace export lassign"), Code::Ok);
        assert_eq!(i.eval_str(IMPORT), Code::Ok);
        assert_eq!(ok(i, b"n::lassign {a b} x y; list $x $y"), b"a b");
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"n::lassign {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::lassign\"");
    });

    // Direct dispatch is unchanged by moving the gate into the shared
    // resolver — both the hidden and the carried release.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"lassign {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"lassign {a b} x y; list $x $y"), b"a b");
    });
}

/// An import keeps the final builtin's registry identity when that source
/// moves into the hidden table. Repinning the runtime surface must gate the
/// visible import, while an explicit `invokehidden` remains the privileged
/// path that deliberately bypasses ordinary command visibility.
#[test]
fn an_imported_hidden_builtin_keeps_its_release_gate() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(
            i.eval_str(
                b"namespace export lpop
                      namespace eval n {namespace import ::lpop}
                      set l {a b c}
                      interp hide {} lpop hp
                      n::lpop l",
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"c");

        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(i.eval_str(b"n::lpop l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::hp\"");
        assert_eq!(ok(i, b"set l"), b"a b");

        // Release visibility grants no cross-provider List backing. Supply
        // fresh current-release input after observing the earlier effect.
        assert_eq!(ok(i, b"set l {a b}"), b"a b");
        assert_eq!(
            i.eval_str(b"interp invokehidden {} hp l"),
            Code::Ok,
            "result={:?}, refusal={:?}",
            i.result_bytes(),
            i.native_execution_refusal(),
        );
        assert_eq!(i.result_bytes(), b"b");

        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp expose {} hp lz"), Code::Ok);
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(i.eval_str(b"lz l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");
        assert_eq!(ok(i, b"info commands lz"), b"");
        i.set_runtime_version(TclVersion::V9_0);
    });
}

/// The same gate on the two *non*-dispatch shapes that would hand a
/// release-hidden builtin back ungated: `rename`, which would rebind it
/// under a name the registry has no spec for, and `interp hide`, which
/// would park it where `interp invokehidden` reaches it.
///
/// `rename`'s refusal is oracled against tclsh 8.6.16 / 9.0.4:
/// `rename nosuchcmd zz` → `can't rename "nosuchcmd": command doesn't
/// exist`; an empty destination uses `can't delete`. Hiding a command that
/// does not exist raises `unknown command`, matching Tcl_HideCommand and
/// avoiding a swallowed typo in a security-sensitive path.
#[test]
fn the_release_command_surface_gates_rename_and_hide() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"rename lassign lz"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't rename \"lassign\": command doesn't exist"
        );
        assert_eq!(i.eval_str(b"lz {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");

        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
        assert_eq!(
            i.eval_str(b"interp invokehidden {} lassign {a b} x y"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"invalid hidden command name \"lassign\"");
        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
    });
    // Both stay ordinary operations on a release that carries `lassign`.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"rename lassign lz"), Code::Ok);
        assert_eq!(ok(i, b"lz {a b} x y; list $x $y"), b"a b");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Ok);
        assert_eq!(
            ok(i, b"interp invokehidden {} lassign {a b} x y; list $x $y"),
            b"a b"
        );
    });
}

/// A fresh interpreter installs *its own* grammar rather than inheriting
/// whatever an earlier interpreter left in the thread-ambient slot — the
/// unchanged-version short-circuit in `set_runtime_version` would otherwise
/// leave a default-release interp reading numerals as 8.6.
#[cfg(have_tommath)]
#[test]
fn a_fresh_interp_reinstalls_the_number_grammar_after_an_8_6_interp() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"expr {0755}"), b"493");
    });
    // Same thread, ambient left at 8.6 by the interp above.
    leak_free(|i| {
        assert_eq!(i.runtime_version(), DEFAULT_RUNTIME_VERSION);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
        // …and re-pinning the release it already reports still installs it.
        i.set_runtime_version(DEFAULT_RUNTIME_VERSION);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
    });
}

/// `return -code` / `try on` read their integer through the same one
/// facility, so the spellings they accept track the emulated release
/// (C reads it with `Tcl_GetIntFromObj`, whose grammar is the release's).
/// The ambient is thread-local, so each dialect is installed explicitly
/// here rather than inherited from another test.
#[test]
fn completion_code_integers_follow_the_release_selected_number_grammar() {
    use tcl_syntax::number::{set_runtime_syntax, NumberSyntax};

    // Release-independent: decimal, hex, the full signed *and* unsigned
    // 32-bit window, and its reduction to an `int`.
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        set_runtime_syntax(syntax);
        assert_eq!(parse_completion_int(b"42"), Some(42), "{syntax:?}");
        assert_eq!(parse_completion_int(b"+42"), Some(42), "{syntax:?}");
        assert_eq!(parse_completion_int(b"-7"), Some(-7), "{syntax:?}");
        assert_eq!(parse_completion_int(b" 7 "), Some(7), "{syntax:?}");
        assert_eq!(parse_completion_int(b"0x1f"), Some(31), "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"-2147483648"),
            Some(i32::MIN),
            "{syntax:?}"
        );
        // The unsigned half is reachable and wraps to an `int`.
        assert_eq!(parse_completion_int(b"0xFFFFFFFF"), Some(-1), "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"2147483648"),
            Some(i32::MIN),
            "{syntax:?}"
        );
        // Outside the window, a bare prefix, junk, a float, and a magnitude
        // past a wide are all rejected.
        assert_eq!(parse_completion_int(b"4294967296"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"-2147483649"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"0x"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b""), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"abc"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"1.5"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"12x"), None, "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"99999999999999999999"),
            None,
            "{syntax:?}"
        );
        // One sign only — the facility consumes it, so `--5` is not 5.
        assert_eq!(parse_completion_int(b"--5"), None, "{syntax:?}");
    }

    // Octal-by-leading-zero up to 8.6, decimal from 9.0.
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        set_runtime_syntax(syntax);
        assert_eq!(parse_completion_int(b"010"), Some(8), "{syntax:?}");
        assert_eq!(parse_completion_int(b"-010"), Some(-8), "{syntax:?}");
        // An invalid octal digit stops the scan, so the whole word is not a
        // number (C's "bad octal" report).
        assert_eq!(parse_completion_int(b"08"), None, "{syntax:?}");
    }
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(parse_completion_int(b"010"), Some(10));
    assert_eq!(parse_completion_int(b"-010"), Some(-10));
    assert_eq!(parse_completion_int(b"08"), Some(8));

    // `0o`/`0b` arrive in 8.5, `0d` and `_` separators in 9.0 — an
    // unavailable prefix is not a prefix, so the word is not an integer.
    set_runtime_syntax(NumberSyntax::Tcl84);
    assert_eq!(parse_completion_int(b"0o17"), None);
    assert_eq!(parse_completion_int(b"0b101"), None);
    assert_eq!(parse_completion_int(b"0d99"), None);
    assert_eq!(parse_completion_int(b"1_0"), None);
    set_runtime_syntax(NumberSyntax::Tcl85);
    assert_eq!(parse_completion_int(b"0o17"), Some(15));
    assert_eq!(parse_completion_int(b"0b101"), Some(5));
    assert_eq!(parse_completion_int(b"0d99"), None);
    assert_eq!(parse_completion_int(b"1_0"), None);
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(parse_completion_int(b"0o17"), Some(15));
    assert_eq!(parse_completion_int(b"0b101"), Some(5));
    assert_eq!(parse_completion_int(b"0d99"), Some(99));
    assert_eq!(parse_completion_int(b"1_0"), Some(10));
}

/// The same release gate reached through the script surface: `return -level
/// 0 -code 010` completes with code 8 up to 8.6 and 10 from 9.0.
#[test]
fn return_code_word_reads_its_integer_in_the_emulated_release() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"8");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"10");
    });
}

/// An empty operand (and a whitespace-only or sign-only one) must report a
/// Tcl error on **every** release rather than abort the process. Regression:
/// the shared parser's octal-by-leading-zero branch read its first byte
/// unguarded, so with a pre-9.0 grammar installed — which is exactly what
/// `set_runtime_version` now does — `expr {$empty + 1}` panicked in
/// `tcl_syntax::number::parse` instead of failing the expression.
#[cfg(have_tommath)]
#[test]
fn an_empty_numeral_errors_instead_of_panicking_on_every_release() {
    use tcl_dialect::TclVersion;

    for version in [TclVersion::V8_4, TclVersion::V8_6, TclVersion::V9_0] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(
                i.eval_str(b"set x {}; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
            assert_eq!(
                i.eval_str(b"set x { }; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
            assert_eq!(
                i.eval_str(b"set x -; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
        });
    }
}

/// The core packages a bare interpreter pre-provides follow the pinned
/// release, so `package require Tcl 8.5` fails under a 9.x pin exactly as
/// `tclsh9.0` fails it, rather than succeeding the way it would if both
/// engines hardcoded `9.0.4`/`Tcl`+`tcl` regardless of the pin.
///
/// Measured (`package provide <name>` in a fresh `tclsh`):
/// 8.4.20 → `Tcl` = `8.4`, no `tcl`; 8.5.19 → `Tcl` = `8.5.19`;
/// 8.6.14 → `Tcl` = `8.6.14`, `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all
/// four names, at the patch level / `1.3.1`.
#[cfg(have_tommath)]
#[test]
fn core_package_provides_follow_the_pinned_release() {
    use tcl_dialect::TclVersion;

    for version in TclVersion::ALL {
        leak_free(|i| {
            i.set_runtime_version(version);
            for core in version.core_provided_packages() {
                let script = format!("package provide {}", core.name);
                assert_eq!(
                    ok(i, script.as_bytes()),
                    core.version.as_bytes(),
                    "{version:?} provides {}",
                    core.name
                );
            }
            // FN guard: a name this release does not pre-provide answers
            // with the empty string, not a stale earlier pin's version.
            if version < TclVersion::V9_0 {
                assert_eq!(ok(i, b"package provide tcl"), b"", "{version:?}");
            }
            if version < TclVersion::V8_6 {
                assert_eq!(ok(i, b"package provide TclOO"), b"", "{version:?}");
            }
            // `package require Tcl 8.5` means [8.5, 9) — satisfied on
            // 8.5/8.6, a version conflict on 8.4 and on every 9.x.
            let wanted = matches!(version, TclVersion::V8_5 | TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"package require Tcl 8.5") == Code::Ok,
                wanted,
                "{version:?}: package require Tcl 8.5"
            );
        });
    }
}

/// `::tcl::build-info` reports the pinned release's build identity, and
/// splits its fields the way C does — `patchlevel` up to the `+`,
/// `version` up to the second `.`. Measured: `tclsh9.0` answers
/// `9.0.4` / `9.0`, and `tclsh9.1` answers `9.1.0` / `9.1`.
#[cfg(have_tommath)]
#[test]
fn build_info_follows_the_pinned_release() {
    use tcl_dialect::TclVersion;

    for (version, patchlevel, short) in [
        (TclVersion::V9_0, &b"9.0.4"[..], &b"9.0"[..]),
        (TclVersion::V9_1, &b"9.1.0"[..], &b"9.1"[..]),
    ] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(ok(i, b"::tcl::build-info patchlevel"), patchlevel);
            assert_eq!(ok(i, b"::tcl::build-info version"), short);
            // An unset build flag reports 0 rather than erroring.
            assert_eq!(ok(i, b"::tcl::build-info memdebug"), b"0");
        });
    }
}

/// `set_dialect_profile` is the profile form of `pin_context`: the context a
/// profile names, resolved through the ingress, pins the same profile, the
/// same release and the same identity.
#[test]
fn the_profile_form_of_a_pin_is_the_context_the_profile_names() {
    for profile in tcl_dialect::DialectProfile::all().iter().chain([
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_dialect::DialectProfile::tk(),
    ]) {
        let mut by_profile = Interp::new();
        by_profile.set_dialect_profile(profile);
        let mut by_context = Interp::new();
        by_context
            .pin_context(&tcl_registry::model::runtime_context_for_profile(profile))
            .unwrap_or_else(|error| panic!("{}: {error}", profile.name));

        assert!(
            std::ptr::eq(by_context.dialect_profile(), by_profile.dialect_profile()),
            "{}",
            profile.name
        );
        assert_eq!(by_context.runtime_version(), by_profile.runtime_version());
        assert_eq!(by_context.runtime_context(), by_profile.runtime_context());
        assert_eq!(by_context.held_identity(), by_profile.held_identity());
    }
}

/// The runtime states the identity of the world it is pinned to: the
/// context's fields, this build's ABI, intrinsic table and embedded library,
/// and no pack facts.
#[test]
fn an_interp_states_the_identity_of_the_world_it_is_pinned_to() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    let mut interp = Interp::new();
    interp.set_dialect_profile(profile);
    let expected = tcl_registry::model::runtime_context_for_profile(profile)
        .identity(&[], tcl_registry::intrinsic_table_hash());
    assert_eq!(interp.held_identity(), expected);
    assert_eq!(expected.environment, "tcl8.6");
    assert_eq!(expected.release, "8.6");
    assert_eq!(
        expected.abi_version,
        tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION
    );
    assert!(expected.packs.is_empty());

    interp.set_dialect_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert_eq!(interp.held_identity().environment, "tcl");
}

/// A context the ingress does not agree with is an error and leaves the pin
/// as it was; an overlay nothing has installed is one of them, and is never
/// the un-overlaid generation under another name.
#[test]
fn a_context_the_ingress_refuses_leaves_the_pin_unchanged() {
    use tcl_registry::model::PinError;

    const OVERLAY: u64 = 0x0C0_1704;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    let context = tcl_registry::model::runtime_context_for_profile(profile);
    let mut interp = Interp::new();
    interp
        .pin_context(&context)
        .expect("the profile's own context");

    let mut unknown = context.clone();
    unknown.environment = "no-such-environment".to_owned();
    let mut wrong_build = context.clone();
    wrong_build.build = tcl_dialect::model::BuildProfileId::JimFull;
    let mut missing_overlay = context.clone();
    missing_overlay.overlay_generation = OVERLAY;
    for (what, refused) in [
        ("unknown", unknown),
        ("build", wrong_build),
        ("overlay", missing_overlay.clone()),
    ] {
        assert!(interp.pin_context(&refused).is_err(), "{what}");
        assert!(std::ptr::eq(interp.dialect_profile(), profile), "{what}");
        assert_eq!(interp.runtime_context(), context, "{what}");
    }
    assert!(matches!(
        interp.pin_context(&missing_overlay),
        Err(PinError::OverlayMiss(miss)) if miss.overlay == OVERLAY
    ));

    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    interp
        .pin_context(&missing_overlay)
        .expect("installed, so it pins");
    assert_eq!(interp.runtime_context().overlay_generation, OVERLAY);
    assert!(std::ptr::eq(interp.dialect_profile(), profile));
}
