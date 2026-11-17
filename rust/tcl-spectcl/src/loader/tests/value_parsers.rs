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

//! Word and value parsers: dialect shorthands, Jim release words and object classes.

use super::*;
use tcl_registry::abbrev::PrefixMatching;

#[test]
fn dialect_shorthands_close_over_the_right_versions() {
    let mut log = Log::default();
    assert_eq!(
        parse_dialects("tcl8.6+", 1, &mut log),
        Some(SpecSurface::TCL86_PLUS)
    );
    assert_eq!(
        parse_dialects("all-tcl f5-irules", 1, &mut log),
        Some(SpecSurface::ALL_TCL_AND_IRULES)
    );
    assert_eq!(
        parse_dialects("tcl8.x", 1, &mut log),
        Some(SpecSurface::TCL8X)
    );
    assert!(log.notices.is_empty());
}

/// Jim's rows: a bare `jim` is every Jim release, `jimX.Y+` the row
/// open-ended at that release, as `regsub -command` declares it beside
/// Tcl 9.0 and later.
#[test]
fn jim_dialect_words_name_every_release_or_one_and_later() {
    use tcl_dialect::model::Family;
    const FROM_0_84: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.84", None)])];
    let mut log = Log::default();
    assert_eq!(parse_dialects("jim", 1, &mut log), Some(SpecSurface::JIM));
    assert_eq!(parse_dialects("jim0.84+", 1, &mut log), Some(FROM_0_84));
    let regsub_command = parse_dialects("tcl9.0+ jim0.84+", 1, &mut log).expect("two rows");
    assert_eq!(
        regsub_command,
        [SpecSurface::TCL90_PLUS, FROM_0_84].concat().as_slice()
    );
    assert!(log.notices.is_empty(), "{:?}", log.notices);
    for unknown in ["jim+", "jim0.+", "jimx+", "jim0.84"] {
        let mut log = Log::default();
        assert_eq!(parse_dialects(unknown, 1, &mut log), None, "{unknown}");
        assert_eq!(log.notices.len(), 1, "{unknown}");
    }
}

/// The frozen spelling, whole: `object_class NAME ?-superclass {…}?
/// ?-allow-unknown? ?-method-prefix-matching Enabled|Strict?
/// { method … }`, with `method` rows reusing the
/// `subcommand` body grammar.
#[test]
fn an_object_class_carries_its_name_flags_and_method_table() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n\
             command factory {\n\
               arity 1..\n\
               object_class ::probe::Widget -superclass {::probe::Base ::probe::Mixin} \
                 -allow-unknown -method-prefix-matching Enabled {\n\
                 method configure {\n\
                   arity 1..\n\
                   detail   {Reconfigure the widget.}\n\
                   synopsis {$w configure ?-opt value ...?}\n\
                   mutator\n\
                   option -text -takes value\n\
                 }\n\
                 method cget { arity 1 ; detail {Read one option.} ; destructive }\n\
               }\n\
             }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("factory").expect("factory loads").spec;
    let class = spec.object_class.expect("the object_class is read");
    assert_eq!(class.class_name, "::probe::Widget");
    assert_eq!(class.superclasses, &["::probe::Base", "::probe::Mixin"]);
    assert!(class.allow_unknown_methods);
    assert_eq!(class.method_prefix_matching, PrefixMatching::Enabled);

    let names: Vec<&str> = class.instance_methods.iter().map(|m| m.name).collect();
    assert_eq!(names, ["configure", "cget"]);
    let configure = class.instance_method("configure").expect("by name");
    assert_eq!(configure.detail, "Reconfigure the widget.");
    assert_eq!(configure.synopsis, "$w configure ?-opt value ...?");
    assert!(configure.mutator);
    assert_eq!(configure.options.len(), 1);
    assert_eq!(configure.options[0].name, "-text");
    assert!(class.instance_method("cget").expect("by name").destructive);

    // The method table is the class's, not the command's: a factory
    // declaring no `subcommand` rows still has none.
    assert!(spec.subcommands.is_empty());
}

/// `object_class NAME` with neither flags nor a block is the documented
/// short form — a class known by name whose methods are not enumerated.
#[test]
fn an_object_class_may_be_a_bare_name() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command factory { object_class ::probe::Opaque }\n\
             }",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let class = pack
        .command("factory")
        .expect("factory loads")
        .spec
        .object_class
        .expect("the object_class is read");
    assert_eq!(class.class_name, "::probe::Opaque");
    assert!(class.instance_methods.is_empty());
    assert!(class.superclasses.is_empty());
    assert!(!class.allow_unknown_methods);
    assert_eq!(class.method_prefix_matching, PrefixMatching::Strict);
}

/// A nameless class, an unknown flag, and a non-`method` member row are
/// each a degradation with a notice, never a failure.
#[test]
fn a_malformed_object_class_degrades_with_a_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.0 {\n\
             command nameless { object_class }\n\
             command odd { object_class ::probe::C -mixin {::X} { subcommand s { arity 1 } } }\n\
             }",
    );
    let said = |needle: &str| {
        pack.notices
            .iter()
            .any(|notice| notice.message.contains(needle))
    };
    assert!(
        pack.command("nameless")
            .expect("still loads")
            .spec
            .object_class
            .is_none()
    );
    assert!(said("`object_class` with no class name dropped"));

    let odd = pack.command("odd").expect("still loads").spec;
    let class = odd.object_class.expect("the named class survives");
    assert_eq!(class.class_name, "::probe::C");
    assert!(class.instance_methods.is_empty());
    assert!(said("unknown flag `-mixin` on `object_class` dropped"));
    assert!(said("unknown property `subcommand` dropped"));
}

/// Issue #1712: purity is derived, so a pack's `traits {PURE}` and a
/// subcommand's `pure` row are dropped with a notice naming what carries
/// the fact now, and neither makes the command pure.
#[test]
fn retired_purity_spellings_say_where_purity_went() {
    let pack = evaluate_pack(
        "speclib probe 2.2 { command demo { arity 1 ; traits {PURE} ; \
         subcommand read { arity 0 ; pure } } }",
    );
    let messages: Vec<&str> = pack
        .notices
        .iter()
        .map(|notice| notice.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("retired trait `PURE` dropped: purity is derived")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("retired row `pure` dropped: purity is derived")),
        "{messages:?}"
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert!(
        tcl_registry::purity::PurityDomains::of_command(spec)
            .pure_releases(tcl_dialect::model::Family::Tcl)
            .is_empty()
    );
}
