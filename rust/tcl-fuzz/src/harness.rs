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
    /// Process exited; carries normalised stdout and whether it reported an
    /// error (non-zero exit). Stderr is folded into the error flag, not
    /// compared verbatim — error *message* text legitimately differs between
    /// engines.
    Ran {
        /// Normalised stdout.
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
                    stdout: normalise(&String::from_utf8_lossy(&out)),
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

/// Normalise stdout for comparison: trailing whitespace per line removed and a
/// single trailing newline, so insignificant spacing differences don't trip a
/// false finding.
fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for line in s.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
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
    fn normalise_trims_and_collapses() {
        assert_eq!(normalise("a  \nb\n\n\n"), "a\nb\n");
    }
}
