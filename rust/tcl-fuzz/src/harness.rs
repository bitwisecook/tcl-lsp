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
//! `(stdout, status)`; a mismatch in either is a finding.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One backend's outcome on a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Process exited; carries the **verbatim** stdout (lossy-UTF-8 decoded,
    /// but no whitespace or blank-line normalisation) and whether it reported an
    /// error (non-zero exit). Stderr is folded into the error flag, not
    /// compared verbatim — error *message* text legitimately differs between
    /// engines. Keeping stdout byte-faithful is what lets the differential
    /// harness see trailing-whitespace / trailing-blank-line divergences
    /// (`format "%-5s"` / `string repeat " "` padding) — the exact class of
    /// output bug it exists to catch.
    Ran {
        /// Verbatim stdout (no whitespace normalisation).
        stdout: String,
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
    /// Identical stdout and matching error status.
    Match,
    /// stdout differed.
    StdoutMismatch,
    /// One engine errored and the other did not.
    StatusMismatch,
    /// One engine hung.
    Timeout,
    /// A backend was unavailable — the script is skipped, not a finding.
    Skipped,
}

/// Run `script` through the backend `binary` with a wall-clock `timeout`.
#[must_use]
pub fn run_backend(binary: &Path, script_file: &Path, timeout: Duration) -> Outcome {
    let mut child = match Command::new(binary)
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
                    .map(|o| o.stdout)
                    .unwrap_or_default();
                return Outcome::Ran {
                    stdout: String::from_utf8_lossy(&out).into_owned(),
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

/// Compare the reference (`tclsh`) and subject (`tclvm`) outcomes.
#[must_use]
// The unavailable-backend and reference-timeout arms both skip, but for
// distinct reasons worth keeping as separate arms.
#[allow(clippy::match_same_arms)]
pub fn compare(reference: &Outcome, subject: &Outcome) -> Verdict {
    match (reference, subject) {
        (Outcome::Unavailable(_), _) | (_, Outcome::Unavailable(_)) => Verdict::Skipped,
        // A reference timeout means the *script* is pathological; skip it
        // rather than blame the subject.
        (Outcome::Timeout, _) => Verdict::Skipped,
        (_, Outcome::Timeout) => Verdict::Timeout,
        (
            Outcome::Ran {
                stdout: rs,
                errored: re,
            },
            Outcome::Ran {
                stdout: ss,
                errored: se,
            },
        ) => {
            if re != se {
                Verdict::StatusMismatch
            } else if !re && rs != ss {
                // Only compare stdout when neither errored (an errored run's
                // partial stdout is not meaningfully comparable).
                Verdict::StdoutMismatch
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
        Outcome::Ran {
            stdout: stdout.to_owned(),
            errored,
        }
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
}
