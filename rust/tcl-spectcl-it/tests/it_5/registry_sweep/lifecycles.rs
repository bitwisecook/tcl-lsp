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

//! Lifecycle ordering is valid on every command, subcommand, option and value.

use super::*;

/// Every declared lifecycle in every registry — commands, subcommands,
/// second-level subcommands, options, option constraints, forms, side
/// effects, enumerable argument values, versioned argument values, iRules
/// events, and BIG-IP profile types — must order its releases legally
/// and a child's window must lie inside its parent's.
///
/// Bad ordering (`deprecated < introduced`, `retired < deprecated`, …) is
/// unrepresentable downstream: `state_at` would report a state the data
/// cannot actually reach, so it is rejected here rather than at a consumer.
/// A child that outlives its parent is the same kind of unreachable claim —
/// an option of a command retired at 3.0 cannot itself survive to 4.0 — so
/// the intersection with the parent must come back unchanged.
///
/// registry-metadata: structural invariants over registry data, not C-Tcl
/// facts.
/// Assert one lifecycle orders its releases legally.
fn check_lifecycle(what: &str, lifecycle: tcl_registry::lifecycle::Lifecycle) {
    assert_eq!(lifecycle.validate(), Ok(()), "{what}: {lifecycle:?}");
}

/// Assert `child` stays inside `parent`'s window.
///
/// A child inherits its parent's gate, so every release the child *declares
/// for itself* must survive being narrowed by the parent — anything else
/// claims availability the parent does not have. A release the child leaves
/// open is the inherited one and is not compared, and the narrowed window
/// must itself still be a legal ordering (a child introduced after its
/// parent retired is unreachable).
fn check_lifecycle_contained(
    what: &str,
    child: tcl_registry::lifecycle::Lifecycle,
    parent: tcl_registry::lifecycle::Lifecycle,
) {
    let merged = child.intersect(parent);
    for (release, own, narrowed) in [
        ("introduced", child.introduced, merged.introduced),
        ("deprecated", child.deprecated, merged.deprecated),
        ("retired", child.retired, merged.retired),
    ] {
        if own.is_some() {
            assert_eq!(
                own, narrowed,
                "{what}: declared {release} {own:?} reaches outside its parent {parent:?}"
            );
        }
    }
    assert_eq!(
        merged.validate(),
        Ok(()),
        "{what}: {child:?} narrowed by its parent {parent:?} is unreachable"
    );
}

/// Both lifecycle checks at once, for a child of `parent`.
fn check_lifecycle_under(
    what: &str,
    child: tcl_registry::lifecycle::Lifecycle,
    parent: tcl_registry::lifecycle::Lifecycle,
) {
    check_lifecycle(what, child);
    check_lifecycle_contained(what, child, parent);
}

/// Every lifecycle hung off one subcommand: its options, option constraints,
/// side effects, second-level subcommands, and both argument-value tables.
fn check_subcommand_lifecycles(path: &str, sub: &tcl_registry::SubCommand) {
    for opt in sub.options {
        check_lifecycle_under(
            &format!("{path} {}", opt.name),
            opt.lifecycle,
            sub.lifecycle,
        );
    }
    for constraint in sub.option_relations {
        let what = format!("{path} relation {}", constraint.describe());
        check_lifecycle_under(&what, constraint.lifecycle, sub.lifecycle);
    }
    for effect in sub.side_effects.lower_bound() {
        let what = format!("{path} side_effect {:?}", effect.target);
        check_lifecycle_under(&what, effect.lifecycle, sub.lifecycle);
    }
    for sub_sub in sub.sub_subcommands {
        let what = format!("{path} {}", sub_sub.name);
        check_lifecycle_under(&what, sub_sub.lifecycle, sub.lifecycle);
    }
    for (index, values) in sub.arg_values {
        for value in *values {
            let what = format!("{path} arg{index} = {}", value.value);
            check_lifecycle_under(&what, value.lifecycle, sub.lifecycle);
        }
    }
    for gate in sub.versioned_arg_values {
        let what = format!("{path} arg{} = {}", gate.index, gate.value);
        check_lifecycle_under(&what, gate.lifecycle, sub.lifecycle);
    }
}

/// Every lifecycle hung off one command, recursing into its subcommands.
fn check_command_lifecycles(root: &str, spec: &tcl_registry::CommandSpec) {
    check_lifecycle(root, spec.lifecycle);
    for opt in spec.options {
        check_lifecycle_under(
            &format!("{root} {}", opt.name),
            opt.lifecycle,
            spec.lifecycle,
        );
    }
    for constraint in spec.option_relations {
        let what = format!("{root} relation {}", constraint.describe());
        check_lifecycle_under(&what, constraint.lifecycle, spec.lifecycle);
    }
    for form in spec.forms {
        let what = format!("{root} form {}", form.synopsis);
        check_lifecycle_under(&what, form.lifecycle, spec.lifecycle);
    }
    for effect in spec.side_effects.lower_bound() {
        let what = format!("{root} side_effect {:?}", effect.target);
        check_lifecycle_under(&what, effect.lifecycle, spec.lifecycle);
    }
    for (index, values) in spec.arg_values {
        for value in *values {
            let what = format!("{root} arg{index} = {}", value.value);
            check_lifecycle_under(&what, value.lifecycle, spec.lifecycle);
        }
    }
    for gate in spec.versioned_arg_values {
        let what = format!("{root} arg{} = {}", gate.index, gate.value);
        check_lifecycle_under(&what, gate.lifecycle, spec.lifecycle);
    }
    for sub in spec.subcommands {
        let path = format!("{root} {}", sub.name);
        check_lifecycle_under(&path, sub.lifecycle, spec.lifecycle);
        check_subcommand_lifecycles(&path, sub);
    }
}

#[test]
fn sweep_lifecycle_ordering_is_valid_everywhere() {
    crate::install_surfaces();
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            check_command_lifecycles(&format!("{dname}/{name}"), spec);
        }
    }

    let events = EventRegistry::build();
    for event in events.all_event_names() {
        check_lifecycle(event, events.event_lifecycle(event).expect("known event"));
    }

    let profiles = ProfileRegistry::build();
    for name in profiles.all_profile_names() {
        check_lifecycle(
            name,
            profiles.profile_lifecycle(name).expect("known profile"),
        );
    }
}

/// The lifecycle seed cases: each of the four states is reachable
/// and independently observable on real registry data.
///
/// f5-dialect: the event/profile releases are F5 facts, not tclsh.
#[test]
fn sweep_lifecycle_seed_cases() {
    crate::install_surfaces();
    use tcl_registry::lifecycle::LifecycleState;

    let events = EventRegistry::build();

    // Classic XML parser events: introduced 9.0.3, deprecated *and* retired
    // at 10.0.0 — retirement is exclusive, so 9.4.8 still has them.
    let xml = events
        .event_lifecycle("XML_BEGIN_DOCUMENT")
        .expect("classic XML event");
    assert_eq!(xml.state_at(Some("9.0.2")), LifecycleState::NotIntroduced);
    assert_eq!(xml.state_at(Some("9.4.8")), LifecycleState::Available);
    assert_eq!(xml.state_at(Some("10.0.0")), LifecycleState::Retired);

    // XML_CONTENT_BASED_ROUTING is *not* part of the retired classic set.
    let cbr = events
        .event_lifecycle("XML_CONTENT_BASED_ROUTING")
        .expect("CBR event");
    assert_eq!(cbr.introduced, Some("10.2.0"));
    assert_eq!(cbr.retired, None);
    assert_eq!(cbr.state_at(Some("17.1.0")), LifecycleState::Available);

    // AUTH_*: deprecated but still present, so deprecated and retired are
    // independently testable states.
    for event in [
        "AUTH_ERROR",
        "AUTH_FAILURE",
        "AUTH_SUCCESS",
        "AUTH_WANTCREDENTIAL",
    ] {
        let life = events.event_lifecycle(event).expect("AUTH event");
        assert_eq!(life.introduced, Some("9.0.0"), "{event}");
        assert_eq!(life.deprecated, Some("9.4.0"), "{event}");
        assert_eq!(life.retired, None, "{event}");
        assert_eq!(life.state_at(Some("17.1.0")), LifecycleState::Deprecated);
        assert!(events.event_available_at(event, "17.1.0"), "{event}");
    }

    // QOE_PARSE_DONE: deprecated in 15.0.0, never retired.
    let qoe = events.event_lifecycle("QOE_PARSE_DONE").expect("QoE event");
    assert_eq!(qoe.introduced, Some("11.5.0"));
    assert_eq!(qoe.deprecated, Some("15.0.0"));
    assert_eq!(qoe.retired, None);
    assert_eq!(qoe.state_at(Some("14.1.0")), LifecycleState::Available);

    // AIMCP profile: introduced in BIG-IP 21.1, no deprecation or retirement.
    let profiles = ProfileRegistry::build();
    let aimcp = profiles.profile_lifecycle("AIMCP").expect("AIMCP profile");
    assert_eq!(aimcp.introduced, Some("21.1.0"));
    assert_eq!(aimcp.deprecated, None);
    assert_eq!(aimcp.retired, None);
    assert!(!profiles.profile_available_at("AIMCP", "17.1.0"));
    assert!(profiles.profile_available_at("AIMCP", "21.1.0"));

    // A Tk command and option carry the same shape on the package axis.
    let tk = registry_for_dialect("tcl9.0");
    let ttk_button = tk.get("ttk::button").expect("ttk::button");
    assert_eq!(ttk_button.lifecycle.introduced, Some("8.5"));
    assert!(!ttk_button.available_for_version(Some("8.4")));
    let entry = tk.get("entry").expect("entry");
    let placeholder = entry
        .options
        .iter()
        .find(|o| o.name == "-placeholder")
        .expect("entry -placeholder");
    assert_eq!(placeholder.lifecycle.introduced, Some("8.7"));
    assert_eq!(
        placeholder.lifecycle_state(Some("8.6")),
        LifecycleState::NotIntroduced
    );
}
