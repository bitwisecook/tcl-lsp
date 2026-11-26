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

//! **The emulated release governs the grammar and the command surface, not
//! just the runtime semantics.**
//!
//! A version-pinned VM must reject what its emulated
//! release rejects. Two facts move together with the release:
//!
//! - the **lexing grammar** — `{*}` expansion is TIP 157 (8.5+), so
//!   under an 8.4 pin it is a hard `extra characters after close-brace`
//!   exactly as `tclsh8.4` reports it;
//! - the **builtin command surface** — `lassign` is 8.5+, `lpop` is
//!   9.0+, so under an older pin they resolve to `invalid command name`,
//!   while user-defined procs (the polyfill pattern) always stay callable.
//!
//! The host wiring below mirrors `tclvm`: one resolved `DialectProfile`
//! drives the compiler's `LexerConfig` + registry AND the VM's runtime pin.
//! Vectors are pinned against tclsh 8.4.20 / 8.6.16 / 9.0.4 behaviour and
//! byte-compared against a real interpreter whenever one is on `PATH`
//! (`TCLSH84` / `TCLSH86` / `TCLSH90` override the binary names).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_vm::{Code, CompileError, CompileService, ProcedureCompileTarget, ProcedureDispatch};

mod compile_service_profiles;
mod hide_expose;
mod pinned_context;
mod profile_mutation;
mod release_surface;
mod rename_provenance;
mod self_mutating_commands;

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run `src` on a VM pinned to `profile`, compiling for that profile exactly
/// as `tclvm --tcl-version` does. Returns the `puts` output; a compile
/// rejection or an uncaught runtime error becomes the error's message, so a
/// vector can pin `extra characters after close-brace` and `invalid command
/// name "lassign"` the same way it pins a value.
fn profile_output(src: &str, profile: &'static DialectProfile) -> String {
    let cap = Capture::default();
    let mut vm = vm_for_profile(profile, Box::new(cap.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    // A script the release cannot compile at all reports through `Err`.
    let completion = match vm.eval_source(src) {
        Ok(completion) => completion,
        Err(error) => {
            return error
                .message_unicode()
                .map_or_else(|failure| failure.to_string(), |message| message.to_string());
        }
    };
    let out = String::from_utf8_lossy(&cap.0.borrow()).trim().to_string();
    if !completion.code.is_ok() || out.is_empty() {
        // An uncaught error's message, or — for a profile with no `puts`
        // (the iRules sandbox bans it) — the script's final result.
        return completion.result.to_str().trim().to_string();
    }
    out
}

/// A VM pinned to `profile`. A vendor profile has no native core of its own:
/// it runs on the engine of the release it emulates.
fn vm_for_profile(profile: &'static DialectProfile, out: Box<dyn std::io::Write>) -> tcl_vm::Vm {
    if tcl_registry::InvocationDialect::of_profile(profile)
        .native_bootstrap_protocol()
        .is_some()
    {
        return tcl_vm::test_support::native_vm_for_profile(profile, out);
    }
    let mut vm = tcl_vm::test_support::native_vm_for_release(profile.vm_runtime_version, out);
    vm.set_dialect_profile(profile);
    vm
}

/// [`profile_output`] for a plain release pin (the `--tcl-version` path).
fn vm_output(src: &str, version: TclVersion) -> String {
    profile_output(
        src,
        tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
            .analyser_profile(),
    )
}

fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    let release = std::iter::once(bin_env)
        .chain(names.iter().copied())
        .find_map(oracle_release)
        .unwrap_or_else(|| panic!("no Tcl release named by {bin_env} / {names:?}"));
    shared_tclsh_output(release, src)
}

/// The release a `TCLSH86` variable or `tclsh8.6` binary name stands for.
fn oracle_release(name: &str) -> Option<tcl_dialect::TclVersion> {
    let digits: String = name.chars().filter(char::is_ascii_digit).collect();
    tcl_dialect::TclVersion::ALL
        .into_iter()
        .find(|version| version.version_string().replace('.', "") == digits)
}

/// `src` under the shared oracle locator: a missing interpreter is reported,
/// or fails the test when `TCL_LSP_REQUIRE_TCLSH` requires the release.
fn shared_tclsh_output(release: tcl_dialect::TclVersion, src: &str) -> Option<String> {
    let tclsh = tcl_test_support::witness_tclsh(release)?;
    let outcome =
        tcl_test_support::run_script(&tclsh.path, src.as_bytes()).expect("the oracle runs");
    Some(String::from_utf8_lossy(&outcome.stdout).trim().to_owned())
}
