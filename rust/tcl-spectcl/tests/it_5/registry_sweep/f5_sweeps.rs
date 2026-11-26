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

//! Sweeps 2 to 4: every BIG-IP object and property, iRules event and command legality, and taint-source colours.

use super::*;

// SWEEP 2 — BIG-IP object specs and property specs.

/// Recursively touch every accessor on a property spec and its block.
fn visit_property(p: &tcl_registry::bigip::BigipPropertySpec, kind: &str) -> usize {
    assert!(!p.name.is_empty(), "{kind}: empty property name");
    // value_type tag round-trips through as_str (exercise the vocabulary).
    let tag = p.value_type.as_str();
    assert!(!tag.is_empty(), "{kind}/{}: empty value tag", p.name);
    // is_list_valued and is_block agree with the underlying slices.
    assert_eq!(
        p.is_list_valued(),
        !p.list_operators.is_empty(),
        "{kind}/{}: is_list_valued disagrees",
        p.name
    );
    assert_eq!(
        p.is_block(),
        !p.block.is_empty(),
        "{kind}/{}: is_block disagrees",
        p.name
    );
    // An Enum-kinded property should enumerate at least one value (or carry
    // a shape_kind override); touch the enum slice regardless.
    if p.value_type == ValueKind::Enum {
        assert!(
            !p.enum_values.is_empty() || p.shape_kind.is_some(),
            "{kind}/{}: Enum kind with no enum_values and no shape_kind",
            p.name
        );
    }
    for ev in p.enum_values {
        assert!(!ev.is_empty(), "{kind}/{}: empty enum value", p.name);
    }
    // Reference kinds: touch the references slice + shape_kind.
    for r in p.references {
        assert!(!r.is_empty(), "{kind}/{}: empty reference target", p.name);
    }
    let _ = (
        p.shape_kind,
        p.default,
        p.required,
        p.repeated,
        p.allow_none,
    );
    let _ = (p.min_value, p.max_value, p.pattern, p.description);
    let _ = (p.usage_flags, p.list_operators);
    // matches_section: unconstrained matches anything; constrained matches
    // exactly its listed sections.
    if p.in_sections.is_empty() {
        assert!(
            p.matches_section(None),
            "{kind}/{}: unconstrained rejects None",
            p.name
        );
        assert!(
            p.matches_section(Some("anything")),
            "{kind}/{}: unconstrained rejects a section",
            p.name
        );
    } else {
        for &sec in p.in_sections {
            assert!(
                p.matches_section(Some(sec)),
                "{kind}/{}: declared section {sec:?} not matched",
                p.name
            );
        }
    }
    let mut count = 1;
    for sub in p.block {
        count += visit_property(sub, kind);
    }
    count
}

/// Iterate EVERY BIG-IP object spec and EVERY property spec (recursing into
/// block sub-properties), querying every kind / value / display accessor.
///
/// bigip: BIG-IP object specs are F5 tmsh schema data, not real Tcl. The
/// assertions are structural invariants over that data.
#[test]

fn sweep_every_bigip_object_and_property() {
    let reg: &BigipRegistry = default_registry();
    assert!(!reg.is_empty(), "bigip registry empty");
    assert!(
        reg.len() > 100,
        "bigip registry unexpectedly small: {}",
        reg.len()
    );

    // `specs()` and `kind_names()` must agree on cardinality / membership.
    let specs = reg.specs();
    assert_eq!(specs.len(), reg.len(), "specs() len disagrees with len()");
    let kind_names = reg.kind_names();
    assert_eq!(kind_names.len(), specs.len(), "kind_names len disagrees");
    // kind_names is sorted (the accessor sorts) and unique.
    let mut sorted = kind_names.clone();
    sorted.sort_unstable();
    assert_eq!(kind_names, sorted, "kind_names not sorted");
    let unique: BTreeSet<&str> = kind_names.iter().copied().collect();
    assert_eq!(unique.len(), kind_names.len(), "duplicate kind names");

    let mut props_seen = 0usize;
    let mut headers_seen = 0usize;

    for spec in specs {
        let kind = spec.kind();
        assert!(!kind.is_empty(), "empty kind name");
        // kind() round-trips through get().
        assert!(
            reg.get(kind).is_some(),
            "kind {kind:?} does not round-trip through get()"
        );
        // kind_spec identity fields.
        let ks = spec.kind_spec;
        assert_eq!(ks.kind, kind, "kind_spec.kind disagrees with kind()");
        let _ = (ks.table_name, ks.resolver_name, ks.module, ks.object_types);

        // Every header maps back to *some* spec via get_by_header, and resolves
        // to a kind via kind_for_header.
        for &(module, object_type) in spec.header_types {
            headers_seen += 1;
            assert!(
                reg.get_by_header(module, object_type).is_some(),
                "{kind}: header ({module:?},{object_type:?}) does not resolve"
            );
            let kfh = reg.kind_for_header(module, object_type);
            assert!(
                kfh.is_some(),
                "{kind}: kind_for_header({module:?},{object_type:?}) is None"
            );
            // The display accessor on the exact "module object_type" string must
            // include a kind (exact header match yields exactly one).
            let display = format!("{module} {object_type}");
            let by_display = reg.candidate_registry_kinds_for_display(&display);
            assert!(
                !by_display.is_empty(),
                "{kind}: display {display:?} fanned out to nothing"
            );
            // candidate_kinds_for_key / _for_section_item must not panic for any
            // declared property name in this container.
            for p in spec.properties {
                let _ = reg.candidate_kinds_for_key(p.name, None, Some(module), Some(object_type));
                let _ =
                    reg.candidate_kinds_for_section_item(p.name, Some(module), Some(object_type));
            }
        }

        for p in spec.properties {
            props_seen += visit_property(p, kind);
        }
    }

    // The reconciled spec corpus is large; floors guard against accidental
    // truncation of the generated data without pinning an exact (drifting) count.
    assert!(headers_seen > 100, "too few headers swept: {headers_seen}");
    assert!(props_seen > 1000, "too few properties swept: {props_seen}");

    // ValueKind tag vocabulary is fully exercised (round-trip every variant).
    for vk in [
        ValueKind::String,
        ValueKind::Integer,
        ValueKind::Float,
        ValueKind::Boolean,
        ValueKind::Enum,
        ValueKind::Reference,
        ValueKind::List,
        ValueKind::Block,
        ValueKind::Unknown,
        ValueKind::IpAddress,
        ValueKind::Endpoint,
        ValueKind::Object,
    ] {
        assert!(!vk.as_str().is_empty(), "ValueKind {vk:?} has empty tag");
    }
    // bigip: spot-check two stable wire tags.
    assert_eq!(ValueKind::IpAddress.as_str(), "ip-address");
    assert_eq!(ValueKind::Reference.as_str(), "reference");
}

/// A miss on every BIG-IP lookup accessor returns the empty/None case (not a
/// panic) — covers the negative branches of the query helpers.
///
/// bigip.
#[test]
fn bigip_lookup_misses_are_graceful() {
    let reg = default_registry();
    assert!(reg.get("__not_a_real_kind__").is_none());
    assert!(reg.get_by_header("nope", "nope").is_none());
    assert!(reg.kind_for_header("nope", "nope").is_none());
    assert_eq!(reg.candidate_registry_kinds_for_display("").len(), 0);
    assert_eq!(reg.candidate_kinds_for_key("k", None, None, None).len(), 0);
    assert_eq!(
        reg.candidate_kinds_for_section_item("s", Some("nope"), Some("nope"))
            .len(),
        0
    );
}

// SWEEP 3 — iRules event ⇄ command cross-product accessors over the corpus.

/// Drive `event_info`, `valid_irules_commands_for_event`,
/// `is_irules_command_legal_in_event`, and `irules_events_for_command` across
/// EVERY known event, asserting the legality listing and the O(1) check agree,
/// and that `event_info` fields are internally consistent.
///
/// f5-dialect: iRules events / profiles are F5 concepts, not tclsh.
#[test]
fn sweep_event_command_legality_consistent() {
    let (reg, ()) = (registry_for_dialect("f5-irules"), ());
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();

    let all_events = events.all_event_names();
    assert!(
        all_events.len() > 50,
        "too few events: {}",
        all_events.len()
    );

    for event in &all_events {
        // Version-aware "known": an event retired before the default target
        // release (the classic pre-10.0 XML parser set) is correctly reported
        // unknown *there*, so it is checked at a release inside its own
        // lifetime instead. Everything else uses the permissive `None`.
        let lifecycle = events.event_lifecycle(event).expect("listed event");
        let target = lifecycle.retired.and(lifecycle.introduced);
        let listed = reg.valid_irules_commands_for_event(event, &events, &profiles, target);
        // Listing is sorted + unique (the accessor sorts).
        let mut sorted = listed.clone();
        sorted.sort_unstable();
        assert_eq!(listed, sorted, "{event}: valid-command listing not sorted");

        // Cross-check: a representative slice of the listing must pass the O(1)
        // legality check. (Full cross-product is large; the invariant is
        // per-command, so a sample is sufficient and keeps the test fast.)
        for cmd in listed.iter().take(25) {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, event, &events, &profiles),
                "{cmd} listed valid in {event} but is_legal disagrees"
            );
        }

        // event_info agrees with the listing on known-ness and count.
        let info = reg.event_info(event, &events, &profiles, target);
        assert!(info.known, "{event}: listed event reports known=false");
        assert_eq!(
            info.event,
            event.to_uppercase(),
            "{event}: event_info name not upper-cased"
        );
        assert_eq!(
            info.valid_command_count(),
            listed.len(),
            "{event}: event_info count disagrees with listing"
        );
        // side label is one of the documented vocabulary.
        assert!(
            [
                "client-side",
                "server-side",
                "client-side and server-side",
                "global"
            ]
            .contains(&info.side),
            "{event}: unexpected side label {:?}",
            info.side
        );
    }

    // Inverse listing for a handful of representative commands: every event it
    // lists must independently pass the legality check.
    for cmd in ["HTTP::header", "TCP::collect", "SSL::cert", "when", "log"] {
        let legal = reg.irules_events_for_command(cmd, &events, &profiles);
        let mut sorted = legal.clone();
        sorted.sort_unstable();
        assert_eq!(legal, sorted, "{cmd}: events_for_command not sorted");
        for ev in &legal {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, ev, &events, &profiles),
                "{cmd}: events_for_command lists {ev} but is_legal disagrees"
            );
        }
    }

    // An unknown event is illegal for everything and yields an empty listing.
    assert_eq!(
        reg.valid_irules_commands_for_event("__FAKE__", &events, &profiles, None)
            .len(),
        0
    );
    let unknown = reg.event_info("__fake__", &events, &profiles, None);
    assert!(!unknown.known);
    assert_eq!(unknown.side, "unknown");
}

// SWEEP 4 — taint-source / colour metadata across the source corpus.

/// Every command the registry reports as a taint source resolves to a spec
/// whose `taint_source` colour is set; the colour includes TAINTED (a source is
/// attacker-controlled by definition). Exercises the `TAINT_SOURCE_INDEX` and the
/// `taint_source` query helper across the whole index.
///
/// registry-metadata / f5-dialect: taint colours are registry security data.
#[test]
fn sweep_taint_source_colours() {
    // Build a registry that has the iRules pack loaded so the F5 getters'
    // specs are resolvable for the cross-check.
    let reg = registry_for_dialect("f5-irules");

    let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
    let mut sources_seen = 0usize;
    for name in &names {
        if let Some(colour) = reg.taint_source(name) {
            sources_seen += 1;
            // A taint source's colour is non-empty and marks the value tainted.
            assert!(
                !colour.is_empty(),
                "{name}: taint_source returned an empty colour set"
            );
            assert!(
                colour.contains(TaintColour::TAINTED),
                "{name}: taint source colour lacks TAINTED: {colour:?}"
            );
        }
    }
    assert!(
        sources_seen > 0,
        "no taint sources found in the f5-irules registry"
    );

    // Spot-check a couple of well-known colour-carrying sources (f5-dialect):
    // HTTP::path / HTTP::uri are path-prefixed sources.
    for src in ["HTTP::path", "HTTP::uri"] {
        if let Some(c) = reg.taint_source(src) {
            assert!(
                c.contains(TaintColour::TAINTED),
                "{src}: expected TAINTED in colour {c:?}"
            );
        }
    }
}
