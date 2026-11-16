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

use super::*;

#[test]
fn build_profile_registry() {
    let reg = ProfileRegistry::build();
    assert!(reg.profile_count() > 10);
    assert!(reg.namespace_count() > 10);
}

#[test]
fn profile_lookup() {
    let reg = ProfileRegistry::build();
    let http = reg.get_profile("HTTP").unwrap();
    assert_eq!(http.layer, "application");
    assert!(http.inferred_parents().contains(&"TCP"));
}

#[test]
fn aimcp_is_gated_to_bigip_21_1() {
    let reg = ProfileRegistry::build();
    let aimcp = reg.get_profile("AIMCP").expect("AIMCP profile registered");
    assert_eq!(aimcp.layer, "application");
    assert_eq!(aimcp.inferred_parents(), vec!["HTTP"]);
    assert!(!reg.profile_available_at("AIMCP", "21.0.0"));
    assert!(reg.profile_available_at("AIMCP", "21.1.0"));
}

#[test]
fn namespace_lookup() {
    let reg = ProfileRegistry::build();
    let http_ns = reg.get_namespace("HTTP").unwrap();
    assert!(http_ns.profiles.contains(&"HTTP"));
    assert_eq!(http_ns.layer, "application");
}

#[test]
fn parse_profile_directive_comma_and_space() {
    let got = parse_profile_directive("# profiles: HTTP, clientssl serverssl\nset x 1\n");
    let mut v: Vec<&str> = got.iter().map(String::as_str).collect();
    v.sort_unstable();
    assert_eq!(v, ["CLIENTSSL", "HTTP", "SERVERSSL"]);
}

#[test]
fn parse_profile_directive_singular_and_stops_at_code() {
    // Singular `profile:` is accepted; scanning stops at the first
    // non-comment line, so a later directive is ignored.
    let got = parse_profile_directive("# profile : TCP\nset x 1\n# profiles: HTTP\n");
    let v: Vec<&str> = got.iter().map(String::as_str).collect();
    assert_eq!(v, ["TCP"]);
}

#[test]
fn parse_profile_directive_rejects_non_directive_comments() {
    assert!(parse_profile_directive("# just a comment\nwhen HTTP_REQUEST {}\n").is_empty());
}

#[test]
fn scan_file_events_finds_uppercase_events() {
    let evs = scan_file_events("when HTTP_REQUEST {\n}\nwhen CLIENT_ACCEPTED { }\n");
    let mut v: Vec<&str> = evs.iter().map(String::as_str).collect();
    v.sort_unstable();
    assert_eq!(v, ["CLIENT_ACCEPTED", "HTTP_REQUEST"]);
}

#[test]
fn scan_file_events_respects_command_identity_and_normalises_case() {
    assert!(scan_file_events("awhen HTTP_REQUEST {}").is_empty());
    assert_eq!(
        scan_file_events("::when http_request { if {1} { :::when client_data {} } }"),
        FxHashSet::from_iter(["HTTP_REQUEST".to_owned()])
    );
}

#[test]
fn compute_file_profiles_unions_directive_and_inferred() {
    let events = crate::events::EventRegistry::build();
    let profiles = ProfileRegistry::build();
    // CLIENTSSL_HANDSHAKE infers CLIENTSSL; the directive adds HTTP.  Both
    // expand transitively to include TCP (the parent of each).
    let got = compute_file_profiles(
        "# profiles: HTTP\nwhen CLIENTSSL_HANDSHAKE { }\n",
        &events,
        &profiles,
    );
    assert!(got.contains(&"HTTP".to_string()));
    assert!(got.contains(&"CLIENTSSL".to_string()));
    assert!(
        got.contains(&"TCP".to_string()),
        "transitive parent: {got:?}"
    );
    // Sorted output.
    assert!(got.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn ssl_namespace_side_selectable() {
    let reg = ProfileRegistry::build();
    let ssl = reg.get_namespace("SSL").unwrap();
    assert!(ssl.side_selectable);
}

#[test]
fn modification_specs_exist() {
    let reg = ProfileRegistry::build();
    assert_eq!(reg.modifications().len(), 4);
}

/// `stack_satisfies` checks membership rather than computing a
/// per-candidate closure. The two agree because the active expansion is
/// a fixed point over the inference edges — this holds that claim over
/// every ordered pair the shipped table can form, plus a name the
/// registry does not know.
#[test]
fn stack_satisfies_agrees_with_the_subset_formulation() {
    let reg = ProfileRegistry::build();
    let names = reg.all_profile_names();
    let subset_formulation = |required: &[&str], active: &[&str]| {
        if required.is_empty() {
            return true;
        }
        let active_expanded = reg.expand_profile_stack(active);
        required.iter().any(|candidate| {
            reg.expand_profile_stack(std::slice::from_ref(candidate))
                .is_subset(&active_expanded)
        })
    };
    let mut checked = 0_usize;
    for required in &names {
        for active in &names {
            for stack in [
                vec![*active],
                vec![*active, "TCP"],
                vec![*active, "UNKNOWN_PROFILE"],
            ] {
                assert_eq!(
                    reg.stack_satisfies(&[*required], &stack),
                    subset_formulation(&[*required], &stack),
                    "required {required} against stack {stack:?}"
                );
                checked += 1;
            }
        }
        assert_eq!(
            reg.stack_satisfies(&["UNKNOWN_PROFILE"], &[*required]),
            subset_formulation(&["UNKNOWN_PROFILE"], &[*required]),
            "unknown requirement against {required}"
        );
    }
    assert!(checked > 10_000, "expected a real sweep, checked {checked}");
}

/// Every profile relation names its own row as its subject, and every term
/// is the registry's own upper-case spelling — the two invariants the
/// closure and [`ProfileFacts`] rely on for a plain string comparison.
#[test]
fn profile_relations_are_row_owned_and_canonically_spelled() {
    let reg = ProfileRegistry::build();
    for name in reg.all_profile_names() {
        let spec = reg.get_profile(name).expect("listed profile resolves");
        for relation in spec.relations {
            assert_eq!(
                relation.subject,
                Some(ProfileTerm(spec.name)),
                "{}: a row-owned relation must name its own row",
                spec.name
            );
            for term in relation.terms {
                assert_eq!(
                    term.0.to_uppercase(),
                    term.0,
                    "{}: term {} is not canonically spelled",
                    spec.name,
                    term.0
                );
            }
        }
    }
}

#[test]
fn selected_command_namespace_profiles_keep_surface_case_and_holder_purpose() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let profiles = ProfileRegistry::build();
    let context =
        crate::model::ingress::resolve_environment("f5-irules").default_context_registry();
    let selected = context
        .context()
        .resolve_spec(context.commands(), "DIAMETER::retransmission_default")
        .expect("actual selected F5 command descriptor");
    assert_eq!(
        profiles
            .namespace_for_command_spec(selected)
            .unwrap()
            .prefix,
        "DIAMETER"
    );
    let descriptor = |name, surface| crate::CommandSpec {
        name,
        surface,
        ..crate::CommandSpec::DEFAULT
    };
    for name in ["DIAMETER::foo", "::DIAMETER::foo"] {
        let own = descriptor(name, Some(tcl_dialect::model::SpecSurface::IRULES));
        assert_eq!(
            profiles.namespace_for_command_spec(&own).unwrap().profiles,
            &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"]
        );
    }
    for name in [
        "diameter::foo",
        "DIAMETER",
        ":DIAMETER::foo",
        ":::DIAMETER::foo",
        "DIAMETER::nested::foo",
    ] {
        let own = descriptor(name, Some(tcl_dialect::model::SpecSurface::IRULES));
        assert!(
            profiles.namespace_for_command_spec(&own).is_none(),
            "{name}"
        );
    }
    for surface in [None, Some(tcl_dialect::model::SpecSurface::TCL86)] {
        let foreign = descriptor("DIAMETER::foo", surface);
        assert!(profiles.namespace_for_command_spec(&foreign).is_none());
    }
    let plain = crate::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    assert!(
        plain
            .context()
            .resolve_spec(plain.commands(), selected.name)
            .is_none()
    );
}
