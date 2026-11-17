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

//! What a spec declares: switches, options, hover sources, forms, closed value arguments, body kinds, namespace-name arguments and ensemble option tables.

use super::*;

// Registry structure

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
    let switches = socket.switch_names(ds);
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

// Form kind and form resolution

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

/// `HTTP::uri` with no args is a reusable getter; with one arg it is a mutating
/// setter. `resolve_call` picks the matching form by arity.
///
/// f5-dialect: `HTTP::uri` is not real Tcl. The getter→read / setter→write
/// classification is registry side-effect metadata.
#[test]
fn http_uri_getter_setter_forms() {
    let (reg, ds) = reg_and_set("f5-irules");
    // No-arg form resolves (getter): a repeat in the same event reads the
    // same URI, so the getter is a CSE candidate.
    let getter = reg.resolve_call("HTTP::uri", &[], ds).expect("getter form");
    assert!(
        getter.spec.traits.contains(Traits::CSE_CANDIDATE),
        "getter is reusable"
    );
    // Single-arg form resolves (setter) under the same command.
    let setter = reg
        .resolve_call("HTTP::uri", &["/new/path"], ds)
        .expect("setter form");
    assert_eq!(setter.spec.name, "HTTP::uri");
}

// Subcommand form resolution

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

// Closed value arguments

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
    assert_eq!(spec.closed_value_args.len(), 0);
}

// `when` argument values cover the event corpus

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

// CFG-rewrite alias resolution (`cfg_rewrite_name`)

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

// Body kind / namespace export

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
/// of their own.
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

/// `namespace`'s namespace-name argument positions — the registry data
/// navigation is driven by.  Which words name a namespace is a
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

/// The registry's `namespace ensemble` option tables carry exactly the names
/// their owning utility does.
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
/// state.
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

    // Methods and Tcl 9.0 properties own option tables. The other operations
    // declare nothing and inherit; absent and explicitly empty stay distinct.
    let info = registry.get("info").expect("`info` is a core command");
    let object = info
        .resolve_subcommand("object")
        .expect("`info object` is a subcommand");
    let properties = object
        .resolve_sub_subcommand("properties")
        .expect("`info object properties`");
    assert_eq!(
        properties
            .options
            .expect("`properties` declares its own table")
            .iter()
            .map(|option| option.name)
            .collect::<Vec<_>>(),
        ["-all", "-readable", "-writable"]
    );
    let methods = object
        .resolve_sub_subcommand("methods")
        .expect("`info object methods`");
    let methods_options = methods.options.expect("`methods` declares its own table");
    assert_eq!(
        methods_options
            .iter()
            .map(|option| option.name)
            .collect::<Vec<_>>(),
        ["-all", "-localprivate", "-private", "-scope"]
    );
    // Tcl 8.6.18 accepts three method flags; Tcl 9.0.4 and 9.1.0 add -scope.
    for (release, expected) in [
        ("8.6", &["-all", "-localprivate", "-private"][..]),
        ("9.0", &["-all", "-localprivate", "-private", "-scope"][..]),
        ("9.1", &["-all", "-localprivate", "-private", "-scope"][..]),
    ] {
        let dialect = Some(SurfaceQuery::core(Family::Tcl, release));
        let scope = object.option_scope(Some("methods"), dialect, None, info.surface);
        assert_eq!(scope.sub_subcommand, Some("methods"));
        assert!(std::ptr::eq(scope.options, methods_options));
        assert_eq!(
            scope
                .options
                .iter()
                .filter(|option| option.supports_dialect(dialect, scope.surface))
                .map(|option| option.name)
                .collect::<Vec<_>>(),
            expected,
            "info object methods options in Tcl {release}",
        );
    }
    for op in object
        .sub_subcommands
        .iter()
        .filter(|op| !matches!(op.name, "properties" | "methods"))
    {
        assert!(
            op.options.is_none(),
            "`info object {}` declares nothing about options",
            op.name
        );
    }
}
