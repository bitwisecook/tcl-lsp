// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Interpreter constructors for tests and harnesses, enabled by the
//! `test-support` feature.
//!
//! Every interpreter names its commands through the native core of an explicit
//! release ([`Vm::with_native_core`]). An interpreter without one refuses every
//! command name, so there is no public constructor that builds one. These
//! helpers keep a harness to one line: a native-core VM of the named release,
//! over the platform's standard host ([`crate::standard_host`]), with the
//! core's bootstrap variables only and no package path.
//!
//! Embedders that ship construct with [`Vm::with_native_core`] themselves and
//! name their own host and bootstrap inputs.

use std::io::Write;

use crate::Vm;

/// The engine a harness runs when it names no release: Tcl 9.0's native core.
pub const DEFAULT_ENVIRONMENT: &str = "tcl9.0";

/// The pinned profile of `environment` (a dialect name such as `"tcl8.6"`),
/// resolved through the one environment ingress.
#[must_use]
pub fn profile(environment: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::resolve_environment(environment).unit_profile()
}

/// [`native_vm_with_output`] writing `puts` output to stdout.
#[must_use]
pub fn native_vm() -> Vm {
    native_vm_with_output(Box::new(std::io::stdout()))
}

/// The harness default: the Tcl 9.0 native core names commands, and the VM
/// is pinned to the permissive fallback profile
/// ([`tcl_dialect::DialectProfile::plain_tcl`]) that a profile-less module —
/// one lowered against `CommandRegistry::build_default()`, or compiled by
/// `BytecodeCompileService::default()` — is compiled for. Native admission
/// still requires its preflight capabilities. A harness that compiles for a
/// named release builds with
/// [`native_vm_for_release`] or [`native_vm_for_profile`] instead.
#[must_use]
pub fn native_vm_with_output(out: Box<dyn Write>) -> Vm {
    let mut vm = native_vm_for_profile(profile(DEFAULT_ENVIRONMENT), out);
    vm.set_dialect_profile(tcl_dialect::DialectProfile::plain_tcl());
    vm
}

/// A native-core VM of `release` (the plain-Tcl profile of that release, as
/// [`Vm::set_runtime_version`] pins it) writing `puts` output to `out`.
#[must_use]
pub fn native_vm_for_release(release: tcl_dialect::TclVersion, out: Box<dyn Write>) -> Vm {
    native_vm_for_profile(profile(release.dialect_name()), out)
}

/// A native-core VM of `environment` (a dialect name such as `"tcl8.6"`)
/// writing `puts` output to `out`.
#[must_use]
pub fn native_vm_for(environment: &str, out: Box<dyn Write>) -> Vm {
    native_vm_for_profile(profile(environment), out)
}

/// A native-core VM of `profile` writing `puts` output to `out`.
///
/// # Panics
///
/// When `profile` names no native engine (it has no measured execution point,
/// so no interpreter of it can name a command).
#[must_use]
pub fn native_vm_for_profile(
    profile: &'static tcl_dialect::DialectProfile,
    out: Box<dyn Write>,
) -> Vm {
    Vm::with_native_core(
        out,
        crate::standard_host(),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap_or_else(|| {
        panic!(
            "profile {} has no native core to build an interpreter over",
            profile.name
        )
    })
}
