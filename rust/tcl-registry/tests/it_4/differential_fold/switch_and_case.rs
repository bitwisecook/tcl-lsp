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

//! Which arm `switch` and `case` select, including fall-through bodies, against every release.

use super::*;

/// One `switch` witness: the words before the subject, the subject, the
/// arms as `(pattern, whether its body is the fall-through body)`, whether
/// the arms are one clause-list word, and the variables read back after it.
type SwitchWitness = (
    &'static [&'static str],
    &'static str,
    &'static [(&'static str, bool)],
    bool,
    &'static [&'static str],
);

/// The selection witnesses — the first five are ordered patterns, the final
/// default, a `-` arm whose next pattern never matches supplying its body,
/// regexp captures and a malformed regexp — each measured
/// on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0: the captures and
/// `-nocase` raise `bad option` on 8.4, and a subject spelled like an
/// option is one there.
const SWITCH_WITNESSES: &[SwitchWitness] = &[
    (
        &["-glob", "--"],
        "abc",
        &[("a*", false), ("ab*", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-exact", "--"],
        "zzz",
        &[("a", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-glob", "--"],
        "x",
        &[("x", true), ("y", false)],
        true,
        &[],
    ),
    (
        &["-regexp", "-matchvar", "m", "-indexvar", "i", "--"],
        "abc",
        &[("(a)(x)?b", false)],
        true,
        &["m", "i"],
    ),
    (&["-regexp", "--"], "abc", &[("(", false)], true, &[]),
    (
        &["--"],
        "zzz",
        &[("default", false), ("b", false)],
        false,
        &[],
    ),
    (&["-nocase", "--"], "ABC", &[("abc", false)], true, &[]),
    (
        &["-regexp", "-matchvar", "m", "--"],
        "abc",
        &[("x", false), ("default", false)],
        true,
        &["m"],
    ),
    (
        &["-glob"],
        "-x",
        &[("a", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob"],
        "-x",
        &[("a", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-regexp", "--"],
        "aXb",
        &[("a.b", true), ("zz", false), ("q", false)],
        false,
        &[],
    ),
    (&["-exact"], "b", &[("a", false), ("b", false)], true, &[]),
    (&["-glob", "--"], "a[b", &[("a\\[*", false)], true, &[]),
    (
        &["-regexp", "-nocase", "--"],
        "ABC",
        &[("^a", false)],
        true,
        &[],
    ),
    // The glob grammar's edges, which every release reads alike.
    (
        &["-glob", "--"],
        "m",
        &[("[z-a]", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob", "--"],
        "b",
        &[("[!a]", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-glob", "--"],
        "[a",
        &[("[a", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob", "--"],
        "a",
        &[("[a", false), ("default", false)],
        true,
        &[],
    ),
];

/// The words `switch` receives for `witness`, each body `body(arm)` or the
/// fall-through body.
fn switch_words(
    (before, subject, arms, list, _): SwitchWitness,
    body: impl Fn(usize) -> String,
) -> Vec<String> {
    let clauses: Vec<String> = arms
        .iter()
        .enumerate()
        .flat_map(|(arm, &(pattern, falls))| {
            [
                pattern.to_owned(),
                if falls { "-".to_owned() } else { body(arm) },
            ]
        })
        .collect();
    let mut words: Vec<String> = before.iter().map(|&word| word.to_owned()).collect();
    words.push(subject.to_owned());
    if list {
        let mut rendered = Vec::new();
        for (index, clause) in clauses.iter().enumerate() {
            if index != 0 {
                rendered.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut rendered, clause.as_bytes(), index == 0);
        }
        words.push(String::from_utf8(rendered).expect("text"));
    } else {
        words.extend(clauses);
    }
    words
}

/// What `switch`'s selection answers for `witness` under `profile`: the arm
/// whose body runs (`none` when none does), then each observed variable —
/// its written value, or `<unset>` — or `None` when the selection declines.
fn switch_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: SwitchWitness,
) -> Option<Vec<String>> {
    use tcl_registry::value_transfer::{
        Budget, FactDomain, LiteralInputs, StoreOutcome, TransferAnswer, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get("switch").expect("switch"), None, None);
    let semantics = semantics.semantics().expect("switch's selection contract");
    let words = switch_words(witness, |arm| format!("B{arm}"));
    let args: Vec<&str> = words.iter().map(String::as_str).collect();
    // Each word delimited as the oracle spells it: `-` bare, every other
    // word quoted.
    let inputs = args.iter().enumerate().fold(
        LiteralInputs::new("switch", None, &args, profile),
        |inputs, (index, &word)| {
            let id = tcl_registry::value_transfer::OperandId(index);
            if word == "-" {
                inputs.with_bare(id)
            } else {
                inputs.with_quoted(id)
            }
        },
    );
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    assert_eq!(fact.selected.len(), 1, "one member");
    let mut seen = vec![fact.bodies[0].map_or_else(|| "none".to_owned(), |arm| arm.to_string())];
    for name in witness.4 {
        let written = fact.writes[0].iter().rev().find_map(|store| match store {
            StoreOutcome::Write { target, value } if args[(target.0).0] == *name => {
                Some(String::from_utf8(value.bytes.clone()).expect("text"))
            }
            _ => None,
        });
        seen.push(written.unwrap_or_else(|| "<unset>".to_owned()));
    }
    Some(seen)
}

/// What `tclsh` answers for the same witness, one line per value, or
/// `None` when the command raises.
fn switch_oracle(tclsh: &str, witness: SwitchWitness) -> Option<Vec<String>> {
    use std::fmt::Write as _;
    let words = switch_words(witness, |arm| format!("set __body {arm}"));
    let mut script = String::from("set __body none\nif {[catch {switch");
    for word in &words {
        script.push(' ');
        // A fall-through body is spelled bare: 9.1b0's byte-compiled
        // `switch` reads only a bare `-` as one (`IsFallthroughToken` in
        // `tclCompCmdsSZ.c` measures the word with its delimiters), where
        // 8.4 to 9.0, and 9.1's own interpreted path, read the value.
        if word == "-" {
            script.push('-');
        } else {
            script.push_str(&tcl_quoted_word(word));
        }
    }
    script.push_str("}]} {exit 1}\nputs $__body\n");
    let _ = writeln!(
        script,
        "foreach __name {{{}}} {{\n\
         if {{[info exists $__name]}} {{puts [set $__name]}} else {{puts <unset>}}\n\
         }}",
        witness.4.join(" ")
    );
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out.lines().map(str::to_owned).collect()),
        (false, _) => None,
    }
}

/// `switch`'s selection, per release found on `PATH`, each under
/// that release's profile against the real `tclsh`: when the selection
/// answers, the body it names runs and the variables it writes hold what
/// it wrote; when `tclsh` raises, the selection declines; the first five
/// witnesses answer under every release that runs them, the captures from
/// 8.5.
#[test]
fn switch_witnesses_match_every_release_on_path() {
    // The plan's witnesses, which every release that runs them answers.
    const REQUIRED: [usize; 5] = [0, 1, 2, 3, 4];
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        for (index, &witness) in SWITCH_WITNESSES.iter().enumerate() {
            let want = switch_oracle(&tclsh, witness);
            let got = switch_route(&reg, profile, witness);
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: switch {:?}",
                        version.version_string(),
                        switch_words(witness, |arm| format!("B{arm}"))
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on switch {:?}, the selection answered {got:?}",
                    version.version_string(),
                    switch_words(witness, |arm| format!("B{arm}"))
                ),
                (Some(_), None) => assert!(
                    !REQUIRED.contains(&index),
                    "tclsh{}: the selection declined the witness switch {:?}",
                    version.version_string(),
                    switch_words(witness, |arm| format!("B{arm}"))
                ),
                (None, None) => {}
            }
        }
        // 8.4 raises on the captures and `-nocase`; every other release
        // answers all but the subject spelled like an option, which the
        // plan's layout abstains on.
        assert!(
            agreed >= 7,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// Run `script` from a file under `tclsh`, returning `(exit_ok, stdout)`.
/// Unlike [`run_tcl`], whose stdin commands are each compiled, a script
/// file's own top-level commands run on the interpreted path.
fn run_tcl_file(tclsh: &str, script: &str) -> Option<(bool, String)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "differential-fold-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, script).ok()?;
    let output = Command::new(tclsh)
        .arg(&path)
        .stderr(Stdio::null())
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// A fall-through body spelled `-` reads two ways on 9.1b0 when it is
/// quoted: its byte-compiled `switch` — here inside a procedure —
/// recognises only a bare `-` (`IsFallthroughToken` in `tclCompCmdsSZ.c`
/// measures the word with its quotes) and runs a quoted one as a command,
/// where its interpreted path — a script file's own top level — reads the
/// word's value and falls through, as 8.4.20 to 9.0.4 do on both paths.
/// So the selection declines the quoted form under a profile that may be
/// 9.1 (`tcl9.1`, and `tcl`, which names no release) and decides it under
/// `tcl8.6` and `tcl9.0`, the body it names being the one those releases
/// run; the bare form decides under every one of them.
#[test]
fn a_quoted_fallthrough_body_reads_two_ways_on_91() {
    use tcl_registry::value_transfer::{
        Axis, Budget, DeclineReason, FactDomain, LiteralInputs, OperandId, TransferAnswer,
        resolve_semantics,
    };
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("switch").expect("switch"), None, None);
    let semantics = semantics.semantics().expect("switch's selection contract");
    let words = ["-glob", "--", "a", "a", "-", "b", "B"];
    let bodies = |dialect: &str, quoted: bool| {
        let profile = tcl_dialect::DialectProfile::find(dialect);
        let inputs = (0..words.len()).fold(
            LiteralInputs::new("switch", None, &words, profile),
            |inputs, index| {
                if index == 4 && !quoted {
                    inputs.with_bare(OperandId(index))
                } else {
                    inputs.with_quoted(OperandId(index))
                }
            },
        );
        match semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation()) {
            TransferAnswer::Selection(fact) => Ok(fact.bodies),
            TransferAnswer::Declined(reason) => Err(reason),
            other => panic!("{dialect}: {other:?}"),
        }
    };
    let ambiguous = Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(
        tcl_dialect::model::SpecSurface::TCL91[0],
    )));
    for dialect in ["tcl9.1", "tcl"] {
        assert_eq!(bodies(dialect, true), ambiguous, "{dialect}: quoted");
        assert_eq!(bodies(dialect, false), Ok(vec![Some(1)]), "{dialect}: bare");
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert_eq!(
            bodies(dialect, true),
            Ok(vec![Some(1)]),
            "{dialect}: quoted"
        );
        assert_eq!(bodies(dialect, false), Ok(vec![Some(1)]), "{dialect}: bare");
    }

    // The oracle: the arm's body, `B`, or the error of running `-`.
    let in_proc = |body: &str| {
        format!(
            "proc p {{}} {{switch -glob -- a a {body} b {{return B}}}}\n\
             if {{[catch p result]}} {{puts error}} else {{puts $result}}\n"
        )
    };
    let at_top = |body: &str| format!("puts [switch -glob -- a a {body} b {{set _ B}}]\n");
    for (series, quoted_in_proc) in [("8.6", "B"), ("9.0", "B"), ("9.1", "error")] {
        let Some(tclsh) = find_tclsh(series) else {
            continue;
        };
        for (body, in_proc_prints) in [("\"-\"", quoted_in_proc), ("-", "B")] {
            assert_eq!(
                run_tcl_file(&tclsh, &in_proc(body)),
                Some((true, format!("{in_proc_prints}\n"))),
                "tclsh{series}: {body} inside a procedure"
            );
            assert_eq!(
                run_tcl_file(&tclsh, &at_top(body)),
                Some((true, "B\n".to_owned())),
                "tclsh{series}: {body} at the top level"
            );
        }
    }
}

/// One `case` witness: the subject, whether the `in` word follows it, the
/// patterns in order (arm `N`'s body sets `__body` to `N`), whether the
/// clauses are one list word, and whether the final pattern has no body.
type CaseWitness = (&'static str, bool, &'static [&'static str], bool, bool);

/// The `case` witnesses, each measured on tclsh 8.4.20, 8.5.19 and
/// 8.6.18, which answer every one alike: glob patterns, the `in` word and
/// the one-word form, a literal pattern, pattern lists and a backslash that
/// makes a pattern one, `default` wherever it stands and matched literally,
/// the last `default` winning, a list's `default` element no fallback, `-`
/// an ordinary pattern, no match, the missing body raised only when the
/// scan reaches it, and a malformed pattern list raising.
const CASE_WITNESSES: &[CaseWitness] = &[
    ("abc", true, &["a*", "default"], false, false),
    ("zzz", true, &["a*", "default"], false, false),
    ("abc", false, &["a*", "default"], false, false),
    ("abc", false, &["a*", "default"], true, false),
    ("abc", true, &["a*", "default"], true, false),
    ("abc", true, &["abc"], false, false),
    ("abc", true, &["x a*", "default"], false, false),
    ("abc", true, &["a\\*", "default"], false, false),
    ("a*", true, &["a\\*", "default"], false, false),
    ("zzz", true, &["default", "a"], false, false),
    ("default", true, &["default", "def*"], false, false),
    ("abc", true, &["-", "default"], false, false),
    ("abc", true, &["q"], false, false),
    ("abc", true, &["a b"], false, false),
    ("a b", true, &["a b", "default"], false, false),
    ("a b", true, &["{a b}", "default"], false, false),
    ("abc", true, &["", "default"], false, false),
    ("", true, &["", "default"], false, false),
    ("abc", true, &["[a-c]*"], false, false),
    ("abc", true, &["A*", "default"], false, false),
    ("abc", true, &["a*\\", "default"], false, false),
    ("zzz", true, &["default", "default"], false, false),
    ("abc", true, &["default x", "a*"], false, false),
    ("zzz", true, &["default x", "b*"], false, false),
    ("in", true, &["in", "default"], false, false),
    ("abc", false, &["-", "default"], false, false),
    ("abc", true, &["a {"], false, false),
    ("zzz", true, &["a*", "b*"], false, true),
    // The plan abstains on these two, which tclsh answers: the missing
    // body the scan never reaches, and a subject spelled like an option.
    ("abc", true, &["a*", "b*"], false, true),
    ("-x", true, &["-*"], false, false),
];

/// The witnesses the plan abstains on: the last two.
const CASE_ABSTAINS: usize = 2;

/// The words `case` receives for `witness`, each body `body(arm)`.
fn case_words(
    (subject, separated, patterns, list, dangling): CaseWitness,
    body: impl Fn(usize) -> String,
) -> Vec<String> {
    let mut clauses = Vec::new();
    for (arm, &pattern) in patterns.iter().enumerate() {
        clauses.push(pattern.to_owned());
        if !(dangling && arm + 1 == patterns.len()) {
            clauses.push(body(arm));
        }
    }
    let mut words = vec![subject.to_owned()];
    if separated {
        words.push("in".to_owned());
    }
    if list {
        let mut rendered = Vec::new();
        for (index, clause) in clauses.iter().enumerate() {
            if index != 0 {
                rendered.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut rendered, clause.as_bytes(), index == 0);
        }
        words.push(String::from_utf8(rendered).expect("text"));
    } else {
        words.extend(clauses);
    }
    words
}

/// The arm whose body `case`'s selection runs for `witness` under
/// `profile` (`none` when none does), or `None` when it declines.
fn case_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: CaseWitness,
) -> Option<String> {
    use tcl_registry::value_transfer::{
        Budget, FactDomain, LiteralInputs, TransferAnswer, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get("case").expect("case"), None, None);
    let semantics = semantics.semantics().expect("case's selection contract");
    let words = case_words(witness, |arm| format!("B{arm}"));
    let args: Vec<&str> = words.iter().map(String::as_str).collect();
    let inputs = LiteralInputs::new("case", None, &args, profile);
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    assert_eq!(fact.selected.len(), 1, "one member");
    assert_eq!(fact.selected, fact.bodies, "no fall-through body");
    assert!(fact.writes.iter().all(Vec::is_empty), "no writes");
    Some(fact.bodies[0].map_or_else(|| "none".to_owned(), |arm| arm.to_string()))
}

/// What `tclsh` answers for the same witness, or `None` when it raises.
fn case_oracle(tclsh: &str, witness: CaseWitness) -> Option<String> {
    let words = case_words(witness, |arm| format!("set __body {arm}"));
    let mut script = String::from("set __body none\nif {[catch {case");
    for word in &words {
        script.push(' ');
        script.push_str(&tcl_quoted_word(word));
    }
    script.push_str("}]} {exit 1}\nputs -nonewline $__body\n");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// `case`'s selection against the real `tclsh` of each release that
/// has the command — 8.4, 8.5 and 8.6, each under its own profile, and the
/// iRules profile on its 8.4 base (ruling 8): where tclsh answers, the
/// selection names the body tclsh runs, or abstains on the two witnesses
/// the layout its plan does not read; where tclsh raises, the selection
/// declines. From 9.0 there is no `case` to select with.
#[test]
fn case_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    for (series, dialect) in [
        ("8.4", "tcl8.4"),
        ("8.5", "tcl8.5"),
        ("8.6", "tcl8.6"),
        ("8.4", "f5-irules"),
    ] {
        let Some(tclsh) = find_tclsh(series) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(dialect);
        assert!(profile.is_some(), "{dialect}");
        for (index, &witness) in CASE_WITNESSES.iter().enumerate() {
            let want = case_oracle(&tclsh, witness);
            let got = case_route(&reg, profile, witness);
            let words = case_words(witness, |arm| format!("B{arm}"));
            let abstains = index >= CASE_WITNESSES.len() - CASE_ABSTAINS;
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(got, want, "tclsh{series} ({dialect}): case {words:?}");
                    assert!(!abstains, "{dialect}: case {words:?} no longer abstains");
                }
                (None, Some(got)) => {
                    panic!("tclsh{series} raises on case {words:?}, the selection answered {got}")
                }
                (Some(_), None) => assert!(
                    abstains,
                    "tclsh{series} ({dialect}): the selection declined case {words:?}"
                ),
                (None, None) => {}
            }
        }
    }
    for series in ["9.0", "9.1"] {
        if let Some(tclsh) = find_tclsh(series) {
            assert_eq!(
                run_tcl(&tclsh, "puts [catch {case a in a {}} m]$m"),
                Some((true, "1invalid command name \"case\"\n".to_owned())),
                "tclsh{series} has no case"
            );
        }
    }
}
