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

//! Unit boundaries scanned from the lowered IR under supplied availability.

use super::*;

/// `scan_unit_linkage` reads the registry, so it recognises a boundary
/// however it is spelled — and never fires on a mere mention.
#[test]
fn unit_linkage_is_scanned_from_the_lowered_ir_not_the_raw_text() {
    let reg = registry();
    let cases = [
        ("package provide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
        ("::package\tprovide mylib 1.0\n", Traits::PROVIDES_PACKAGE),
        ("if {1} { source other.tcl }\n", Traits::LOADS_EXTERNAL_UNIT),
        (
            "namespace eval ::a { namespace export * }\n",
            Traits::EXPORTS_COMMAND,
        ),
        (
            "# this file does not package provide anything\n",
            Traits::empty(),
        ),
        ("set msg \"package provide\"\n", Traits::empty()),
        ("namespace import ::lib::helper\n", Traits::empty()),
    ];
    for (src, want) in cases {
        let module = retained_scope_fixture(src, &reg);
        assert_eq!(scan_unit_linkage(&module, &reg, None), want, "{src:?}");
    }
}

#[test]
fn unit_linkage_keeps_supplied_availability_without_inventing_missing_boundaries() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional Registry trait inventory only; no actual loading or export claim.
    let baseline = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut descriptor = registry.get("dict").unwrap().clone();
    descriptor.traits |= Traits::LOADS_EXTERNAL_UNIT;
    registry.insert(descriptor);
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let mut module = supplied_unit_fixture("dict create key value", &context);
    let registry = context.commands();
    let input = module.source_metadata_input.clone().unwrap();
    assert!(std::sync::Arc::ptr_eq(&input.context_registry(), &context));
    assert_eq!(
        scan_unit_linkage(&module, registry, None),
        Traits::LOADS_EXTERNAL_UNIT
    );
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let original_script = module.top_level.clone();
    for withheld in [
        Some(supplied_unit_input(&input, older)),
        Some(supplied_unit_input(
            &input,
            crate::environment_ingress::context_for_profile(
                tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            ),
        )),
        None,
    ] {
        module.source_metadata_input = withheld;
        assert_eq!(
            scan_unit_linkage(&module, registry, Some(input.unit_profile())),
            Traits::empty()
        );
        assert_eq!(module.top_level, original_script);
    }
}
