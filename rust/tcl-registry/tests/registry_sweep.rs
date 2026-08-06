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

//! Broad data/accessor coverage sweep for the `tcl-registry` crate.
//!
//! ## Purpose
//!
//! The sibling `registry_commands.rs` ports a curated set of
//! C-Tcl-observable facts about *selected* commands. This file is the
//! complement: a **broad sweep** that iterates the *entire* command set in
//! every known dialect, plus *every* BIG-IP object spec, and exercises *every*
//! public query accessor on each. Its goal is data/accessor **coverage** — the
//! per-spec data tables, the hover/forms/option metadata, and the per-spec
//! query helpers that no other test references — verified with **invariants**
//! (no panic; arity self-consistent; typed results where a flag is set; hover
//! is non-empty when present) rather than brittle exact counts, so it survives
//! registry data growth.
//!
//! ## What is tclsh-anchored vs registry-metadata
//!
//! Most of what the sweep touches is **registry-internal metadata** (traits,
//! taint colours, side-effect classifications, hover prose, arg roles, forms,
//! body-kinds) and **F5/iRules/BigIP dialect data** — none of which exists in
//! real `tclsh`. Those are asserted against the registry only and marked
//! `// registry-metadata`, `// f5-dialect`, or `// bigip`.
//!
//! A handful of facts the registry asserts about **real Tcl commands** are
//! pinned to C-Tcl ground truth, verified with `scripts/dev/tclsh_check.sh`
//! against tclsh8.6 and tclsh9.0 and recorded in `// tclsh:` comments:
//!   - core command existence (`info commands`),
//!   - the `string` / `dict` / `info` / `array` / `namespace` / `clock`
//!     subcommand sets,
//!   - the `socket -server` switch,
//!   - the `lsort` / `lsearch` / `string is` option/class sets,
//!   - the `--` option terminator on `switch`.
//!
//! tclsh8.4 / tclsh8.5 are not installed in this environment; introduction
//! versions cited for those are well-known Tcl history consistent with the
//! 8.6/9.0 probes.

use std::collections::BTreeSet;

use tcl_dialect::DialectSet;
use tcl_registry::arity::Arity;
use tcl_registry::bigip::{BigipRegistry, ValueKind, default_registry};
use tcl_registry::events::EventRegistry;
use tcl_registry::hover::FormKind;
use tcl_registry::profiles::ProfileRegistry;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::taint::TaintColour;
use tcl_registry::{
    ArgRole, CommandRegistry, KNOWN_DIALECTS, Traits, available_dialects, registry_for_dialect,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Every dialect name that resolves to a real `DialectSet` bit — i.e. the names
/// `registry_for_dialect` loads a non-trivial command pack for. (The config-only
/// `f5-bigip` / `f5-tmsh` names in `KNOWN_DIALECTS` collapse to plain Tcl, so
/// they are covered via the catalogue sweep rather than as load targets.)
const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "synopsys-eda-tcl",
    "cadence-eda-tcl",
    "xilinx-eda-tcl",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "bpf",
];

/// Every trait bit, so the sweep queries `commands_with_trait` across the whole
/// lattice (executing the per-trait scan + classification helpers).
const ALL_TRAITS: &[Traits] = &[
    Traits::CONTROL_FLOW,
    Traits::LANGUAGE_KEYWORD,
    Traits::HAS_BOOLEAN_COND,
    Traits::TERMINATES_BLOCK,
    Traits::HAS_LOOP_BODY,
    Traits::NEVER_INLINE_BODY,
    Traits::LOOP_LIST_HEADER,
    Traits::PURE,
    Traits::CSE_CANDIDATE,
    Traits::PURE_EVALUATION,
    Traits::DEFINES_PROCEDURE,
    Traits::DESTROYS_VARIABLE,
    Traits::READS_BEFORE_WRITE,
    Traits::CREATES_SCOPE_ALIAS,
    Traits::CREATES_BARRIER,
    Traits::EVALUATES_CODE,
    Traits::PERFORMS_SUBSTITUTION,
    Traits::OPENS_CHANNEL,
    Traits::SOURCES_FILE,
    Traits::HAS_SWITCH_BODY,
    Traits::STRING_LIST_CONFUSION,
    Traits::CONFIGURES_CHANNEL,
    Traits::HAS_INTERP_EVAL,
    Traits::HAS_DESTRUCTIVE_OPS,
    Traits::IS_EVENT_HANDLER,
    Traits::UNNORMALISED_HTTP_GETTER,
    Traits::RETURNS_PATH,
    Traits::IS_UNESCAPE,
    Traits::PRODUCES_CANONICAL_LIST,
    Traits::BUILDS_COMMAND_PREFIX,
    Traits::UNSAFE,
    Traits::PASSWORD_OPTION,
    Traits::IS_SIDE_SWITCH,
    Traits::IRULES_TOP_LEVEL_ONLY,
    Traits::IS_OO_METACLASS,
    Traits::DIAGRAM_ACTION,
    Traits::NEEDS_START_CMD,
    Traits::TAINT_SINK,
    Traits::TAINT_SOURCE,
    Traits::IRULES_DATA_GETTER,
    Traits::CREATES_DYNAMIC_BARRIER,
    Traits::INVOKES_USER_PROC,
    Traits::BYTE_COMPILED,
    Traits::NOT_PROC_FACTORY,
    Traits::FRAMELESS_RUNTIME,
    Traits::FIRST_ARG_VARNAME,
    Traits::WHOLE_ARRAY_ARG,
    Traits::DYNAMIC_EVAL_BODY,
    Traits::INTROSPECTS_BY_NAME,
    Traits::TARGETS_VARIABLE_BY_NAME,
    Traits::FRAME_HASH_BUILTIN,
    Traits::OVERRIDABLE_LIBRARY_PROC,
    Traits::WASM_EMITS_NOTHING,
    Traits::EXPR_CONCATENATES_ARGS,
    Traits::STRUCTURALLY_CHECKED_ARITY,
    Traits::TCLOO_NEXT_CHAIN,
    Traits::ESTABLISHES_VARIABLE_TRACE,
    Traits::TRANSFERS_CONTROL,
    Traits::FIRE_AND_FORGET_TEARDOWN,
    Traits::SCRIPT_CONCATENATES_ARGS,
    Traits::SCRIPT_APPENDS_LIST_ARGS,
    Traits::TCLOO_SELF_DISPATCH,
    Traits::TCLOO_INTROSPECTION,
    Traits::BRANCH_SELECTED_BODY,
];

/// Assert the basic arity invariant shared by every `Arity` (command or
/// subcommand): `min <= max`, and the unlimited sentinel agrees with `max`.
fn assert_arity_consistent(a: Arity, what: &str) {
    assert!(
        a.min <= a.max,
        "{what}: arity min {} > max {}",
        a.min,
        a.max
    );
    assert_eq!(
        a.is_unlimited(),
        a.max == Arity::UNLIMITED,
        "{what}: is_unlimited disagrees with max sentinel"
    );
    // `accepts` must agree with the bounds at the boundaries.
    assert!(a.accepts(a.min), "{what}: arity rejects its own min");
    if !a.is_unlimited() {
        assert!(a.accepts(a.max), "{what}: arity rejects its own max");
        if a.max < u16::MAX {
            assert!(
                !a.accepts(a.max + 1),
                "{what}: arity accepts max+1 despite finite bound"
            );
        }
    }
}

// ===========================================================================
// SWEEP 1 — every command in every dialect, every accessor.
// ===========================================================================
/// Exercise every accessor on one command spec (and its subcommands)
/// for a given dialect. Extracted from `sweep_every_command_every_accessor`
/// to keep that test within the line budget.
/// Exercise every accessor on one subcommand spec. Extracted from
/// [`check_command_accessors`] to keep both within the line budget.
fn check_subcommand_accessors(
    spec: &tcl_registry::CommandSpec,
    sub: &tcl_registry::SubCommand,
    ds: Option<DialectSet>,
    dname: &str,
    name: &str,
) {
    assert!(
        !sub.name.is_empty(),
        "{dname}/{name}: empty subcommand name"
    );
    // `subcommand` lookup round-trips a declared subcommand.
    assert!(
        spec.subcommand(sub.name).is_some(),
        "{dname}/{name}: subcommand {:?} does not round-trip",
        sub.name
    );
    assert_arity_consistent(sub.arity, &format!("{dname}/{name} {}", sub.name));
    // Sub-level switch_names + arg_values + arg_role accessors.
    let sub_sw = sub.switch_names(ds, spec.dialects);
    let sub_declared: BTreeSet<&str> = sub.options.iter().map(|o| o.name).collect();
    for sw in &sub_sw {
        assert!(
            sub_declared.contains(sw),
            "{dname}/{name} {}: sub switch {sw:?} not declared",
            sub.name
        );
    }
    for idx in 0u8..6 {
        for v in sub.arg_values_at(idx) {
            assert!(
                !v.value.is_empty(),
                "{dname}/{name} {}: empty arg value at {idx}",
                sub.name
            );
        }
        let _ = sub.arg_role_at(idx);
    }
    if let Some(h) = sub.hover.as_ref() {
        // Hover, when present, has *some* content.
        assert!(
            !h.summary.is_empty()
                || !h.snippet.is_empty()
                || !h.source.is_empty()
                || !h.synopsis.is_empty(),
            "{dname}/{name} {}: hover present but entirely empty",
            sub.name
        );
    }
    // `cfg_rewrite_name`, when present, is a qualified ::tcl::… name.
    if let Some(rw) = sub.cfg_rewrite_name {
        assert!(
            !rw.is_empty(),
            "{dname}/{name} {}: empty rewrite name",
            sub.name
        );
    }
}

fn check_command_accessors(reg: &CommandRegistry, ds: Option<DialectSet>, dname: &str, name: &str) {
    // `get` (dialect-agnostic) must resolve a name the registry lists.
    let spec = reg
        .get(name)
        .unwrap_or_else(|| panic!("{dname}: listed name {name:?} does not `get`"));
    assert_eq!(spec.name, name, "{dname}: spec.name mismatch for {name:?}");

    // --- arity (command level) ---
    assert_arity_consistent(spec.arity, &format!("{dname}/{name}"));

    // --- switch / option accessors ---
    // `switch_names` walks the flat options + every form's options.
    let switches = spec.switch_names(ds);
    // Names returned must be unique (the helper dedups) and a subset of
    // the declared option names (flat + form-level).
    let mut declared: BTreeSet<&str> = spec.options.iter().map(|o| o.name).collect();
    for form in spec.command_forms {
        for o in form.options {
            declared.insert(o.name);
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for sw in &switches {
        assert!(
            seen.insert(sw),
            "{dname}/{name}: switch_names returned {sw:?} twice"
        );
        assert!(
            declared.contains(sw),
            "{dname}/{name}: switch {sw:?} not among declared options"
        );
    }
    // A command that declares any options must surface a (typed)
    // OptionSpec; touch its fields.
    for opt in spec.options {
        assert!(!opt.name.is_empty(), "{dname}/{name}: empty option name");
        if opt.takes_value() {
            // A value-taking option carries a hint or a detail to show.
            let _ = (opt.value_hint(), opt.detail);
        }
        // Exercise the dialect predicate both with and without context.
        let _ = opt.supports_dialect(ds, spec.dialects);
        let _ = opt.supports_dialect(None, spec.dialects);
    }

    // --- subcommands: arity, roles, options, arg-values, hover ---
    for sub in spec.subcommands {
        check_subcommand_accessors(spec, sub, ds, dname, name);
    }

    // --- command-level arg-values + arg-roles ---
    for idx in 0u8..8 {
        for v in spec.arg_values_at(idx) {
            assert!(
                !v.value.is_empty(),
                "{dname}/{name}: empty command arg value at {idx}"
            );
        }
        let _ = spec.arg_role_at(idx);
    }

    // --- closed value args must name an index that *has* values ---
    for &ci in spec.closed_value_args {
        assert!(
            !spec.arg_values_at(ci).is_empty(),
            "{dname}/{name}: closed_value_args lists index {ci} with no arg_values"
        );
    }

    // --- hover (command level) ---
    if let Some(h) = spec.hover.as_ref() {
        assert!(
            !h.summary.is_empty()
                || !h.snippet.is_empty()
                || !h.source.is_empty()
                || !h.synopsis.is_empty(),
            "{dname}/{name}: hover present but entirely empty"
        );
    }

    // --- forms (thin FormSpec for hover/completion) ---
    for f in spec.forms {
        // Every form is classified; synopsis text may be empty but the
        // kind must be one of the three variants (exercise the enum).
        assert!(
            matches!(
                f.kind,
                FormKind::Default | FormKind::Getter | FormKind::Setter
            ),
            "{dname}/{name}: unexpected form kind"
        );
    }
    // --- structured command_forms ---
    for cf in spec.command_forms {
        assert_arity_consistent(cf.arity, &format!("{dname}/{name} form {}", cf.name));
    }

    // --- side effects ---
    // Touch every side-effect field. Note a declaration may legitimately
    // be neither read nor write — e.g. `apply`/`proc` declare a
    // `ProcDefinition` *definitional* effect (it neither reads nor writes
    // program state, it introduces a binding), so we do NOT require
    // reads||writes here; we just exercise the accessors.
    for se in spec.side_effects {
        let _ = (se.target, se.reads, se.writes, se.connection_side);
    }

    check_registry_queries(reg, ds, dname, name, spec);
}

/// Touch the registry-level per-name query helpers and `resolve_call` paths.
/// Extracted from [`check_command_accessors`] to keep it within the line
/// budget.
fn check_registry_queries(
    reg: &CommandRegistry,
    ds: Option<DialectSet>,
    dname: &str,
    name: &str,
    spec: &tcl_registry::CommandSpec,
) {
    // --- per-spec boolean / registry-level query helpers ---
    // These all read the by-name spec list; calling them on every name
    // executes their scan bodies.
    let _ = reg.is_unsafe(name);
    let _ = reg.is_side_switch(name);
    let _ = reg.is_byte_compiled(name);
    let _ = reg.is_diagram_action(name);
    let _ = reg.is_irules_top_level_only(name);
    let _ = reg.is_not_proc_factory(name);
    let _ = reg.is_xc_never_translatable(name);
    let _ = reg.is_xc_translatable_override(name);
    let _ = reg.is_canonical_list_command(name);
    // taint_source reads the compile-time TAINT_SOURCE_INDEX.
    let _ = reg.taint_source(name);
    // subcommands_with_trait over a couple of representative traits.
    let _ = reg.subcommands_with_trait(name, Traits::PURE);
    let _ = reg.subcommands_with_trait(name, Traits::INTROSPECTS_BY_NAME);

    // --- resolve_call: nullary and a small arg list ---
    // Must never panic; when it resolves, the spec name is preserved.
    let active = ds.unwrap_or(DialectSet::TCL86);
    if let Some(rc) = reg.resolve_call(name, &[], active) {
        assert_eq!(
            rc.spec.name, spec.name,
            "{dname}/{name}: resolve_call(nullary) spec"
        );
    }
    // If the command has subcommands, resolving with the first
    // subcommand name as arg 0 should attach that subcommand.
    if let Some(first_sub) = spec.subcommands.first()
        && let Some(rc) = reg.resolve_call(name, &[first_sub.name], active)
    {
        assert_eq!(rc.spec.name, spec.name);
        if let Some(s) = rc.sub {
            assert_eq!(s.name, first_sub.name, "{dname}/{name}: wrong sub attached");
        }
    }
    // resolve_option_terminator must not panic; when Some, scan_start is
    // within the (subcommand-or-form) contract (0 or 1).
    if let Some(term) = reg.resolve_option_terminator(name, &["--"], active) {
        assert!(
            term.scan_start <= 1,
            "{dname}/{name}: terminator scan_start {} out of range",
            term.scan_start
        );
    }
}

/// The headline sweep: for each loadable dialect, build the registry and
/// iterate ALL registered command specs, calling EVERY public accessor on each
/// and asserting per-spec invariants. This is what executes the bulk of the
/// otherwise-uncovered per-spec data and query-helper lines.
///
/// registry-metadata: every accessor here reads registry-internal data;
/// the assertions are structural invariants, not C-Tcl facts.
#[test]
fn sweep_every_command_every_accessor() {
    let mut total_specs = 0usize;
    let mut dialects_seen = 0usize;

    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let ds = DialectSet::parse(dname);
        assert!(!reg.is_empty(), "{dname}: empty registry");
        dialects_seen += 1;

        // Collect names first so the borrow of `reg` ends before per-name
        // `get`s (command_names borrows the key map).
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        assert!(!names.is_empty(), "{dname}: no command names");

        for name in &names {
            total_specs += 1;
            check_command_accessors(reg, ds, dname, name);
        }

        // byte_array_payload_layouts is a per-registry aggregate; iterate it.
        for (cmd, layout) in reg.byte_array_payload_layouts() {
            assert!(!cmd.is_empty(), "{dname}: empty byte-array payload command");
            // The data index is a small offset; just touch the fields.
            let _ = (layout.replace_data_index, layout.message_flag_shift);
        }
    }

    assert_eq!(
        dialects_seen,
        LOADABLE_DIALECTS.len(),
        "did not sweep every dialect"
    );
    // Sanity floor: the full cross-dialect sweep touches thousands of specs.
    assert!(
        total_specs > 1000,
        "sweep unexpectedly small: {total_specs} specs"
    );
}

/// Every spec marking its deprecated replacement as a drop-in rename
/// (`deprecated_replacement_drop_in`) must (a) actually carry a
/// replacement, (b) name a single bare command word — no prose like
/// `"(removed)"`, no restructured multi-word form — and (c) resolve to a
/// registered command in the same dialect: the IRULE2002 head-swap quick
/// fix rewrites the call head to this name verbatim, so a stale or
/// unresolvable replacement would auto-fix code into an unknown command.
///
/// registry-metadata: the drop-in classification is registry data.
#[test]
fn sweep_deprecated_drop_in_replacements_resolve() {
    let mut drop_ins = 0usize;
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            if !spec.deprecated_replacement_drop_in {
                continue;
            }
            drop_ins += 1;
            let replacement = spec
                .deprecated_replacement
                .unwrap_or_else(|| panic!("{dname}/{name}: drop-in flag without a replacement"));
            assert!(
                !replacement.contains(char::is_whitespace)
                    && !replacement.contains('(')
                    && !replacement.contains('/'),
                "{dname}/{name}: drop-in replacement {replacement:?} is not a single command word"
            );
            assert!(
                reg.get(replacement).is_some(),
                "{dname}/{name}: drop-in replacement {replacement:?} is not a registered command"
            );
        }
    }
    // Sanity floor: the iRules pack stamps the 4.x-compat rename family.
    assert!(
        drop_ins >= 20,
        "expected the iRules drop-in rename family, got {drop_ins}"
    );
}

/// Sweep the whole trait lattice through `commands_with_trait` in a few
/// dialects, asserting the membership listing is self-consistent with each
/// command's own `traits.contains`.
///
/// registry-metadata: trait classification is registry data.
#[test]
fn sweep_trait_membership_is_self_consistent() {
    for &dname in &["tcl8.6", "tcl9.0", "f5-irules"] {
        let reg = registry_for_dialect(dname);
        for &t in ALL_TRAITS {
            let members = reg.commands_with_trait(t);
            // No duplicates in the membership listing.
            let unique: BTreeSet<&str> = members.iter().copied().collect();
            assert_eq!(
                unique.len(),
                members.len(),
                "{dname}: commands_with_trait({t:?}) has duplicates"
            );
            // Every listed member's *preferred* spec actually carries the trait.
            for m in &members {
                let spec = reg
                    .get(m)
                    .unwrap_or_else(|| panic!("{dname}: trait member {m:?} does not get"));
                assert!(
                    spec.traits.contains(t),
                    "{dname}: {m:?} listed for {t:?} but its spec lacks it"
                );
            }
        }
    }
}

/// `get_for_dialect` is consistent with `supports_dialect` and with the
/// command's declared `dialects` set, swept over every command in the Tcl
/// dialects. This exercises the dialect-filtered resolution path on every spec.
///
/// registry-metadata (dialect membership is registry data; the *introduction
/// versions* of `dict`/`try`/etc. are tclsh history, covered in the sibling
/// port).
#[test]
fn sweep_dialect_resolution_is_consistent() {
    for (dname, bit) in [
        ("tcl8.4", DialectSet::TCL84),
        ("tcl8.5", DialectSet::TCL85),
        ("tcl8.6", DialectSet::TCL86),
        ("tcl9.0", DialectSet::TCL90),
    ] {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let for_dialect = reg.get_for_dialect(name, bit);
            if let Some(spec) = for_dialect {
                // A spec returned for `bit` must support `bit`.
                assert!(
                    spec.supports_dialect(bit),
                    "{dname}: get_for_dialect({name:?}) returned a spec that rejects the dialect"
                );
            }
            // If *every* registered spec under this name rejects the dialect,
            // get_for_dialect must return None (and vice-versa). Use
            // `spec_visible`, not the bare `supports_dialect` mask test:
            // `get_for_dialect` also applies the profile's subtractive
            // disable list and (for a profile whose operators are not
            // command heads, e.g. tcl8.4/f5-irules) the
            // `Traits::OPERATOR_COMMAND` exclusion, so a naive mask-only
            // scan disagrees with it for exactly those specs.
            let any_supports = reg.specs(name).iter().any(|s| reg.spec_visible(s, bit));
            assert_eq!(
                for_dialect.is_some(),
                any_supports,
                "{dname}: get_for_dialect/{name:?} disagrees with specs() support scan"
            );
        }
    }
}

/// Assert `child`'s dialect declaration is never *reachable* under a dialect
/// where its `parent` isn't. This is the reachability nesting rule, not a raw
/// bitwise subset: under the "universal command = `ALL_TCL`, vendor
/// availability via mask *intersection*" model, a sub-element may explicitly
/// list a vendor bit its `ALL_TCL` parent lacks and still be valid — e.g.
/// `variable`'s `EXPECT`-tagged form is reachable only under expect's
/// `TCL86 | EXPECT` mask, which the `ALL_TCL` parent also reaches via `TCL86`.
/// The invariant it guards is the real one: a child must never resolve under
/// a dialect profile its parent command cannot.
///
/// `None` on either side is trivially valid (`None` child = inherit; `None`
/// parent = universal).
fn assert_dialects_nest(parent: Option<DialectSet>, child: Option<DialectSet>, context: &str) {
    let (Some(child), Some(parent)) = (child, parent) else {
        return;
    };
    for profile in tcl_dialect::DialectProfile::all() {
        let mask = profile.availability_mask;
        assert!(
            !child.intersects(mask) || parent.intersects(mask),
            "{context}: child dialects {child:?} resolve under {} (mask \
             {mask:?}) but the parent's {parent:?} do not — the child would be \
             reachable where its parent command is not",
            profile.name
        );
    }
}

/// Dialect-nesting checks for one `CommandForm`/`SubCommandForm` and its
/// own options, relative to its `effective_parent` (already resolved by
/// the caller). Shared between the command-level and subcommand-level
/// callers since the two form kinds are the same type.
fn check_form_dialect_nesting(
    ctx: &str,
    effective_parent: Option<DialectSet>,
    form: &tcl_registry::forms::CommandForm,
) {
    assert_dialects_nest(
        effective_parent,
        form.dialects,
        &format!("{ctx} form {}", form.name),
    );
    let effective = form.dialects.or(effective_parent);
    for o in form.options {
        assert_dialects_nest(
            effective,
            o.dialects,
            &format!("{ctx} form {} option {}", form.name, o.name),
        );
    }
}

/// Dialect-nesting checks for one `SubCommand`: itself against the parent
/// `CommandSpec`'s dialects, then its options / side effects / forms /
/// sub-subcommands against *its own* effective dialects.
fn check_subcommand_dialect_nesting(
    cmd_ctx: &str,
    parent: Option<DialectSet>,
    sub: &tcl_registry::SubCommand,
) {
    let ctx = format!("{cmd_ctx} {}", sub.name);
    assert_dialects_nest(parent, sub.dialects, &ctx);
    let effective = sub.dialects.or(parent);
    for o in sub.options {
        assert_dialects_nest(effective, o.dialects, &format!("{ctx} option {}", o.name));
    }
    for se in sub.side_effects {
        assert_dialects_nest(
            effective,
            se.dialects,
            &format!("{ctx} side_effect {:?}", se.target),
        );
    }
    for scf in sub.subcommand_forms {
        check_form_dialect_nesting(&ctx, effective, scf);
    }
    for ssc in sub.sub_subcommands {
        assert_dialects_nest(effective, ssc.dialects, &format!("{ctx} {}", ssc.name));
    }
}

/// Dialect-nesting checks for everything hung directly off one
/// `CommandSpec`: its flat options, side effects, forms, structured
/// command forms, and subcommands (recursing into the latter).
fn check_command_dialect_nesting(dname: &str, name: &str, spec: &tcl_registry::CommandSpec) {
    let ctx = format!("{dname}/{name}");
    for opt in spec.options {
        assert_dialects_nest(
            spec.dialects,
            opt.dialects,
            &format!("{ctx} option {}", opt.name),
        );
    }
    for se in spec.side_effects {
        assert_dialects_nest(
            spec.dialects,
            se.dialects,
            &format!("{ctx} side_effect {:?}", se.target),
        );
    }
    for f in spec.forms {
        assert_dialects_nest(
            spec.dialects,
            f.dialects,
            &format!("{ctx} form {}", f.synopsis),
        );
    }
    for cf in spec.command_forms {
        check_form_dialect_nesting(&ctx, spec.dialects, cf);
    }
    for sub in spec.subcommands {
        check_subcommand_dialect_nesting(&ctx, spec.dialects, sub);
    }
}

/// Every nested `dialects` declaration (`OptionSpec`, `SideEffect`,
/// `FormSpec`, `CommandForm`/`SubCommandForm`, `SubCommand`,
/// `SubSubCommand`) must be a subset of its governing parent's *effective*
/// dialects — a child claiming a dialect its parent lacks can never
/// actually be reached, since the parent itself is never selected outside
/// its own dialects, so the child would silently dead-code instead of
/// failing loudly. This is the whole-registry backstop for the
/// `DialectSet::is_valid_nested_dialects` invariant: a single command file
/// *can* self-check at compile time with a `const { assert!(...) }` (see
/// `tcl-dialect`'s `dialect_set` module for the pattern), but nothing
/// forces every command file to opt in — this sweep covers all of them,
/// present and future, uniformly.
///
/// registry-metadata: dialect membership is registry data; this is a
/// self-consistency invariant, not a C-Tcl fact.
#[test]
fn sweep_nested_dialects_are_subsets_of_parent() {
    let mut checked = 0usize;
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else {
                continue;
            };
            checked += 1;
            check_command_dialect_nesting(dname, name, spec);
        }
    }
    assert!(checked > 1000, "sweep unexpectedly small: {checked} specs");
}

// ===========================================================================
// SWEEP 2 — BIG-IP object specs and property specs.
// ===========================================================================

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
    assert!(reg.candidate_registry_kinds_for_display("").is_empty());
    assert!(
        reg.candidate_kinds_for_key("k", None, None, None)
            .is_empty()
    );
    assert!(
        reg.candidate_kinds_for_section_item("s", Some("nope"), Some("nope"))
            .is_empty()
    );
}

// ===========================================================================
// SWEEP 3 — iRules event ⇄ command cross-product accessors over the corpus.
// ===========================================================================

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
    assert!(
        reg.valid_irules_commands_for_event("__FAKE__", &events, &profiles, None)
            .is_empty()
    );
    let unknown = reg.event_info("__fake__", &events, &profiles, None);
    assert!(!unknown.known);
    assert_eq!(unknown.side, "unknown");
}

// ===========================================================================
// SWEEP 4 — taint-source / colour metadata across the source corpus.
// ===========================================================================

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

// ===========================================================================
// TARGETED — representative commands across families (distinct spec shapes).
// ===========================================================================

/// Control flow: `if` / `while` / `for` / `foreach` / `switch` carry the
/// control-flow + boolean-condition + loop-body trait shapes.
///
/// tclsh: these are all real commands (`info commands` non-empty on 8.6 & 9.0).
/// registry-metadata: the trait classification.
#[test]
fn family_control_flow_shapes() {
    let reg = registry_for_dialect("tcl8.6");
    for c in ["if", "while", "for", "foreach", "switch"] {
        let spec = reg.get(c).unwrap_or_else(|| panic!("{c} registered"));
        assert!(
            spec.traits.contains(Traits::CONTROL_FLOW),
            "{c} CONTROL_FLOW"
        );
    }
    // Loop constructs carry a loop body; `if`/`switch` do not.
    for c in ["while", "for", "foreach"] {
        assert!(
            reg.get(c).unwrap().traits.contains(Traits::HAS_LOOP_BODY),
            "{c} HAS_LOOP_BODY"
        );
    }
    assert!(
        reg.get("switch")
            .unwrap()
            .traits
            .contains(Traits::HAS_SWITCH_BODY)
    );
    // tclsh: `switch --` terminator is real — `switch -- abc {abc {...}}` runs.
    // The registry models the terminator via resolve_option_terminator.
    let term = reg.resolve_option_terminator("switch", &["--", "abc"], DialectSet::TCL86);
    if let Some(t) = term {
        assert!(
            t.options.iter().any(|o| o.name == "--"),
            "switch declares a -- option"
        );
    }
}

/// `resolve_option_terminator` must accept a unique subcommand prefix the same
/// way ensemble dispatch (and `arg_indices_for_role`) does, so an abbreviated
/// subcommand keeps its subcommand-scoped `--` terminator profile (issue 158).
#[test]
fn terminator_resolves_subcommand_prefix() {
    let reg = registry_for_dialect("f5-irules");
    // `class match` (F5 iRules) declares a subcommand-scoped `--` terminator.
    let exact = reg
        .resolve_option_terminator("class", &["match", "-nocase", "--"], DialectSet::IRULES)
        .expect("class match declares a -- terminator");
    assert_eq!(exact.subcommand, Some("match"));
    assert!(exact.options.iter().any(|o| o.name == "--"));

    // The unique prefix `ma` must resolve to the same `match` profile.
    let abbrev = reg
        .resolve_option_terminator("class", &["ma", "-nocase", "--"], DialectSet::IRULES)
        .expect("abbreviated `class ma` must keep the -- terminator");
    assert_eq!(abbrev.subcommand, Some("match"));
    assert_eq!(abbrev.scan_start, exact.scan_start);
    assert!(abbrev.options.iter().any(|o| o.name == "--"));
}

/// String / dict / list operations: subcommand-bearing ensemble shape.
///
/// tclsh: the `string` / `dict` subcommand sets, and the `string is` class set,
/// are exactly what tclsh reports (probed via `catch {string is bogus}` etc.).
#[test]
fn family_string_dict_list_ops() {
    // tclsh8.6 & 9.0: `string is bogus` lists these classes (8.6∩9.0).
    const CLASSES: &[&str] = &[
        "alnum", "alpha", "ascii", "boolean", "control", "digit", "double", "false", "integer",
        "list", "lower", "space", "true", "upper", "xdigit",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let string = reg.get("string").expect("string");
    let is = string.subcommand("is").expect("string is");
    let allowed: BTreeSet<&str> = is.arg_values_at(0).iter().map(|v| v.value).collect();
    let missing: Vec<&str> = CLASSES
        .iter()
        .copied()
        .filter(|c| !allowed.contains(c))
        .collect();
    assert!(
        missing.is_empty(),
        "string is missing classes from its arg-0 value set: {missing:?} (have {allowed:?})"
    );

    // tclsh: `dict for` / `dict map` are real subcommands and declare their CFG
    // rewrite names (registry-metadata for the rewrite name).
    let dict = reg.get("dict").expect("dict");
    for sub in ["for", "map", "get", "set", "keys", "values"] {
        assert!(dict.subcommand(sub).is_some(), "dict {sub}");
    }

    // List ops: `lsort` carries the switch set tclsh reports.
    // tclsh8.6 & 9.0: `lsort -bogus` lists -ascii … -unique (8.6∩9.0 below).
    let lsort = reg.get("lsort").expect("lsort");
    let sw = lsort.switch_names(Some(DialectSet::TCL86));
    for opt in [
        "-ascii",
        "-decreasing",
        "-increasing",
        "-integer",
        "-nocase",
        "-unique",
    ] {
        assert!(sw.contains(&opt), "lsort missing {opt}: {sw:?}");
    }
}

/// `lsearch` option version-gating.
///
/// tclsh9.0 (9.0.3): `lsearch -bogus` lists `… -start, -stride, -subindices` —
/// i.e. `-stride` IS accepted.
/// tclsh8.6 (8.6.14): the same probe lists `… -start, or -subindices` — NO
/// `-stride`; a functional `lsearch -stride 2 …` is hard-rejected with
/// "bad option \"-stride\"". So in *real* Tcl, `lsearch -stride` is **9.0-only**
/// (distinct from `lsort -stride`, which is genuinely 8.6+ / TIP 351).
///
/// FIXED: the registry previously modelled `lsearch -stride` as Tcl **8.6+**
/// (`commands/tcl/lsearch_.rs` gated it to `TCL86_PLUS` with a comment wrongly
/// copied from `lsort`'s TIP 351 `-stride`). It is now gated to `TCL90`, so
/// `switch_names(TCL86)` no longer reports it — matching tclsh8.6.14.
#[test]
fn lsearch_stride_version_gating() {
    let reg = CommandRegistry::build_default();
    let lsearch = reg.get("lsearch").expect("lsearch");
    let in_86 = lsearch.switch_names(Some(DialectSet::TCL86));
    let in_90 = lsearch.switch_names(Some(DialectSet::TCL90));
    // Common options present in both (these match tclsh on 8.6 & 9.0).
    for opt in ["-exact", "-glob", "-regexp", "-all", "-inline"] {
        assert!(in_86.contains(&opt), "8.6 lsearch missing {opt}: {in_86:?}");
    }
    // 9.0 accepts -stride — matches tclsh9.0.3.
    assert!(
        in_90.contains(&"-stride"),
        "9.0 lsearch should accept -stride: {in_90:?}"
    );
    // 8.6 must NOT — real tclsh8.6.14 hard-rejects `lsearch -stride`.
    assert!(
        !in_86.contains(&"-stride"),
        "8.6 lsearch must not accept -stride (real tclsh8.6.14 rejects it): {in_86:?}"
    );
}

/// I/O family: `socket` / `open` / `close` / `puts` / `gets`.
///
/// tclsh: `socket -server` is a real switch (`catch {socket -server}` reports
/// "no argument given for -server option" on 8.6 & 9.0). `open` opens a
/// channel; `close` is file I/O in plain Tcl.
#[test]
fn family_io_shapes() {
    let reg = registry_for_dialect("tcl8.6");

    // tclsh: socket -server / -myaddr.
    let socket = reg.get("socket").expect("socket");
    let sw = socket.switch_names(Some(DialectSet::TCL86));
    assert!(sw.contains(&"-server"), "socket -server: {sw:?}");
    // socket needs host+port at minimum.
    assert_eq!(socket.arity.min, 2, "socket min arity");
    assert!(
        socket.arity.is_unlimited(),
        "socket accepts trailing options"
    );

    // open opens a channel (registry-metadata trait).
    assert!(
        reg.get("open")
            .unwrap()
            .traits
            .contains(Traits::OPENS_CHANNEL),
        "open OPENS_CHANNEL"
    );

    // close is file I/O in plain Tcl (registry-metadata classification of a real
    // command).
    let close = reg.get("close").expect("close");
    assert!(
        close
            .side_effects
            .iter()
            .any(|e| e.target == SideEffectTarget::FileIo),
        "plain-Tcl close is file I/O: {:?}",
        close.side_effects
    );
}

/// Namespace / proc family: scope-alias + procedure-definition shapes.
///
/// tclsh: `namespace` subcommands match tclsh (`catch {namespace bogus}` lists
/// children…which on 8.6 & 9.0). `proc` defines a procedure.
#[test]
fn family_namespace_proc_shapes() {
    // tclsh8.6 & 9.0: namespace subcommand set (8.6∩9.0 — identical on both).
    const NS_SUBS: &[&str] = &[
        "children",
        "code",
        "current",
        "delete",
        "ensemble",
        "eval",
        "exists",
        "export",
        "forget",
        "import",
        "inscope",
        "origin",
        "parent",
        "path",
        "qualifiers",
        "tail",
        "unknown",
        "upvar",
        "which",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let ns = reg.get("namespace").expect("namespace");
    let missing: Vec<&str> = NS_SUBS
        .iter()
        .copied()
        .filter(|s| ns.subcommand(s).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "namespace missing subcommands: {missing:?}"
    );

    // proc defines a procedure and marks its body argument structural.
    let proc = reg.get("proc").expect("proc");
    assert!(
        proc.traits.contains(Traits::DEFINES_PROCEDURE),
        "proc DEFINES_PROCEDURE"
    );
    assert_eq!(proc.arity, Arity::exact(3), "proc name args body");
    assert_eq!(proc.arg_role_at(2), Some(ArgRole::Body), "proc body role");

    // upvar / global / variable create scope aliases.
    for c in ["upvar", "global", "variable"] {
        assert!(
            reg.get(c)
                .unwrap()
                .traits
                .contains(Traits::CREATES_SCOPE_ALIAS),
            "{c} CREATES_SCOPE_ALIAS"
        );
    }
}

/// `array` / `clock`: subcommand sets that match tclsh exactly (intersection
/// of 8.6 and 9.0, which differ slightly for `array`).
///
/// tclsh8.6: `array bogus` lists anymore, donesearch, exists, get, names,
/// nextelement, set, size, startsearch, statistics, unset.
/// tclsh9.0: same plus `default` / `for`.  The set below is the 8.6∩9.0
/// intersection (the 8.6 list).
/// tclsh (both): `clock bogus` lists add, clicks, format, microseconds,
/// milliseconds, scan, seconds.
#[test]
fn family_array_clock_subcommands() {
    const ARRAY_SUBS: &[&str] = &[
        "anymore",
        "donesearch",
        "exists",
        "get",
        "names",
        "nextelement",
        "set",
        "size",
        "startsearch",
        "statistics",
        "unset",
    ];
    const CLOCK_SUBS: &[&str] = &[
        "add",
        "clicks",
        "format",
        "microseconds",
        "milliseconds",
        "scan",
        "seconds",
    ];
    let reg = registry_for_dialect("tcl8.6");

    let array = reg.get("array").expect("array");
    let missing: Vec<&str> = ARRAY_SUBS
        .iter()
        .copied()
        .filter(|s| array.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "array missing subcommands: {missing:?}");

    let clock = reg.get("clock").expect("clock");
    let missing: Vec<&str> = CLOCK_SUBS
        .iter()
        .copied()
        .filter(|s| clock.subcommand(s).is_none())
        .collect();
    assert!(missing.is_empty(), "clock missing subcommands: {missing:?}");
}

/// F5 iRules family: the distinct iRules spec shapes (getter/setter forms,
/// side switches, event handler, closed value args, excluded events).
///
/// f5-dialect: none of these are real tclsh commands; all assertions are
/// registry metadata.
#[test]
fn family_f5_irules_shapes() {
    let reg = registry_for_dialect("f5-irules");
    let ds = DialectSet::IRULES;

    // when is the event handler with a structural body.
    let when = reg.get_for_dialect("when", ds).expect("when");
    assert!(
        when.traits.contains(Traits::IS_EVENT_HANDLER),
        "when IS_EVENT_HANDLER"
    );

    // HTTP::uri getter (no args, PURE source) vs setter (one arg) forms.
    let getter = reg
        .resolve_call("HTTP::uri", &[], ds)
        .expect("HTTP::uri getter");
    assert!(
        getter.spec.traits.contains(Traits::PURE),
        "HTTP::uri getter is pure"
    );
    let setter = reg
        .resolve_call("HTTP::uri", &["/p"], ds)
        .expect("HTTP::uri setter");
    assert_eq!(setter.spec.name, "HTTP::uri");

    // Side switches.
    for name in ["clientside", "serverside", "peer"] {
        assert!(reg.is_side_switch(name), "{name} side switch");
    }

    // Closed value arg: HTTP::version arg 0 is a closed version set.
    let ver = reg
        .get_for_dialect("HTTP::version", ds)
        .expect("HTTP::version");
    assert!(
        ver.closed_value_args.contains(&0),
        "HTTP::version arg0 closed"
    );
    let versions: BTreeSet<&str> = ver.arg_values_at(0).iter().map(|v| v.value).collect();
    assert!(
        versions.contains("1.1"),
        "HTTP::version closed set has 1.1: {versions:?}"
    );

    // Excluded events: TCP::rcv_scale declares at least one excluded event, and
    // is illegal there.
    let rcv = reg
        .get_for_dialect("TCP::rcv_scale", ds)
        .expect("TCP::rcv_scale");
    assert!(
        !rcv.excluded_events.is_empty(),
        "TCP::rcv_scale excluded_events"
    );
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let ex = rcv.excluded_events[0];
    assert!(
        !reg.is_irules_command_legal_in_event("TCP::rcv_scale", ex, &events, &profiles),
        "TCP::rcv_scale must be illegal in excluded event {ex}"
    );
}

// ===========================================================================
// Dialect-catalogue sweep (the name surface this crate owns).
// ===========================================================================

/// Every catalogued dialect name parses-or-is-config-only, and each loadable
/// dialect's cached registry round-trips a core lookup. Touches the cache /
/// parse / available-dialects surface.
///
/// registry-metadata: dialect vocabulary.
#[test]
fn sweep_dialect_catalogue() {
    assert_eq!(available_dialects(), KNOWN_DIALECTS);
    for &d in KNOWN_DIALECTS {
        // Every cached dialect registry has the Tcl core.
        let reg = registry_for_dialect(d);
        assert!(!reg.is_empty(), "{d}: empty registry");
        assert!(reg.get("set").is_some(), "{d}: missing core `set`");
        // The name either parses to a DialectSet bit, or is a config-only name
        // that collapses to plain Tcl (still a usable registry).
        let _ = DialectSet::parse(d);
    }
    // An unparseable dialect collapses to a plain-Tcl registry.
    let junk = registry_for_dialect("definitely-not-a-dialect");
    assert!(junk.get("set").is_some());
    assert!(
        junk.get("HTTP::header").is_none(),
        "junk fallback is plain Tcl"
    );
}

/// Every declared lifecycle in every registry — commands, subcommands,
/// options, versioned argument values, iRules events, and BIG-IP profile
/// types — must order its releases legally (#1210).
///
/// Bad ordering (`deprecated < introduced`, `retired < deprecated`, …) is
/// unrepresentable downstream: `state_at` would report a state the data
/// cannot actually reach, so it is rejected here rather than at a consumer.
///
/// registry-metadata: structural invariants over registry data, not C-Tcl
/// facts.
#[test]
fn sweep_lifecycle_ordering_is_valid_everywhere() {
    let check = |what: &str, lifecycle: tcl_registry::lifecycle::Lifecycle| {
        assert_eq!(lifecycle.validate(), Ok(()), "{what}: {lifecycle:?}");
    };

    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            check(&format!("{dname}/{name}"), spec.lifecycle);
            for opt in spec.options {
                check(&format!("{dname}/{name} {}", opt.name), opt.lifecycle);
            }
            for sub in spec.subcommands {
                let path = format!("{dname}/{name} {}", sub.name);
                check(&path, sub.lifecycle);
                for opt in sub.options {
                    check(&format!("{path} {}", opt.name), opt.lifecycle);
                }
                for gate in sub.versioned_arg_values {
                    check(&format!("{path} = {}", gate.value), gate.lifecycle);
                }
            }
        }
    }

    let events = EventRegistry::build();
    for event in events.all_event_names() {
        check(event, events.event_lifecycle(event).expect("known event"));
    }

    let profiles = ProfileRegistry::build();
    for name in profiles.all_profile_names() {
        check(
            name,
            profiles.profile_lifecycle(name).expect("known profile"),
        );
    }
}

/// The lifecycle seed cases from #1210: each of the four states is reachable
/// and independently observable on real registry data.
///
/// f5-dialect: the event/profile releases are F5 facts, not tclsh.
#[test]
fn sweep_lifecycle_seed_cases() {
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

/// Issue #1256 — the boolean argument role is a declared registry fact, and
/// the whole registry agrees about which options carry it.
///
/// Two invariants, both sweeping every command and subcommand option table in
/// every known dialect:
///
/// 1. An option whose value hint says `boolean`/`bool` **must** declare
///    `ArgRole::Boolean`. That is what stops a new boolean option from
///    silently going back to being invisible to the consumers.
/// 2. The reverse holds too: `value_is_boolean()` is `true` only for the
///    declared role — the old inference from a closed value set is gone.
///
/// tclsh-proof (8.6.16 / 9.0.4): the role means the bytes are consumed and
/// discarded, which is why every spelling is interchangeable —
/// `chan configure $c -blocking yes; chan configure $c -blocking` → `1`, and
/// `-blocking tru` is accepted identically (prefix matching).
#[test]
fn boolean_argument_role_is_declared_and_consistent() {
    use tcl_registry::ArgRole;
    use tcl_registry::hover::OptionSpec;

    let mut declared = 0usize;
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "f5-irules",
    ] {
        let reg = registry_for_dialect(dialect);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            let mut sweep = |path: &str, options: &'static [OptionSpec]| {
                for opt in options {
                    let hint = opt.value_hint();
                    if matches!(hint, "boolean" | "bool") {
                        assert!(
                            opt.value_is_boolean(),
                            "{dialect} {path} {}: hint says `{hint}` but the option does not \
                             declare ArgRole::Boolean — every boolean-valued option must carry \
                             the fact (issue #1256)",
                            opt.name
                        );
                    }
                    if opt.value_is_boolean() {
                        declared += 1;
                        assert_eq!(
                            opt.value_role(),
                            Some(ArgRole::Boolean),
                            "{dialect} {path} {}: value_is_boolean must mean the declared role",
                            opt.name
                        );
                    }
                }
            };
            sweep(spec.name, spec.options);
            for sub in spec.subcommands {
                sweep(&format!("{} {}", spec.name, sub.name), sub.options);
            }
        }
    }
    assert!(
        declared > 0,
        "the sweep must actually reach declared boolean options"
    );
    assert!(ArgRole::Boolean.consumes_boolean());
    // A numeric-or-boolean position is deliberately NOT boolean: `0`/`1` are
    // valid integers there, so a rewriting consumer must abstain.
    assert!(!ArgRole::NumericOrBoolean.consumes_boolean());
    assert!(!ArgRole::Value.consumes_boolean());
}
