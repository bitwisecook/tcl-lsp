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
//! These assert behaviour on the `tcl_registry` crate via `reg.get_for_surface`,
//! `spec.switch_names(Some(ds))`, the `spec.options` scan, `spec.arg_values_at`,
//! `reg.command_names()`, and per-dialect `reg.get_for_surface`.
//!
//! NOTE on dialect detection: the dialect-detection function
//! (`detect_dialect_from_source`) does NOT live in the `tcl-registry` crate —
//! it is implemented in `tcl-compiler` / `tcl-lsp-core`, which are out of
//! scope for this crate's tests. The
//! dialect-name surface that *is* in `tcl-registry` (`KNOWN_DIALECTS`,
//! `available_dialects()`, `DialectProfile::find`, `tcl_dialect::DialectProfile::name_is_irules`)
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

use tcl_dialect::model::{SurfaceQuery};
use tcl_dialect::model::{SurfaceLayer, Family};
use tcl_registry::arity::Arity;
use tcl_registry::events::EventRegistry;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::profiles::ProfileRegistry;
use tcl_dialect::model::{SpecSurface};
use tcl_registry::{
    ArgRole, CommandRegistry, DataCollectionAction, KNOWN_DIALECTS, MethodDispatchKind,
    PayloadCollectionRequirement, SideSwitchTarget, Traits, available_dialects,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `static_context_for(name).commands()` returns a registry with that dialect loaded;
/// pair it with the parsed `SpecSurface` for `get_for_surface`.
fn reg_and_set(dialect: &str) -> (&'static CommandRegistry, Option<SurfaceQuery<'static>>) {
    // "Available under dialect D" is membership in D's *profile availability
    // mask*, not the bare single dialect bit. For the additive vendor
    // dialects the mask is `(base Tcl version | vendor bit)` — e.g. expect is
    // `TCL86 | EXPECT` — so a core command (now an explicit `ALL_TCL` group,
    // no longer universal `None`) resolves through the version half. Using
    // the bare bit here would wrongly exclude the whole Tcl core from every
    // vendor dialect.
    (
        static_context_for(dialect).commands(),
        tcl_registry::model::ingress::resolve_environment(dialect)
            .analyser_profile()
            .surface_query(),
    )
}

/// Whether `cmd` resolves in `dialect`.
fn present_in(dialect: &str, cmd: &str) -> bool {
    let (reg, ds) = reg_and_set(dialect);
    reg.get_for_surface(cmd, ds).is_some()
}

// ===========================================================================
// Registry structure
// ===========================================================================

/// `socket` carries `-server` and `-myaddr`.
///
/// tclsh (8.6 & 9.0): `socket -server` is a real switch — `catch {socket
/// -server} m` reports `no argument given for -server option` (i.e. `-server`
/// is recognised, just missing its value).
#[test]
fn socket_is_registered_with_switches() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_surface("socket", ds)
        .expect("socket registered");
    let switches = socket.switch_names(Some(ds));
    assert!(switches.contains(&"-server"), "switches={switches:?}");
    assert!(switches.contains(&"-myaddr"), "switches={switches:?}");
}

/// `HTTP::header` exists in f5-irules but NOT in tcl8.6.
///
/// f5-dialect: `HTTP::header` is not in core tclsh (`info commands
/// HTTP::header` is empty on 8.6 & 9.0); its dialect-scoping is registry
/// behaviour.
#[test]
fn irules_http_commands_are_dialect_scoped() {
    assert!(present_in("f5-irules", "HTTP::header"));
    let (reg86, ds86) = reg_and_set("tcl8.6");
    assert!(
        reg86.get_for_surface("HTTP::header", ds86).is_none(),
        "HTTP::header must not be visible in tcl8.6"
    );
}

/// `insert` and `replace` are valid first-argument values of `HTTP::header`.
///
/// These keywords are registered as the command's subcommands (rather than as
/// first-argument values).
/// f5-dialect: `HTTP::header` is not real Tcl; subcommand metadata is registry data.
#[test]
fn http_header_subcommand_values_are_registered() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert!(spec.subcommand("insert").is_some());
    assert!(spec.subcommand("replace").is_some());
}

/// The `-server` option is present and documented.
///
/// The option's hover summary ("server mode") is registry-internal
/// hover prose, recorded on `OptionSpec` as `detail`. We assert the
/// option exists, consumes a value, and carries a non-empty detail string
/// (registry-metadata).
#[test]
fn socket_server_option_is_documented() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_surface("socket", ds)
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

/// Every core Tcl command resolves in each `tclN` dialect.
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
            .filter(|c| reg.get_for_surface(c, ds).is_none())
            .collect();
        assert!(missing.is_empty(), "{d} missing core commands: {missing:?}");
    }
}

/// `parray` has hover with a documentation source.
///
/// registry-metadata: hover source attribution is registry data (`parray` is a
/// real Tcl library proc, but the doc string is ours).
#[test]
fn parray_has_hover_with_source() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let parray = reg
        .get_for_surface("parray", ds)
        .expect("parray registered");
    let hover = parray.hover.as_ref().expect("parray has hover");
    assert!(!hover.source.is_empty(), "parray hover has a source");
}

/// iRules specs carry a clouddocs.f5.com hover source, and a curated override
/// wins over generated data.
///
/// f5-dialect: `ACCESS::acl` / `HTTP::header` are not real Tcl; the doc URLs are
/// registry metadata. Rather than parsing the URL host to avoid a
/// substring-sanitisation bug, we assert the recorded source URL directly.
#[test]
fn irules_specs_carry_clouddocs_source() {
    let (reg, ds) = reg_and_set("f5-irules");
    let acl = reg
        .get_for_surface("ACCESS::acl", ds)
        .expect("ACCESS::acl registered");
    let acl_src = acl.hover.as_ref().expect("hover").source;
    assert!(
        acl_src.starts_with("https://clouddocs.f5.com/"),
        "ACCESS::acl source = {acl_src}"
    );

    // The curated HTTP::header override pins the exact documentation URL.
    let hh = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert_eq!(
        hh.hover.as_ref().expect("hover").source,
        "https://clouddocs.f5.com/api/irules/HTTP__header.html"
    );
}

/// `socket`'s arity is "at least 2, unlimited"; `ACCESS::acl` is unlimited.
///
/// tclsh: `socket` needs at minimum a host and a port (2 args) and accepts
/// more with options. f5-dialect: `ACCESS::acl` arity is registry data.
#[test]
fn validation_metadata_is_available() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let socket = reg
        .get_for_surface("socket", ds)
        .expect("socket registered");
    assert_eq!(socket.arity.min, 2);
    assert!(socket.arity.is_unlimited());

    let (rg, irules) = reg_and_set("f5-irules");
    let acl = rg
        .get_for_surface("ACCESS::acl", irules)
        .expect("ACCESS::acl registered");
    assert!(acl.arity.is_unlimited());
}

// ===========================================================================
// Control-flow and needs-start-cmd traits
// ===========================================================================

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

/// The splice-safe set (`CommandRegistry::is_splice_safe`) must cover exactly
/// the inliner's former `SPLICE_SAFE_COMMANDS` list — frame-independent
/// value/IO builtins — and exclude the frame-observing / variable-writing /
/// control-transferring commands the old list deliberately left out.
///
/// registry-metadata: splice-safety is our classification (a composition of
/// the `FRAMELESS_RUNTIME` / frame-sensitivity / variable-name traits).
#[test]
fn splice_safe_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in [
        "list", "lindex", "lrange", "linsert", "llength", "lsort", "lsearch", "lreverse",
        "lreplace", "lrepeat", "concat", "split", "join", "string", "expr", "puts",
    ] {
        assert!(reg.is_splice_safe(c), "{c} must be splice-safe");
    }
    // The old list's documented exclusions: frame-affecting control flow,
    // frame-observing scope commands, and variable writers.
    for c in [
        "return",
        "break",
        "continue",
        "uplevel",
        "upvar",
        "global",
        "variable",
        "set",
        "append",
        "lassign",
        "namespace",
        "info",
        "unset",
    ] {
        assert!(!reg.is_splice_safe(c), "{c} must NOT be splice-safe");
    }
}

/// The frame-sensitive set (`CommandRegistry::is_frame_sensitive`) must cover
/// every member of the inline-proc code action's former `UNSAFE` list — the
/// commands whose meaning changes when a proc body is textually inlined into
/// a caller — and exclude plain value/IO commands.
///
/// registry-metadata: frame-sensitivity is our classification (the
/// `TERMINATES_BLOCK` | `TRANSFERS_CONTROL` | `CREATES_SCOPE_ALIAS` |
/// `CREATES_BARRIER` trait union).
/// Every `::tcl::mathop` operator command is `PURE`.
///
/// The namespace spec carried `PURE` but the commands generated under it did
/// not, so `classify_side_effects` treated `[+ $a $b]` as impure and the GVN
/// pass would neither hoist nor CSE it. Found by scanning all specs for trait
/// pairs that never co-occur: `OPERATOR_COMMAND` (87 specs) and `PURE` (379)
/// had no overlap at all, which for arithmetic is not a plausible fact.
///
/// Purity here is a statement about the command, not a policy: an operator
/// reads its operands and returns a result, touching no variable, channel, or
/// interpreter state. A bad operand raises an error, which the registry
/// models as control flow rather than a side effect.
#[test]
fn operator_commands_are_pure() {
    let reg = CommandRegistry::build_default();
    let ops = [
        "!", "!=", "%", "&", "*", "**", "+", "-", "/", "<", "<<", "<=", "==", ">", ">=", ">>", "^",
        "eq", "ge", "gt", "in", "le", "lt", "ne", "ni", "|", "~",
    ];
    for op in ops {
        let spec = reg
            .get(op)
            .unwrap_or_else(|| panic!("{op} is a registered operator"));
        assert!(
            spec.traits.contains(Traits::OPERATOR_COMMAND),
            "{op} is an operator command"
        );
        assert!(
            spec.traits.contains(Traits::PURE),
            "{op} computes a value with no side effect and must be PURE, or \
             the optimiser cannot hoist or CSE it"
        );
    }
    // The qualified spellings carry the traits too — the optimiser sees
    // whichever spelling the source used.
    for name in ["::tcl::mathop::+", "tcl::mathop::eq"] {
        let spec = reg
            .get(name)
            .unwrap_or_else(|| panic!("{name} is registered"));
        assert!(spec.traits.contains(Traits::PURE), "{name} must be PURE");
    }
}

#[test]
fn frame_sensitive_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in [
        "return", "break", "continue", "tailcall", "yield", "uplevel", "upvar", "global",
        "variable",
    ] {
        assert!(reg.is_frame_sensitive(c), "{c} must be frame-sensitive");
        assert!(
            reg.frame_sensitive_commands().contains(&c),
            "{c} in the scan set"
        );
    }
    for c in ["puts", "set", "expr", "string", "list"] {
        assert!(
            !reg.is_frame_sensitive(c),
            "{c} must NOT be frame-sensitive"
        );
    }
}

/// Coroutine-family membership is registry data consumed by restricted
/// backends; spelling and release handling must therefore resolve here rather
/// than in each backend.
#[test]
fn coroutine_primitive_membership_is_versioned_and_qualified() {
    let (tcl86, tcl86_set) = reg_and_set("tcl8.6");
    for name in ["coroutine", "::coroutine", "yield", "::yield", "yieldto"] {
        assert!(
            tcl86
                .invocation_traits(name, &[], tcl86_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} must resolve through the Tcl 8.6 coroutine family"
        );
    }
    for name in ["coroinject", "coroprobe"] {
        assert!(
            !tcl86
                .invocation_traits(name, &[], tcl86_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} is not available before Tcl 9.0"
        );
    }

    let (tcl90, tcl90_set) = reg_and_set("tcl9.0");
    for name in ["coroinject", "::coroinject", "coroprobe", "::coroprobe"] {
        assert!(
            tcl90
                .invocation_traits(name, &[], tcl90_set)
                .contains(Traits::COROUTINE_PRIMITIVE),
            "{name} must resolve through the Tcl 9.0 coroutine family"
        );
    }

    let (tcl85, tcl85_set) = reg_and_set("tcl8.5");
    assert!(
        !tcl85
            .invocation_traits("coroutine", &[], tcl85_set)
            .contains(Traits::COROUTINE_PRIMITIVE)
    );
}

/// The fire-and-forget teardown set (`FIRE_AND_FORGET_TEARDOWN`) must cover
/// the W302 suppression idiom's documented members, at both the command and
/// the subcommand level, and stay off destructive-but-not-teardown
/// subcommands (`file rename` / `file mkdir` fail for real reasons).
///
/// registry-metadata: the fire-and-forget classification is our metadata.
#[test]
fn fire_and_forget_teardown_membership() {
    let (reg, _) = reg_and_set("tcl8.6");
    for c in ["close", "unset", "rename"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(
            spec.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN),
            "{c}"
        );
    }
    for (c, sub) in [
        ("after", "cancel"),
        ("chan", "close"),
        ("array", "unset"),
        ("dict", "unset"),
        ("interp", "delete"),
        ("file", "delete"),
        ("namespace", "delete"),
        ("namespace", "forget"),
    ] {
        let sc = reg
            .get(c)
            .and_then(|s| s.subcommand(sub))
            .unwrap_or_else(|| panic!("{c} {sub} registered"));
        assert!(
            sc.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN),
            "{c} {sub}"
        );
    }
    for (c, sub) in [
        ("file", "rename"),
        ("file", "mkdir"),
        ("chan", "configure"),
        ("namespace", "eval"),
    ] {
        let sc = reg.get(c).and_then(|s| s.subcommand(sub));
        assert!(
            sc.is_none_or(|s| !s.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)),
            "{c} {sub} must NOT be fire-and-forget"
        );
    }
}

// ===========================================================================
// Commands-for-event / command legality (iRules event ⇄ command cross-product)
// ===========================================================================

/// `HTTP::header` is valid/legal in `HTTP_REQUEST`.
///
/// f5-dialect: events and the HTTP profile model are F5 concepts, not tclsh.
#[test]
fn http_header_legal_in_http_request() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let valid = reg.valid_irules_commands_for_event("HTTP_REQUEST", &events, &profiles, None);
    assert!(valid.contains(&"HTTP::header"));
    assert!(reg.is_irules_command_legal_in_event(
        "HTTP::header",
        "HTTP_REQUEST",
        &events,
        &profiles
    ));
}

/// `HTTP::header` requires an HTTP profile, which `RULE_INIT` lacks.
///
/// f5-dialect.
#[test]
fn http_header_not_legal_in_rule_init() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    assert!(!reg.is_irules_command_legal_in_event("HTTP::header", "RULE_INIT", &events, &profiles));
    let valid = reg.valid_irules_commands_for_event("RULE_INIT", &events, &profiles, None);
    assert!(!valid.contains(&"HTTP::header"));
}

/// An unknown event is illegal for every command and yields an empty valid
/// set.
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
        reg.valid_irules_commands_for_event("TOTALLY_FAKE_EVENT", &events, &profiles, None)
            .is_empty()
    );
}

/// The HTTP2 family is legal in `HTTP_REQUEST` and in the message-routing
/// events.
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
        let listed = reg.valid_irules_commands_for_event(event, &events, &profiles, None);
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

/// A command's `excluded_events` make it illegal in those events even when the
/// profile requirements would otherwise pass. `irules_events_for_command` is
/// the inverse listing and excludes them.
///
/// f5-dialect.
#[test]
fn excluded_events_are_respected() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("TCP::rcv_scale", ds)
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
// Registry info helpers (event_info)
// ===========================================================================

/// `event_info` resolves a known event with a side label, ≥1 valid command,
/// and serialises a dual transport as `tcp/udp`.
///
/// f5-dialect.
#[test]
fn event_info_for_known_events() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();

    let http = reg.event_info("http_request", &events, &profiles, None);
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

    let ca = reg.event_info("client_accepted", &events, &profiles, None);
    assert_eq!(ca.transport.as_deref(), Some("tcp/udp"));
}

/// An unknown event is `known=false`, empty valid commands, side `"unknown"`.
///
/// f5-dialect.
#[test]
fn event_info_for_unknown_event() {
    let (reg, _) = reg_and_set("f5-irules");
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let info = reg.event_info("totally_fake_event", &events, &profiles, None);
    assert_eq!(info.event, "TOTALLY_FAKE_EVENT");
    assert!(!info.known);
    assert!(info.valid_commands.is_empty());
    assert_eq!(info.side, "unknown");
    assert!(info.transport.is_none());
}

// ===========================================================================
// Form kind and form resolution
// ===========================================================================

/// The `FormKind` variants exist.
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

/// `HTTP::uri` with no args is a pure getter; with one arg it is a mutating
/// setter. `resolve_call` picks the matching form by arity.
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
// Subcommand form resolution
// ===========================================================================

/// `HTTP::header lws` takes zero args.
///
/// f5-dialect: arity is registry metadata.
#[test]
fn http_header_lws_subcommand_is_nullary() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    let lws = spec.subcommand("lws").expect("lws subcommand");
    assert_eq!(lws.arity, Arity::exact(0));
}

/// `HTTP::header value` takes exactly one arg (the header name), and resolves
/// to no forms.
///
/// f5-dialect.
#[test]
fn http_header_value_subcommand_takes_one_arg() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    let value = spec.subcommand("value").expect("value subcommand");
    assert_eq!(value.arity, Arity::exact(1));
}

// ===========================================================================
// Closed value arguments
// ===========================================================================

/// `HTTP::version` arg 0 is a closed value set of exactly the HTTP/1.x version
/// strings.
///
/// f5-dialect: `HTTP::version` is not real Tcl. The allowed-version set is
/// registry metadata. (HTTP itself only ever speaks 0.9/1.0/1.1 over the
/// classic text protocol, which is what the closed set encodes.)
#[test]
fn http_version_arg0_is_closed_value_set() {
    let (reg, ds) = reg_and_set("f5-irules");
    let spec = reg
        .get_for_surface("HTTP::version", ds)
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

/// `set` declares no closed-value argument indices.
///
/// tclsh: `set` accepts any value; the absence of a closed set is correct.
#[test]
fn set_has_no_closed_value_args() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg.get_for_surface("set", ds).expect("set registered");
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
    let when = reg.get_for_surface("when", ds).expect("when registered");
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
// Body kind / namespace export
// ===========================================================================

/// `proc`'s body argument is structural.
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

/// `namespace export ?-clear?` and `namespace import ?-force?` hand-parse a
/// single optional leading flag in C (`NamespaceExportCmd` /
/// `NamespaceImportCmd`, `tclNamesp.c`), so exactly one leading option word is
/// consumed and every further word is positional however it is spelled.
///
/// Oracle (tclsh 8.6.14 / 9.0.4): `namespace export -clear -clear p` leaves
/// `-clear p` exported — and `namespace import ::src::*` then binds a genuine
/// `::dst::-clear` command — while `namespace import -force -force ::src::*`
/// aborts with `no namespace specified in import pattern "-force"`.
/// `namespace export a -clear` exports both, so the flag is only ever the
/// first word. Consumers read the limit from here rather than bounding a loop
/// of their own (PR #1102 review finding 3).
///
/// registry-metadata: `max_leading_option_words`.
#[test]
fn namespace_export_and_import_consume_one_leading_option_word() {
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("namespace").expect("namespace registered");
    for (sub, flag) in [("export", "-clear"), ("import", "-force")] {
        let s = spec
            .subcommand(sub)
            .unwrap_or_else(|| panic!("namespace {sub} registered"));
        assert_eq!(
            s.max_leading_option_words,
            Some(1),
            "namespace {sub} consumes exactly one leading option word",
        );
        assert!(
            s.options.iter().any(|o| o.name == flag && !o.takes_value()),
            "namespace {sub} declares {flag} as a flag",
        );
        // The cap is what the shared leading-option walk applies: a second
        // occurrence is an ordinary argument word, not a second flag.
        assert_eq!(s.leading_option_word_count(&[flag, flag, "p"]), 1);
        assert_eq!(s.leading_option_word_count(&["p", flag]), 0);
    }
}

/// `tcltest::test` declares it is namespace-exported (so the bare `test` can
/// resolve to it).
///
/// registry-metadata.
#[test]
fn tcltest_test_is_namespace_exported() {
    let (reg, _) = reg_and_set("tcl8.6");
    let spec = reg.get("tcltest::test").expect("tcltest::test registered");
    assert!(spec.is_namespace_exported);
}

/// The iRules `when` body runs in the dispatcher's frame, not the caller's.
///
/// `when` declares its event name (argument 0) as a navigable definition, so
/// the document-symbol / workspace-symbol providers list event handlers the
/// way they list a `proc` — registry data, not a command-name check.
///
/// f5-dialect + registry-metadata.
#[test]
fn when_defines_its_event_as_an_outline_symbol() {
    use tcl_registry::DefinedSymbolKind;
    let (reg, ds) = reg_and_set("f5-irules");
    let sym = reg
        .defines_symbol("when", ds)
        .expect("when should declare a symbol definer");
    assert_eq!(sym.name_arg, 0, "the event name is the first argument");
    assert_eq!(sym.kind, DefinedSymbolKind::Event);
    // Every `when` call binds a handler — there is no getter form to gate on.
    assert_eq!(sym.requires_arg, None);
    assert!(reg.commands_defining_symbols().contains(&"when"));
}

/// f5-dialect + registry-metadata.
#[test]
fn when_body_is_structural() {
    use tcl_registry::body_kind::BodyKind;
    let (reg, ds) = reg_and_set("f5-irules");
    let when = reg.get_for_surface("when", ds).expect("when registered");
    assert_eq!(when.body_kind, BodyKind::Structural);
}

// ===========================================================================
// iRules nesting script bodies (side switches)
// ===========================================================================

/// `clientside`, `serverside`, `peer` are side-switches.
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

/// Collect/release/payload and side-switch behaviour is explicit registry
/// data. In particular, `UDP::payload` is available per datagram and must not
/// lead a generic consumer to invent a nonexistent `UDP::collect` command.
///
/// f5-dialect + registry-metadata.
#[test]
fn irules_collection_and_side_switch_facts_are_registry_driven() {
    let (reg, _) = reg_and_set("f5-irules");
    let http_payload = reg
        .data_collection_operation("HTTP::payload")
        .expect("HTTP payload lifecycle fact");
    assert_eq!(http_payload.action, DataCollectionAction::Payload);
    assert_eq!(http_payload.protocol.name, "HTTP");
    assert_eq!(
        http_payload.protocol.payload_requirement,
        PayloadCollectionRequirement::ExplicitCollect
    );
    assert_eq!(
        reg.data_collection_collect_command("HTTP")
            .expect("HTTP collect command")
            .name,
        "HTTP::collect"
    );

    let udp = reg
        .data_collection_operation("UDP::payload")
        .expect("UDP payload lifecycle fact");
    assert_eq!(
        udp.protocol.payload_requirement,
        PayloadCollectionRequirement::NotRequired
    );
    assert!(
        reg.data_collection_collect_command("UDP").is_none(),
        "UDP has no explicit collect command"
    );

    assert_eq!(
        reg.side_switch_target("clientside"),
        Some(SideSwitchTarget::Client)
    );
    assert_eq!(
        reg.side_switch_target("serverside"),
        Some(SideSwitchTarget::Server)
    );
    assert_eq!(reg.side_switch_target("peer"), Some(SideSwitchTarget::Peer));
    assert_eq!(SideSwitchTarget::Client.execution_side("server"), "client");
    assert_eq!(SideSwitchTarget::Server.execution_side("client"), "server");
    assert_eq!(SideSwitchTarget::Peer.execution_side("client"), "server");
    assert_eq!(SideSwitchTarget::Peer.execution_side("server"), "client");

    let priority = reg
        .event_handler_priority("when")
        .expect("when priority policy");
    let handler = reg.event_handler_spec().expect("event handler spec");
    assert_eq!(handler.name, "when");
    assert_eq!(handler.event_handler_priority, Some(priority));
    assert_eq!(priority.keyword, "priority");
    assert_eq!(priority.default_priority, 500);
    assert!(
        !priority.warn_when_implicit,
        "BIG-IP accepts an omitted priority"
    );

    assert!(
        reg.get("HTTP::header")
            .is_some_and(|spec| spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT)),
        "ordinary HTTP transaction commands require the live context"
    );
    assert!(
        reg.get("HTTP::has_responded")
            .is_some_and(|spec| !spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT)),
        "the response-state query remains valid after a response is committed"
    );
}

/// `clientside` / `serverside` take zero or one argument; `peer` requires its
/// script — arity bounds.
///
/// f5-dialect: arity is registry metadata.
#[test]
fn side_switch_arities() {
    let (reg, ds) = reg_and_set("f5-irules");
    for name in ["clientside", "serverside"] {
        let spec = reg
            .get_for_surface(name, ds)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(spec.arity.min, 0, "{name} min");
        assert_eq!(spec.arity.max, 1, "{name} max");
    }
    let peer = reg.get_for_surface("peer", ds).expect("peer registered");
    assert_eq!(peer.arity.min, 1);
    assert_eq!(peer.arity.max, 1);
}

// ===========================================================================
// Dialect-differentiated side effects
// ===========================================================================

/// `close` is a file-I/O op in plain Tcl and a connection-control op in iRules
/// (two specs under one name).
///
/// tclsh: `close` closes a channel. f5-dialect: the iRules `close` overlay
/// (connection control) is F5-specific. The dialect-specific *classification*
/// is registry metadata.
#[test]
fn close_side_effects_differ_by_dialect() {
    use tcl_registry::side_effects::SideEffectTarget;
    let (tcl_reg, tcl_ds) = reg_and_set("tcl8.6");
    let tcl_close = tcl_reg
        .get_for_surface("close", tcl_ds)
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
        .get_for_surface("close", ir_ds)
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
// Lazy dialect loading
// ===========================================================================

/// A freshly built default registry has core commands but not iRules / non-Tk
/// dialect commands.
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
/// dialect (`SpecSurface::TK_AND_TCL`) — they must resolve in a `wish` /
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
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
            "{name} available under tcl8.6 (wish / package require Tk)"
        );
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
            "{name} available under tcl9.0"
        );
        assert!(
            spec.supports_dialect(Some(SurfaceQuery::any_release(Family::Tcl).with_packages(&["Tk"]))),
            "{name} available under the tk dialect"
        );
        // NOT available in the F5 embedded dialects.
        assert!(
            !spec.supports_dialect(Some(SurfaceQuery::any_release(Family::F5Irules))),
            "{name} must NOT be offered in iRules"
        );
        assert!(
            !spec.supports_dialect(Some(SurfaceQuery::core(Family::Tcl, "8.4").with_packages(&["iapps"]))),
            "{name} must NOT be offered in iApps"
        );
    }
}

/// `load_dialect` makes iRules commands resolvable and listed.
///
/// f5-dialect.
#[test]
fn load_dialect_makes_irules_commands_visible() {
    let mut reg = CommandRegistry::build_default();
    assert!(reg.get("HTTP::header").is_none());
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    assert!(reg.get("HTTP::header").is_some());
    let names: std::collections::HashSet<&str> = reg.command_names().collect();
    assert!(names.contains("HTTP::header"));
}

/// `load_dialect` is idempotent.
///
/// f5-dialect / registry behaviour.
#[test]
fn load_dialect_is_idempotent() {
    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let after_first = reg.len();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, "")); // second load is a no-op
    assert_eq!(
        reg.len(),
        after_first,
        "double-load must not duplicate specs"
    );
}

// ===========================================================================
// Signature membership per dialect
// ===========================================================================

/// `dict`/`try`/`tailcall` are absent in 8.4.
///
/// tclsh: `dict` (8.5), `try`/`tailcall` (8.6) — all post-date 8.4. We can't
/// run tclsh8.4 here (not installed), but these introduction versions are
/// well-known Tcl history and match the 8.5/8.6 listings observed above.
#[test]
fn tcl84_excludes_newer_commands() {
    let (reg, ds) = reg_and_set("tcl8.4");
    assert!(reg.get_for_surface("dict", ds).is_none(), "dict is 8.5+");
    assert!(reg.get_for_surface("try", ds).is_none(), "try is 8.6+");
    assert!(
        reg.get_for_surface("tailcall", ds).is_none(),
        "tailcall is 8.6+"
    );
}

/// `dict` present in 8.5, `try` absent.
///
/// tclsh: `dict` exists in 8.5+ (present in both 8.6 & 9.0 subcommand probes);
/// `try` is 8.6+.
#[test]
fn tcl85_has_dict_not_try() {
    let (reg, ds) = reg_and_set("tcl8.5");
    assert!(reg.get_for_surface("dict", ds).is_some());
    assert!(reg.get_for_surface("try", ds).is_none());
}

/// `TclOO` (`oo::class`, `oo::define`) is 8.6+.
///
/// tclsh: `oo::class` is present on 8.6 (`info commands oo::class` non-empty);
/// `TclOO` landed in 8.6.
#[test]
fn tcloo_is_tcl86_plus() {
    let (r86, d86) = reg_and_set("tcl8.6");
    assert!(r86.get_for_surface("oo::class", d86).is_some());
    assert!(r86.get_for_surface("oo::define", d86).is_some());

    let (r85, d85) = reg_and_set("tcl8.5");
    assert!(r85.get_for_surface("oo::class", d85).is_none());
    assert!(r85.get_for_surface("oo::define", d85).is_none());
}

/// The iRules profile pulls in `when`, the HTTP family, the seeded AAA catalog,
/// the `class`/`table` generated commands, and deprecated profile commands.
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
            reg.get_for_surface(name, ds).is_some(),
            "{name} should be in the f5-irules profile"
        );
    }
}

/// `proc` keeps its core arity (exactly 3) and body role even under the iRules
/// profile.
///
/// tclsh: `proc name args body` is exactly 3 args. registry-metadata: the
/// body role.
#[test]
fn f5_irules_keeps_base_proc_signature() {
    let (reg, ds) = reg_and_set("f5-irules");
    let proc = reg.get_for_surface("proc", ds).expect("proc registered");
    assert_eq!(proc.arity, Arity::exact(3));
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body));
}

/// Curated iRules commands have concrete arity and subcommands.
///
/// f5-dialect.
#[test]
fn f5_irules_curated_signatures_are_concrete() {
    let (reg, ds) = reg_and_set("f5-irules");
    let header = reg
        .get_for_surface("HTTP::header", ds)
        .expect("HTTP::header registered");
    assert!(header.subcommand("value").is_some());
    assert!(header.subcommand("insert").is_some());

    let when = reg.get_for_surface("when", ds).expect("when registered");
    assert_eq!(when.arity.max, 6);
    assert_eq!(reg.get_for_surface("pool", ds).expect("pool").arity.min, 1);
    assert_eq!(reg.get_for_surface("node", ds).expect("node").arity.min, 1);
    assert_eq!(
        reg.get_for_surface("HTTP::respond", ds)
            .expect("HTTP::respond")
            .arity
            .min,
        1
    );
}

/// The f5-iapps profile has `iapp::template` / `iapp::conf` but NOT the iRules
/// catalog.
///
/// f5-dialect.
#[test]
fn f5_iapps_profile_membership() {
    let (reg, ds) = reg_and_set("f5-iapps");
    assert!(reg.get_for_surface("iapp::template", ds).is_some());
    assert!(reg.get_for_surface("iapp::conf", ds).is_some());
    // f5-iapps is a separate catalog from f5-irules.
    assert!(
        reg.get_for_surface("AAA::acct_result", ds).is_none(),
        "iRules catalog must not leak into f5-iapps"
    );
}

/// The expect profile adds `spawn`/`expect`/… on top of base Tcl and excludes
/// iRules.
///
/// expect is a real Tcl extension; `spawn`/`expect`/`send`/`interact` are its
/// commands (not in bare tclsh, but part of the `expect` interpreter). The
/// dialect partitioning is registry behaviour.
#[test]
fn expect_profile_membership() {
    let (reg, ds) = reg_and_set("expect");
    for name in ["spawn", "expect", "send", "interact", "log_user"] {
        assert!(
            reg.get_for_surface(name, ds).is_some(),
            "{name} should be in the expect profile"
        );
    }
    // Base Tcl is still present.
    for name in ["set", "proc", "if"] {
        assert!(reg.get_for_surface(name, ds).is_some(), "{name} (base tcl)");
    }
    // iRules commands are not.
    assert!(reg.get_for_surface("when", ds).is_none());
    assert!(reg.get_for_surface("HTTP::header", ds).is_none());
}

/// `when EVENT … { body }` marks the trailing body argument.
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

/// The `TclOO` definition bodies resolve to the BODY role.
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

/// The BODY role on a proc helper — `proc helper x { body }` marks the body
/// argument.
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
    assert!(tcl_dialect::DialectProfile::name_is_irules(Some("f5-irules")));
    assert!(tcl_dialect::DialectProfile::name_is_irules(Some("irules")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some("tcl8.6")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some("f5-iapps")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(Some("f5-bigip")));
    assert!(!tcl_dialect::DialectProfile::name_is_irules(None));
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
        assert!(tcl_dialect::DialectProfile::find(d).map(tcl_dialect::DialectProfile::surface_query).is_some(), "{d} should parse");
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
    assert!(tcl_dialect::DialectProfile::find("unknown").map(tcl_dialect::DialectProfile::surface_query).is_none());
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

/// `DialectProfile::find` round-trips the canonical names and rejects junk —
/// underpinning the cache key normalisation a typo'd dialect collapses to.
///
/// registry-metadata.
#[test]
fn dialect_parse_roundtrip() {
    assert_eq!(tcl_dialect::DialectProfile::find("tcl8.6").map(tcl_dialect::DialectProfile::surface_query), Some(SpecSurface::TCL86));
    assert_eq!(tcl_dialect::DialectProfile::find("tcl9.0").map(tcl_dialect::DialectProfile::surface_query), Some(SpecSurface::TCL90));
    assert_eq!(tcl_dialect::DialectProfile::find("f5-irules").map(tcl_dialect::DialectProfile::surface_query), Some(SpecSurface::IRULES));
    assert_eq!(tcl_dialect::DialectProfile::find("expect").map(tcl_dialect::DialectProfile::surface_query), Some(SpecSurface::EXPECT));
    assert_eq!(tcl_dialect::DialectProfile::find("definitely-not-a-dialect").map(tcl_dialect::DialectProfile::surface_query), None);
}

// ===========================================================================
// C-Tcl ground-truth: subcommand sets of string / dict / info match tclsh.
//
// These are direct tclsh facts. The registry must register at least the
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
    let in_86 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(in_86.contains(&"-all"), "8.6: {in_86:?}");
    assert!(in_86.contains(&"-nocase"), "8.6: {in_86:?}");
    assert!(
        !in_86.contains(&"-command"),
        "9.0-only -command leaked into 8.6: {in_86:?}"
    );
    let in_90 = regsub.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert!(in_90.contains(&"-command"), "9.0: {in_90:?}");
}

/// C-Tcl: `source -nopkg` is a Tcl 9.0-only flag.
///
/// tclsh8.6: `catch {source -nopkg missing.tcl} message` reports a wrong
/// number of arguments. tclsh9.0 accepts the flag, then reports that the file
/// does not exist. The different errors establish both the parser contract and
/// the version boundary without needing a source file.
#[test]
fn source_nopkg_is_version_gated() {
    let reg = CommandRegistry::build_default();
    let source = reg.get("source").expect("source registered");
    let in_86 = source.switch_names(Some(SurfaceQuery::core(Family::Tcl, "8.6")));
    assert!(
        !in_86.contains(&"-nopkg"),
        "Tcl 9-only -nopkg leaked into 8.6: {in_86:?}"
    );
    let in_90 = source.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")));
    assert!(in_90.contains(&"-nopkg"), "9.0: {in_90:?}");
    let in_91 = source.switch_names(Some(SurfaceQuery::core(Family::Tcl, "9.1")));
    assert!(in_91.contains(&"-nopkg"), "9.1: {in_91:?}");
}

/// C-Tcl: `const` is a Tcl 9.0 builtin, present in 9.0.
///
/// tclsh8.6: `info commands const` → empty. tclsh9.0: → `const`.
///
/// `const` (Tcl 9.0, TIP 677) is present from 9.0 and — after the
/// registry-wide explicit-dialect sweep — correctly ABSENT before it. It used
/// to be declared `surface: None` (universal) as a workaround to reach iRules
/// events, which wrongly made `get_for_surface("const", TCL86)` return `Some`
/// even though real tclsh8.6 lacks the command. It is now `TCL90_PLUS`, so the
/// registry matches C-Tcl on both axes: present in 9.0/9.1, absent in 8.x (and
/// absent from iRules, whose embedded 8.4.6 also predates it).
#[test]
fn const_is_present_in_tcl90_and_absent_before() {
    assert!(
        present_in("tcl9.0", "const"),
        "const must be present in tcl9.0"
    );
    assert!(
        present_in("tcl9.1", "const"),
        "const must be present in tcl9.1"
    );
    let (reg86, ds86) = reg_and_set("tcl8.6");
    assert!(
        reg86.get_for_surface("const", ds86).is_none(),
        "const is a Tcl 9.0 command; it must not resolve under tcl8.6"
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
        let reg = static_context_for(d).commands();
        assert!(!reg.is_empty(), "{d} registry is empty");
        assert!(reg.get("set").is_some(), "{d} should have `set`");
    }
    // An unparseable dialect collapses to the plain-Tcl entry (still usable).
    let junk = static_context_for("not-a-real-dialect").commands();
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
        .get_for_surface("report::defstyle", ds)
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
        .get_for_surface("report::report", ds)
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
            .get_for_surface(name, ds)
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
            .get_for_surface(name, ds)
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
        // This is a pointer-identity check, and it is only meaningful because
        // each widget's table is a `static SUBCOMMANDS: [SubCommand; N]` that
        // both use sites reach via `&SUBCOMMANDS` — one static, one address.
        // It used to be a `const`, whose referent has NO stable address (every
        // use site may materialise its own copy); the assertion then passed at
        // opt-level 0 purely by accident of codegen and failed the moment the
        // crate was built at opt-level 2.  If you turn these tables back into
        // `const`s, this assertion becomes a coin flip again — keep them
        // `static`.
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

#[test]
fn instance_methods_follow_the_owning_package_lifecycle() {
    // `current` is a Tk 9.1 addition to ttk::treeview. The class itself is
    // older, so this exercises method-row lifecycle filtering rather than
    // command-level availability. The registry must make the same answer
    // available to every downstream object-method consumer.
    for (dialect, current_is_available) in [("tcl9.0", false), ("tcl9.1", true)] {
        let reg = static_context_for(dialect).commands();
        assert_eq!(
            reg.instance_method("ttk::treeview", "current").is_some(),
            current_is_available,
            "ttk::treeview current under {dialect}"
        );
        assert_eq!(
            reg.instance_methods("ttk::treeview")
                .iter()
                .any(|method| method.name == "current"),
            current_is_available,
            "instance-method enumeration under {dialect}"
        );
        assert!(
            reg.instance_method("ttk::treeview", "instate").is_some(),
            "an older ttk::treeview method remains available under {dialect}"
        );
    }
}

#[test]
fn object_method_prefix_policy_is_strict_by_default_and_enabled_for_tk() {
    let (reg, ds) = reg_and_set("tcl8.6");

    let entry = reg.get_for_surface("entry", ds).unwrap();
    let entry_class = entry.object_class.unwrap();
    assert_eq!(
        entry_class.method_prefix_matching,
        tcl_registry::abbrev::PrefixMatching::Enabled
    );
    assert!(
        reg.instance_method("entry", "get").is_some(),
        "exact Tk method"
    );
    assert!(
        reg.instance_method("entry", "conf").is_some(),
        "unique Tk method prefix"
    );
    assert!(
        reg.instance_method("entry", "c").is_none(),
        "ambiguous Tk method prefix must abstain"
    );

    let report = reg.get_for_surface("report::report", ds).unwrap();
    let report_class = report.object_class.unwrap();
    assert_eq!(
        report_class.method_prefix_matching,
        tcl_registry::abbrev::PrefixMatching::Strict
    );
    assert!(reg.instance_method("report::report", "destroy").is_some());
    assert!(
        reg.instance_method("report::report", "des").is_none(),
        "non-Tk object methods are exact by default"
    );
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
            .get_for_surface(cmd, ds)
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

/// The `::tcl::` namespace does not exist at all before Tcl 8.5 (TIP 174
/// added `::tcl::mathop`; there is no `::tcl` namespace in a real Tcl 8.4).
/// Every dated addition under it must be gated to its real introduction
/// release, not left universally available (`surface: None`), which would
/// wrongly resolve under `tcl8.4` and every pre-that-version profile.
#[test]
fn tcl_namespace_commands_gated_to_their_real_introduction_version() {
    // (command, dialects it must resolve under, dialects it must not)
    let cases: &[(&str, &[&str], &[&str])] = &[
        // Tcl Modules (TIP 189) — 8.5+.
        (
            "tcl::tm::path",
            &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4"],
        ),
        (
            "tcl::tm::roots",
            &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4"],
        ),
        // Coroutines (TIP 396) and their `corotype` introspection — 8.6+;
        // empirically confirmed present on a real tclsh 8.6.14, not 9.0
        // only as an earlier (incorrect) version of this data claimed.
        (
            "::tcl::unsupported::corotype",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        // `cookiejar` package's IDNA helpers — bundled since Tcl 8.6.
        (
            "tcl::idna::decode",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        (
            "tcl::idna::encode",
            &["tcl8.6", "tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5"],
        ),
        // `::tcl::build-info` — Tcl 9.0+ (both spellings carry their own spec).
        (
            "tcl::build-info",
            &["tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5", "tcl8.6"],
        ),
        (
            "::tcl::build-info",
            &["tcl9.0", "tcl9.1"],
            &["tcl8.4", "tcl8.5", "tcl8.6"],
        ),
    ];
    for &(cmd, present, absent) in cases {
        for d in present {
            assert!(present_in(d, cmd), "{cmd} must be available under {d}");
        }
        for d in absent {
            assert!(!present_in(d, cmd), "{cmd} must be unavailable under {d}");
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
        .get_for_surface("coroutine", ds)
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
        .get_for_surface("interp", ds)
        .expect("interp registered in tcl8.6");
    let create = spec.subcommand("create").expect("interp create modelled");
    assert_eq!(create.defines_command_at, Some(0), "create names sub-arg 0");
    // The other interp subcommands bind no command name.
    let eval = spec.subcommand("eval").expect("interp eval modelled");
    assert_eq!(eval.defines_command_at, None);
}

// ===========================================================================
// TclOO method-context keywords + the `Tcl_ConcatObj` eval family (#1050, #1051)
// ===========================================================================

/// The `TclOO` method-context keyword set is exactly `my` / `next` / `nextto`
/// / `self`, and `CommandRegistry::method_dispatch_keyword` answers with the
/// right kind for each.
///
/// This is the guard for issue #1050: the consumer-visible keyword set must
/// equal the trait-carrying specs, so a consumer can never drift from the
/// registry by re-adding a `head == "my"` literal.  `link` is deliberately
/// out — it *creates* per-class bareword commands rather than dispatching one
/// (issue #1026).
///
/// registry-metadata: which keyword is dispatch and which is introspection is
/// our classification.  (`my`, `next`, `nextto`, `self`, and `link` are all
/// real tclsh 8.6/9.0 `TclOO` commands.)
#[test]
fn tcloo_dispatch_keyword_membership() {
    let reg = static_context_for("tcl8.6").commands();
    assert_eq!(
        reg.method_dispatch_keyword("my"),
        Some(MethodDispatchKind::SelfDispatch)
    );
    assert_eq!(
        reg.method_dispatch_keyword("next"),
        Some(MethodDispatchKind::NextChain)
    );
    assert_eq!(
        reg.method_dispatch_keyword("nextto"),
        Some(MethodDispatchKind::NextChain)
    );
    assert_eq!(
        reg.method_dispatch_keyword("self"),
        Some(MethodDispatchKind::Introspection)
    );
    // A leading `::` resolves to the bare name — consumers used to match
    // `"my" | "::my"` by hand.
    assert_eq!(
        reg.method_dispatch_keyword("::my"),
        Some(MethodDispatchKind::SelfDispatch)
    );
    for negative in ["puts", "set", "link", "proc", "oo::define"] {
        assert_eq!(
            reg.method_dispatch_keyword(negative),
            None,
            "{negative} is not a method-dispatch keyword"
        );
    }
    // The trait-carrying spec set and the query agree, in both directions.
    let mut carriers: Vec<&str> = reg
        .commands_with_trait(Traits::TCLOO_SELF_DISPATCH)
        .into_iter()
        .chain(reg.commands_with_trait(Traits::TCLOO_NEXT_CHAIN))
        .chain(reg.commands_with_trait(Traits::TCLOO_INTROSPECTION))
        .collect();
    carriers.sort_unstable();
    assert_eq!(carriers, ["my", "next", "nextto", "self"]);
    for name in carriers {
        assert!(
            reg.method_dispatch_keyword(name).is_some(),
            "{name} carries a dispatch trait so the query must answer for it"
        );
    }
}

/// The method-context-scoped set (issue #1026) — which bare spellings only
/// resolve inside a `TclOO` method body — is exactly `callback` / `link` /
/// `my` / `mymethod` / `next` / `nextto` / `self` / `classvariable`, and
/// never the qualified `oo::Helpers::…` spellings.
///
/// Pinned against tclsh 9.0.4: each bare word at the top level raises
/// `invalid command name`, `info commands ::link` is empty, and inside
/// `oo::class create C { method m {} { … } }` `namespace which -command
/// link` answers `::oo::Helpers::link` while `… my` answers
/// `::oo::ObjN::my`. tclsh 8.6.14 agrees for the four members it ships.
/// The qualified spelling is a real global command — calling it outside a
/// method fails with `::oo::Helpers::link may only be called from inside a
/// method`, a *runtime* error rather than an unknown command — so it must
/// answer `false`.
///
/// The guard against a consumer re-deriving the set from a name list: W123,
/// hover, and completion all ask
/// `CommandRegistry::resolves_only_in_method_context`.
#[test]
fn tcloo_method_context_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_METHOD_CONTEXT);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(
        carriers,
        [
            "callback",
            "classvariable",
            "link",
            "my",
            "mymethod",
            "next",
            "nextto",
            "self"
        ]
    );
    for name in carriers {
        assert!(
            reg.resolves_only_in_method_context(name),
            "{name} carries the trait so the query must answer for it"
        );
    }
    for qualified in [
        "oo::Helpers::link",
        "::oo::Helpers::link",
        "oo::Helpers::self",
        "oo::Helpers::classvariable",
    ] {
        assert!(
            reg.get(qualified).is_some(),
            "{qualified} is a real global command in Tcl 9.0"
        );
        assert!(
            !reg.resolves_only_in_method_context(qualified),
            "{qualified} resolves from anywhere"
        );
    }
    for negative in ["puts", "set", "proc", "oo::define", "oo::class"] {
        assert!(
            !reg.resolves_only_in_method_context(negative),
            "{negative} is an ordinary global command"
        );
    }
    // `commands_with_trait` enumerates the whole spec table, so the dialect
    // gate lives in the query itself: under 8.6 `callback` (9.0+ core only,
    // no Tcllib route) is not method-context-scoped because it does not
    // exist, while `link`, `mymethod`, and `classvariable` still are — each
    // has an 8.6 spelling via the `ooutil`-gated twin.
    let reg86 = static_context_for("tcl8.6").commands();
    assert!(!reg86.resolves_only_in_method_context("callback"));
    for name in [
        "link",
        "my",
        "mymethod",
        "next",
        "nextto",
        "self",
        "classvariable",
    ] {
        assert!(
            reg86.resolves_only_in_method_context(name),
            "{name}: 8.6 has it (via ooutil for link/mymethod/classvariable)"
        );
    }
    // 8.5 has no TclOO at all, so the whole query answers `false` there.
    let reg85 = static_context_for("tcl8.5").commands();
    for name in ["link", "my", "next", "nextto", "self", "classvariable"] {
        assert!(
            !reg85.resolves_only_in_method_context(name),
            "{name}: tcl8.5 has no TclOO"
        );
    }
}

/// `link` — and only `link` — declares that it binds bareword aliases for
/// the current object's methods, so the analyser's class-body walk finds
/// those calls through the registry instead of a `texts[0] == "link"`
/// literal (issue #1026).
#[test]
fn tcloo_method_alias_binding_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_BINDS_METHOD_ALIAS);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(carriers, ["link"]);
    assert!(reg.binds_method_alias("link"));
    assert!(reg.binds_method_alias("::link"));
    for negative in ["my", "next", "self", "oo::Helpers::link", "proc"] {
        assert!(
            !reg.binds_method_alias(negative),
            "{negative} binds no method alias"
        );
    }
    // 8.5 has no `link` at all.
    assert!(
        !static_context_for("tcl8.5")
            .commands()
            .binds_method_alias("link")
    );
}

/// **Resolving is not calling.** Every `::oo::Helpers` member needs a real
/// method invocation; `my` does not (Codex review of PR #1084).
///
/// tclsh 9.0.4, inside `oo::class create ::P { initialize { … } }` — a frame
/// with the class object's namespace current and `namespace path` =
/// `::oo::Helpers ::oo`:
///
/// ```text
/// link:          which='::oo::Helpers::link'           call -> link may only be called from inside a method
/// next:          which='::oo::Helpers::next'           call -> next may only be called from inside a method
/// nextto:        which='::oo::Helpers::nextto'         call -> nextto may only be called from inside a method
/// self:          which='::oo::Helpers::self'           call -> self may only be called from inside a method
/// classvariable: which='::oo::Helpers::classvariable'  call -> classvariable may only be called from inside a method
/// callback:      which='::oo::Helpers::callback'       call -> callback may only be called from inside a method
/// mymethod:      which='::oo::Helpers::mymethod'       call -> mymethod may only be called from inside a method
/// my:            which='::oo::Obj20::my'               call -> OK  (`my new` returns ::oo::Obj22)
/// ```
///
/// Everything here still carries `TCLOO_METHOD_CONTEXT` — outside any object
/// frame there is no such command at all — so this set is a strict subset.
#[test]
fn tcloo_method_frame_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_REQUIRES_METHOD_FRAME);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(
        carriers,
        [
            "callback",
            "classvariable",
            "link",
            "mymethod",
            "next",
            "nextto",
            "self"
        ],
        "`my` is `::oo::ObjN::my`, callable from any object frame",
    );
    for name in carriers {
        assert!(reg.requires_oo_method_frame(name), "{name}");
        assert!(
            reg.resolves_only_in_method_context(name),
            "{name}: the method-frame set is a subset of the scoped set",
        );
    }
    assert!(reg.resolves_only_in_method_context("my"));
    assert!(
        !reg.requires_oo_method_frame("my"),
        "`my new` in a class `initialize` body really does make an instance",
    );
    for qualified in ["oo::Helpers::link", "::oo::Helpers::self"] {
        assert!(
            !reg.requires_oo_method_frame(qualified),
            "{qualified} is an ordinary global command",
        );
    }
    assert!(!reg.requires_oo_method_frame("puts"));
    // 8.6 core has the three it ships plus `ooutil`'s `link` and
    // `classvariable`; `callback` is 9.0-only (no Tcllib route). 8.5 has no
    // TclOO at all.
    let reg86 = static_context_for("tcl8.6").commands();
    assert!(!reg86.requires_oo_method_frame("callback"));
    assert!(!reg86.requires_oo_method_frame("my"));
    for name in ["link", "next", "nextto", "self", "classvariable"] {
        assert!(reg86.requires_oo_method_frame(name), "{name} under 8.6");
    }
    let reg85 = static_context_for("tcl8.5").commands();
    for name in ["link", "my", "next", "nextto", "self", "classvariable"] {
        assert!(!reg85.requires_oo_method_frame(name), "{name}: no TclOO");
    }
}

/// The **qualified** `oo::Helpers::…` spelling must be gated exactly like its
/// bare twin per dialect — same dialect availability, same package
/// requirement — because they are the same underlying command (Codex review
/// of PR #1084).
///
/// The regression this pins: `oo::Helpers::link` was derived only from the
/// 9.0 core spec, so an 8.6 document with `package require ooutil` — where
/// Tcllib installs a real `::oo::Helpers::link` — found the qualified call
/// unknown.
#[test]
fn qualified_oo_helpers_spellings_track_their_bare_twin_per_dialect() {
    for dialect in ["tcl8.5", "tcl8.6", "tcl8.7", "tcl9.0"] {
        let reg = static_context_for(dialect).commands();
        let mask = reg
            .profile()
            .expect("built for a dialect")
            .surface_query();
        for bare in ["link", "next", "nextto", "self", "classvariable"] {
            let qualified = format!("oo::Helpers::{bare}");
            let bare_spec = reg.get_for_surface(bare, mask);
            let qual_spec = reg.get_for_surface(&qualified, mask);
            assert_eq!(
                bare_spec.is_some(),
                qual_spec.is_some(),
                "{dialect}: {bare} and {qualified} must agree on existence",
            );
            if let (Some(b), Some(q)) = (bare_spec, qual_spec) {
                assert_eq!(b.surface, q.surface, "{dialect}: {qualified} dialects");
                assert_eq!(
                    b.required_package, q.required_package,
                    "{dialect}: {qualified} required_package",
                );
            }
        }
    }
}

/// The consumer-visible keyword set equals the trait-carrying specs across
/// every dialect a `TclOO` consumer may run under, including the F5 ones.
///
/// The guard for the #1050 migration: `tcl-lsp-core`'s references,
/// call-hierarchy, definition, hover, rename, and semantic-token providers,
/// plus the compiler's `var_command` / `elimination` / `class_lattice` /
/// `var_observability` passes, all resolve these words through
/// `method_dispatch_keyword`. If a consumer ever re-adds a literal name list
/// it will disagree with this set the moment a dialect changes — which is
/// exactly the drift this asserts cannot happen silently.
#[test]
fn tcloo_dispatch_keyword_set_matches_the_trait_carriers_per_dialect() {
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules", "f5-iapps", "expect"] {
        let reg = static_context_for(dialect).commands();
        let mut carriers: Vec<&str> = reg
            .commands_with_trait(Traits::TCLOO_SELF_DISPATCH)
            .into_iter()
            .chain(reg.commands_with_trait(Traits::TCLOO_NEXT_CHAIN))
            .chain(reg.commands_with_trait(Traits::TCLOO_INTROSPECTION))
            .filter(|name| reg.method_dispatch_keyword(name).is_some())
            .collect();
        carriers.sort_unstable();
        let mut answered: Vec<&str> = ["my", "next", "nextto", "self", "link", "puts", "proc"]
            .into_iter()
            .filter(|name| reg.method_dispatch_keyword(name).is_some())
            .collect();
        answered.sort_unstable();
        assert_eq!(
            carriers, answered,
            "under {dialect} the answered keyword set must be exactly the trait carriers"
        );
    }
}

/// All four keywords are `TCL86_PLUS`, so a pre-8.6 dialect registry answers
/// `None` — dialect-awareness comes from the registry instance, never from a
/// per-consumer version check.
///
/// tclsh: `TclOO` shipped with 8.6; `my` / `next` / `self` do not exist in
/// 8.4 or 8.5.
#[test]
fn tcloo_dispatch_keywords_are_absent_before_86() {
    for dialect in ["tcl8.4", "tcl8.5"] {
        let reg = static_context_for(dialect).commands();
        for keyword in ["my", "next", "nextto", "self"] {
            assert_eq!(
                reg.method_dispatch_keyword(keyword),
                None,
                "{keyword} must not resolve under {dialect}"
            );
        }
    }
    // …and still resolves under 9.0.
    let reg = static_context_for("tcl9.0").commands();
    assert_eq!(
        reg.method_dispatch_keyword("my"),
        Some(MethodDispatchKind::SelfDispatch)
    );
}

/// `BRANCH_SELECTED_BODY` is exactly `if` and `try` — the commands whose
/// bodies are chosen at run time without iterating, so nothing established
/// inside one dominates the code after it.
///
/// Deliberately narrower than `CONTROL_FLOW`, which also covers the loops.
/// A loop body is skippable *and* repeatable, a different question with a
/// different consumer (`control_flow_body_depth` versus
/// `conditional_depth`) — the two must not be merged.
///
/// registry-metadata: "which bodies are branch-selected" is our
/// classification. (`if`, `try`, `while`, `for`, `foreach`, and `switch` are
/// all real tclsh 8.6/9.0 commands.)
#[test]
fn branch_selected_body_trait_membership() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let mut carriers = reg.commands_with_trait(Traits::BRANCH_SELECTED_BODY);
    carriers.sort_unstable();
    assert_eq!(carriers, ["if", "try"]);
    for negative in ["while", "for", "foreach", "switch", "lmap", "catch"] {
        assert!(
            !reg.get_for_surface(negative, ds)
                .expect("registered")
                .traits
                .contains(Traits::BRANCH_SELECTED_BODY),
            "{negative} iterates or is otherwise not branch-selected"
        );
    }
    // …and every carrier is control flow, so the narrower trait is a strict
    // subset rather than an orthogonal axis.
    for carrier in carriers {
        assert!(
            reg.get_for_surface(carrier, ds)
                .expect("registered")
                .traits
                .contains(Traits::CONTROL_FLOW),
            "{carrier} must also be control flow"
        );
    }
}

/// The `Tcl_ConcatObj` eval family: every command whose trailing words after
/// the first `ArgRole::Body` argument are part of the script it evaluates.
///
/// tclsh8.6.14 / tclsh9.0.4: `eval set l2 hello` sets `l2` to `hello`;
/// `namespace eval ::n set l2 hello` sets `::n::l2`; `interp eval i {set y} 7`
/// sets `y` to `7` in the child.  `catch` and `apply` take a single bounded
/// script/lambda argument (`apply {{} {set q 1}} extra` errors "wrong # args"),
/// so both stay out.
#[test]
fn script_concat_family_membership() {
    let (reg, ds) = reg_and_set("tcl8.6");
    for name in ["eval", "uplevel"] {
        assert!(
            reg.get_for_surface(name, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{name} concatenates its script words"
        );
    }
    let ns = reg.get_for_surface("namespace", ds).expect("namespace");
    for sub in ["eval", "inscope"] {
        assert!(
            ns.subcommand(sub)
                .expect("modelled")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "namespace {sub} concatenates its script words"
        );
    }
    assert!(
        reg.get_for_surface("interp", ds)
            .expect("interp")
            .subcommand("eval")
            .expect("interp eval modelled")
            .traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
    );
    for negative in ["catch", "apply", "subst", "if"] {
        assert!(
            !reg.get_for_surface(negative, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_CONCATENATES_ARGS),
            "{negative} takes a bounded script argument, not a concatenated tail"
        );
    }
}

/// `namespace inscope` is the one family member whose trailing words are
/// appended as **list elements** rather than space-joined, so it carries the
/// refinement trait and nothing else does.
///
/// tclsh8.6.14 / tclsh9.0.4: `namespace inscope :: {puts} {a b}` prints
/// `a b` (one argument), while `namespace eval :: {puts} {a b}` errors
/// "can not find channel named "a"".
#[test]
fn script_append_list_refinement_is_inscope_only() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let ns = reg.get_for_surface("namespace", ds).expect("namespace");
    let refined = reg.subcommands_with_trait("namespace", Traits::SCRIPT_APPENDS_LIST_ARGS);
    assert_eq!(refined, ["inscope"]);
    assert!(
        !ns.subcommand("eval")
            .expect("modelled")
            .traits
            .contains(Traits::SCRIPT_APPENDS_LIST_ARGS),
        "namespace eval space-joins its tail"
    );
    for name in ["eval", "uplevel"] {
        assert!(
            !reg.get_for_surface(name, ds)
                .expect("registered")
                .traits
                .contains(Traits::SCRIPT_APPENDS_LIST_ARGS)
        );
    }
}

// ===========================================================================
// issue #923 idx 3/4 — tcllib `textutil::adjust` submodule vs the `textutil`
// umbrella package's flattened re-export aliases.
//
// Ground truth (tclsh 9.0.4 + real tcllib-2.0 `modules/textutil/`, verified
// directly — see the PR's commit message for the exact transcripts):
//   - `package require textutil::adjust` alone creates only the three-segment
//     `::textutil::adjust::adjust` / `::textutil::adjust::indent` /
//     `::textutil::adjust::undent` (plus `readPatterns`/`listPredefined`/
//     `getPredefined`) — no bare `::textutil::adjust` command exists.
//   - `package require textutil` (the umbrella) additionally creates the
//     flattened `::textutil::adjust` / `::textutil::indent` / `::textutil::undent`
//     aliases via `namespace import -force adjust::adjust adjust::indent
//     adjust::undent` — real and callable, but ONLY after requiring the
//     umbrella, never after requiring the submodule alone.
// The same shape recurs in `textutil::trim` (trim/trimleft/trimright) and
// `textutil::tabify` (tabify/untabify/tabify2/untabify2).
// ===========================================================================
mod textutil_submodule_vs_umbrella {
    use super::*;

    /// TP — the real submodule-qualified commands the common `package
    /// require textutil::adjust; namespace import textutil::adjust::*`
    /// idiom (tcllib-consuming corpora like `georgtree_argparse`) actually
    /// resolves to must be registered, gated on the submodule package.
    #[test]
    fn textutil_adjust_submodule_commands_registered() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for (name, pkg) in [
            ("textutil::adjust::adjust", "textutil::adjust"),
            ("textutil::adjust::indent", "textutil::adjust"),
            ("textutil::adjust::undent", "textutil::adjust"),
        ] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} must be registered"));
            assert_eq!(
                spec.required_package,
                Some(pkg),
                "{name} must be gated on the submodule package, not the umbrella"
            );
        }
    }

    /// TP — the umbrella package's flattened re-export aliases are also
    /// registered, distinctly gated on `textutil` (not `textutil::adjust`).
    #[test]
    fn textutil_umbrella_aliases_registered_distinctly() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for name in ["textutil::adjust", "textutil::indent"] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} (umbrella alias) must be registered"));
            assert_eq!(
                spec.required_package,
                Some("textutil"),
                "{name} is the umbrella's re-export, gated on requiring `textutil` itself"
            );
        }
    }

    /// TN — the umbrella-vs-submodule distinction is not collapsed: the
    /// 2-segment alias and the 3-segment submodule command are two distinct
    /// registry entries with two distinct package gates, so requiring
    /// `textutil::adjust` alone must not read as granting the umbrella's
    /// `textutil::adjust` alias name (they carry different `required_package`
    /// values even though one's name is a prefix of the other's).
    #[test]
    fn submodule_and_umbrella_names_are_distinct_keys() {
        let (reg, ds) = reg_and_set("tcl8.6");
        let submodule = reg
            .get_for_surface("textutil::adjust::adjust", ds)
            .expect("submodule command registered");
        let umbrella = reg
            .get_for_surface("textutil::adjust", ds)
            .expect("umbrella alias registered");
        assert_ne!(submodule.required_package, umbrella.required_package);
        assert_eq!(umbrella.required_package, Some("textutil"));
        assert_eq!(submodule.required_package, Some("textutil::adjust"));
    }

    /// TN — a name shaped like the wrong guess this finding started from
    /// (`textutil::indent` treated as if it were its own *package*, i.e. a
    /// `textutil::indent::indent` submodule path) does not exist: `indent`
    /// lives inside the `textutil::adjust` package only, and there never was
    /// a separate `textutil::indent` package in real tcllib.
    #[test]
    fn no_fabricated_textutil_indent_package_submodule() {
        let (reg, ds) = reg_and_set("tcl8.6");
        assert!(
            reg.get_for_surface("textutil::indent::indent", ds)
                .is_none()
        );
    }

    /// TP/TN sibling sweep (verified against tclsh 9.0.4 the same way):
    /// `textutil::trim` and `textutil::tabify` show the exact same
    /// meta-package-flattening shape — the submodule-qualified commands must
    /// be registered too, distinctly from their umbrella aliases.
    #[test]
    fn sibling_submodules_trim_and_tabify_also_registered() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for name in [
            "textutil::trim::trim",
            "textutil::trim::trimleft",
            "textutil::trim::trimright",
            "textutil::tabify::tabify",
            "textutil::tabify::untabify",
            "textutil::tabify::tabify2",
            "textutil::tabify::untabify2",
            "textutil::string::longestCommonPrefix",
        ] {
            assert!(
                reg.get_for_surface(name, ds).is_some(),
                "{name} must be registered (tclsh 9.0.4-verified real tcllib command)"
            );
        }
        // Their umbrella aliases remain separately registered too.
        for name in [
            "textutil::trim",
            "textutil::trimleft",
            "textutil::trimright",
            "textutil::tabify",
            "textutil::untabify",
            "textutil::tabify2",
            "textutil::untabify2",
        ] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} (umbrella alias) must be registered"));
            assert_eq!(spec.required_package, Some("textutil"));
        }
    }

    /// Contract test (code-review follow-up on this PR): the umbrella alias
    /// and the submodule-qualified canonical name are the *same real tcllib
    /// command* under two different call spellings, so every trait
    /// descriptor (`Traits::PURE` and any future one) must agree between
    /// them — a consumer doing purity-based GVN/DCE must not see a different
    /// answer purely because of which spelling a call site happens to use.
    /// `rust/tcl-registry/src/commands/tcllib/misc_ext.rs`'s
    /// `sync_textutil_submodule_traits` is what keeps this true (it derives
    /// the submodule spec's traits from the umbrella alias file rather than
    /// declaring them a second time); this test is the general guard that
    /// catches the *next* drift — a newly added re-exported command whose
    /// submodule row is never wired into that sync list — not just today's.
    #[test]
    fn umbrella_alias_and_submodule_canonical_name_agree_on_traits() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for (umbrella, submodule) in [
            ("textutil::adjust", "textutil::adjust::adjust"),
            ("textutil::indent", "textutil::adjust::indent"),
            ("textutil::undent", "textutil::adjust::undent"),
            ("textutil::trim", "textutil::trim::trim"),
            ("textutil::trimleft", "textutil::trim::trimleft"),
            ("textutil::trimright", "textutil::trim::trimright"),
            ("textutil::tabify", "textutil::tabify::tabify"),
            ("textutil::untabify", "textutil::tabify::untabify"),
            ("textutil::tabify2", "textutil::tabify::tabify2"),
            ("textutil::untabify2", "textutil::tabify::untabify2"),
        ] {
            let umbrella_spec = reg
                .get_for_surface(umbrella, ds)
                .unwrap_or_else(|| panic!("{umbrella} (umbrella alias) must be registered"));
            let submodule_spec = reg.get_for_surface(submodule, ds).unwrap_or_else(|| {
                panic!("{submodule} (submodule canonical name) must be registered")
            });
            assert_eq!(
                umbrella_spec.traits, submodule_spec.traits,
                "{umbrella} and {submodule} are the same real tcllib command \
                 and must carry identical traits (got {:?} vs {:?})",
                umbrella_spec.traits, submodule_spec.traits
            );
        }
        // The `textutil::string` package has no bare `longestCommonPrefix`
        // 2-segment umbrella alias file of its own name (the umbrella only
        // re-exports it as `textutil::longestCommonPrefix`), covered above
        // by `textutil_umbrella_aliases_registered_distinctly`'s sibling
        // tests; check it here too for the same trait-parity property.
        let umbrella_spec = reg
            .get_for_surface("textutil::longestCommonPrefix", ds)
            .expect("textutil::longestCommonPrefix (umbrella alias) must be registered");
        let submodule_spec = reg
            .get_for_surface("textutil::string::longestCommonPrefix", ds)
            .expect("textutil::string::longestCommonPrefix must be registered");
        assert_eq!(umbrella_spec.traits, submodule_spec.traits);
    }
}

/// `namespace`'s namespace-name argument positions — the registry data issue
/// #1088's navigation is driven by.  Which words name a namespace is a
/// registry fact, so no analyser or provider matches a subcommand spelling.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, byte-identical): `namespace children
/// ::never` fails `namespace "::never" not found`; `namespace exists ::never`
/// answers `0`; `namespace delete ::a ::b` deletes both and errors on the
/// first unknown one; `namespace upvar ::rel::kid v alias` binds through the
/// namespace word; and inside `namespace eval ::outer`, `namespace exists
/// inner` is `1` while the same words at global scope are `0`.
///
/// registry-metadata: `arg_roles` / `arg_role_resolver`.
#[test]
fn namespace_subcommands_declare_their_namespace_name_arguments() {
    let (reg, _) = reg_and_set("tcl8.6");
    for (args, expected) in [
        (vec!["children", "::a"], vec![1usize]),
        (vec!["children", "::a", "pat*"], vec![1]),
        (vec!["exists", "::a"], vec![1]),
        (vec!["parent", "::a"], vec![1]),
        (vec!["eval", "::a", "{}"], vec![1]),
        (vec!["inscope", "::a", "{}"], vec![1]),
        (vec!["upvar", "::a", "v", "l"], vec![1]),
        (vec!["delete", "::a", "::b"], vec![1, 2]),
    ] {
        assert_eq!(
            reg.arg_indices_for_role("namespace", &args, ArgRole::NamespaceName),
            expected,
            "namespace {args:?} namespace-name positions",
        );
    }
    // `children`'s second word filters the *result*; it is a glob pattern.
    assert_eq!(
        reg.arg_indices_for_role("namespace", &["children", "::a", "pat*"], ArgRole::Pattern),
        vec![2],
    );
}

/// The `namespace` subcommands whose arguments only *look* like namespaces
/// must declare no namespace-name position at all.
///
/// Oracle (both interpreters): `namespace tail a::b` → `b` and `namespace
/// qualifiers a::b` → `a` whether or not `::a` exists, so those words are
/// arbitrary strings; `import`/`export`/`forget` take glob patterns;
/// `origin`/`which` name commands; `code` takes a script.
///
/// registry-metadata: `arg_roles`.
#[test]
fn namespace_string_and_command_subcommands_declare_no_namespace_name() {
    let (reg, _) = reg_and_set("tcl8.6");
    for args in [
        vec!["tail", "a::b"],
        vec!["qualifiers", "a::b"],
        vec!["import", "::a::*"],
        vec!["export", "p"],
        vec!["forget", "::a::*"],
        vec!["origin", "::a::p"],
        vec!["which", "-command", "::a::p"],
        vec!["code", "{puts hi}"],
        vec!["current"],
        vec!["path", "{::a ::b}"],
    ] {
        assert!(
            reg.arg_indices_for_role("namespace", &args, ArgRole::NamespaceName)
                .is_empty(),
            "namespace {args:?} must declare no namespace-name argument",
        );
    }
}

/// Only `namespace eval` **declares** a namespace, and the fact is a trait
/// rather than a spelling check — `namespace inscope`'s argument layout and
/// analyser hook are identical but it requires the namespace to exist.
///
/// Oracle (both interpreters): `namespace eval ::brandnew {}` creates it;
/// `namespace inscope ::brandnew {}` on an undeclared namespace fails; and no
/// other form creates one (`proc ::nope::gone::p {} {}` → `can't create
/// procedure "::nope::gone::p": unknown namespace`).
///
/// registry-metadata: `Traits::DECLARES_NAMESPACE`.
#[test]
fn only_namespace_eval_declares_a_namespace() {
    let (reg, _) = reg_and_set("tcl8.6");
    assert!(
        reg.call_traits("namespace", &["eval", "::a", "{}"])
            .contains(Traits::DECLARES_NAMESPACE),
    );
    for sub in ["inscope", "children", "exists", "delete", "parent", "upvar"] {
        assert!(
            !reg.call_traits("namespace", &[sub, "::a"])
                .contains(Traits::DECLARES_NAMESPACE),
            "namespace {sub} must not be a declaring form",
        );
    }
    // No other command in any dialect claims it either — a declaring form the
    // navigation tier does not know about would silently lose definitions.
    let declarers: Vec<&str> = reg
        .command_names()
        .filter(|name| {
            reg.get(name).is_some_and(|spec| {
                spec.traits.contains(Traits::DECLARES_NAMESPACE)
                    || spec
                        .subcommands
                        .iter()
                        .any(|sub| sub.traits.contains(Traits::DECLARES_NAMESPACE))
            })
        })
        .collect();
    assert_eq!(declarers, vec!["namespace"]);
}

// ---------------------------------------------------------------------------
// Object-handle bindings (issue #1185) — which argument of a call becomes a
// handle, and of what class, is registry data rather than a walker's
// command-name match.
// ---------------------------------------------------------------------------

/// `set` declares the handle-binding layout its value word implies, and the
/// query resolves it through the same `get` path every other registry query
/// uses — so `::set` answers identically to the bare spelling.
#[test]
fn set_declares_its_handle_binding_for_every_spelling() {
    use tcl_registry::{BoundHandle, HandleClassSource};

    let reg = CommandRegistry::build_default();
    for spelling in ["set", "::set"] {
        let layout = reg
            .handle_binding(spelling)
            .unwrap_or_else(|| panic!("`{spelling}` must declare a handle binding"));
        assert_eq!(
            layout.resolve(&["axis", "[verticalAxis $win.a]"]),
            Some(BoundHandle {
                name: "axis",
                class_word: "[verticalAxis $win.a]",
                class_source: HandleClassSource::ConstructionValue(1),
            }),
            "`{spelling}` must bind argument 0 from its value word"
        );
        // The one-argument read form binds nothing.
        assert_eq!(layout.resolve(&["axis"]), None);
    }
    // A command with no such layout answers `None` — no guessing.
    assert!(reg.handle_binding("puts").is_none());
    assert!(reg.handle_binding("nosuchcommand").is_none());
}

/// snit's `install` is a **member-body** command, not a global one: it must
/// carry its layout on the definition-body grammar and must NOT be registered
/// as a global spec (which would make a user's own `proc install` look like a
/// built-in everywhere).
#[test]
fn snit_install_is_a_member_body_binding_not_a_global_command() {
    use tcl_registry::{BoundHandle, HandleClassSource};

    let reg = CommandRegistry::build_default();
    assert!(
        reg.get("install").is_none(),
        "`install` must not become a global command spec"
    );

    let bindings = reg.member_body_handle_bindings();
    let (_, layout) = bindings
        .iter()
        .find(|(name, _)| *name == "install")
        .unwrap_or_else(|| panic!("snit must declare an `install` handle binding: {bindings:?}"));
    assert_eq!(
        layout.resolve(&["axis", "using", "verticalAxis", "$win.a"]),
        Some(BoundHandle {
            name: "axis",
            class_word: "verticalAxis",
            class_source: HandleClassSource::Word(2),
        })
    );
    // Without the `using` keyword the layout does not fire, so a user's own
    // three-word `install` binds nothing.
    assert_eq!(layout.resolve(&["a", "b", "c"]), None);
    // The snit `type` and `widget` grammars share one command set — the query
    // deduplicates rather than reporting the same word twice.
    assert_eq!(
        bindings.iter().filter(|(n, _)| *n == "install").count(),
        1,
        "member-body bindings must be deduplicated: {bindings:?}"
    );
}

/// Whether a class family constructs from a **bare instance name** is a
/// grammar fact, not a metaclass-name prefix test.
#[test]
fn bare_word_construction_is_declared_per_definer_family() {
    use tcl_registry::definer::{ITCL_GRAMMAR, SNIT_GRAMMAR, SNIT_WIDGET_GRAMMAR, TCLOO_GRAMMAR};

    // snit(n), "The Type Command": `$type name ?args?` implies `create`;
    // tclsh 9.0.4: `oo::class create ::C {}; ::C x` -> `unknown method "x"`,
    // so only the snit family constructs by bare word.
    let families = [
        ("snit::type", &SNIT_GRAMMAR, true),
        ("snit::widget", &SNIT_WIDGET_GRAMMAR, true),
        ("TclOO", &TCLOO_GRAMMAR, false),
        ("itcl::class", &ITCL_GRAMMAR, false),
    ];
    for (name, grammar, expected) in families {
        assert_eq!(
            grammar.bare_word_construction, expected,
            "{name}: bare_word_construction"
        );
        // Only snit injects member-body commands today; the others inject none.
        assert_eq!(
            grammar.member_body_command("install").is_some(),
            expected,
            "{name}: member-body `install`"
        );
        // An unknown word is not a member-body command of any family.
        assert!(grammar.member_body_command("nosuchword").is_none());
    }
}

/// Manufacturer spelling and word layout live together in the grammar. A
/// consumer must not infer the name/body positions from `create`-shaped text:
/// `TclOO`'s three manufacturers have three distinct layouts, while snit's
/// homonymous method creates an ordinary instance and has no definition body.
#[test]
fn manufacturer_layout_is_declared_per_definer_family() {
    use tcl_registry::definer::{SNIT_GRAMMAR, TCLOO_GRAMMAR};

    let create = TCLOO_GRAMMAR.manufacturer("create").unwrap();
    assert_eq!(create.names_instance_at, Some(1));
    assert_eq!(create.definition_body_at, Some(2));
    assert_eq!(create.constructor_args_from, 2);

    let new = TCLOO_GRAMMAR.manufacturer("new").unwrap();
    assert_eq!(new.names_instance_at, None);
    assert_eq!(new.definition_body_at, Some(1));
    assert_eq!(new.constructor_args_from, 1);

    let with_ns = TCLOO_GRAMMAR.manufacturer("createWithNamespace").unwrap();
    assert_eq!(with_ns.names_instance_at, Some(1));
    assert_eq!(with_ns.definition_body_at, Some(3));
    assert_eq!(with_ns.constructor_args_from, 3);

    let snit_create = SNIT_GRAMMAR.manufacturer("create").unwrap();
    assert_eq!(snit_create.names_instance_at, Some(1));
    assert_eq!(snit_create.definition_body_at, None);
    assert_eq!(snit_create.constructor_args_from, 2);

    let registry = CommandRegistry::build_default();
    assert_eq!(
        registry.uniform_manufacturer_names_instance_at("create"),
        Some(1)
    );
    assert_eq!(registry.uniform_manufacturer_names_instance_at("new"), None);
    assert!(registry.is_possible_class_construction_word("new"));
    assert!(registry.is_possible_class_construction_word("%AUTO%"));
    assert!(registry.is_possible_class_construction_word(".widget"));
    assert!(!registry.is_possible_class_construction_word("ordinaryMethod"));
}

#[test]
fn registered_metaclass_manufacturer_surface_matches_c_tcl() {
    let registry = CommandRegistry::build_default();

    assert!(
        registry
            .manufacturer_method("oo::class", "create")
            .is_some()
    );
    assert!(
        registry.manufacturer_method("oo::class", "new").is_some(),
        "C Tcl retains private `new` on oo::class itself"
    );
    assert!(
        registry
            .manufacturer_method("oo::class", "createWithNamespace")
            .is_some(),
        "C Tcl retains private createWithNamespace on oo::class itself"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::class", "new")
            .is_none(),
        "ordinary dispatch cannot reach root `new`"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::class", "createWithNamespace")
            .is_none(),
        "ordinary dispatch cannot reach root createWithNamespace"
    );
    assert!(
        registry
            .manufacturer_method("oo::abstract", "new")
            .is_some(),
        "the Tcl 9 derived metaclass commands retain their inherited `new`"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::abstract", "new")
            .is_some(),
        "ordinary dispatch reaches an exported manufacturer"
    );
    assert!(
        registry
            .manufacturer_method("oo::abstract", "createWithNamespace")
            .is_some(),
        "the private method still exists for self-dispatch and export modelling"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::abstract", "createWithNamespace")
            .is_none(),
        "ordinary dispatch must not pretend an unexported manufacturer is callable"
    );
}

/// The built-in **object** methods each definer family supplies, and — the
/// load-bearing half — which dispatch spellings can reach each one.
///
/// oracle: tclsh 9.0.4 and 8.6.16, byte-identical on both:
///
/// ```text
/// info class methods ::oo::object -all -private
///   -> <cloned> destroy eval unknown variable varname
/// info class methods ::oo::class -all -private
///   -> <cloned> create createWithNamespace destroy eval new unknown variable varname
/// ```
///
/// and, from inside a method of a class declaring only `probe`:
///
/// ```text
/// my varname v      -> ::oo::Obj22::v
/// [self] varname v  -> unknown method "varname": must be destroy or probe
/// $obj varname v    -> unknown method "varname": must be destroy or probe
/// ```
///
/// That last pair is the guard for issue #1329: `my` bypasses `TclOO`'s export
/// filter and the object's own command does not, so a consumer diagnosing an
/// unknown method must ask with the right `MethodReach` or it will either
/// false-positive on the idiomatic `my variable v` or wave through the real
/// error `$obj varname v`.
#[test]
fn builtin_object_methods_are_reach_gated() {
    use tcl_registry::definer::{
        BuiltinMethodReceiver, ITCL_GRAMMAR, MethodReach, SNIT_GRAMMAR, TCLOO_GRAMMAR,
    };

    // `destroy` is the one exported member every TclOO object has: both
    // reaches find it.
    for reach in [MethodReach::ObjectCommand, MethodReach::SelfDispatch] {
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method("destroy", reach)
                .is_some(),
            "`destroy` is exported, so every reach finds it"
        );
    }

    // The unexported five are reachable only through `my`.
    for name in ["eval", "unknown", "variable", "varname", "<cloned>"] {
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method(name, MethodReach::SelfDispatch)
                .is_some(),
            "`my {name}` must resolve"
        );
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method(name, MethodReach::ObjectCommand)
                .is_none(),
            "`$obj {name}` / `[self] {name}` is a real error, not a builtin"
        );
    }

    // The class-object constructors are recorded as such, so a consumer with
    // receiver-kind evidence can narrow; one without may accept both.
    for name in ["new", "create"] {
        let m = TCLOO_GRAMMAR
            .builtin_object_method(name, MethodReach::ObjectCommand)
            .unwrap_or_else(|| panic!("`{name}` is an exported class-object method"));
        assert_eq!(m.receiver, BuiltinMethodReceiver::ClassObject, "{name}");
    }
    assert!(
        TCLOO_GRAMMAR
            .builtin_object_method("createWithNamespace", MethodReach::ObjectCommand)
            .is_none(),
        "createWithNamespace is present but unexported in C Tcl"
    );

    // A word no class system supplies answers for nobody.
    for grammar in [&TCLOO_GRAMMAR, &SNIT_GRAMMAR, &ITCL_GRAMMAR] {
        assert!(
            grammar
                .builtin_object_method("nosuchmethod", MethodReach::SelfDispatch)
                .is_none()
        );
    }

    // snit and itcl dispatch through their own generated machinery, which has
    // no `my`-only tier — every builtin of theirs is reachable either way.
    for (what, grammar, names) in [
        (
            "snit",
            &SNIT_GRAMMAR,
            &["configure", "configurelist", "cget", "destroy", "info"][..],
        ),
        (
            "itcl",
            &ITCL_GRAMMAR,
            &["configure", "cget", "isa", "info"][..],
        ),
    ] {
        for name in names {
            for reach in [MethodReach::ObjectCommand, MethodReach::SelfDispatch] {
                assert!(
                    grammar.builtin_object_method(name, reach).is_some(),
                    "{what} instances have `{name}`"
                );
            }
        }
        // `new` is TclOO's constructor spelling and belongs to no other family.
        assert!(
            grammar
                .builtin_object_method("new", MethodReach::ObjectCommand)
                .is_none(),
            "{what} constructs through its own type command, never `new`"
        );
    }
}

#[test]
fn irules_payload_lifecycle_inventory_is_registry_complete() {
    use tcl_registry::DataCollectionAction::{Collect, Payload, Release};

    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let mut actual: Vec<_> = reg
        .command_names()
        .filter_map(|name| {
            reg.data_collection_operation(name)
                .map(|op| (name.to_string(), op.protocol.name, op.action))
        })
        .collect();
    actual.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    let mut expected = vec![
        ("ASM::payload".to_string(), "ASM", Payload),
        ("CACHE::payload".to_string(), "CACHE", Payload),
        ("DIAMETER::payload".to_string(), "DIAMETER", Payload),
        ("GTP::payload".to_string(), "GTP", Payload),
        ("HTTP::collect".to_string(), "HTTP", Collect),
        ("HTTP::payload".to_string(), "HTTP", Payload),
        ("HTTP::release".to_string(), "HTTP", Release),
        ("MQTT::collect".to_string(), "MQTT", Collect),
        ("MQTT::payload".to_string(), "MQTT", Payload),
        ("MQTT::release".to_string(), "MQTT", Release),
        ("MR::collect".to_string(), "MR", Collect),
        ("MR::payload".to_string(), "MR", Payload),
        ("MR::release".to_string(), "MR", Release),
        ("REWRITE::payload".to_string(), "REWRITE", Payload),
        ("RTSP::collect".to_string(), "RTSP", Collect),
        ("RTSP::payload".to_string(), "RTSP", Payload),
        ("RTSP::release".to_string(), "RTSP", Release),
        ("SCTP::collect".to_string(), "SCTP", Collect),
        ("SCTP::payload".to_string(), "SCTP", Payload),
        ("SCTP::release".to_string(), "SCTP", Release),
        ("SIP::payload".to_string(), "SIP", Payload),
        ("SSL::collect".to_string(), "SSL", Collect),
        ("SSL::payload".to_string(), "SSL", Payload),
        ("SSL::release".to_string(), "SSL", Release),
        ("TCP::collect".to_string(), "TCP", Collect),
        ("TCP::payload".to_string(), "TCP", Payload),
        ("TCP::release".to_string(), "TCP", Release),
        ("UDP::payload".to_string(), "UDP", Payload),
        ("WS::collect".to_string(), "WS", Collect),
        ("WS::payload".to_string(), "WS", Payload),
        ("WS::release".to_string(), "WS", Release),
        ("XML::payload".to_string(), "XML", Payload),
    ];
    expected.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(actual, expected);

    // UDP::release ends UDP::hold rather than a payload-collection lifecycle.
    // XML::collect/release are retired legacy XML parser controls; the current
    // XML::payload is event-owned. Neither pair belongs in this inventory.
    assert!(reg.data_collection_operation("UDP::release").is_none());
    assert!(reg.data_collection_operation("XML::collect").is_none());
    assert!(reg.data_collection_operation("XML::release").is_none());
}

#[test]
fn mqtt_payload_forms_declare_event_and_collection_contracts() {
    use tcl_registry::PayloadCollectionRequirement::{ExplicitCollect, NotRequired};

    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let operation = reg
        .data_collection_operation("MQTT::payload")
        .expect("MQTT payload lifecycle descriptor");
    assert_eq!(
        operation.payload_requirement_for_args(&[]),
        Some(ExplicitCollect)
    );
    assert_eq!(
        operation.payload_requirement_for_args(&["append", "bytes"]),
        Some(ExplicitCollect)
    );
    for form in ["length", "replace", "prepend"] {
        assert_eq!(
            operation.payload_requirement_for_args(&[form]),
            Some(NotRequired),
            "{form} is available on the current MQTT PUBLISH message"
        );
    }
    assert_eq!(operation.payload_requirement_for_args(&["$form"]), None);

    let mqtt = reg.get("MQTT::payload").expect("MQTT payload spec");
    let ingress = mqtt.event_requirements_for_args(&["length"]);
    assert!(ingress.matched_form);
    assert!(ingress.only_in.contains(&"MQTT_CLIENT_INGRESS"));
    let collected = mqtt.event_requirements_for_args(&[]);
    assert!(collected.matched_form);
    assert_eq!(collected.only_in, &["MQTT_CLIENT_DATA", "MQTT_SERVER_DATA"]);
    let dynamic = mqtt.event_requirements_for_args(&["$form"]);
    assert!(!dynamic.matched_form);
    assert!(dynamic.only_in.is_empty());

    let diameter = reg
        .data_collection_operation("DIAMETER::payload")
        .expect("DIAMETER payload availability descriptor");
    assert_eq!(
        diameter.payload_requirement_for_args(&[]),
        Some(NotRequired)
    );
    assert!(reg.get("DIAMETER::collect").is_none());
}

/// The registry's `namespace ensemble` option tables carry exactly the names
/// their owning utility does (issue #1610).
///
/// `tcl_cmd_core::ensemble::{CREATE_OPTIONS, CONFIG_OPTIONS}` is the port of
/// `tclEnsemble.c`'s two tables and the owner of that fact; the registry rows
/// exist to add what an editor needs on top (detail, dialect gate, value
/// shape). Membership is the half both must agree on, and a single merged
/// registry table disagreeing with it is precisely the bug this pins: it
/// offered `configure` a `-command` that always errors and hid the
/// `-namespace` it accepts.
#[test]
fn the_ensemble_option_tables_match_their_owning_utility() {
    let registry =
        std::sync::Arc::clone(tcl_registry::model::ingress::static_context_for("tcl").commands());
    let spec = registry
        .get("namespace")
        .expect("`namespace` is a core command");
    let ensemble = spec
        .resolve_subcommand("ensemble")
        .expect("`namespace ensemble` is a subcommand");

    let declared = |op: &str| -> Vec<&'static str> {
        ensemble
            .resolve_sub_subcommand(op)
            .unwrap_or_else(|| panic!("`namespace ensemble {op}` is a second-level subcommand"))
            .options
            .unwrap_or_default()
            .iter()
            .map(|o| o.name)
            .collect()
    };

    assert_eq!(
        declared("create"),
        tcl_cmd_core::ensemble::CREATE_OPTIONS.names().to_vec(),
        "`create`'s registry table must match `ensembleCreateOptions`"
    );
    assert_eq!(
        declared("configure"),
        tcl_cmd_core::ensemble::CONFIG_OPTIONS.names().to_vec(),
        "`configure`'s registry table must match `ensembleConfigOptions`"
    );

    // The union the subcommand keeps for the abstain path is the two tables
    // merged and nothing else — it must not gain an option neither has.
    let mut union: Vec<&str> = declared("create");
    union.extend(declared("configure"));
    union.sort_unstable();
    union.dedup();
    let mut carried: Vec<&str> = ensemble.options.iter().map(|o| o.name).collect();
    carried.sort_unstable();
    assert_eq!(carried, union);
}

/// An operation's option table is three-valued, and `exists` needs the third
/// state (issue #1610, Codex review).
///
/// `Some(&[])` says "no options here" and must not fall back to the parent;
/// `None` says nothing and must. Collapsing the two is what offered
/// `namespace ensemble exists` the whole union.
#[test]
fn an_explicitly_empty_operation_table_does_not_inherit_the_parents() {
    let registry =
        std::sync::Arc::clone(tcl_registry::model::ingress::static_context_for("tcl").commands());
    let spec = registry
        .get("namespace")
        .expect("`namespace` is a core command");
    let ensemble = spec
        .resolve_subcommand("ensemble")
        .expect("`namespace ensemble` is a subcommand");

    let exists = ensemble
        .resolve_sub_subcommand("exists")
        .expect("`namespace ensemble exists`");
    assert_eq!(
        exists.options.map(<[_]>::len),
        Some(0),
        "`exists` declares an empty table, not an absent one"
    );

    // …and the selector honours it: an empty scope, not the parent's union.
    let scope = ensemble.option_scope(Some("exists"), None, None, spec.surface);
    assert!(
        scope.options.is_empty(),
        "`exists` must resolve to no options: {:?}",
        scope.options.iter().map(|o| o.name).collect::<Vec<_>>()
    );
    assert_eq!(scope.sub_subcommand, Some("exists"));

    // The other second-level ensembles in the registry declare nothing, so
    // they still inherit — the default must stay "inherit", or every one of
    // them silently loses its parent's options.
    let info = registry.get("info").expect("`info` is a core command");
    let object = info
        .resolve_subcommand("object")
        .expect("`info object` is a subcommand");
    for op in object.sub_subcommands {
        assert!(
            op.options.is_none(),
            "`info object {}` declares nothing about options",
            op.name
        );
    }
}
