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

//! Instance-method lookup through superclasses and the native root factory construction grammar.

use super::*;

#[test]
fn instance_method_walks_superclasses_breadth_first() {
    use crate::spec::{ObjectClassSpec, SubCommand};

    // Diamond hierarchy: Diamond → {Left, Right} → Base. Both Left and
    // Right (and Base) define `m`; a breadth-first walk in declaration
    // order must find Left's `m` first, whereas a `Vec::pop` LIFO walk
    // would reverse the siblings and return Right's.
    static M_LEFT: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "left",
        ..SubCommand::DEFAULT
    }];
    static M_RIGHT: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "right",
        ..SubCommand::DEFAULT
    }];
    static M_BASE: [SubCommand; 1] = [SubCommand {
        name: "m",
        detail: "base",
        ..SubCommand::DEFAULT
    }];
    static DIAMOND: ObjectClassSpec = ObjectClassSpec {
        class_name: "Diamond",
        instance_methods: &[],
        superclasses: &["Left", "Right"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static LEFT: ObjectClassSpec = ObjectClassSpec {
        class_name: "Left",
        instance_methods: &M_LEFT,
        superclasses: &["Base"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static RIGHT: ObjectClassSpec = ObjectClassSpec {
        class_name: "Right",
        instance_methods: &M_RIGHT,
        superclasses: &["Base"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static BASE: ObjectClassSpec = ObjectClassSpec {
        class_name: "Base",
        instance_methods: &M_BASE,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };

    let mut reg = CommandRegistry::build_default();
    for oc in [&DIAMOND, &LEFT, &RIGHT, &BASE] {
        reg.insert(CommandSpec {
            name: oc.class_name,
            object_class: Some(oc),
            ..CommandSpec::DEFAULT
        });
    }

    let resolved = reg
        .instance_method("Diamond", "m")
        .expect("method resolves");
    assert_eq!(
        resolved.detail, "left",
        "breadth-first, declaration-ordered walk visits Left before Right"
    );
}

#[test]
fn instance_method_at_honours_explicit_package_and_dialect_gates() {
    use crate::spec::ObjectClassSpec;

    static METHODS: [SubCommand; 1] = [SubCommand {
        name: "later",
        surface: Some(SpecSurface::TCL91),
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..SubCommand::DEFAULT
    }];
    static CLASS: ObjectClassSpec = ObjectClassSpec {
        class_name: "VersionedClass",
        instance_methods: &METHODS,
        superclasses: &[],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };

    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: CLASS.class_name,
        required_package: Some("Demo"),
        object_class: Some(&CLASS),
        ..CommandSpec::DEFAULT
    });

    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.0"),
            Some(SurfaceQuery::core(Family::Tcl, "9.1"))
        )
        .is_none(),
        "the explicit owning-package floor rejects a future method"
    );
    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.1"),
            Some(SurfaceQuery::core(Family::Tcl, "9.0"))
        )
        .is_none(),
        "the method's own dialect gate remains independent of lifecycle"
    );
    assert!(
        reg.instance_method_at(
            "VersionedClass",
            "later",
            Some("9.1"),
            Some(SurfaceQuery::core(Family::Tcl, "9.1"))
        )
        .is_some(),
        "both registry-declared availability axes admit the method"
    );
}

#[test]
fn native_root_factory_descriptor_uses_actual_axes() {
    let older_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let mut registry = CommandRegistry::build_default();
    registry.set_profile(older_profile);
    for (version, admitted) in [
        (tcl_dialect::TclVersion::V8_4, false),
        (tcl_dialect::TclVersion::V8_6, true),
        (tcl_dialect::TclVersion::V9_0, true),
        (tcl_dialect::TclVersion::V9_1, true),
    ] {
        assert_eq!(
            registry
                .native_default_construction_grammar(
                    "oo::class",
                    tcl_native_model::InvocationDialect::for_version(version),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_some(),
            admitted,
            "the editor's older profile cannot replace the actual factory release",
        );
    }
    assert!(registry.default_construction_grammar("oo::class").is_none());
    let physically_projected = CommandRegistry::build_default().project_for_profile(older_profile);
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        assert!(
            physically_projected
                .native_default_construction_grammar(
                    "oo::class",
                    tcl_native_model::InvocationDialect::for_version(version),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_none(),
            "actual axes cannot restore metadata removed by physical projection"
        );
    }
    let jim = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    assert!(
        registry
            .native_default_construction_grammar(
                "oo::class",
                jim,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )
            .is_none()
    );
}
