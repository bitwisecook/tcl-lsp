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

use super::SourceAnalysisOptions;
use tcl_registry::{InvocationDialect, native_compiled_variables::LogicalCompiledVariableProvider};
use tcl_syntax::naming::{NativeCompiledVariableAuthority, NativeCompiledVariableRecipe};

#[test]
fn supplied_live_entry_withdraws_missing_compiler_local_policy() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    assert!(entry.compiled_variable_protocol.is_some());
    entry.compiled_variable_protocol = None;
    let options = SourceAnalysisOptions {
        native_entry: Some(&entry),
        invocation_dialect: Some(InvocationDialect::for_version(
            tcl_dialect::TclVersion::V9_0,
        )),
        compiled_variable_provider: Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation),
        ..Default::default()
    };
    assert!(options.compiled_variable_protocol().is_none());
}

#[test]
fn explicit_local_compiler_provider_participates_in_owned_baseline_identity() {
    let mut ordinary = super::BindingBaseline::default();
    ordinary.refresh_fingerprint();
    let mut authored = ordinary.clone();
    authored.compiled_variable_provider =
        Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation);
    authored.refresh_fingerprint();
    assert_ne!(ordinary, authored);
    assert_ne!(ordinary.fingerprint, authored.fingerprint);
    let retained = authored.clone();
    assert_eq!(retained, authored);
    assert_eq!(
        retained.compiled_variable_provider,
        authored.compiled_variable_provider
    );
}

#[test]
fn authored_f5_local_compiler_requires_its_explicit_provider() {
    let f5 = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    let mut options = SourceAnalysisOptions {
        invocation_dialect: Some(f5),
        ..Default::default()
    };
    assert!(options.compiled_variable_protocol().is_none());
    options.compiled_variable_provider = Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation);
    let policy = options.compiled_variable_protocol().unwrap();
    assert_eq!(
        policy.authority(),
        NativeCompiledVariableAuthority::AuthoredSimulation
    );
    assert_eq!(
        policy.recipe(),
        NativeCompiledVariableRecipe::C(tcl_dialect::TclVersion::V8_4)
    );
    options.invocation_dialect = Some(InvocationDialect::for_version(
        tcl_dialect::TclVersion::V9_0,
    ));
    assert!(options.compiled_variable_protocol().is_none());
}
