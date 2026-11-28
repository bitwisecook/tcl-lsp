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

use tcl_bigip::apl::{
    extract_iapp_var_refs, parse_apl, validate_iapp_implementation, validate_iapp_presentation,
};
use tcl_bigip::validator::ConfigDiagnostic;
const PRESENTATION: &str = "section basic {\n  string addr\n  string port\n}\n";
fn codes(diags: &[ConfigDiagnostic]) -> Vec<&str> {
    diags.iter().map(|d| d.code.as_str()).collect()
}
#[test]
fn iapp7001_fires_for_undefined_variable() {
    crate::install();
    let model = parse_apl(PRESENTATION);
    let refs = extract_iapp_var_refs("set x $::basic__missing");
    let diags = validate_iapp_implementation(
        &refs,
        Some(&model),
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert!(codes(&diags).contains(&"IAPP7001"));
}

#[test]
fn iapp7001_quiet_for_defined_variable() {
    crate::install();
    let model = parse_apl(PRESENTATION);
    let refs = extract_iapp_var_refs("set x $::basic__addr");
    let diags = validate_iapp_implementation(
        &refs,
        Some(&model),
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert_eq!(diags, [] as [tcl_bigip::validator::ConfigDiagnostic; 0]);
}

#[test]
fn iapp7002_fires_for_unused_field() {
    crate::install();
    let model = parse_apl(PRESENTATION);
    // Implementation references only `basic.addr`, leaving `basic.port`.
    let refs = extract_iapp_var_refs("set x $::basic__addr");
    let diags = validate_iapp_presentation(
        &model,
        Some(&refs),
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert!(codes(&diags).contains(&"IAPP7002"));
}

#[test]
fn no_iapp_checks_outside_iapp_dialects() {
    crate::install();
    let model = parse_apl(PRESENTATION);
    let refs = extract_iapp_var_refs("set x $::basic__missing");
    assert_eq!(
        validate_iapp_implementation(
            &refs,
            Some(&model),
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()
        ),
        [] as [tcl_bigip::validator::ConfigDiagnostic; 0]
    );
    assert_eq!(
        validate_iapp_presentation(&model, Some(&refs), tcl_dialect::DialectProfile::irules()),
        [] as [tcl_bigip::validator::ConfigDiagnostic; 0]
    );
}
