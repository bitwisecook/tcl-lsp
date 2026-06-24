//! Tcl 9 `reg.test` as a typed Rust cargo test.
//!
//! Each row of `tests/data/reg_cases.tsv` was produced by driving the real Tcl
//! 9 engine (`testregexp`, in the built `tcltest` shell) over every variant of
//! every `reg.test` directive — including the `&` ARE/BRE expansion — so the
//! recorded `nsub`, `re_info` bits, compile-error codes, and `-indices`
//! submatch positions are authoritative. Matching them is equivalent to passing
//! `reg.test`, since `reg.test`'s own assertions are against this same engine.
//!
//! The corpus is materialised into strongly-typed [`Case`] values; the flag
//! string is interpreted into typed `cflags`/`eflags` exactly as
//! `tclTest.c`'s `TestregexpXflags` + reg.test's `TestFlags` do.

// Engine spans are small char offsets that always fit in `i64`; the
// `usize -> i64` casts when converting to Tcl `-indices` pairs cannot wrap.
#![allow(clippy::cast_possible_wrap)]

use tcl_regex::Regex;
use tcl_regex::defs::{
    REG_ADVANCED, REG_ADVF, REG_BOSONLY, REG_EXPANDED, REG_EXPECT, REG_EXTENDED, REG_ICASE,
    REG_NEWLINE, REG_NLANCH, REG_NLSTOP, REG_NOSUB, REG_NOTBOL, REG_NOTEOL, REG_QUOTE,
};

const CORPUS: &str = include_str!("data/reg_cases.tsv");

/// The expected outcome for a compiled+executed case.
#[derive(Debug)]
enum Outcome {
    /// Compile fails with this `REG_*` error name.
    Error(String),
    /// Compiles; `nsub` + info bits, plus (when a target is present) whether it
    /// matched and the per-group inclusive `{so eo}` index pairs.
    Ok {
        nsub: usize,
        info: Vec<String>,
        target: Option<Vec<u32>>,
        matched: Option<bool>,
        pairs: Vec<(i64, i64)>,
    },
}

struct Case {
    id: String,
    flags: String,
    re: Vec<u32>,
    outcome: Outcome,
}

/// Tcl flag string -> (cflags, eflags), porting reg.test `TestFlags` plus
/// `tclTest.c` `TestregexpXflags`.
fn flags_to_cflags(flags: &str) -> (i32, i32) {
    let mut cflags = REG_ADVANCED;
    let mut eflags = 0;
    let mut xflags = String::new();
    for c in flags.chars() {
        match c {
            'i' => cflags |= REG_ICASE,
            'x' => cflags |= REG_EXPANDED,
            'n' => cflags |= REG_NEWLINE,
            'p' => cflags |= REG_NLSTOP,
            'w' => cflags |= REG_NLANCH,
            '-' => {}
            other => xflags.push(other),
        }
    }
    for c in xflags.chars() {
        match c {
            'a' => cflags |= REG_ADVF,
            'b' => cflags &= !REG_ADVANCED,
            'e' => {
                cflags &= !REG_ADVANCED;
                cflags |= REG_EXTENDED;
            }
            'q' => {
                cflags &= !REG_ADVANCED;
                cflags |= REG_QUOTE;
            }
            'o' => cflags |= REG_NOSUB,
            's' => cflags |= REG_BOSONLY,
            't' => cflags |= REG_EXPECT,
            '^' => eflags |= REG_NOTBOL,
            '$' => eflags |= REG_NOTEOL,
            _ => {}
        }
    }
    (cflags, eflags)
}

fn parse_cps(field: &str) -> Vec<u32> {
    if field.is_empty() {
        return Vec::new();
    }
    field.split(',').map(|s| s.parse().unwrap()).collect()
}

fn parse_cases() -> Vec<Case> {
    let mut out = Vec::new();
    for line in CORPUS.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        // id flags reCp hasTarget targetCp status nsub info matched pairs
        let id = f[0].to_string();
        let flags = f[1].to_string();
        let re = parse_cps(f[2]);
        let has_target = f[3] == "1";
        let target = if has_target {
            Some(parse_cps(f[4]))
        } else {
            None
        };
        let status = f[5];
        let outcome = if let Some(name) = status.strip_prefix("error:") {
            Outcome::Error(name.to_string())
        } else {
            let nsub: usize = f[6].parse().unwrap_or(0);
            let info: Vec<String> = if f.get(7).is_none_or(|s| s.is_empty()) {
                Vec::new()
            } else {
                f[7].split(',').map(str::to_string).collect()
            };
            let matched = match f.get(8) {
                Some(&"1") => Some(true),
                Some(&"0") => Some(false),
                _ => None,
            };
            let pairs = match f.get(9) {
                Some(s) if !s.is_empty() => s
                    .split(',')
                    .map(|p| {
                        let (a, b) = p.split_once(':').unwrap();
                        (a.parse().unwrap(), b.parse().unwrap())
                    })
                    .collect(),
                _ => Vec::new(),
            };
            Outcome::Ok {
                nsub,
                info,
                target,
                matched,
                pairs,
            }
        };
        out.push(Case {
            id,
            flags,
            re,
            outcome,
        });
    }
    out
}

/// Convert engine spans (half-open char offsets) to Tcl `-indices` pairs
/// (inclusive end; `{-1 -1}` for a non-participating subexpression).
fn to_pairs(groups: &[Option<tcl_regex::Span>]) -> Vec<(i64, i64)> {
    groups
        .iter()
        .map(|g| match g {
            Some(s) => (s.start as i64, s.end as i64 - 1),
            None => (-1, -1),
        })
        .collect()
}

fn check(case: &Case) -> Result<(), String> {
    let (cflags, eflags) = flags_to_cflags(&case.flags);
    let compiled = Regex::compile(&case.re, cflags);
    match &case.outcome {
        Outcome::Error(name) => match compiled {
            Ok(_) => Err(format!("expected compile error {name}, but compiled")),
            Err(e) => {
                let got = format!("REG_{}", e.name());
                if &got == name {
                    Ok(())
                } else {
                    Err(format!("compile error {got}, expected {name}"))
                }
            }
        },
        Outcome::Ok {
            nsub,
            info,
            target,
            matched,
            pairs,
        } => {
            let re = compiled.map_err(|e| format!("unexpected compile error REG_{}", e.name()))?;
            if re.nsub() != *nsub {
                return Err(format!("nsub {} != expected {}", re.nsub(), nsub));
            }
            let got_info: Vec<String> = re
                .info()
                .flags()
                .iter()
                .map(|f| f.name().to_string())
                .collect();
            if got_info != *info {
                return Err(format!("info {got_info:?} != expected {info:?}"));
            }
            if let Some(target) = target {
                let got = re.exec(target, 0, eflags);
                match (got, matched) {
                    (None, Some(true)) => Err("expected match, got none".to_string()),
                    (Some(_), Some(false)) => Err("expected no match, got one".to_string()),
                    (Some(groups), Some(true)) => {
                        // Under REG_NOSUB the engine tracks no submatches, so
                        // `testregexp -indices` yields meaningless slots; only
                        // the match/no-match outcome is well defined.
                        if cflags & REG_NOSUB != 0 {
                            return Ok(());
                        }
                        let got_pairs = to_pairs(&groups);
                        if got_pairs == *pairs {
                            Ok(())
                        } else {
                            Err(format!("indices {got_pairs:?} != expected {pairs:?}"))
                        }
                    }
                    (None, Some(false)) | (_, None) => Ok(()),
                }
            } else {
                Ok(())
            }
        }
    }
}

#[test]
fn reg_test_corpus() {
    let cases = parse_cases();
    assert!(!cases.is_empty(), "no cases parsed");
    let mut failures: Vec<(String, String)> = Vec::new();
    for case in &cases {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(case))) {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => failures.push((case.id.clone(), msg)),
            Err(_) => failures.push((case.id.clone(), "PANIC".to_string())),
        }
    }
    let total = cases.len();
    let passed = total - failures.len();
    eprintln!(
        "reg.test corpus: {passed}/{total} passed, {} failed",
        failures.len()
    );
    for (id, msg) in failures.iter().take(60) {
        eprintln!("  FAIL reg-{id}: {msg}");
    }
    assert!(
        failures.is_empty(),
        "{} reg.test cases failed (see list above)",
        failures.len()
    );
}

/// reg-0.1: the error-message text for an invalid quantifier operand.
#[test]
fn reg_0_1_error_message() {
    let err = Regex::compile_str("(*)", REG_ADVANCED)
        .err()
        .expect("(*) must fail to compile");
    assert_eq!(err.message(), "invalid quantifier operand");
}
