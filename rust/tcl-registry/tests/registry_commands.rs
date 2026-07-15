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

//! C-Tcl-observable behaviour of the command registry:
//!   - registry structure / switches / options / argument values / hover /
//!     subcommands / dialect scoping
//!   - per-dialect signature membership
//!   - the `is_irules_dialect` predicate
//!   - the dialect-name surface (see the NOTE below)
//!
//! These assert behaviour on the `tcl_registry` crate via `reg.get_for_dialect`,
//! `spec.switch_names(Some(ds))`, the `spec.options` scan, `spec.arg_values_at`,
//! `reg.command_names()`, and per-dialect `reg.get_for_dialect`.
//!
//! NOTE on dialect detection: the dialect-detection function
//! (`detect_dialect_from_source`) does NOT live in the `tcl-registry` crate —
//! in the Rust workspace it is implemented in `tcl-compiler` /
//! `tcl-lsp-core`, which are out of scope for this crate's tests. The
//! dialect-name surface that *is* in `tcl-registry` (`KNOWN_DIALECTS`,
//! `available_dialects()`, `DialectSet::parse`, `DialectSet::is_irules_dialect`)
//! is covered below instead.
//!
//! ## C-Tcl proof
//! Facts observable in real Tcl (core-command existence, which switches a
//! command accepts, the subcommand set of `string`/`dict`/`info`, the
//! `socket -server` option, the 9.0-only `const`) were verified with
//! `scripts/dev/tclsh_check.sh` against tclsh8.6 and tclsh9.0 and are recorded
//! in `// tclsh:` comments. F5/iRules commands and the `f5-irules` dialect are
//! not real Tcl — they are marked `// f5-dialect`. Hover summary text and
//! arg-role classification are registry-internal metadata, marked
//! `// registry-metadata`.

use tcl_registry::arity::Arity;
use tcl_registry::dialects::DialectSet;
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;
use tcl_registry::{
    ArgRole, CommandRegistry, KNOWN_DIALECTS, Traits, available_dialects, registry_for_dialect,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `registry_for_dialect(name)` returns a registry with that dialect loaded;
/// pair it with the parsed `DialectSet` for `get_for_dialect`.
fn reg_and_set(dialect: &str) -> (&'static CommandRegistry, DialectSet) {
    (
        registry_for_dialect(dialect),
        DialectSet::parse(dialect).unwrap_or_else(|| panic!("dialect {dialect} parses")),
    )
}

/// Whether `cmd` is present in `dialect` (the Rust equivalent of
/// `REGISTRY.get(cmd, dialect) is not None`).
fn present_in(dialect: &str, cmd: &str) -> bool {
    let (reg, ds) = reg_and_set(dialect);
    reg.get_for_dialect(cmd, ds).is_some()
}

// ===========================================================================
// test_command_registry.py :: TestRegistryStructure
// ===========================================================================

/// `test_socket_is_registered_with_switches` — socket carries `-server` and
/// `-myaddr`.
///
/// tclsh (8.6 & 9.0): `socket -server` is a real switch — `catch {socket
/// -server} m` reports `no argument given for -server option` (i.e. `-server`
/// is recognised, just missing its value).
#[test]
fn socket_is_registered_with_switches() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_dialect("socket", ds)
        .expect("socket registered");
    let switches = socket.switch_names(Some(ds));
    assert!(switches.contains(&"-server"), "switches={switches:?}");
    assert!(switches.contains(&"-myaddr"), "switches={switches:?}");
}

/// `test_irules_http_commands_are_dialect_scoped` — `HTTP::header` exists in
/// f5-irules but NOT in tcl8.6.
///
/// f5-dialect: `HTTP::header` is not in core tclsh (`info commands
/// HTTP::header` is empty on 8.6 & 9.0); its dialect-scoping is registry
/// behaviour.
#[test]
fn irules_http_commands_are_dialect_scoped() {
    assert!(present_in("f5-irules", "HTTP::header"));
    let (reg86, ds86) = reg_and_set("tcl8.6");
    assert!(
        reg86.get_for_dialect("HTTP::header", ds86).is_none(),
        "HTTP::header must not be visible in tcl8.6"
    );
}

/// `test_http_header_subcommand_values_are_registered` — `insert` and
/// `replace` are valid first-argument values of `HTTP::header`.
///
/// These keywords are registered as the command's subcommands (rather than as
/// first-argument values).
/// f5-dialect: `HTTP::header` is not real Tcl; subcommand metadata is registry data.
#[test]
fn http_header_subcommand_values_are_registered() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_dialect("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert!(spec.subcommand("insert").is_some());
    assert!(spec.subcommand("replace").is_some());
}

/// `test_socket_server_option_has_hover_snippet` — the `-server` option is
/// present and documented.
///
/// The option's hover summary ("server mode") is registry-internal
/// hover prose, recorded on `OptionSpec` as `detail`. We assert the
/// option exists, consumes a value, and carries a non-empty detail string
/// (registry-metadata).
#[test]
fn socket_server_option_is_documented() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_dialect("socket", ds)
        .expect("socket registered");
    let server = socket
        .options
        .iter()
        .find(|o| o.name == "-server")
        .expect("-server option present");
    assert!(server.takes_value(), "-server takes a callback value");
    assert!(
        !server.detail.is_empty(),
        "registry-metadata: -server has a description"
    );
}

/// `test_registry_covers_all_tcl_core_signature_commands` — every core Tcl
/// command resolves in each `tclN` dialect.
///
/// tclsh (8.6 & 9.0): each name below is in `info commands` (verified in the
/// sibling `registry.rs`). `const` is 9.0-only, so it is excluded from
/// the cross-dialect list and checked separately.
#[test]
fn registry_covers_core_commands_in_every_tcl_dialect() {
    const CORE: &[&str] = &[
        "set",
        "proc",
        "if",
        "for",
        "foreach",
        "while",
        "switch",
        "expr",
        "incr",
        "append",
        "lappend",
        "list",
        "lindex",
        "llength",
        "lrange",
        "lsort",
        "dict",
        "string",
        "info",
        "array",
        "regexp",
        "regsub",
        "catch",
        "return",
        "namespace",
        "variable",
        "upvar",
        "uplevel",
        "puts",
        "open",
        "close",
        "read",
        "gets",
    ];
    // `dict` arrived in 8.5, so skip 8.4 for the cross-dialect sweep.
    for d in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let (reg, ds) = reg_and_set(d);
        let missing: Vec<&str> = CORE
            .iter()
            .copied()
            .filter(|c| reg.get_for_dialect(c, ds).is_none())
            .collect();
        assert!(missing.is_empty(), "{d} missing core commands: {missing:?}");
    }
}

/// `test_generated_doc_snippet_present_for_non_overridden_command` — `parray`
/// has hover with a documentation source.
///
/// registry-metadata: hover source attribution is registry data (`parray` is a
/// real Tcl library proc, but the doc string is ours).
#[test]
fn parray_has_hover_with_source() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let parray = reg
        .get_for_dialect("parray", ds)
        .expect("parray registered");
    let hover = parray.hover.as_ref().expect("parray has hover");
    assert!(!hover.source.is_empty(), "parray hover has a source");
}

/// `test_generated_irules_doc_snippet_present_for_non_overridden_command` +
/// `test_curated_irules_override_wins_over_generated_data` — iRules specs
/// carry a clouddocs.f5.com hover source.
///
/// f5-dialect: `ACCESS::acl` / `HTTP::header` are not real Tcl; the doc URLs are
/// registry metadata. Rather than parsing the URL host to avoid a
/// substring-sanitisation bug, we assert the recorded source URL directly.
#[test]
fn irules_specs_carry_clouddocs_source() {
    let (reg, ds) = reg_and_set("f5-irules");
    let acl = reg
        .get_for_dialect("ACCESS::acl", ds)
        .expect("ACCESS::acl registered");
    let acl_src = acl.hover.as_ref().expect("hover").source;
    assert!(
        acl_src.starts_with("https://clouddocs.f5.com/"),
        "ACCESS::acl source = {acl_src}"
    );

    // The curated HTTP::header override pins the exact documentation URL.
    let hh = reg
        .get_for_dialect("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert_eq!(
        hh.hover.as_ref().expect("hover").source,
        "https://clouddocs.f5.com/api/irules/HTTP__header.html"
    );
}

/// `test_registry_validation_metadata_is_available` — socket's arity is
/// "at least 2, unlimited"; `ACCESS::acl` is unlimited.
///
/// tclsh: `socket` needs at minimum a host and a port (2 args) and accepts
/// more with options. f5-dialect: `ACCESS::acl` arity is registry data.
#[test]
fn validation_metadata_is_available() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_dialect("socket", ds)
        .expect("socket registered");
    assert_eq!(socket.arity.min, 2);
    assert!(socket.arity.is_unlimited());

    let (rg, irules) = reg_and_set("f5-irules");
    let acl = rg
        .get_for_dialect("ACCESS::acl", irules)
        .expect("ACCESS::acl registered");
    assert!(acl.arity.is_unlimited());
}

// ===========================================================================
// test_command_registry.py :: TestControlFlowAndStartCmdTraits
// ===========================================================================

/// `test_control_flow_includes_if_for_while` + `test_control_flow_excludes_non_control`.
///
/// registry-metadata: the `CONTROL_FLOW` trait is our classification. (`if`,
/// `for`, `while`, `foreach` and `set`, `puts` are all real tclsh commands —
/// verified in `registry.rs` — but "is control flow" is registry data.)
#[test]
fn control_flow_trait_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    let cf = reg.commands_with_trait(Traits::CONTROL_FLOW);
    for c in ["if", "for", "while", "foreach"] {
        assert!(cf.contains(&c), "{c} must be control-flow");
    }
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::CONTROL_FLOW)
    );
    assert!(
        !reg.get("puts")
            .unwrap()
            .traits
            .contains(Traits::CONTROL_FLOW)
    );
}

/// `test_needs_start_cmd_includes_expr_break_continue` +
/// `test_needs_start_cmd_excludes_non_matching`.
///
/// registry-metadata: `NEEDS_START_CMD` is our classification.
#[test]
fn needs_start_cmd_trait_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in ["expr", "break", "continue"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(spec.traits.contains(Traits::NEEDS_START_CMD), "{c}");
    }
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::NEEDS_START_CMD)
    );
    assert!(
        !reg.get("puts")
            .unwrap()
            .traits
            .contains(Traits::NEEDS_START_CMD)
    );
}

// ===========================================================================
// Commands-for-event / command legality (iRules event ⇄ command cross-product)
// ===========================================================================

/// `test_http_request_includes_http_header` + `test_legality_is_legal_for_known_event`
/// — `HTTP::header` is valid/legal in `HTTP_REQUEST`.
///
/// f5-dialect: events and the HTTP profile model are F5 concepts, not tclsh.
#[test]
fn http_header_legal_in_http_request() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let valid = reg.valid_irules_commands_for_event("HTTP_REQUEST", &events, &profiles);
    assert!(valid.contains(&"HTTP::header"));
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP::header",
        "HTTP_REQUEST",
        &events,
        &profiles
    ));
}

/// `test_out_of_event_for_rule_init` + `test_legality_not_legal_in_rule_init`
/// — `HTTP::header` requires an HTTP profile, which `RULE_INIT` lacks.
///
/// f5-dialect.
#[test]
fn http_header_not_legal_in_rule_init() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    assert!(!reg.is_irules_command_legal_in_event("HTTP::header", "RULE_INIT", &events, &profiles));
    let valid = reg.valid_irules_commands_for_event("RULE_INIT", &events, &profiles);
    assert!(!valid.contains(&"HTTP::header"));
}

/// `test_unknown_event_demotes_event_requires_commands` /
/// `test_unknown_event_keeps_neutral_commands` — an unknown event is illegal
/// for every command and yields an empty valid set.
///
/// f5-dialect.
#[test]
fn unknown_event_is_illegal_for_all() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    assert!(!reg.is_irules_command_legal_in_event(
        "HTTP::header",
        "TOTALLY_FAKE_EVENT",
        &events,
        &profiles
    ));
    assert!(
        reg.valid_irules_commands_for_event("TOTALLY_FAKE_EVENT", &events, &profiles)
            .is_empty()
    );
}

/// `test_http2_commands_are_legal_in_http_events` — HTTP2 family is legal in
/// `HTTP_REQUEST` and in the message-routing events.
///
/// f5-dialect.
#[test]
fn http2_commands_legal_in_http_and_mr_events() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    for command in [
        "HTTP2::active",
        "HTTP2::concurrency",
        "HTTP2::stream",
        "HTTP2::version",
    ] {
        assert!(
            reg.is_irules_command_legal_in_event(command, "HTTP_REQUEST", &events, &profiles),
            "{command} should be legal in HTTP_REQUEST"
        );
    }
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP2::active",
        "MR_INGRESS",
        &events,
        &profiles
    ));
    assert!(reg.is_irules_command_legal_in_event("HTTP2::active", "MR_EGRESS", &events, &profiles));
}

/// The O(1) legality check agrees with the bulk valid-command listing for
/// every known event.
///
/// f5-dialect.
#[test]
fn legality_matches_valid_command_listing() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    for event in events.all_event_names() {
        let listed = reg.valid_irules_commands_for_event(event, &events, &profiles);
        // Spot-check a handful per event rather than the full cross-product
        // (the full product is large; the invariant is per-command).
        for cmd in listed.iter().take(20) {
            assert!(
                reg.is_irules_command_legal_in_event(cmd, event, &events, &profiles),
                "{cmd} listed valid in {event} but is_legal disagrees"
            );
        }
    }
}

/// `test_excluded_events_respected` — a command's `excluded_events` make it
/// illegal in those events even when the profile requirements would otherwise
/// pass. `irules_events_for_command` is the inverse listing and excludes them.
///
/// f5-dialect.
#[test]
fn excluded_events_are_respected() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_dialect("TCP::rcv_scale", ds)
        .expect("TCP::rcv_scale registered");
    assert!(
        !spec.excluded_events.is_empty(),
        "TCP::rcv_scale should declare excluded events"
    );
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let excluded = spec.excluded_events[0];
    assert!(
        !reg.is_irules_command_legal_in_event("TCP::rcv_scale", excluded, &events, &profiles),
        "TCP::rcv_scale must be illegal in its excluded event {excluded}"
    );
    let legal_events = reg.irules_events_for_command("TCP::rcv_scale", &events, &profiles);
    assert!(!legal_events.contains(&excluded));
}

// ===========================================================================
// test_command_registry.py :: TestRegistryInfoHelpers (event_info)
// ===========================================================================

/// `test_lookup_event_info_known_event` + `_dual_transport_is_serialised` —
/// `event_info` resolves a known event with a side label, ≥1 valid command,
/// and serialises a dual transport as `tcp/udp`.
///
/// f5-dialect.
#[test]
fn event_info_for_known_events() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();

    let http = reg.event_info("http_request", &events, &profiles);
    assert_eq!(http.event, "HTTP_REQUEST", "name is upper-cased");
    assert!(http.known);
    assert!(http.valid_command_count() >= 1);
    assert!(
        [
            "client-side",
            "server-side",
            "client-side and server-side",
            "global"
        ]
        .contains(&http.side),
        "side={}",
        http.side
    );

    let ca = reg.event_info("client_accepted", &events, &profiles);
    assert_eq!(ca.transport.as_deref(), Some("tcp/udp"));
}

/// `test_lookup_event_info_unknown_event` — an unknown event is `known=false`,
/// empty valid commands, side `"unknown"`.
///
/// f5-dialect.
#[test]
fn event_info_for_unknown_event() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = reg.event_info("totally_fake_event", &events, &profiles);
    assert_eq!(info.event, "TOTALLY_FAKE_EVENT");
    assert!(!info.known);
    assert!(info.valid_commands.is_empty());
    assert_eq!(info.side, "unknown");
    assert!(info.transport.is_none());
}

// ===========================================================================
// test_command_registry.py :: TestFormKindAndResolveForm
// ===========================================================================

/// `test_form_kind_enum_values` — the `FormKind` variants exist.
///
/// registry-metadata.
#[test]
fn form_kind_variants_exist() {
    use tcl_registry::hover::FormKind;
    // Distinctness of the variants is the contract under test.
    assert_ne!(FormKind::Default, FormKind::Getter);
    assert_ne!(FormKind::Getter, FormKind::Setter);
    assert_ne!(FormKind::Default, FormKind::Setter);
}

/// `test_resolve_form_http_uri_getter` / `_setter` — `HTTP::uri` with no args
/// is a pure getter; with one arg it is a mutating setter. The Rust
/// `resolve_call` picks the matching form by arity.
///
/// f5-dialect: `HTTP::uri` is not real Tcl. The getter→read / setter→write
/// classification is registry side-effect metadata.
#[test]
fn http_uri_getter_setter_forms() {
    let (reg, ds) = reg_and_set("f5-irules");
    // No-arg form resolves (getter); the spec is marked a pure taint source.
    let getter = reg.resolve_call("HTTP::uri", &[], ds).expect("getter form");
    assert!(getter.spec.traits.contains(Traits::PURE), "getter is pure");
    // Single-arg form resolves (setter) under the same command.
    let setter = reg
        .resolve_call("HTTP::uri", &["/new/path"], ds)
        .expect("setter form");
    assert_eq!(setter.spec.name, "HTTP::uri");
}

// ===========================================================================
// test_command_registry.py :: TestSubCommandResolveForm
// ===========================================================================

/// `test_http_header_lws_no_forms` — `HTTP::header lws` takes zero args.
///
/// f5-dialect: arity is registry metadata.
#[test]
fn http_header_lws_subcommand_is_nullary() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_dialect("HTTP::header", ds)
        .expect("HTTP::header registered");
    let lws = spec.subcommand("lws").expect("lws subcommand");
    assert_eq!(lws.arity, Arity::exact(0));
}

/// `test_subcommand_without_forms_returns_none` (value subcommand) +
/// `TestSubCommandResolveForm` shape — `HTTP::header value` takes exactly one
/// arg (the header name).
///
/// f5-dialect.
#[test]
fn http_header_value_subcommand_takes_one_arg() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_dialect("HTTP::header", ds)
        .expect("HTTP::header registered");
    let value = spec.subcommand("value").expect("value subcommand");
    assert_eq!(value.arity, Arity::exact(1));
}

// ===========================================================================
// test_command_registry.py :: TestClosedValueArgs
// ===========================================================================

/// `test_http_version_value_arg_is_closed` — `HTTP::version` arg 0 is a closed
/// value set of exactly the HTTP/1.x version strings.
///
/// f5-dialect: `HTTP::version` is not real Tcl. The allowed-version set is
/// registry metadata. (HTTP itself only ever speaks 0.9/1.0/1.1 over the
/// classic text protocol, which is what the closed set encodes.)
#[test]
fn http_version_arg0_is_closed_value_set() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_dialect("HTTP::version", ds)
        .expect("HTTP::version registered");
    assert!(
        spec.closed_value_args.contains(&0),
        "arg 0 should be a closed value set"
    );
    let allowed: std::collections::BTreeSet<&str> =
        spec.arg_values_at(0).iter().map(|v| v.value).collect();
    let expected: std::collections::BTreeSet<&str> = ["0.9", "1.0", "1.1"].into_iter().collect();
    assert_eq!(allowed, expected);
}

/// `test_unmarked_command_has_no_closed_args` — `set` declares no closed-value
/// argument indices.
///
/// tclsh: `set` accepts any value; the absence of a closed set is correct.
#[test]
fn set_has_no_closed_value_args() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg.get_for_dialect("set", ds).expect("set registered");
    assert!(spec.closed_value_args.is_empty());
}

// ===========================================================================
// `when` argument values cover the event corpus
// ===========================================================================

/// Event-name completion is driven by `Traits::IS_EVENT_HANDLER` + the
/// `EventRegistry` rather than baked into `when`'s `arg_values`. Assert that
/// linkage: `when` is an event handler and the registry knows the canonical
/// events.
///
/// f5-dialect: events and `when` are F5 concepts, not tclsh.
#[test]
fn when_is_event_handler_and_events_are_known() {
    let (reg, ds) = reg_and_set("f5-irules");
    let when = reg.get_for_dialect("when", ds).expect("when registered");
    assert!(
        when.traits.contains(Traits::IS_EVENT_HANDLER),
        "when drives event-name completion"
    );
    let events = EventRegistry::build();
    for ev in ["HTTP_REQUEST", "CLIENT_ACCEPTED", "SERVER_CONNECTED"] {
        assert!(events.is_known(ev), "{ev} should be a known event");
    }
}

// ===========================================================================
// CFG-rewrite alias resolution (`cfg_rewrite_name`)
// ===========================================================================

/// `dict for` / `dict map` declare their CFG-rewrite qualified names on the
/// subcommand.
///
/// registry-metadata: the `::tcl::dict::for` rewrite name is an internal
/// lowering detail. (`dict for` and `dict map` are real tclsh subcommands —
/// verified in the dict subcommand set below.)
#[test]
fn dict_iteration_subcommands_declare_cfg_rewrite_names() {
    let (reg, _) = reg_and_set("tcl8.6");
    let dict = reg.get("dict").expect("dict registered");
    assert_eq!(
        dict.subcommand("for").and_then(|s| s.cfg_rewrite_name),
        Some("::tcl::dict::for")
    );
    assert_eq!(
        dict.subcommand("map").and_then(|s| s.cfg_rewrite_name),
        Some("::tcl::dict::map")
    );
}

// ===========================================================================
// test_command_registry.py :: TestBodyKind / namespace export
// ===========================================================================

/// `test_proc_body_is_structural` — `proc`'s body argument is structural.
///
/// registry-metadata: `BodyKind` classification. (`proc name args body` is real
/// Tcl, but "the body runs in its own frame" is our SSA-facing annotation.)
#[test]
fn proc_body_is_structural() {
    use tcl_registry::body_kind::BodyKind;
    let (reg, _) = reg_and_set("tcl8.6");
    let proc = reg.get("proc").expect("proc registered");
    assert_eq!(proc.body_kind, BodyKind::Structural);
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body));
}

/// `test_exported_short_name_resolves_via_registry_property` — `tcltest::test`
/// declares it is namespace-exported (so the bare `test` can resolve to it).
///
/// registry-metadata.
#[test]
fn tcltest_test_is_namespace_exported() {
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("tcltest::test").expect("tcltest::test registered");
    assert!(spec.is_namespace_exported);
}

/// `test_when_body_is_structural` — iRules `when` body runs in the dispatcher's
/// frame, not the caller's.
///
/// f5-dialect + registry-metadata.
#[test]
fn when_body_is_structural() {
    use tcl_registry::body_kind::BodyKind;
    let (reg, ds) = reg_and_set("f5-irules");
    let when = reg.get_for_dialect("when", ds).expect("when registered");
    assert_eq!(when.body_kind, BodyKind::Structural);
}

// ===========================================================================
// test_command_registry.py :: TestIrulesNestingScriptBody (side switches)
// ===========================================================================

/// `test_peer_script_is_inline_body` / `..._is_side_switch` — `clientside`,
/// `serverside`, `peer` are side-switches.
///
/// f5-dialect: side switches are an iRules concept.
#[test]
fn irules_side_switches_are_flagged() {
    let (reg, _) = reg_and_set("f5-irules");
    for name in ["clientside", "serverside", "peer"] {
        assert!(reg.is_side_switch(name), "{name} should be a side switch");
    }
    // A plain command is not a side switch.
    assert!(!reg.is_side_switch("set"));
}

/// `test_clientside_serverside_arity_is_zero_or_one` /
/// `peer` requires its script — arity bounds.
///
/// f5-dialect: arity is registry metadata.
#[test]
fn side_switch_arities() {
    let (reg, ds) = reg_and_set("f5-irules");
    for name in ["clientside", "serverside"] {
        let spec = reg
            .get_for_dialect(name, ds)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(spec.arity.min, 0, "{name} min");
        assert_eq!(spec.arity.max, 1, "{name} max");
    }
    let peer = reg.get_for_dialect("peer", ds).expect("peer registered");
    assert_eq!(peer.arity.min, 1);
    assert_eq!(peer.arity.max, 1);
}

// ===========================================================================
// test_command_registry.py :: dialect-differentiated side effects
// ===========================================================================

/// `test_close_hints_differ_by_dialect` — `close` is a file-I/O op in plain
/// Tcl and a connection-control op in iRules (two specs under one name).
///
/// tclsh: `close` closes a channel. f5-dialect: the iRules `close` overlay
/// (connection control) is F5-specific. The dialect-specific *classification*
/// is registry metadata.
#[test]
fn close_side_effects_differ_by_dialect() {
    use tcl_registry::side_effects::SideEffectTarget;
    let (tcl_reg, tcl_ds) = reg_and_set("tcl8.6");
    let tcl_close = tcl_reg
        .get_for_dialect("close", tcl_ds)
        .expect("close in tcl");
    assert!(
        tcl_close
            .side_effects
            .iter()
            .any(|e| e.target == SideEffectTarget::FileIo),
        "tcl close is file I/O"
    );

    let (ir_reg, ir_ds) = reg_and_set("f5-irules");
    let ir_close = ir_reg
        .get_for_dialect("close", ir_ds)
        .expect("close in irules");
    assert!(
        ir_close
            .side_effects
            .iter()
            .any(|e| e.target == SideEffectTarget::ConnectionControl),
        "irules close is connection control"
    );
}

// ===========================================================================
// test_command_registry.py :: TestLazyDialectLoading
// ===========================================================================

/// `test_dialect_specs_not_in_default_registry` — a freshly built default
/// registry has core commands but not iRules / non-Tk dialect commands.
///
/// Tk is folded into the base registry by design (a `.tcl` may `package
/// require Tk`), so `button` IS present — a deliberate design choice here:
/// Tk is not gated behind a separate lazy loader.
#[test]
fn default_registry_has_core_but_not_irules() {
    let reg = CommandRegistry::build_default();
    assert!(reg.get("set").is_some(), "set is in the base registry");
    assert!(
        reg.get("HTTP::header").is_none(),
        "iRules commands are not in the default registry"
    );
    // Rust folds Tk into the base registry (documented divergence).
    assert!(
        reg.get("button").is_some(),
        "Tk is part of the base registry"
    );
}

/// Tk widget/window commands are dialect-gated to standard Tcl + the `tk`
/// dialect (`DialectSet::TK_AND_TCL`) — they must resolve in a `wish` /
/// `package require Tk` `.tcl` file but never in the restricted embedded
/// dialects (F5 iRules / iApps), where Tk does not exist.  (The *loaded*
/// gating — only offered once `package require Tk` ran — is layered on in
/// the LSP completion path, not here.)
#[test]
fn tk_commands_are_gated_to_tcl_and_tk_not_irules_or_iapps() {
    let reg = CommandRegistry::build_default();
    for name in ["button", "pack", "wm", "winfo", "ttk::treeview", "tkwait"] {
        let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
        assert_eq!(spec.required_package, Some("Tk"), "{name} requires Tk");
        // Available in standard Tcl (a `.tcl` that loads Tk) and the `tk`
        // dialect.
        assert!(
            spec.supports_dialect(DialectSet::TCL86),
            "{name} available under tcl8.6 (wish / package require Tk)"
        );
        assert!(
            spec.supports_dialect(DialectSet::TCL90),
            "{name} available under tcl9.0"
        );
        assert!(
            spec.supports_dialect(DialectSet::TK),
            "{name} available under the tk dialect"
        );
        // NOT available in the F5 embedded dialects.
        assert!(
            !spec.supports_dialect(DialectSet::IRULES),
            "{name} must NOT be offered in iRules"
        );
        assert!(
            !spec.supports_dialect(DialectSet::IAPPS),
            "{name} must NOT be offered in iApps"
        );
    }
}

/// `test_get_auto_loads_dialect` / `test_command_names_auto_loads_dialect` —
/// `load_dialect` makes iRules commands resolvable and listed.
///
/// f5-dialect.
#[test]
fn load_dialect_makes_irules_commands_visible() {
    let mut reg = CommandRegistry::build_default();
    assert!(reg.get("HTTP::header").is_none());
    reg.load_dialect(DialectSet::IRULES);
    assert!(reg.get("HTTP::header").is_some());
    let names: std::collections::HashSet<&str> = reg.command_names().collect();
    assert!(names.contains("HTTP::header"));
}

/// `test_load_dialect_specs_returns_*` / `test_load_unknown_dialect_is_noop` —
/// `load_dialect` is idempotent.
///
/// f5-dialect / registry behaviour.
#[test]
fn load_dialect_is_idempotent() {
    let mut reg = CommandRegistry::build_default();
    reg.load_dialect(DialectSet::IRULES);
    let after_first = reg.len();
    reg.load_dialect(DialectSet::IRULES); // second load is a no-op
    assert_eq!(
        reg.len(),
        after_first,
        "double-load must not duplicate specs"
    );
}

// ===========================================================================
// test_dialect_profiles.py  (SIGNATURES membership per dialect)
// ===========================================================================

/// `test_tcl84_removes_newer_commands` — `dict`/`try`/`tailcall` are absent in
/// 8.4.
///
/// tclsh: `dict` (8.5), `try`/`tailcall` (8.6) — all post-date 8.4. We can't
/// run tclsh8.4 here (not installed), but these introduction versions are
/// well-known Tcl history and match the 8.5/8.6 listings observed above.
#[test]
fn tcl84_excludes_newer_commands() {
    let (reg, ds) = reg_and_set("tcl8.4");
    assert!(reg.get_for_dialect("dict", ds).is_none(), "dict is 8.5+");
    assert!(reg.get_for_dialect("try", ds).is_none(), "try is 8.6+");
    assert!(
        reg.get_for_dialect("tailcall", ds).is_none(),
        "tailcall is 8.6+"
    );
}

/// `test_tcl85_keeps_dict_but_not_try` — `dict` present in 8.5, `try` absent.
///
/// tclsh: `dict` exists in 8.5+ (present in both 8.6 & 9.0 subcommand probes);
/// `try` is 8.6+.
#[test]
fn tcl85_has_dict_not_try() {
    let (reg, ds) = reg_and_set("tcl8.5");
    assert!(reg.get_for_dialect("dict", ds).is_some());
    assert!(reg.get_for_dialect("try", ds).is_none());
}

/// `test_tcl86_includes_tcloo_commands` + `test_tcl85_excludes_tcloo_commands`
/// — `TclOO` (`oo::class`, `oo::define`) is 8.6+.
///
/// tclsh: `oo::class` is present on 8.6 (`info commands oo::class` non-empty);
/// `TclOO` landed in 8.6.
#[test]
fn tcloo_is_tcl86_plus() {
    let (r86, d86) = reg_and_set("tcl8.6");
    assert!(r86.get_for_dialect("oo::class", d86).is_some());
    assert!(r86.get_for_dialect("oo::define", d86).is_some());

    let (r85, d85) = reg_and_set("tcl8.5");
    assert!(r85.get_for_dialect("oo::class", d85).is_none());
    assert!(r85.get_for_dialect("oo::define", d85).is_none());
}

/// `test_f5_profile_adds_when_and_http_commands` +
/// `test_f5_irules_generated_commands_are_present` +
/// `test_f5_irules_includes_deprecated_and_disabled_commands` — the iRules
/// profile pulls in `when`, the HTTP family, the seeded AAA catalog, the
/// `class`/`table` generated commands, and deprecated profile commands.
///
/// f5-dialect: none of these are real tclsh commands.
#[test]
fn f5_irules_profile_membership() {
    let (reg, ds) = reg_and_set("f5-irules");
    for name in [
        "when",
        "HTTP::header",
        "AAA::acct_result",
        "class",
        "table",
        "PROFILE::list",
        "SSL::cert",
        "TCP::collect",
        "UDP::payload",
        "HTTP::path",
    ] {
        assert!(
            reg.get_for_dialect(name, ds).is_some(),
            "{name} should be in the f5-irules profile"
        );
    }
}

/// `test_f5_irules_keeps_base_proc_signature` — `proc` keeps its core arity
/// (exactly 3) and body role even under the iRules profile.
///
/// tclsh: `proc name args body` is exactly 3 args. registry-metadata: the
/// body role.
#[test]
fn f5_irules_keeps_base_proc_signature() {
    let (reg, ds) = reg_and_set("f5-irules");
    let proc = reg.get_for_dialect("proc", ds).expect("proc registered");
    assert_eq!(proc.arity, Arity::exact(3));
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body));
}

/// `test_f5_irules_target_family_signatures_are_concrete` — curated iRules
/// commands have concrete arity and subcommands.
///
/// f5-dialect.
#[test]
fn f5_irules_curated_signatures_are_concrete() {
    let (reg, ds) = reg_and_set("f5-irules");
    let header = reg
        .get_for_dialect("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert!(header.subcommand("value").is_some());
    assert!(header.subcommand("insert").is_some());

    let when = reg.get_for_dialect("when", ds).expect("when registered");
    assert_eq!(when.arity.max, 6);
    assert_eq!(reg.get_for_dialect("pool", ds).expect("pool").arity.min, 1);
    assert_eq!(reg.get_for_dialect("node", ds).expect("node").arity.min, 1);
    assert_eq!(
        reg.get_for_dialect("HTTP::respond", ds)
            .expect("HTTP::respond")
            .arity
            .min,
        1
    );
}

/// `test_f5_iapps_profile_adds_iapp_utility_commands` — the f5-iapps profile
/// has `iapp::template` / `iapp::conf` but NOT the iRules catalog.
///
/// f5-dialect.
#[test]
fn f5_iapps_profile_membership() {
    let (reg, ds) = reg_and_set("f5-iapps");
    assert!(reg.get_for_dialect("iapp::template", ds).is_some());
    assert!(reg.get_for_dialect("iapp::conf", ds).is_some());
    // f5-iapps is a separate catalog from f5-irules.
    assert!(
        reg.get_for_dialect("AAA::acct_result", ds).is_none(),
        "iRules catalog must not leak into f5-iapps"
    );
}

/// `test_expect_profile_adds_expect_commands` + `_includes_base_tcl` +
/// `_does_not_include_irules` — the expect profile adds `spawn`/`expect`/… on
/// top of base Tcl and excludes iRules.
///
/// expect is a real Tcl extension; `spawn`/`expect`/`send`/`interact` are its
/// commands (not in bare tclsh, but part of the `expect` interpreter). The
/// dialect partitioning is registry behaviour.
#[test]
fn expect_profile_membership() {
    let (reg, ds) = reg_and_set("expect");
    for name in ["spawn", "expect", "send", "interact", "log_user"] {
        assert!(
            reg.get_for_dialect(name, ds).is_some(),
            "{name} should be in the expect profile"
        );
    }
    // Base Tcl is still present.
    for name in ["set", "proc", "if"] {
        assert!(reg.get_for_dialect(name, ds).is_some(), "{name} (base tcl)");
    }
    // iRules commands are not.
    assert!(reg.get_for_dialect("when", ds).is_none());
    assert!(reg.get_for_dialect("HTTP::header", ds).is_none());
}

/// `test_when_marks_body_argument` + `_marks_last_argument_body_in_extended_form`
/// — `when EVENT … { body }` marks the trailing body argument.
///
/// f5-dialect + registry-metadata (the BODY role).
#[test]
fn when_marks_trailing_body_argument() {
    let (reg, _) = reg_and_set("f5-irules");
    // when EVENT { body }  → body at index 1
    let simple = reg.arg_indices_for_role("when", &["HTTP_REQUEST", "{ puts ok }"], ArgRole::Body);
    assert_eq!(simple, vec![1]);
    // when EVENT priority N { body } → body at index 3
    let extended = reg.arg_indices_for_role(
        "when",
        &["CLIENT_ACCEPTED", "priority", "500", "{ puts ok }"],
        ArgRole::Body,
    );
    assert_eq!(extended, vec![3]);
}

/// `test_oo_class_create_marks_definition_body` +
/// `test_oo_define_method_marks_method_body` + `_script_form_marks_body` — the
/// `TclOO` definition bodies resolve to the BODY role.
///
/// registry-metadata: arg-role classification. (`TclOO` is real 8.6 Tcl.)
#[test]
fn tcloo_definition_bodies_marked_body() {
    let (reg, _) = reg_and_set("tcl8.6");
    // oo::class create Dog { defScript } → body at index 2
    let create = reg.arg_indices_for_role(
        "oo::class",
        &["create", "Dog", "{ method bark {} { puts ok } }"],
        ArgRole::Body,
    );
    assert!(create.contains(&2), "create body: {create:?}");

    // oo::define Dog method bark {} { body } → body at index 4
    let method = reg.arg_indices_for_role(
        "oo::define",
        &["Dog", "method", "bark", "{}", "{ puts ok }"],
        ArgRole::Body,
    );
    assert!(method.contains(&4), "method body: {method:?}");

    // oo::define Dog { defScript } → body at index 1
    let script = reg.arg_indices_for_role(
        "oo::define",
        &["Dog", "{ method bark {} { puts ok } }"],
        ArgRole::Body,
    );
    assert!(script.contains(&1), "script body: {script:?}");
}

/// `test_proc_keeps_base_signature` BODY role (proc helper) — `proc helper x {
/// body }` marks the body argument.
///
/// registry-metadata.
#[test]
fn proc_marks_body_argument() {
    let (reg, _) = reg_and_set("tcl8.6");
    let bodies = reg.arg_indices_for_role("proc", &["helper", "x", "{ return $x }"], ArgRole::Body);
    assert_eq!(bodies, vec![2]);
}

// ===========================================================================
// `is_irules_dialect` predicate
// ===========================================================================

/// The iRules predicate accepts `f5-irules` and the legacy `irules` alias,
/// rejects everything else and `None`.
///
/// registry-metadata: dialect-name predicate.
#[test]
fn is_irules_dialect_predicate() {
    assert!(DialectSet::is_irules_dialect(Some("f5-irules")));
    assert!(DialectSet::is_irules_dialect(Some("irules")));
    assert!(!DialectSet::is_irules_dialect(Some("tcl8.6")));
    assert!(!DialectSet::is_irules_dialect(Some("f5-iapps")));
    assert!(!DialectSet::is_irules_dialect(Some("f5-bigip")));
    assert!(!DialectSet::is_irules_dialect(None));
}

// ===========================================================================
// Dialect-name surface (the part that lives in this crate)
//
// `detect_dialect_from_source` itself is implemented in tcl-compiler /
// tcl-lsp-core, not tcl-registry, so the source-scanning behaviour is out of
// scope here. The dialect *vocabulary* it resolves into — and which this
// crate owns — is covered instead.
// ===========================================================================

/// The detection targets (`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `f5-irules`,
/// `expect`) that dialect detection produces as outputs are all
/// parseable dialect names in this crate.
///
/// registry-metadata: dialect vocabulary.
#[test]
fn detection_target_dialects_are_known() {
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "expect",
    ] {
        assert!(DialectSet::parse(d).is_some(), "{d} should parse");
    }
    // The detection targets are catalogued names.
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "expect",
    ] {
        assert!(KNOWN_DIALECTS.contains(&d), "{d} should be a known dialect");
    }
    // `tcl-dialect: unknown` has no parse — the analogue of returning `None`.
    assert!(DialectSet::parse("unknown").is_none());
}

/// `available_dialects()` is the sorted catalog backing the CLI `--dialect`
/// choices; `KNOWN_DIALECTS` is its data. Sanity-check completeness + order.
///
/// registry-metadata.
#[test]
fn available_dialects_is_sorted_and_complete() {
    let dialects = available_dialects();
    assert_eq!(dialects, KNOWN_DIALECTS);
    let mut sorted = dialects.to_vec();
    sorted.sort_unstable();
    assert_eq!(dialects, sorted.as_slice(), "dialects must be pre-sorted");
    assert!(dialects.contains(&"f5-irules"));
    assert!(dialects.contains(&"tcl9.0"));
}

/// `DialectSet::parse` round-trips the canonical names and rejects junk —
/// underpinning the cache key normalisation a typo'd dialect collapses to.
///
/// registry-metadata.
#[test]
fn dialect_parse_roundtrip() {
    assert_eq!(DialectSet::parse("tcl8.6"), Some(DialectSet::TCL86));
    assert_eq!(DialectSet::parse("tcl9.0"), Some(DialectSet::TCL90));
    assert_eq!(DialectSet::parse("f5-irules"), Some(DialectSet::IRULES));
    assert_eq!(DialectSet::parse("expect"), Some(DialectSet::EXPECT));
    assert_eq!(DialectSet::parse("definitely-not-a-dialect"), None);
}

// ===========================================================================
// C-Tcl ground-truth: subcommand sets of string / dict / info match tclsh.
//
// These are direct tclsh facts. The Rust registry must register at least the
// subcommands common to tclsh8.6 AND tclsh9.0 (the version-specific extras —
// 8.6's `string bytelength`, 9.0's `string insert` / `dict getdef` /
// `info cmdtype` — are registry-superset entries and are not required here).
// ===========================================================================

/// `string` subcommands.
///
/// tclsh8.6: `string` subcommands = bytelength, cat, compare, equal, first,
/// index, is, last, length, map, match, range, repeat, replace, reverse,
/// tolower, totitle, toupper, trim, trimleft, trimright, wordend, wordstart.
/// tclsh9.0: same minus `bytelength`, plus `insert`.
/// The set below is the 8.6∩9.0 intersection.
#[test]
fn string_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "cat",
        "compare",
        "equal",
        "first",
        "index",
        "is",
        "last",
        "length",
        "map",
        "match",
        "range",
        "repeat",
        "replace",
        "reverse",
        "tolower",
        "totitle",
        "toupper",
        "trim",
        "trimleft",
        "trimright",
        "wordend",
        "wordstart",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("string").expect("string registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "string missing subcommands: {missing:?}"
    );
}

/// `dict` subcommands.
///
/// tclsh8.6: dict subcommands = append, create, exists, filter, for, get,
/// incr, info, keys, lappend, map, merge, remove, replace, set, size, unset,
/// update, values, with. tclsh9.0: same plus getdef / getwithdefault. The set
/// below is the 8.6∩9.0 intersection.
#[test]
fn dict_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "append", "create", "exists", "filter", "for", "get", "incr", "info", "keys", "lappend",
        "map", "merge", "remove", "replace", "set", "size", "unset", "update", "values", "with",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("dict").expect("dict registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "dict missing subcommands: {missing:?}");
}

/// `info` subcommands.
///
/// tclsh8.6: info subcommands = args, body, class, cmdcount, commands,
/// complete, coroutine, default, errorstack, exists, frame, functions,
/// globals, hostname, level, library, loaded, locals, nameofexecutable,
/// object, patchlevel, procs, script, sharedlibextension, tclversion, vars.
/// tclsh9.0 adds cmdtype / constant / consts. The set below is the 8.6∩9.0
/// intersection (i.e. the 8.6 list).
#[test]
fn info_subcommands_match_tclsh() {
    const COMMON: &[&str] = &[
        "args",
        "body",
        "class",
        "cmdcount",
        "commands",
        "complete",
        "coroutine",
        "default",
        "errorstack",
        "exists",
        "frame",
        "functions",
        "globals",
        "hostname",
        "level",
        "library",
        "loaded",
        "locals",
        "nameofexecutable",
        "object",
        "patchlevel",
        "procs",
        "script",
        "sharedlibextension",
        "tclversion",
        "vars",
    ];
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("info").expect("info registered");
    let missing: Vec<&str> = COMMON
        .iter()
        .copied()
        .filter(|s| spec.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "info missing subcommands: {missing:?}");
}

/// C-Tcl: `regsub` switch set is dialect-versioned.
///
/// tclsh8.6: `catch {regsub -bogus …}` → "must be -all, -nocase, -expanded,
/// -line, -linestop, -lineanchor, -start, or --". tclsh9.0 adds `-command`.
/// So `-command` must be 9.0-only and `-all`/`-nocase` must be in both.
#[test]
fn regsub_switches_are_version_gated() {
    let reg = CommandRegistry::build_default();
    let regsub = reg.get("regsub").expect("regsub registered");
    let in_86 = regsub.switch_names(Some(DialectSet::TCL86));
    assert!(in_86.contains(&"-all"), "8.6: {in_86:?}");
    assert!(in_86.contains(&"-nocase"), "8.6: {in_86:?}");
    assert!(
        !in_86.contains(&"-command"),
        "9.0-only -command leaked into 8.6: {in_86:?}"
    );
    let in_90 = regsub.switch_names(Some(DialectSet::TCL90));
    assert!(in_90.contains(&"-command"), "9.0: {in_90:?}");
}

/// C-Tcl: `const` is a Tcl 9.0 builtin, present in 9.0.
///
/// tclsh8.6: `info commands const` → empty. tclsh9.0: → `const`.
///
/// NOTE: the strict "absent from tcl8.6" half is deliberately NOT asserted
/// here. The `const` spec is declared
/// `dialects: None` (universal) on purpose — see `registry.rs`
/// `tcl9_commands_gated_to_tcl90`, which documents and asserts that `const` is
/// dialect-agnostic "so it is valid inside iRules events". So
/// `get_for_dialect("const", TCL86)` intentionally returns `Some` even though
/// real tclsh8.6 lacks the command. This is an intentional registry
/// over-approximation, not a bug; only the positive 9.0 fact is a strict
/// C-Tcl assertion. (See the final report for the strict-availability caveat.)
#[test]
fn const_is_present_in_tcl90() {
    assert!(
        present_in("tcl9.0", "const"),
        "const must be present in tcl9.0"
    );
    // Intentional divergence (documented above): const is `dialects: None`, so
    // it resolves under every dialect, including tcl8.6.
    let (reg86, ds86) = reg_and_set("tcl8.6");
    assert!(
        reg86.get_for_dialect("const", ds86).is_some(),
        "const is modelled as universal (dialects: None) by design"
    );
}

// ===========================================================================
// Registry construction basics (the `build_default` checks)
// ===========================================================================

/// The cached per-dialect registries are non-empty and round-trip a lookup.
///
/// registry behaviour.
#[test]
fn cached_dialect_registries_are_populated() {
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "f5-iapps",
        "expect",
    ] {
        let reg = registry_for_dialect(d);
        assert!(!reg.is_empty(), "{d} registry is empty");
        assert!(reg.get("set").is_some(), "{d} should have `set`");
    }
    // An unparseable dialect collapses to the plain-Tcl entry (still usable).
    let junk = registry_for_dialect("not-a-real-dialect");
    assert!(junk.get("set").is_some());
    assert!(
        junk.get("HTTP::header").is_none(),
        "the fallback registry is plain Tcl"
    );
}

// ===========================================================================
// Issue #806 — report package command specs + scoped-body / object model.
// ===========================================================================

/// `report::defstyle` carries the scoped style-definition environment, with the
/// report configuration methods and their operations as registry data.
#[test]
fn report_defstyle_has_scoped_body_environment() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_dialect("report::defstyle", ds)
        .expect("report::defstyle registered in tcl8.6");
    let env = spec
        .body_scope
        .expect("report::defstyle carries a body scope");
    assert!(
        env.include_sibling_definitions,
        "styles callable in sibling bodies"
    );
    // A representative slice of the 19 report methods.
    for cmd in [
        "top",
        "topdatasep",
        "data",
        "botdata",
        "columns",
        "size",
        "pad",
        "justify",
    ] {
        assert!(env.is_command(cmd), "`{cmd}` is a scoped command");
    }
    // Separators support enable/disable; data lines do not.
    let top = env.command("top").unwrap();
    assert!(top.subcommand("enable").is_some(), "separator enables");
    let data = env.command("data").unwrap();
    assert!(data.subcommand("set").is_some(), "data line sets");
    assert!(
        data.subcommand("enable").is_none(),
        "data line does not enable"
    );
    // Config methods are plain (no ensemble ops).
    assert!(env.command("columns").unwrap().subcommands.is_empty());
}

/// `report::report` is a documented object factory with instance methods and
/// option values.
#[test]
fn report_report_object_class_is_modelled() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_dialect("report::report", ds)
        .expect("report::report registered");
    assert_eq!(
        spec.creates_instance_at,
        Some(0),
        "names its object at arg 0"
    );
    assert!(spec.hover.is_some(), "carries hover");
    assert_eq!(spec.arg_role_at(0), Some(ArgRole::Name));
    let class = spec
        .object_class
        .expect("report::report has an object class");
    for m in [
        "destroy",
        "printmatrix",
        "columns",
        "size",
        "pad",
        "justify",
        "top",
        "data",
    ] {
        assert!(class.instance_method(m).is_some(), "method `{m}` modelled");
    }
    // Option values on the padding / justification methods.
    let pad = class.instance_method("pad").unwrap();
    let pad_vals: Vec<&str> = pad.arg_values_at(1).iter().map(|v| v.value).collect();
    assert!(pad_vals.contains(&"both"), "pad where-values: {pad_vals:?}");
    let justify = class.instance_method("justify").unwrap();
    let jvals: Vec<&str> = justify.arg_values_at(1).iter().map(|v| v.value).collect();
    assert!(jvals.contains(&"center"), "justify values: {jvals:?}");
}

/// Every Tk/ttk widget constructor names the widget path it creates at
/// arg 0, so a later `.w <subcommand> …` / `$w <subcommand> …` dispatch can
/// resolve back to this same spec (issue #927:
/// `docs/design/tk-widget-instance-typing.md`).
#[test]
fn tk_widget_constructors_declare_creates_instance_at() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let widgets = [
        // Classic widgets.
        "button",
        "canvas",
        "checkbutton",
        "entry",
        "frame",
        "label",
        "labelframe",
        "listbox",
        "menu",
        "menubutton",
        "message",
        "panedwindow",
        "radiobutton",
        "scale",
        "scrollbar",
        "spinbox",
        "text",
        "toplevel",
        // Raw ttk widgets.
        "ttk::button",
        "ttk::combobox",
        "ttk::entry",
        "ttk::frame",
        "ttk::label",
        "ttk::notebook",
        "ttk::progressbar",
        "ttk::scale",
        "ttk::separator",
        "ttk::sizegrip",
        "ttk::treeview",
        // ttk_extra widgets.
        "ttk::checkbutton",
        "ttk::menubutton",
        "ttk::panedwindow",
        "ttk::radiobutton",
        "ttk::spinbox",
    ];
    for name in widgets {
        let spec = reg
            .get_for_dialect(name, ds)
            .unwrap_or_else(|| panic!("{name} registered"));
        assert_eq!(
            spec.creates_instance_at,
            Some(0),
            "{name} names its widget path at arg 0"
        );
    }
}

/// Widgets with a non-empty `subcommands` table bind a self-referential
/// `object_class` — the created widget's instance command dispatches
/// through the *same* `SubCommand` slice as the constructor's own
/// `subcommands`, so `registry.instance_method` resolves real widget
/// subcommands with no separate hand-maintained method table (and no
/// drift between the two — a drift guard, since `instance_methods` and
/// `subcommands` are asserted to be the literal same slice).
#[test]
fn tk_widgets_with_subcommands_self_reference_their_object_class() {
    let (reg, ds) = reg_and_set("tcl8.6");
    // (widget name, a subcommand it declares, an unrelated widget's
    // subcommand it must NOT accept).
    let cases = [
        ("ttk::treeview", "instate", "curselection"),
        ("ttk::notebook", "instate", "curselection"),
        ("listbox", "curselection", "instate"),
        ("text", "tag", "curselection"),
        ("canvas", "bind", "curselection"),
        ("entry", "icursor", "curselection"),
        ("menu", "add", "curselection"),
        ("panedwindow", "add", "curselection"),
        ("spinbox", "icursor", "curselection"),
    ];
    for (name, own_sub, foreign_sub) in cases {
        let spec = reg
            .get_for_dialect(name, ds)
            .unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            !spec.subcommands.is_empty(),
            "{name} expected to have a non-empty subcommand table"
        );
        let class = spec
            .object_class
            .unwrap_or_else(|| panic!("{name} has a self-referential object class"));
        assert_eq!(
            class.class_name, name,
            "{name}'s object class is self-referential"
        );
        assert!(
            std::ptr::eq(class.instance_methods, spec.subcommands),
            "{name}'s instance_methods must be the literal same slice as its \
             own subcommands — any other pairing risks the two drifting apart"
        );
        assert!(
            reg.instance_method(name, own_sub).is_some(),
            "{name} instance dispatch resolves its own subcommand `{own_sub}`"
        );
        assert!(
            reg.instance_method(name, foreign_sub).is_none(),
            "{name} instance dispatch must not accept `{foreign_sub}` from an unrelated widget"
        );
    }
}

/// Every `::report::*` command has a dedicated, hover-bearing spec.
#[test]
fn report_namespace_commands_have_specs() {
    let (reg, ds) = reg_and_set("tcl8.6");
    for cmd in [
        "report::report",
        "report::defstyle",
        "report::rmstyle",
        "report::stylearguments",
        "report::stylebody",
        "report::styles",
    ] {
        let spec = reg
            .get_for_dialect(cmd, ds)
            .unwrap_or_else(|| panic!("{cmd} present"));
        assert!(spec.hover.is_some(), "{cmd} carries hover");
        assert_eq!(
            spec.required_package,
            Some("report"),
            "{cmd} gated on report"
        );
    }
}

/// The report package requires Tcl 8.5+, so its commands are gated out of the
/// tcl8.4 dialect (version restriction across the 8.4→9.1 range).
#[test]
fn report_commands_gated_out_of_tcl84() {
    for cmd in ["report::report", "report::defstyle", "report::styles"] {
        assert!(!present_in("tcl8.4", cmd), "{cmd} unavailable under tcl8.4");
        for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert!(present_in(d, cmd), "{cmd} available under {d}");
        }
    }
}

// ===========================================================================
// defines_command_at — spec-declared "argument names a new command" facts.
// ===========================================================================

/// `coroutine NAME cmd ?arg …?` creates the command NAME
/// (`TclNRCoroutineObjCmd`, `tclBasic.c`) — the spec carries the name index
/// so the analyser's W123 suppression stays registry-driven.
///
/// tclsh (8.6 & 9.0): `coroutine nextNum apply {{} {yield}}; info commands
/// nextNum` lists it.
#[test]
fn coroutine_defines_command_at_its_name_argument() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_dialect("coroutine", ds)
        .expect("coroutine registered in tcl8.6");
    assert_eq!(spec.defines_command_at, Some(0), "coroutine names arg 0");
}

/// `interp create ?-safe? ?--? ?name?` binds `name` as the child
/// interpreter's command — declared on the `create` subcommand, index
/// relative to the word after `create`.
///
/// tclsh (8.6 & 9.0): `interp create child; child eval {expr 1}` works.
#[test]
fn interp_create_defines_command_at_its_name_argument() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_dialect("interp", ds)
        .expect("interp registered in tcl8.6");
    let create = spec.subcommand("create").expect("interp create modelled");
    assert_eq!(create.defines_command_at, Some(0), "create names sub-arg 0");
    // The other interp subcommands bind no command name.
    let eval = spec.subcommand("eval").expect("interp eval modelled");
    assert_eq!(eval.defines_command_at, None);
}
