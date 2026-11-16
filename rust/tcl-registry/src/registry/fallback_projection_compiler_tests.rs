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

use super::CommandRegistry;
use tcl_dialect::DialectProfile;

#[test]
fn fallback_projection_keeps_the_actual_registered_compilers_release() {
    let source = CommandRegistry::build_default();
    let fallback = source.project_for_profile(DialectProfile::plain_tcl());
    let current = DialectProfile::find("tcl9.0").expect("catalogue profile");
    let dialect = crate::InvocationDialect::of_profile(current);
    let spec = fallback
        .native_compilation_for_registration("catch", dialect)
        .expect("the original C9 catch registration remains represented");
    assert_eq!(
        spec.grammar,
        crate::native_compilation::NativeCompilationGrammar::Catch
    );
    assert_eq!(
        Some(spec),
        source.native_compilation_for_registration("catch", dialect)
    );
    assert!(std::ptr::eq(
        fallback
            .get_for_surface("catch", fallback.own_surface_query())
            .unwrap(),
        source
            .get_for_surface("catch", Some(DialectProfile::plain_tcl().surface_query()))
            .unwrap()
    ));
    assert!(
        fallback
            .native_compilation_for_registration("unregistered_compiler_probe", dialect)
            .is_none()
    );
}

#[test]
fn exact_projection_does_not_borrow_a_newer_registered_compiler_row() {
    let source = CommandRegistry::build_default();
    let old = source.project_for_profile(DialectProfile::find("tcl8.4").unwrap());
    let old_dialect = crate::InvocationDialect::of_profile(DialectProfile::find("tcl8.4").unwrap());
    let new_dialect = crate::InvocationDialect::of_profile(DialectProfile::find("tcl9.0").unwrap());
    assert!(
        old.native_compilation_for_registration("catch", old_dialect)
            .is_some()
    );
    assert!(
        old.native_compilation_for_registration("catch", new_dialect)
            .is_none()
    );
}

#[test]
fn fallback_projection_preserves_an_authored_compiler_descriptor_refusal() {
    let mut source = CommandRegistry::build_default();
    source.insert_static(Box::leak(Box::new(crate::CommandSpec {
        name: "catch",
        ..crate::CommandSpec::DEFAULT
    })));
    let fallback = source.project_for_profile(DialectProfile::plain_tcl());
    let dialect = crate::InvocationDialect::of_profile(DialectProfile::find("tcl9.0").unwrap());
    assert!(
        fallback
            .native_compilation_for_registration("catch", dialect)
            .is_none()
    );
}
