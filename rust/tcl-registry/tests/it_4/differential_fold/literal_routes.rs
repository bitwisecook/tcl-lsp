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

//! Routes over literal operands: file paths, list searches and the tcllib `base32` package.

use super::*;

/// What a pure route declared on `command ?sub?` answers over literal words
/// under `version` — or, with none, the answer every release gives — through
/// the shared literal entry codegen and O129 use.
fn literal_route(
    reg: &CommandRegistry,
    command: &str,
    sub: Option<&str>,
    args: &[&str],
    version: Option<tcl_dialect::TclVersion>,
) -> Option<String> {
    use tcl_registry::value_transfer::{evaluate_literal, resolve_semantics};

    let spec = reg.get(command).expect(command);
    let resolved = match sub {
        Some(name) => resolve_semantics(spec, Some(spec.subcommand(name).expect(name)), None),
        None => resolve_semantics(spec, None, None),
    };
    let semantics = resolved.semantics().expect("a declared route");
    evaluate_literal(semantics, command, sub, args, version)
}

/// `file` path witnesses: `(subcommand, names)`.
type PathWitness = (&'static str, &'static [&'static str]);

/// Names every platform and release reads alike, through each operation.
const PATH_WITNESSES: &[PathWitness] = &[
    ("dirname", &["a"]),
    ("dirname", &["a/b/c.txt"]),
    ("dirname", &["a//b/c"]),
    ("dirname", &["a/./b"]),
    ("dirname", &["../a"]),
    ("dirname", &[""]),
    ("dirname", &["/"]),
    ("dirname", &["/a"]),
    ("dirname", &["/a/b/"]),
    ("dirname", &["a b/c d.e"]),
    ("tail", &["a/b/c.txt"]),
    ("tail", &["/a/b/"]),
    ("tail", &["/"]),
    ("tail", &[""]),
    ("tail", &["a//b"]),
    ("tail", &["./"]),
    ("extension", &["a/b/c.txt"]),
    ("extension", &[".bashrc"]),
    ("extension", &["a/.bashrc"]),
    ("extension", &["a/b.c/"]),
    ("extension", &["a.b/c"]),
    ("extension", &[".."]),
    ("extension", &["x.tar.gz"]),
    ("rootname", &["a/b/c.txt"]),
    ("rootname", &[".bashrc"]),
    ("rootname", &["a/b.c/"]),
    ("rootname", &["/.."]),
    ("rootname", &["x.tar.gz"]),
    ("split", &["a/b/c"]),
    ("split", &["/a//b/"]),
    ("split", &[""]),
    ("split", &["/"]),
    ("split", &["a b/c"]),
    ("split", &["./a/../b"]),
    ("join", &["a", "b"]),
    ("join", &["a", "/b"]),
    ("join", &["/a", "b"]),
    ("join", &["a", "b/"]),
    ("join", &["a/", "b"]),
    ("join", &["a", "", "b"]),
    ("join", &["", "a"]),
    ("join", &["a", ".", ".."]),
    ("join", &["/", "a"]),
    ("join", &["a/b", "/c/d", "e"]),
    ("join", &[" ", "a"]),
    ("join", &["a"]),
];

/// Names Windows or a release reads otherwise: each declines.
const PATH_PLATFORM_WITNESSES: &[PathWitness] = &[
    ("dirname", &["C:/a"]),
    ("tail", &["a\\b"]),
    ("split", &["//srv/share/a"]),
    ("join", &["a", "~b"]),
    ("extension", &["~/x.y"]),
    ("rootname", &["x:y.z"]),
    ("join", &["x", "C:y"]),
];

/// Each `file` path route answers what every release's `tclsh` gives
/// for a name every platform and release reads alike, under each release and
/// with no release named, and declines a name Windows or a release reads
/// otherwise.
#[test]
fn path_routes_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    let mut releases = 0usize;
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        releases += 1;
        for &(sub, names) in PATH_WITNESSES {
            let want = tcl_value(&tclsh, &tcl_command("file", Some(sub), names));
            assert!(
                want.is_some(),
                "tclsh{}: file {sub} {names:?} raised",
                version.version_string()
            );
            let got = literal_route(&reg, "file", Some(sub), names, Some(version));
            assert_eq!(
                got,
                want,
                "tclsh{}: file {sub} {names:?}",
                version.version_string()
            );
        }
        for &(sub, names) in PATH_PLATFORM_WITNESSES {
            let got = literal_route(&reg, "file", Some(sub), names, Some(version));
            assert_eq!(
                got,
                None,
                "{}: file {sub} {names:?} answered",
                version.version_string()
            );
        }
    }
    for &(sub, names) in PATH_WITNESSES {
        assert!(
            literal_route(&reg, "file", Some(sub), names, None).is_some(),
            "no release named: file {sub} {names:?} declined"
        );
    }
    if releases == 0 {
        eprintln!("path_routes_match_every_release_on_path: no tclsh found, skipped");
    }
}

/// `(command, subcommand, args)` for the search routes.
type SearchWitness = (&'static str, Option<&'static str>, &'static [&'static str]);

/// `split`, `string first` and `string match` witnesses, each compared with
/// every release's `tclsh` (a release that raises declines or raises).
const SEARCH_WITNESSES: &[SearchWitness] = &[
    ("split", None, &["a b c"]),
    ("split", None, &[" a  b "]),
    ("split", None, &["a\tb\nc\rd"]),
    ("split", None, &["a\u{b}b\u{c}c d"]),
    ("split", None, &[""]),
    ("split", None, &["a,b,,c", ","]),
    ("split", None, &["abc", ""]),
    ("split", None, &["a.b-c", ".-"]),
    ("split", None, &["#a b"]),
    ("split", None, &["{a} b"]),
    ("split", None, &["a\\ b"]),
    ("split", None, &["/api/v1?x=1&y=2", "?"]),
    ("string", Some("first"), &["a", "abc"]),
    ("string", Some("first"), &["bc", "abc"]),
    ("string", Some("first"), &["x", "abc"]),
    ("string", Some("first"), &["", "abc"]),
    ("string", Some("first"), &["a", ""]),
    ("string", Some("first"), &["a", "aaa", "1"]),
    ("string", Some("first"), &["a", "aaa", "end"]),
    ("string", Some("first"), &["a", "aaa", "end-1"]),
    ("string", Some("first"), &["a", "aaa", "5"]),
    ("string", Some("first"), &["a", "aaa", "-1"]),
    ("string", Some("first"), &["a", "abca", "1+1"]),
    ("string", Some("first"), &["a", "abcabca", "010"]),
    ("string", Some("first"), &["c", "abc", "0x1"]),
    ("string", Some("first"), &["abcd", "abc"]),
    ("string", Some("first"), &["?", "/p?q"]),
    ("string", Some("match"), &["a", "a"]),
    ("string", Some("match"), &["a*", "abc"]),
    ("string", Some("match"), &["*c", "abc"]),
    ("string", Some("match"), &["?b?", "abc"]),
    ("string", Some("match"), &["[ab]c", "bc"]),
    ("string", Some("match"), &["[a-c]", "b"]),
    ("string", Some("match"), &["[a-]", "-"]),
    ("string", Some("match"), &["\\*", "*"]),
    ("string", Some("match"), &["*", ""]),
    ("string", Some("match"), &["", ""]),
    ("string", Some("match"), &["", "a"]),
    ("string", Some("match"), &["-nocase", "A", "a"]),
    ("string", Some("match"), &["-nocase", "[A-C]", "b"]),
    ("string", Some("match"), &["-n", "A", "a"]),
    ("string", Some("match"), &["[!a]", "b"]),
    ("string", Some("match"), &["[z-a]", "m"]),
    ("string", Some("match"), &["a[", "a["]),
    ("string", Some("match"), &["*/api/*", "/x/api/y"]),
    ("string", Some("match"), &["-bogus", "a", "a"]),
];

/// `split`, `string first` and `string match` answer on their direct
/// routes what each release's `tclsh` gives, decline or raise where it
/// raises, and with no release named answer only where every release
/// agrees.
#[test]
fn search_routes_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    let mut answers: Vec<Vec<Option<String>>> = vec![Vec::new(); SEARCH_WITNESSES.len()];
    let mut releases = 0usize;
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        releases += 1;
        for (index, &(command, sub, args)) in SEARCH_WITNESSES.iter().enumerate() {
            let words: Vec<String> = args.iter().map(|arg| tcl_quoted_word(arg)).collect();
            let mut script = command.to_owned();
            if let Some(sub) = sub {
                script.push(' ');
                script.push_str(sub);
            }
            for word in &words {
                script.push(' ');
                script.push_str(word);
            }
            let want = tcl_value(&tclsh, &script);
            let got = literal_route(&reg, command, sub, args, Some(version));
            assert_eq!(got, want, "tclsh{}: {script}", version.version_string());
            answers[index].push(want);
        }
    }
    if releases < tcl_dialect::TclVersion::ALL.len() {
        return;
    }
    let mut unanswered = 0usize;
    for (index, &(command, sub, args)) in SEARCH_WITNESSES.iter().enumerate() {
        let every = &answers[index];
        let agreed = every
            .first()
            .cloned()
            .flatten()
            .filter(|first| every.iter().all(|answer| answer.as_ref() == Some(first)));
        // An index the grammars read apart (`010`) declines even where the
        // answers meet; any answer given is every release's.
        let got = literal_route(&reg, command, sub, args, None);
        if got.is_some() || agreed.is_none() {
            assert_eq!(got, agreed, "no release named: {command} {sub:?} {args:?}");
        } else {
            unanswered += 1;
        }
    }
    assert!(
        unanswered <= 1,
        "{unanswered} agreed answers declined with no release named"
    );
}

/// A non-ASCII operand answers only where the target decodes source as
/// UTF-8, and `string match -nocase` over one never answers: each release
/// folds case by its own tables.
#[test]
fn a_search_route_reads_non_ascii_only_where_the_release_decodes_it() {
    use tcl_dialect::TclVersion;
    let reg = CommandRegistry::build_default();
    let route =
        |command, sub, args: &[&str], version| literal_route(&reg, command, sub, args, version);
    assert_eq!(route("split", None, &["é b"], Some(TclVersion::V8_6)), None);
    assert_eq!(
        route("split", None, &["é b"], Some(TclVersion::V9_0)).as_deref(),
        Some("é b")
    );
    assert_eq!(
        route(
            "string",
            Some("first"),
            &["b", "éb"],
            Some(TclVersion::V9_0)
        )
        .as_deref(),
        Some("1")
    );
    assert_eq!(
        route(
            "string",
            Some("match"),
            &["é*", "éa"],
            Some(TclVersion::V9_0)
        )
        .as_deref(),
        Some("1")
    );
    assert_eq!(
        route(
            "string",
            Some("match"),
            &["-nocase", "É", "é"],
            Some(TclVersion::V9_0)
        ),
        None
    );
    assert_eq!(
        route("file", Some("tail"), &["a/é"], Some(TclVersion::V9_0)).as_deref(),
        Some("é")
    );
    assert_eq!(
        route("file", Some("tail"), &["a/é"], Some(TclVersion::V8_6)),
        None
    );
}

/// tcllib 2.0's module tree, where the oracle's packages load from:
/// `TCLLIB_2_0_DIR` or the checkout's `tmp/tcllib-2.0`.
fn tcllib_modules() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
        std::path::PathBuf::from,
    );
    let modules = root.join("modules");
    modules.join("base32").is_dir().then_some(modules)
}

/// `(command, word)` for the base32 routes: encodings of every final-group
/// length and of bytes past ASCII, and decodings canonical, in either case,
/// non-canonical (a set trailing bit), mis-padded, mis-sized and holding a
/// character outside the alphabet.
const BASE32_WITNESSES: &[(&str, &str)] = &[
    ("base32::encode", ""),
    ("base32::encode", "a"),
    ("base32::encode", "ab"),
    ("base32::encode", "abc"),
    ("base32::encode", "abcd"),
    ("base32::encode", "abcde"),
    ("base32::encode", "abcdef"),
    ("base32::encode", "foobar"),
    ("base32::encode", "a b\tc"),
    ("base32::encode", "\u{ff}\u{80}"),
    ("base32::encode", "\u{100}"),
    ("base32::hex::encode", ""),
    ("base32::hex::encode", "a"),
    ("base32::hex::encode", "foobar"),
    ("base32::hex::encode", "\u{ff}\u{80}"),
    ("base32::decode", ""),
    ("base32::decode", "ME======"),
    ("base32::decode", "my======"),
    ("base32::decode", "MFRA===="),
    ("base32::decode", "MFRGG==="),
    ("base32::decode", "MFRGGZA="),
    ("base32::decode", "MFRGGZDF"),
    ("base32::decode", "MZXW6YTBOI======"),
    ("base32::decode", "mzxw6ytboi======"),
    ("base32::decode", "74AIA==="),
    ("base32::decode", "AA======"),
    ("base32::decode", "MZ======"),
    ("base32::decode", "ABCDEFG="),
    ("base32::decode", "A======="),
    ("base32::decode", "========"),
    ("base32::decode", "ABCDEFG"),
    ("base32::decode", "MZ======MZ======"),
    ("base32::decode", "01234567"),
    ("base32::decode", "MZXW 6YT"),
    ("base32::hex::decode", "C4======"),
    ("base32::hex::decode", "c4======"),
    ("base32::hex::decode", "CPNMUOJ1E8======"),
    ("base32::hex::decode", "0123456V"),
    ("base32::hex::decode", "V8======"),
    ("base32::hex::decode", "CPNMV==="),
    ("base32::hex::decode", "W======="),
];

/// `base32::encode`, `base32::decode` and their `base32::hex` twins take one
/// word: tcllib 2.0's procedures raise `wrong # args` on two under every
/// release from 8.5, and the routes decline them.
#[test]
fn base32_routes_decline_a_word_count_the_package_raises_on() {
    use tcl_dialect::TclVersion;
    let Some(modules) = tcllib_modules() else {
        eprintln!(
            "[differential_fold] skipped: no tcllib 2.0 under tmp/ (fetch it, or set TCLLIB_2_0_DIR)"
        );
        return;
    };
    let reg = CommandRegistry::build_default();
    let cases: &[(&str, &[&str])] = &[
        ("base32::encode", &["a", "b"]),
        ("base32::decode", &["ME======", "ME======"]),
        ("base32::hex::encode", &["a", "b"]),
        ("base32::hex::decode", &["C4======", "C4======"]),
    ];
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        for &(command, words) in cases {
            let quoted: Vec<String> = words.iter().map(|word| tcl_quoted_word(word)).collect();
            let script = format!(
                "lappend auto_path {{{}}}\npackage require base32\npackage require base32::hex\n\
                 exit [catch {{{command} {}}}]",
                modules.display(),
                quoted.join(" "),
            );
            let label = format!("tclsh{}: {command} {words:?}", version.version_string());
            assert!(
                matches!(run_tcl(&tclsh, &script), Some((false, _))),
                "{label}: the package raises"
            );
            assert_eq!(
                literal_route(&reg, command, None, words, Some(version)),
                None,
                "{label}"
            );
        }
    }
}

/// `base32::encode`, `base32::decode` and their `base32::hex` twins
/// answer on their direct routes what tcllib 2.0's own packages give under
/// every release from 8.5 on path, a decoding compared byte for byte; they
/// decline every word the package raises for, and the non-canonical
/// encodings its two implementations read apart; every ASCII word the
/// package answers is answered; and with no release named they answer what
/// every release agrees on.
#[test]
fn base32_routes_match_tcllib_on_every_release_on_path() {
    use std::fmt::Write as _;

    use tcl_dialect::TclVersion;
    let Some(modules) = tcllib_modules() else {
        eprintln!(
            "[differential_fold] skipped: no tcllib 2.0 under tmp/ (fetch it, or set TCLLIB_2_0_DIR)"
        );
        return;
    };
    let reg = CommandRegistry::build_default();
    let hex = |text: &str| -> String {
        text.chars().fold(String::new(), |mut out, c| {
            let _ = write!(out, "{:02x}", u32::from(c));
            out
        })
    };
    let releases = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    let mut answers: Vec<Vec<Option<String>>> = vec![Vec::new(); BASE32_WITNESSES.len()];
    let mut compared = 0usize;
    for version in releases {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        compared += 1;
        for (index, &(command, word)) in BASE32_WITNESSES.iter().enumerate() {
            let decodes = command.ends_with("decode");
            let script = format!(
                "lappend auto_path {{{}}}\npackage require base32\npackage require base32::hex\n\
                 if {{[catch {{{command} {}}} __r]}} {{exit 1}}\n{}",
                modules.display(),
                tcl_quoted_word(word),
                if decodes {
                    "binary scan $__r H* __h\nputs -nonewline $__h"
                } else {
                    "puts -nonewline $__r"
                },
            );
            let want = match run_tcl(&tclsh, &script) {
                Some((true, out)) => Some(out),
                _ => None,
            };
            let got = literal_route(&reg, command, None, &[word], Some(version));
            let got = if decodes {
                got.as_deref().map(hex)
            } else {
                got
            };
            let label = format!("tclsh{}: {command} {word:?}", version.version_string());
            match (&want, &got) {
                (_, None) => assert!(
                    want.is_none() || !word.is_ascii(),
                    "{label}: the package answers {want:?} and the route declines"
                ),
                (Some(want), Some(got)) => assert_eq!(got, want, "{label}"),
                (None, Some(got)) => {
                    panic!("{label}: the package raises and the route gives {got:?}")
                }
            }
            answers[index].push(want);
        }
    }
    if compared < releases.len() {
        return;
    }
    for (index, &(command, word)) in BASE32_WITNESSES.iter().enumerate() {
        let every = &answers[index];
        let agreed = every
            .first()
            .cloned()
            .flatten()
            .filter(|first| every.iter().all(|answer| answer.as_ref() == Some(first)));
        let got = literal_route(&reg, command, None, &[word], None);
        let got = if command.ends_with("decode") {
            got.as_deref().map(hex)
        } else {
            got
        };
        if word.is_ascii() {
            assert_eq!(got, agreed, "no release named: {command} {word:?}");
        } else {
            assert!(
                got.is_none() || got == agreed,
                "no release named: {command} {word:?}"
            );
        }
    }
}
