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

//! Differential harness: run a script through both backends and compare.
//!
//! Each backend runs as a subprocess (so a hang or crash is contained and
//! killed on timeout) reading the script from a file. The outcome is the pair
//! `(stdout, status)`; a mismatch in either is a finding. A backend is not
//! hardcoded to `tclvm`/`tclsh` — [`run_backend`] takes any binary plus a
//! fixed set of leading arguments, so the same harness drives every backend
//! pair (`tclvm`↔`tclsh`, `runtime/rust`↔`tclsh`, `tclvm`↔`runtime/rust`; see
//! `engine.rs`).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One backend's outcome on a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Process exited; carries the **verbatim** stdout (lossy-UTF-8 decoded,
    /// but no whitespace or blank-line normalisation), stderr, and whether it
    /// reported an error (non-zero exit). Stderr is captured but, by default,
    /// only folded into the error flag — error *message* text legitimately
    /// differs between engines, so comparing it is opt-in (`--compare-error-text`,
    /// see [`compare_outcomes`]). Keeping stdout byte-faithful is what lets the
    /// differential harness see trailing-whitespace / trailing-blank-line
    /// divergences (`format "%-5s"` / `string repeat " "` padding) — the exact
    /// class of output bug it exists to catch.
    Ran {
        /// Verbatim stdout (no whitespace normalisation).
        stdout: String,
        /// Verbatim stderr (lossy-UTF-8 decoded), for triage and the optional
        /// error-text comparison. Not shown in a routine stdout/status diff.
        stderr: String,
        /// Whether the engine reported an error (non-zero exit).
        errored: bool,
    },
    /// The process exceeded the wall-clock timeout (a hang).
    Timeout,
    /// The backend could not be launched (binary missing / spawn error).
    Unavailable(String),
}

/// How two backends' outcomes relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Identical stdout and matching error status (and, when error-text
    /// comparison is enabled, matching stderr on a shared error).
    Match,
    /// stdout differed.
    StdoutMismatch,
    /// One engine errored and the other did not.
    StatusMismatch,
    /// Both engines errored, but their error message text differed — only
    /// produced when error-text comparison is explicitly requested (see
    /// [`compare_outcomes`]); folded into [`Verdict::Match`] otherwise, since error
    /// *wording* legitimately differs between independent implementations.
    ErrorTextMismatch,
    /// One engine hung.
    Timeout,
    /// A backend was unavailable — the script is skipped, not a finding.
    Skipped,
}

/// Run `script` through `binary`, invoked as `binary [args...] script_file`,
/// with a wall-clock `timeout`.
#[must_use]
pub fn run_backend(
    binary: &Path,
    args: &[String],
    script_file: &Path,
    timeout: Duration,
) -> Outcome {
    let mut child = match Command::new(binary)
        .args(args)
        .arg(script_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Outcome::Unavailable(format!("{}: {e}", binary.display())),
    };

    // Poll for completion up to the timeout, then kill.
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child
                    .wait_with_output()
                    .unwrap_or_else(|_| std::process::Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                return Outcome::Ran {
                    stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                    errored: !status.success(),
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::Timeout;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Outcome::Unavailable(format!("wait: {e}")),
        }
    }
}

/// Compare two backends' outcomes. When `compare_error_text` is set, a run
/// where both engines errored is additionally checked for matching stderr
/// text, surfacing [`Verdict::ErrorTextMismatch`] on a difference instead of
/// silently folding it into [`Verdict::Match`]. Off (`false`) is the
/// long-standing default behaviour: independent implementations legitimately
/// word errors differently (e.g. "wrong # args" phrasing), so only status
/// (errored or not) is compared; on, it catches the rarer, more interesting
/// case of an engine reporting the *wrong kind* of error for the same
/// failure.
#[must_use]
// The unavailable-backend and reference-timeout arms both skip, but for
// distinct reasons worth keeping as separate arms.
#[allow(clippy::match_same_arms)]
pub fn compare_outcomes(
    reference: &Outcome,
    subject: &Outcome,
    compare_error_text: bool,
) -> Verdict {
    match (reference, subject) {
        (Outcome::Unavailable(_), _) | (_, Outcome::Unavailable(_)) => Verdict::Skipped,
        // A reference timeout means the *script* is pathological; skip it
        // rather than blame the subject.
        (Outcome::Timeout, _) => Verdict::Skipped,
        (_, Outcome::Timeout) => Verdict::Timeout,
        (
            Outcome::Ran {
                stdout: rs,
                stderr: rerr,
                errored: re,
            },
            Outcome::Ran {
                stdout: ss,
                stderr: serr,
                errored: se,
            },
        ) => {
            if re != se {
                Verdict::StatusMismatch
            } else if !re && rs != ss {
                // Only compare stdout when neither errored (an errored run's
                // partial stdout is not meaningfully comparable).
                Verdict::StdoutMismatch
            } else if *re && compare_error_text && rerr != serr {
                Verdict::ErrorTextMismatch
            } else {
                Verdict::Match
            }
        }
    }
}

/// Write `script` to a fresh temp file and return its path. The caller removes
/// it. The file keeps a `.tcl` suffix so a `tclsh` shebang/source works.
pub fn write_script(dir: &Path, seed: u64, script: &str) -> std::io::Result<std::path::PathBuf> {
    let path = dir.join(format!("fuzz-{seed}.tcl"));
    let mut f = std::fs::File::create(&path)?;
    f.write_all(script.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ran(stdout: &str, errored: bool) -> Outcome {
        ran_with_stderr(stdout, "", errored)
    }

    fn ran_with_stderr(stdout: &str, stderr: &str, errored: bool) -> Outcome {
        Outcome::Ran {
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            errored,
        }
    }

    /// Default-mode comparison (error text folded to a bool) — what every
    /// campaign used before `--compare-error-text` existed.
    fn compare(reference: &Outcome, subject: &Outcome) -> Verdict {
        compare_outcomes(reference, subject, false)
    }

    #[test]
    fn identical_is_match() {
        assert_eq!(
            compare(&ran("1\n2\n", false), &ran("1\n2\n", false)),
            Verdict::Match
        );
    }

    #[test]
    fn stdout_divergence_flagged() {
        assert_eq!(
            compare(&ran("1\n", false), &ran("2\n", false)),
            Verdict::StdoutMismatch
        );
    }

    #[test]
    fn status_divergence_flagged() {
        assert_eq!(
            compare(&ran("", true), &ran("", false)),
            Verdict::StatusMismatch
        );
        // Both error → match (error messages aren't compared).
        assert_eq!(compare(&ran("x", true), &ran("y", true)), Verdict::Match);
    }

    #[test]
    fn subject_timeout_flagged_reference_timeout_skipped() {
        assert_eq!(
            compare(&ran("", false), &Outcome::Timeout),
            Verdict::Timeout
        );
        assert_eq!(
            compare(&Outcome::Timeout, &ran("", false)),
            Verdict::Skipped
        );
    }

    #[test]
    fn unavailable_backend_skips() {
        assert_eq!(
            compare(&Outcome::Unavailable("x".into()), &ran("", false)),
            Verdict::Skipped
        );
    }

    #[test]
    fn trailing_whitespace_divergence_is_a_finding() {
        // A padding/whitespace difference (`format "%-5s"`,
        // `string repeat " "`) must surface as a StdoutMismatch, not be
        // normalised away into a false Match.
        assert_eq!(
            compare(&ran("x    \n", false), &ran("x\n", false)),
            Verdict::StdoutMismatch
        );
    }

    #[test]
    fn trailing_blank_line_divergence_is_a_finding() {
        assert_eq!(
            compare(&ran("x\n\n", false), &ran("x\n", false)),
            Verdict::StdoutMismatch
        );
    }

    #[test]
    fn byte_identical_stdout_still_matches() {
        assert_eq!(
            compare(&ran("x    \n", false), &ran("x    \n", false)),
            Verdict::Match
        );
    }

    #[test]
    fn error_text_differs_but_folded_to_match_by_default() {
        // Issue #1313: in the default mode, two engines that both error but
        // disagree on the message text are a Match — wording legitimately
        // differs between independent implementations.
        let a = ran_with_stderr("", "wrong # args: should be \"foo x\"", true);
        let b = ran_with_stderr("", "not enough arguments", true);
        assert_eq!(compare(&a, &b), Verdict::Match);
    }

    #[test]
    fn error_text_comparison_is_opt_in() {
        let a = ran_with_stderr("", "wrong # args: should be \"foo x\"", true);
        let b = ran_with_stderr("", "not enough arguments", true);
        assert_eq!(compare_outcomes(&a, &b, true), Verdict::ErrorTextMismatch);
        // Same text, both errored, error-text comparison on → still Match.
        let c = ran_with_stderr("", "boom", true);
        let d = ran_with_stderr("", "boom", true);
        assert_eq!(compare_outcomes(&c, &d, true), Verdict::Match);
    }

    #[test]
    fn error_text_comparison_never_fires_on_a_status_mismatch() {
        // One side errored and the other didn't: that's StatusMismatch
        // regardless of the (incomparable) stderr contents.
        let a = ran_with_stderr("", "boom", true);
        let b = ran_with_stderr("ok", "", false);
        assert_eq!(compare_outcomes(&a, &b, true), Verdict::StatusMismatch);
    }
}
