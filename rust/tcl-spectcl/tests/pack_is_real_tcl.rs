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
use std::process::{Command, Output, Stdio};

use tcl_dialect::TclVersion;
use tcl_test_support::{locate_tclsh, reference_patchlevel};

const REQUIRE_VAR: &str = "TCL_REQUIRE_SPECTCL_COMPAT";
const COMPLETE_SENTINEL: &str = "__TCL_LSP_SPECTCL_ORACLE_COMPLETE__";

/// Absorb the vocabulary, recurse into the bodies the loader executes, and
/// report only what Tcl could not parse.
const DRIVER: &str = r#"
fconfigure stdout -translation lf

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

fn protocol_error(output: &Output) -> Option<String> {
    if !output.status.success() {
        return Some(format!("interpreter exited with {}", output.status));
    }
    if !output.stderr.is_empty() {
        return Some(format!(
            "interpreter wrote unexpected stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let expected = format!("{COMPLETE_SENTINEL}\n");
    if output.stdout != expected.as_bytes() {
        return Some(format!(
            "driver did not produce its exact completion sentinel; stdout was:\n{}",
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    None
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn every_shipped_pack_is_a_tcl_script_a_real_tclsh_accepts() {
    let tclsh = locate_tclsh(TclVersion::V9_0)
        .unwrap_or_else(|error| panic!("could not resolve the exact Tcl 9.0 oracle: {error}"));
    let Some(tclsh) = tclsh else {
        assert!(
            std::env::var_os(REQUIRE_VAR).is_none(),
            "{REQUIRE_VAR}=1 requires tclsh9.0 at the pinned Tcl {} patchlevel",
            reference_patchlevel(TclVersion::V9_0)
        );
        eprintln!(
            "skipping the real-tclsh pack check: no exact Tcl {} tclsh9.0 on PATH. \
             `make test-spectcl-compat` installs and requires the pinned oracle.",
            reference_patchlevel(TclVersion::V9_0)
        );
        return;
    };

    // `tclsh -` reports a top-level error on stderr but still exits zero. Keep
    // that counter-intuitive process contract pinned here: checking status
    // alone would let a broken driver pass with empty stdout.
    let mut broken = Command::new(&tclsh.path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the probed tclsh spawns for the protocol regression");
    broken
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"error {intentional driver failure}\n")
        .expect("write the broken driver");
    let broken = broken
        .wait_with_output()
        .expect("the broken driver completes");
    assert!(
        broken.status.success(),
        "the regression depends on Tcl's stdin driver returning status zero"
    );
    assert!(
        protocol_error(&broken).is_some(),
        "a status-zero driver failure without the completion sentinel must fail closed"
    );

    let root = repo_root();
    let packs = tcl_spectcl::golden::shipped_packs(&root);
    assert!(
        packs.len() >= 24,
        "the inventory must cover the shipped packs"
    );

    let mut child = Command::new(&tclsh.path)
        .arg("-")
        .args(packs.iter().map(|p| p.as_os_str()))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the probed tclsh spawns");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin
        .write_all(DRIVER.as_bytes())
        .expect("write the driver");
    writeln!(stdin, "puts {COMPLETE_SENTINEL}").expect("write the completion sentinel");
    drop(stdin);
    let out = child.wait_with_output().expect("the probed tclsh runs");
    if let Some(error) = protocol_error(&out) {
        panic!(
            "{} (Tcl {}) did not complete the pack parse oracle: {error}",
            tclsh.path.display(),
            tclsh.patchlevel
        );
    }
}
