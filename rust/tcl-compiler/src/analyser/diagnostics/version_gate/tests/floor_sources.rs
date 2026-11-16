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

//! Where the effective floor comes from: pack ambient declarations, package pins, configured BIG-IP floors and the F5 baseline.

use super::*;

/// Build a registry overlay declaring `package` ambient at `version`,
/// and return an analyser bound to it.
///
/// Goes straight at the registry cache rather than through `tcl-spectcl`:
/// the pack loader depends on this crate, so a test here cannot load a
/// `.tclspec`. What it *can* do is install the same thing the loader
/// installs, which is what the analyser actually reads.
fn analyser_with_ambient(
    dialect: &str,
    key: u64,
    rows: &[(&'static str, &'static str)],
) -> Analyser {
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let _registry = tcl_registry::registry_for_profile_with_overlay(profile, key, |registry| {
        for (package, version) in rows {
            registry.insert_ambient_package(package, version);
        }
    });
    Analyser::new().with_pack_overlay(key)
}

fn gate_messages(analyser: &mut Analyser, source: &str, dialect: &str) -> Vec<(String, String)> {
    analyser
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W135" | "W136" | "W139"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

/// A pack declaring a package ambient floors it without any `package
/// require` in the file — which is the whole point: a package that comes
/// *with* the dialect is never required, so nothing else could floor it.
#[test]
fn a_pack_ambient_declaration_floors_a_package_with_no_require() {
    // Tk is hosted (not ambient) on `tcl8.6`, so with no require and no
    // pack there is no floor at all and the 8.7 option is not gated.
    let src = "entry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(src, "tcl8.6", None).len(),
        0,
        "baseline: no floor, no gate"
    );

    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0001, &[("Tk", "8.6")]);
    let diags = gate_messages(&mut analyser, src, "tcl8.6");
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("a spec pack declares Tk 8.6 ambient here")),
        "the pack's declaration is the guarantor: {diags:?}"
    );
}

/// A pack's `ambient_package` row must silence **W120** too, not just
/// feed the version floor.
///
/// Registering the floor while the same package still draws "needs a
/// `package require`" is the worst of both worlds: the analyser tells the
/// author to add a require for something the pack has just declared the
/// runtime already provides, and offers an insert fix that would be
/// wrong.  Wiring the floor is not enough on its own: the three consumers
/// that ask "is this ambient?" have to honour the declaration too.
#[test]
fn a_pack_ambient_package_is_not_reported_as_a_missing_require() {
    // `entry` belongs to Tk, which is *hosted* on tcl8.6 — so with no
    // pack the missing require is real and W120 fires. That is the
    // baseline the pack has to change.
    let src = "entry .e\n";
    let bare = Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == "W120");
    assert!(bare, "baseline: Tk is hosted on tcl8.6, so W120 is right");

    let mut analyser = analyser_with_ambient("tcl8.6", 0x1642_0001, &[("Tk", "8.6")]);
    let diags = analyser.analyse(src, "tcl8.6").diagnostics;
    let w120: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_str() == "W120")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        w120.len(),
        0,
        "a pack declaring Tk ambient means there is nothing to require: {w120:?}"
    );
}

/// The three floors compose by taking the greatest, exactly as two
/// `package require` lines already do — a floor is a lower bound.
#[test]
fn the_highest_of_the_three_floors_wins() {
    // The pack claims 8.7; the profile pin claims 8.6. The higher claim
    // holds, so the 8.7-introduced option is no longer gated.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0002, &[("Tk", "8.7")]);
    assert_eq!(
        gate_messages(
            &mut analyser,
            "package require Tk\nentry .e -placeholder hi\n",
            "tcl8.6"
        )
        .len(),
        0,
        "the pack's 8.7 outranks the profile's 8.6"
    );

    // The require claims 8.7; the pack claims only 8.6. Same outcome from
    // the other direction.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0003, &[("Tk", "8.6")]);
    assert_eq!(
        gate_messages(
            &mut analyser,
            "package require Tk 8.7\nentry .e -placeholder hi\n",
            "tcl8.6"
        )
        .len(),
        0,
        "the require's 8.7 outranks the pack's 8.6"
    );
}

/// At equal versions the *message* names the source closest to the
/// author's control, because that is the one they can act on: the require
/// in this file before the pack in this workspace before the profile
/// compiled into the server.
#[test]
fn equal_floors_are_reported_in_precedence_order() {
    // Pack and profile pin both say 8.6: the pack wins the report.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0004, &[("Tk", "8.6")]);
    let diags = gate_messages(
        &mut analyser,
        "package require Tk\nentry .e -placeholder hi\n",
        "tcl8.6",
    );
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("a spec pack declares Tk 8.6 ambient here")),
        "pack outranks the profile pin at an equal version: {diags:?}"
    );

    // Require and pack both say 8.6: the require wins the report.
    let mut analyser = analyser_with_ambient("tcl8.6", 0x1627_0005, &[("Tk", "8.6")]);
    let diags = gate_messages(
        &mut analyser,
        "package require Tk 8.6\nentry .e -placeholder hi\n",
        "tcl8.6",
    );
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("`package require` guarantees only 8.6")),
        "the require outranks the pack at an equal version: {diags:?}"
    );
}

/// FN guard for the short-circuit: a session with no packs must answer
/// exactly as it did before this axis existed.
#[test]
fn no_pack_overlay_leaves_every_floor_where_it_was() {
    let src = "package require Tk\nentry .e -placeholder hi\n";
    let before = version_diags_for(src, "tcl8.6", None);
    let mut analyser = Analyser::new();
    assert_eq!(gate_messages(&mut analyser, src, "tcl8.6"), before);
    assert!(
        before
            .iter()
            .any(|(_, m)| m.contains("tcl8.6 ships Tk 8.6")),
        "still the profile pin: {before:?}"
    );
}

#[test]
pub(super) fn tracksbase_tk_pin_floors_an_unversioned_require() {
    // §7.1: `tcl8.6` ships Tk 8.6, so `package require Tk` *without a
    // version* still guarantees only 8.6 — the 8.7-introduced
    // `-placeholder` draws W136 (the old rule was silent here: an
    // unversioned require yielded no floor at all).
    let src = "package require Tk\nentry .e -placeholder hi\n";
    let diags = version_diags_for(src, "tcl8.6", None);
    assert!(
        diags
            .iter()
            .any(|(c, m)| c == "W136" && m.contains("tcl8.6 ships Tk 8.6")),
        "TracksBase floor names the runtime as the guarantor: {diags:?}"
    );
    // TN: Tk 9.0 (tracking the tcl9.0 base) carries the 8.7 additions.
    assert_eq!(
        version_diags_for(src, "tcl9.0", None).len(),
        0,
        "Tk 9.0 meets an 8.7 introduction"
    );
    // An explicit require can only RAISE the pin floor, never lower it:
    // requiring 8.7 on the 8.6 base is satisfied at 8.7.
    let raised = "package require Tk 8.7\nentry .e -placeholder hi\n";
    assert_eq!(
        version_diags_for(raised, "tcl8.6", None),
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[test]
fn keyed_bigip_floor_gates_the_f5_surface() {
    // HTTP2::header was introduced in BIG-IP 16.1.0 (the backfilled
    // datum); the iRules profile keys its surface on BigipVersion.
    let src = "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n";
    // TN at the oldest-supported default (16.1.0 meets 16.1.0)…
    assert!(
        version_diags_for(src, "f5-irules", None).is_empty(),
        "the default floor admits the 16.1.0 surface"
    );
    // …TP pinned below the introduction…
    let below = version_diags_for(src, "f5-irules", Some("15.1.0"));
    assert!(
        below.iter().any(|(c, m)| c == "W135"
            && m.contains("requires f5-irules-cmds 16.1.0")
            && m.contains("f5-irules ships f5-irules-cmds 15.1.0")),
        "a 15.1.0 pin exposes the 16.1.0 introduction: {below:?}"
    );
    // …TN pinned above.
    assert_eq!(
        version_diags_for(src, "f5-irules", Some("17.1.0")).len(),
        0,
        "a 17.1.0 pin satisfies a 16.1.0 introduction"
    );
}

#[test]
fn bigip_21_1_subcommands_follow_the_configured_floor() {
    for source in [
        "SSL::c3d cert_lifespan 5\n",
        "SSL::c3d cert_start_date override\n",
        "persist mcp persistence_name\n",
    ] {
        let below = version_diags_for(source, "f5-irules", Some("21.0.0"));
        assert!(
            below.iter().any(|(code, message)| code == "W135"
                && message.contains("requires f5-irules-cmds 21.1.0")),
            "21.0 must reject the 21.1 subcommand: {below:?}"
        );
        assert_eq!(
            version_diags_for(source, "f5-irules", Some("21.1.0")).len(),
            0,
            "21.1 must admit {source:?}"
        );
    }
}

#[test]
fn persist_table_mcp_mode_follows_the_configured_floor() {
    for operation in ["add", "lookup", "delete"] {
        let source = format!("persist {operation} mcp key\n");
        let below = version_diags_for(&source, "f5-irules", Some("21.0.0"));
        assert!(
            below.iter().any(|(code, message)| code == "W135"
                && message.contains("Argument value 'mcp'")
                && message.contains("requires f5-irules-cmds 21.1.0")),
            "21.0 must reject persist {operation} mcp: {below:?}"
        );
        assert_eq!(
            version_diags_for(&source, "f5-irules", Some("21.1.0")).len(),
            0,
            "21.1 must admit persist {operation} mcp"
        );
    }
}

#[test]
fn baseline_floor_declares_the_f5_surface_15_0_plus() {
    // F5 specs with no explicit introduction inherit the declared
    // 15.0 baseline. TN at the 16.1 default and any 15.0+ pin…
    let src = "when HTTP_REQUEST {\n  pool p\n  HTTP::uri\n}\n";
    for pin in [None, Some("15.0.0"), Some("17.1.0")] {
        let mut a = Analyser::new();
        a.library_versions.bigip_version = pin.map(str::to_owned);
        let w135: Vec<String> = a
            .analyse(src, "f5-irules")
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str() == "W135")
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(w135.len(), 0, "pin {pin:?}: baseline is met, got {w135:?}");
    }
    // …TP below the baseline: the whole modelled surface is declared
    // 15.0+, so a 14.x target flags it.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("14.1.0".to_owned());
    let diags = a.analyse(src, "f5-irules").diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "W135" && d.message.contains("15.0.0")),
        "a pre-baseline pin flags the declared floor: {:?}",
        diags
            .iter()
            .filter(|d| d.code.as_str() == "W135")
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
    // Explicit data still wins over the baseline: HTTP2::header is
    // 16.1.0-introduced, so a 15.1 pin flags it while `pool` is fine.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("15.1.0".to_owned());
    let diags = a
        .analyse(
            "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
            "f5-irules",
        )
        .diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "W135" && d.message.contains("16.1.0")),
        "explicit introduction data outranks the baseline"
    );
}

#[test]
fn event_version_clause_follows_the_declared_range() {
    // Known event at the default target: clean (baseline 15.0 ≤ 16.1).
    let mut a = Analyser::new();
    let diags = a
        .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
        .diagnostics;
    assert!(
        !diags
            .iter()
            .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("target release")),
        "events at the default target stay clean"
    );
    // Pinned below the baseline: the event's declared range excludes it.
    let mut a = Analyser::new();
    a.library_versions.bigip_version = Some("14.1.0".to_owned());
    let diags = a
        .analyse("when HTTP_REQUEST {\n}\n", "f5-irules")
        .diagnostics;
    assert!(
        diags
            .iter()
            .any(|d| d.code.as_str() == "IRULE1002" && d.message.contains("15.0.0")),
        "a pre-baseline target flags the event's declared range: {:?}",
        diags.iter().map(|d| d.code.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn ambient_f5_surface_never_draws_missing_require() {
    // HTTP2::header carries `required_package: f5-irules-cmds`, but the
    // profile ships that surface ambiently (§7.1) — no W120, and the
    // command stays resolved (no W123/W002 either).
    let mut a = Analyser::new();
    let result = a.analyse(
        "when HTTP_REQUEST {\n  HTTP2::header :path\n}\n",
        "f5-irules",
    );
    let noisy: Vec<&str> = result
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .filter(|c| matches!(*c, "W120" | "W123" | "W002"))
        .collect();
    assert_eq!(
        noisy.len(),
        0,
        "ambient vendor surface must not draw require/unknown codes: {noisy:?}"
    );
}
