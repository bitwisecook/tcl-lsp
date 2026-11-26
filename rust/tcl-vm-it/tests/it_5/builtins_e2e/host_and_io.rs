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

//! Commands that reach the host: `exec`, `file`, `source`, `clock`, channels and the bootstrap globals, with a synthetic host.

use super::*;

struct SyntheticHost {
    clock: SyntheticClock,
    stdio: SyntheticStdIo,
    env: SyntheticEnv,
}

impl SyntheticHost {
    pub(super) fn new(entries: &[(&str, &str)]) -> Self {
        Self {
            clock: SyntheticClock,
            stdio: SyntheticStdIo,
            env: SyntheticEnv(
                entries
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            ),
        }
    }
}

struct SyntheticClock;

impl tcl_platform::Clock for SyntheticClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

struct SyntheticStdIo;

impl tcl_platform::StdIo for SyntheticStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}

    fn write_stderr(&self, _bytes: &[u8]) {}
}

struct SyntheticEnv(BTreeMap<String, String>);

impl tcl_platform::Env for SyntheticEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set(&self, _key: &str, _value: &str) {}

    fn unset(&self, _key: &str) {}

    fn vars(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn cwd(&self) -> Result<String, tcl_platform::HostError> {
        Ok("/synthetic".to_string())
    }

    fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
        Ok(())
    }
}

impl tcl_platform::Host for SyntheticHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        tcl_platform::Capabilities::empty()
    }

    fn clock(&self) -> &dyn tcl_platform::Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn tcl_platform::Env {
        &self.env
    }
}

/// The `exec` command, end-to-end through the bytecode pipeline, on both host
/// postures — the capability model proven at the command level (the helper-level
/// proof lives in `capability.rs`).
#[test]
fn exec_command_capability() {
    // Native host (the VM's default): `exec` runs a real subprocess.
    out_eq("puts [exec echo hello]\n", "hello\n");

    // Sandboxed host — subprocess capability off, the posture every WASM/WASI
    // host has. `exec` yields the faithful "unsupported" error, not a panic.
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.set_host(Rc::new(tcl_vm::host_native::NativeHost::sandboxed()));
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir("exec echo hello", &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry).expect("original source namespace");
    let c = vm.run_module(&asm);
    assert!(!c.code.is_ok(), "sandboxed exec should error, got ok");
    assert!(
        c.result.to_str().contains("no subprocess support"),
        "expected the faithful unsupported error, got: {}",
        c.result.to_str()
    );
}

#[test]
fn file_path_ops() {
    out_eq("puts [file join /a b c]\n", "/a/b/c\n");
    out_eq("puts [file join a /b c]\n", "/b/c\n");
    out_eq("puts [file dirname /a/b/c]\n", "/a/b\n");
    out_eq("puts [file tail /a/b/c.txt]\n", "c.txt\n");
    out_eq("puts [file extension foo.tcl]\n", ".tcl\n");
    out_eq("puts [file rootname foo.tcl]\n", "foo\n");
    out_eq("puts [file exists /nonexistent/xyz]\n", "0\n");
}

#[test]
fn bootstrap_globals_present() {
    out_eq("puts $::tcl_platform(platform)\n", "unix\n");
    out_eq("puts [info exists ::env]\n", "1\n");
    out_eq("puts $::tcl_version\n", "9.0\n");
}

#[test]
fn host_rebind_replaces_bytecode_vm_bootstrap_globals() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.set_host(Rc::new(SyntheticHost::new(&[
        ("USER", "first-user"),
        ("TCL_LIBRARY", "/first/lib"),
        ("TCL_WASM_SPEC", "first-wasm"),
        ("FIRST_ONLY", "stale"),
    ])));
    let initial = vm
        .eval_source(
            "list $::tcl_platform(user) $::tcl_platform(wasm) \
             $::tcl_library $::env(FIRST_ONLY)",
        )
        .expect("compile initial bootstrap query");
    assert!(initial.code.is_ok());
    assert_eq!(
        initial.result.to_str().as_ref(),
        "first-user first-wasm /first/lib stale"
    );
    let mutated = vm
        .eval_source(
            "set ::env(EMBEDDER_STALE) old; \
             set ::auto_path /old/auto; \
             set ::tclDefaultLibrary /old/default; \
             set ::tcl_pkgPath /old/pkg",
        )
        .expect("compile stale-global setup");
    assert!(mutated.code.is_ok());

    vm.set_host(Rc::new(SyntheticHost::new(&[
        ("USER", "second-user"),
        ("TCL_LIBRARY", "/second/lib"),
        ("TCL_WASM_SPEC", "second-wasm"),
        ("SECOND_ONLY", "fresh"),
    ])));
    let rebound = vm
        .eval_source(
            "list $::tcl_platform(user) $::tcl_platform(wasm) \
             $::tcl_library $::env(SECOND_ONLY) \
             [info exists ::env(FIRST_ONLY)] \
             [info exists ::env(EMBEDDER_STALE)] \
             [info exists ::tclDefaultLibrary] \
             [info exists ::tcl_pkgPath] [llength $::auto_path]",
        )
        .expect("compile rebound bootstrap query");
    assert!(rebound.code.is_ok(), "{}", rebound.result.to_str());
    assert_eq!(
        rebound.result.to_str().as_ref(),
        "second-user second-wasm /second/lib fresh 0 0 0 0 0"
    );
}

#[test]
fn bootstrap_platform_schema_comes_from_the_shared_owner() {
    let mut keys = tcl_platform::bootstrap::entries()
        .iter()
        .map(|entry| entry.name())
        .collect::<Vec<_>>();
    keys.sort_unstable();
    out_eq(
        "puts [lsort [array names ::tcl_platform]]\n",
        &format!("{}\n", keys.join(" ")),
    );
    out_eq(
        "puts [list [info exists ::tcl_platform(machine)] \
         [info exists ::tcl_platform(user)] \
         [expr {$::tcl_platform(osVersion) eq {}}]]\n",
        "1 1 1\n",
    );
}

#[test]
fn source_command() {
    let mut path = std::env::temp_dir();
    path.push(format!("tclvm_source_{}.tcl", std::process::id()));
    std::fs::write(
        &path,
        "proc greet {n} { return \"hi $n\" }\nset ::loaded 1\n",
    )
    .expect("write temp");
    let src = format!(
        "source {}\nputs [greet bob]\nputs $::loaded\n",
        path.display()
    );
    out_eq(&src, "hi bob\n1\n");
    std::fs::remove_file(&path).ok();
}

#[test]
fn clock_shared_core() {
    // `clock` is net-new (neither runtime had it); shared over tcl-cmd-core::clock.
    // Pinned vs tclsh 9.0 (the civil math is deterministic; UTC via -gmt).
    out_eq(
        "puts [clock format 1700000000 -gmt 1]\n",
        "Tue Nov 14 22:13:20 GMT 2023\n",
    );
    out_eq(
        "puts [clock format 1700000000 -format {%Y-%m-%d %H:%M:%S} -gmt 1]\n",
        "2023-11-14 22:13:20\n",
    );
    out_eq(
        "puts [clock format 1700000000 -format {%a %A %b %B %p %j %u %z} -gmt 1]\n",
        "Tue Tuesday Nov November PM 318 2 +0000\n",
    );
    // Unknown specifier (%F) passes through verbatim, as in tclsh.
    out_eq(
        "puts [clock format 0 -format {%F %T} -gmt 1]\n",
        "%F 00:00:00\n",
    );
    // `clock add` — fixed + calendar units.
    out_eq(
        "puts [clock format [clock add 1700000000 2 days -gmt 1] -format {%Y-%m-%d} -gmt 1]\n",
        "2023-11-16\n",
    );
    out_eq(
        "puts [clock format [clock add 1700000000 3 months -gmt 1] -format {%Y-%m-%d} -gmt 1]\n",
        "2024-02-14\n",
    );
    // Timing subcommands return sane monotone-ish values.
    out_eq("puts [expr {[clock seconds] > 1700000000}]\n", "1\n");
    out_eq(
        "puts [expr {[clock milliseconds] > 1700000000000}]\n",
        "1\n",
    );
    // `clock scan -format` round-trips `clock format` (the inverse).
    out_eq(
        "puts [clock scan {2023-11-14 22:13:20} -format {%Y-%m-%d %H:%M:%S} -gmt 1]\n",
        "1700000000\n",
    );
    out_eq(
        "puts [clock scan {Nov 14 2023} -format {%b %d %Y} -gmt 1]\n",
        "1699920000\n",
    );
    let (ok, msg, _) = run("clock scan {2023-13-01} -format {%Y-%m-%d} -gmt 1");
    assert!(!ok);
    assert_eq!(msg, "unable to convert input string: invalid month");
    // Free-form scan (no -format) is the remaining piece.
    let (ok, msg, _) = run("clock scan tomorrow");
    assert!(!ok);
    assert_eq!(msg, "free-form clock scan is not yet supported");
}

#[test]
fn channel_io() {
    out_eq(
        "set f [open /tmp/zz_tcltest_chan_test.txt w]\nputs $f \"line one\"\nputs $f \"line two\"\nclose $f\n\
         set r [open /tmp/zz_tcltest_chan_test.txt r]\nset a [gets $r]\nset b [gets $r]\nclose $r\n\
         file delete /tmp/zz_tcltest_chan_test.txt\nputs \"$a|$b\"\n",
        "line one|line two\n",
    );
}
