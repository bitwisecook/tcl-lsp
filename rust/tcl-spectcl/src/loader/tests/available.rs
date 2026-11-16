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

//! The `available` statement: parity with `dialects`, the scopes it lands at, package requirements, refused providers and Jim-only windows.

use super::*;

/// The 2.0 round trip: the same body spelled `dialects` and spelled
/// `available` loads to byte-equal specs.
///
/// This is the whole of §6.1's "a new word plus a translation of the
/// legacy word": if the two spellings could ever diverge, `dialects`
/// would be a second representation rather than a translation, and the
/// upgrade tool's U9 byte-identical registry snapshot could not hold.
#[test]
fn available_and_dialects_load_byte_equal_specs() {
    // (legacy dialect word, 2.0 available row) — the whole U2 table
    // this build can express on the 1.x fields.
    let pairs = [
        ("tcl8.5", "{tcl 8.5}"),
        ("tcl8.6+", "{tcl 8.6-}"),
        ("tcl8.4+", "{tcl 8.4-}"),
        ("all-tcl", "{tcl 8.4-}"),
        ("tcl8.x", "{tcl 8.4-9.0}"),
        ("f5-irules", "{f5-irules}"),
        ("tk", "{package Tk}"),
        ("{tcl8.6 tcl9.0}", "{tcl 8.6} {tcl 9.0}"),
        ("{tcl8.6+ f5-irules}", "{tcl 8.6-} {f5-irules}"),
    ];
    for (legacy, modern) in pairs {
        let old = evaluate_pack(&format!(
            "speclib probe 1.2 {{\n command demo {{\n arity 1\n dialects {legacy}\n \
                 subcommand sub {{\n arity 0\n dialects {legacy}\n }}\n \
                 option -x -dialects {legacy}\n }}\n}}"
        ));
        let new = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n command demo {{\n arity 1\n available {modern}\n \
                 subcommand sub {{\n arity 0\n available {modern}\n }}\n \
                 option -x -available {{{modern}}}\n }}\n}}"
        ));
        assert!(old.notices.is_empty(), "{legacy}: {:?}", old.notices);
        assert!(new.notices.is_empty(), "{modern}: {:?}", new.notices);
        let old_spec = old.command("demo").expect("legacy demo loads").spec;
        let new_spec = new.command("demo").expect("2.0 demo loads").spec;
        assert_eq!(old_spec.surface, new_spec.surface, "{legacy} vs {modern}");
        assert_eq!(
            old_spec.subcommands[0].surface, new_spec.subcommands[0].surface,
            "{legacy} vs {modern}"
        );
        assert_eq!(
            old_spec.options[0].surface, new_spec.options[0].surface,
            "{legacy} vs {modern}"
        );
        assert_eq!(
            format!("{old_spec:?}"),
            format!("{new_spec:?}"),
            "{legacy} vs {modern}"
        );
    }
}

/// `available` is accepted at every scope the legacy word is, and the
/// projection is identical at each.
#[test]
fn available_lands_at_every_dialects_scope() {
    let legacy = evaluate_pack(
        "speclib probe 1.2 {\n default dialects tcl8.6+\n \
             command demo {\n arity 1..\n \
             option -x -dialects tcl8.6+\n \
             form Default {demo word} -dialects tcl8.6+\n \
             side_effect FileIo -writes -dialects tcl8.6+\n \
             option_conflict {-x -y} -dialects tcl8.6+\n \
             subcommand sub {\n arity 0\n dialects tcl8.6+\n \
             sub_subcommand deep -dialects tcl8.6+\n }\n }\n}",
    );
    let modern = evaluate_pack(
        "speclib probe 2.0 {\n default available {tcl 8.6-}\n \
             command demo {\n arity 1..\n \
             option -x -available {tcl 8.6-}\n \
             form Default {demo word} -available {tcl 8.6-}\n \
             side_effect FileIo -writes -available {tcl 8.6-}\n \
             option_conflict {-x -y} -available {tcl 8.6-}\n \
             subcommand sub {\n arity 0\n available {tcl 8.6-}\n \
             sub_subcommand deep -available {tcl 8.6-}\n }\n }\n}",
    );
    assert!(legacy.notices.is_empty(), "{:?}", legacy.notices);
    assert!(modern.notices.is_empty(), "{:?}", modern.notices);
    let legacy_spec = legacy.command("demo").expect("legacy demo loads").spec;
    let modern_spec = modern.command("demo").expect("2.0 demo loads").spec;
    assert_eq!(
        format!("{legacy_spec:?}"),
        format!("{modern_spec:?}"),
        "every scope projects alike"
    );
    assert_eq!(modern_spec.surface, Some(SpecSurface::TCL86_PLUS));
}

/// `available {package NAME}` fills `required_package` at command
/// scope, and must agree with a `required_package` the spec declares.
#[test]
fn available_carries_a_package_requirement_at_command_scope() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {package Tcllib}\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    assert_eq!(
        pack.command("demo")
            .expect("demo loads")
            .spec
            .required_package,
        Some("Tcllib")
    );

    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             required_package Snit\n available {package Tcllib}\n }\n}",
    );
    assert_eq!(
        pack.command("demo")
            .expect("demo loads")
            .spec
            .required_package,
        Some("Snit"),
        "the declared requirement wins"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("disagrees with `required_package Snit`")),
        "{:?}",
        pack.notices
    );
}

/// A package-only `available` row gates, it does not narrow.
///
/// `available {package Foo}` is 2.0's spelling of 1.x's bare
/// `required_package Foo`, which never said *where* a command lives. If
/// the row counted towards the surface it would leave an empty
/// disjunction — a command available nowhere, with nothing logged — so
/// the surface stays open and the requirement alone gates it.
#[test]
fn a_package_only_available_row_leaves_the_surface_open() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n ambient_package Foo 1.0\n \
             command demo {\n arity 0\n available {package Foo}\n }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let spec = pack.command("demo").expect("demo loads").spec;
    assert_eq!(spec.surface, None, "the row states no provider surface");
    assert_eq!(spec.required_package, Some("Foo"));

    let context = tcl_registry::model::ingress::static_document_context_for("tcl9.0");
    assert!(
        context.spec_available(spec),
        "an open world admits a command gated on an installable package"
    );

    // A core row alongside it still narrows, and the package still gates.
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 0\n \
             available {tcl 8.6-} {package Foo}\n }\n}",
    );
    let spec = pack.command("demo").expect("demo loads").spec;
    assert!(spec.surface.is_some(), "the core row is the surface");
    assert_eq!(spec.required_package, Some("Foo"));
    assert!(
        !context_for("tcl8.4").spec_available(spec),
        "8.4 is below the declared floor"
    );
}

fn context_for(environment: &str) -> &'static tcl_registry::model::ResolvedContext {
    tcl_registry::model::ingress::static_document_context_for(environment)
}

/// Q3: `f5-bigip` leaves the Tcl axis, so it is never an `available`
/// provider — and an unknown provider is reported, not guessed at.
#[test]
fn available_refuses_f5_bigip_and_unknown_providers() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {f5-bigip}\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("off the Tcl axis")),
        "{:?}",
        pack.notices
    );
    assert_eq!(
        pack.command("demo").expect("demo still loads").spec.surface,
        None,
        "a dropped row narrows nothing"
    );

    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {klingon 1.0}\n }\n}",
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.message.contains("unknown `available` provider `klingon`")),
        "{:?}",
        pack.notices
    );
}

/// A Jim-only window names the Jim family directly (Q13): the retired
/// bitmask has no Jim member, so relying on it alone would gate the
/// command off entirely rather than say what the row meant.
#[test]
fn a_jim_only_window_names_the_jim_family() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n command demo {\n arity 1\n \
             available {jim 0.78-}\n }\n}",
    );
    let rows = pack
        .command("demo")
        .expect("demo loads")
        .spec
        .surface
        .expect("the row list is stated");
    assert!(
        rows.iter()
            .all(|row| row.provider == SpecProvider::Core(Family::Jim)),
        "{rows:?}"
    );
    assert!(
        surface_admits(rows, Some(&SurfaceQuery::core(Family::Jim, "0.82"))),
        "{rows:?}"
    );
    assert!(
        !surface_admits(rows, Some(&SurfaceQuery::core(Family::Jim, "0.76"))),
        "{rows:?}"
    );
    assert!(
        !surface_admits(rows, Some(&SurfaceQuery::core(Family::Tcl, "8.6"))),
        "{rows:?}"
    );
}
