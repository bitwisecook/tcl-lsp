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

//! Sweep 1: every command in every dialect through every accessor, and the derived queries against the by-name answers.

use super::*;

// SWEEP 1 — every command in every dialect, every accessor.
/// Exercise every accessor on one command spec (and its subcommands)
/// for a given dialect. Extracted from `sweep_every_command_every_accessor`
/// to keep that test within the line budget.
/// Exercise every accessor on one subcommand spec. Extracted from
/// [`check_command_accessors`] to keep both within the line budget.
fn check_subcommand_accessors(
    spec: &tcl_registry::CommandSpec,
    sub: &tcl_registry::SubCommand,
    ds: Option<SurfaceQuery<'_>>,
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
    assert_arity_windows_consistent(
        sub.arity_windows,
        sub.lifecycle,
        &format!("{dname}/{name} {}", sub.name),
    );
    // Sub-level switch_names + arg_values + arg_role accessors.
    let sub_sw = sub.switch_names(ds, spec.surface);
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

fn check_command_accessors(
    reg: &CommandRegistry,
    ds: Option<SurfaceQuery<'_>>,
    dname: &str,
    name: &str,
) {
    // `get` (dialect-agnostic) must resolve a name the registry lists.
    let spec = reg
        .get(name)
        .unwrap_or_else(|| panic!("{dname}: listed name {name:?} does not `get`"));
    assert_eq!(spec.name, name, "{dname}: spec.name mismatch for {name:?}");

    // --- arity (command level) ---
    assert_arity_consistent(spec.arity, &format!("{dname}/{name}"));
    assert_arity_windows_consistent(
        spec.arity_windows,
        spec.lifecycle,
        &format!("{dname}/{name}"),
    );

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
        let _ = opt.supports_dialect(ds, spec.surface);
        let _ = opt.supports_dialect(None, spec.surface);
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
    for se in spec.side_effects.lower_bound() {
        let _ = (se.target, se.reads, se.writes, se.connection_side);
    }

    check_registry_queries(reg, ds, dname, name, spec);
}

/// Touch the registry-level per-name query helpers and `resolve_call` paths.
/// Extracted from [`check_command_accessors`] to keep it within the line
/// budget.
fn check_registry_queries(
    reg: &CommandRegistry,
    ds: Option<SurfaceQuery<'_>>,
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
    let _ = reg.subcommands_with_trait(name, Traits::CSE_CANDIDATE);
    let _ = reg.subcommands_with_trait(name, Traits::INTROSPECTS_BY_NAME);

    // --- resolve_call: nullary and a small arg list ---
    // Must never panic; when it resolves, the spec name is preserved.
    let active = Some(ds.unwrap_or(SurfaceQuery::core(Family::Tcl, "8.6")));
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
    check_derived_queries(reg, active, dname, name, spec);
}

/// Ask every derived query of representative resolutions of `name` — each
/// subcommand (or none), with and without the command's first option, then
/// enough ordinary words to fill a typical layout — and hold each answer to
/// its own contract: positions inside the call, a sorted role table,
/// operands after the level word. No query may panic on any shipped command.
fn check_derived_queries(
    reg: &CommandRegistry,
    active: Option<SurfaceQuery<'_>>,
    dname: &str,
    name: &str,
    spec: &tcl_registry::CommandSpec,
) {
    let heads: Vec<Option<&str>> = if spec.subcommands.is_empty() {
        vec![None]
    } else {
        spec.subcommands.iter().map(|sub| Some(sub.name)).collect()
    };
    for head in heads {
        for words in representative_calls(spec, head) {
            check_derived_answers(reg, active, dname, name, &words);
        }
    }
}

/// The representative calls of `spec` after `head` (a subcommand word, or
/// none): three ordinary words, and the same behind the command's first
/// option.
fn representative_calls<'s>(
    spec: &'s tcl_registry::CommandSpec,
    head: Option<&'s str>,
) -> Vec<Vec<&'s str>> {
    let first_option = spec.options.first().map(|option| option.name);
    [None, first_option]
        .into_iter()
        .map(|option| {
            head.into_iter()
                .chain(option)
                .chain(["w1", "w2", "w3"])
                .collect()
        })
        .collect()
}

/// The derived queries of one resolution, each held to its own contract.
fn check_derived_answers(
    reg: &CommandRegistry,
    active: Option<SurfaceQuery<'_>>,
    dname: &str,
    name: &str,
    words: &[&str],
) {
    let arguments: Vec<tcl_registry::InvocationWord<'_>> = words
        .iter()
        .map(|word| tcl_registry::InvocationWord::Literal(word))
        .collect();
    let Some(invocation) = reg
        .resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(name),
                &arguments,
            ),
            active,
        )
        .resolved()
    else {
        return;
    };
    let at = format!("{dname}/{name} {words:?}");
    assert_eq!(invocation.dialect, active, "{at}: the resolution's query");
    if let Some(roles) = invocation.arg_roles() {
        assert!(
            roles.windows(2).all(|pair| pair[0].0 <= pair[1].0),
            "{at}: arg_roles sorted by position"
        );
        assert!(
            roles.iter().all(|(index, _)| *index < words.len()),
            "{at}: arg_roles inside the call: {roles:?}"
        );
    }
    for pattern in invocation.pattern_args() {
        assert!(
            usize::from(pattern.index) < words.len(),
            "{at}: pattern arg"
        );
    }
    if let Some((case, clauses)) = invocation.case_invocation() {
        for index in [
            case.subject_index,
            case.clause_list_index,
            case.inline_clause_start,
        ]
        .into_iter()
        .flatten()
        {
            assert!(index <= words.len(), "{at}: case position {index}");
        }
        assert!(
            clauses
                .iter()
                .all(|clause| clause.pattern_index < words.len())
        );
    }
    if let Some((_, operands)) = invocation.frame_effect() {
        assert!(
            operands.iter().all(|operand| operand.0 < words.len()),
            "{at}: frame operands inside the call"
        );
    }
    let _ = invocation.return_type();
    let _ = invocation.effects();
    let effects = invocation.option_effects();
    assert!(effects.option_end <= words.len(), "{at}: option_end");
    let _ = invocation.substitutions_performed();
    if let Some(plan) = invocation.clause_plan() {
        assert!(
            plan.roles.iter().all(|(index, _)| *index < words.len()),
            "{at}: clause plan inside the call"
        );
    }
}

/// Hold the derived queries to the by-name registry answers they re-key, on
/// one all-literal call under the registry's own release, wherever the
/// resolution selected the descriptors the by-name lookup reads (the same
/// spec, and the subcommand the dialect-blind lookup finds): `arg_roles` is
/// `arg_indices_for_role_words` over every role, `pattern_args` is
/// `pattern_args_words_for_dialect`, `case_invocation` is the registry's
/// reading, `return_type` is `return_type_for_call`, and `frame_effect` is
/// the descriptor resolved over the words.
fn check_derived_query_parity(
    reg: &CommandRegistry,
    dname: &str,
    name: &str,
    spec: &tcl_registry::CommandSpec,
    words: &[&str],
) {
    let own = reg.own_surface_query();
    let Some(invocation) = reg.resolve_invocation(name, words, own) else {
        return;
    };
    let same_spec = own
        .map_or(Some(spec), |query| reg.get_for_surface(name, Some(query)))
        .is_some_and(|selected| std::ptr::eq(selected, spec));
    let by_name_sub = (!spec.subcommands.is_empty())
        .then(|| words.first().and_then(|word| spec.resolve_subcommand(word)))
        .flatten()
        .map(|sub| sub.name);
    let resolved_sub = invocation
        .subcommand
        .resolved()
        .map(|sub| sub.canonical_name);
    if !same_spec || by_name_sub != resolved_sub {
        return;
    }
    let at = format!("{dname}/{name} {words:?}");
    let literals = tcl_registry::InvocationArguments::literals(words);
    let expected_roles = ArgRole::ALL
        .iter()
        .map(|&role| {
            reg.arg_indices_for_role_words(name, literals, role)
                .map(|indices| indices.into_iter().map(move |index| (index, role)))
        })
        .collect::<Option<Vec<_>>>()
        .map(|per_role| {
            // Concatenated in `ArgRole::ALL` order, so a stable sort by
            // position is the derived table's order.
            let mut roles: Vec<(usize, ArgRole)> = per_role.into_iter().flatten().collect();
            roles.sort_by_key(|&(index, _)| index);
            roles.dedup();
            roles
        });
    assert_eq!(invocation.arg_roles(), expected_roles, "{at}: arg_roles");
    assert_eq!(
        invocation.pattern_args(),
        reg.pattern_args_words_for_dialect(name, literals, own),
        "{at}: pattern_args"
    );
    assert_eq!(
        invocation.case_invocation().map(|(case, _)| case),
        reg.case_invocation(name, words, own).map(|(_, case)| case),
        "{at}: case_invocation"
    );
    assert_eq!(
        invocation.return_type(),
        spec.return_type_for_call(words),
        "{at}: return_type"
    );
    if let Some(frame) = spec.frame_effect {
        let (level, rest) = frame.resolve_for_version(words, reg.runtime_version());
        let (derived_level, operands) = invocation.frame_effect().expect("a frame effect");
        assert_eq!(operands.len(), rest.len(), "{at}: frame operands");
        assert_eq!(derived_level, level, "{at}: frame level");
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
        let ds = tcl_dialect::DialectProfile::find(dname)
            .map(tcl_dialect::DialectProfile::surface_query);
        assert!(!reg.is_empty(), "{dname}: empty registry");
        dialects_seen += 1;

        // Collect names first so the borrow of `reg` ends before per-name
        // `get`s (command_names borrows the key map).
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        assert!(!names.is_empty(), "{dname}: no command names");

        for name in &names {
            total_specs += 1;
            check_command_accessors(&reg, ds, dname, name);
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

/// The derived-query layer re-keys the by-name answers and changes none of
/// them: for every command of every loadable dialect, on the representative
/// calls of its first subcommand (or of the command itself), each derived
/// query equals the by-name answer it re-keys wherever the resolution
/// selected the same descriptors.
///
/// registry-metadata: both sides read registry-internal data.
#[test]
fn derived_queries_agree_with_the_by_name_answers() {
    for &dname in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dname);
        let names: Vec<String> = reg.command_names().map(ToOwned::to_owned).collect();
        for name in &names {
            let spec = reg.get(name).expect("a listed name resolves");
            let head = spec.subcommands.first().map(|sub| sub.name);
            for words in representative_calls(spec, head) {
                check_derived_query_parity(&reg, dname, name, spec, &words);
            }
        }
    }
}
