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

//! A `.tclspec` pack is a **Tcl script**, and the arbiter of that is a real
//! Tcl interpreter, not our own parser.
//!
//! Design E's whole claim is that a pack is a program the VM executes, with
//! the registration vocabulary installed as host commands. If a shipped pack
//! were only *nearly* Tcl — a stray unbalanced brace inside a comment, a `$`
//! or `[` our segmenter happens to tolerate — the claim would be false and
//! our own agreement with ourselves would not notice.
//!
//! So: hand every shipped pack to `tclsh9.0` and require it to source
//! cleanly. `unknown` swallows every vocabulary word, and the three words
//! whose trailing brace group the loader really does evaluate as a script
//! (`speclib`, `command`, `subcommand` — a row word's body is captured
//! verbatim and replayed, never executed) recurse into it, so a real
//! interpreter parses every nested block rather than stopping at the outer
//! braces.
//!
//! A body evaluated this way raises ordinary runtime errors — a hook body
//! reads `ctx`, which does not exist here — so only a *parse* failure counts.
//! Tcl leaves `::errorCode` as `NONE` for those and sets a `TCL …` code for
//! everything it managed to parse and then could not run.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Absorb the vocabulary, recurse into the bodies the loader executes, and
/// report only what Tcl could not parse.
const DRIVER: &str = r#"
proc unknown {args} { return "" }

set failures {}

# Evaluate one block body. A body raises ordinary runtime errors here — a
# hook reads `ctx`, which does not exist — and Tcl sets a `TCL …` errorCode
# for anything it parsed before failing, so `NONE` isolates the parse errors.
proc block {body} {
    if {![catch {uplevel #0 $body} err]} { return }
    if {$::errorCode ne "NONE"} { return }
    lappend ::failures $err
}

# The three words whose trailing brace group the loader evaluates as a
# script; every other word's body is captured verbatim and replayed.
foreach word {speclib command subcommand} {
    proc $word {args} {
        if {[llength $args]} { block [lindex $args end] }
        return ""
    }
}

foreach path $argv {
    set failures {}
    if {[catch {uplevel #0 [list source $path]} err] && $::errorCode eq "NONE"} {
        lappend failures $err
    }
    foreach failure $failures { puts "$path: $failure" }
}
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// The first interpreter on `PATH` that runs and reports a usable release.
///
/// The pack syntax is plain Tcl, so any modern interpreter is a fair judge;
/// 9.x is preferred only because it is what the packs target. Follows the
/// house pattern of `tcl-registry`'s dialect oracle: probe, do not assume —
/// the `rust-tests` CI job installs Rust and nextest, not Tcl.
fn find_tclsh() -> Option<&'static str> {
    ["tclsh9.1", "tclsh9.0", "tclsh8.6", "tclsh"]
        .into_iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("-")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
}

#[test]
fn every_shipped_pack_is_a_tcl_script_a_real_tclsh_accepts() {
    let Some(tclsh) = find_tclsh() else {
        eprintln!(
            "skipping the real-tclsh pack gate: no working tclsh on PATH. \
             `make ensure-test-deps` installs the interpreters; CI runs this \
             gate on the jobs that have them."
        );
        return;
    };

    let root = repo_root();
    let packs = tcl_spectcl::golden::shipped_packs(&root);
    assert!(
        packs.len() >= 24,
        "the inventory must cover the shipped packs"
    );

    let mut child = Command::new(tclsh)
        .arg("-")
        .args(packs.iter().map(|p| p.as_os_str()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the probed tclsh spawns");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(DRIVER.as_bytes())
        .expect("write the driver");
    let out = child.wait_with_output().expect("the probed tclsh runs");
    let failures = String::from_utf8_lossy(&out.stdout);
    assert!(
        failures.trim().is_empty(),
        "{tclsh} refuses to source these packs:\n{failures}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
