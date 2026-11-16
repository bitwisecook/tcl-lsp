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

//! Argument-role, repeated-layout, clause-grammar and option-effect declarations are well formed.

use super::*;

/// The boolean argument role is a declared registry fact, and
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

/// Option and positional callback metadata must be internally meaningful in
/// every shipped spec. The `SpecTcl` loader rejects the same contradictions for
/// packs; this sweep keeps hand-written Rust declarations equally honest.
#[test]
fn callback_and_variable_option_metadata_is_well_formed() {
    use tcl_registry::hover::{OptionSpec, OptionValue, ScriptTiming, VariableScope};

    fn check_options(path: &str, options: &[OptionSpec]) {
        for option in options {
            let OptionValue::Takes(arg) = option.value else {
                continue;
            };
            let executable = arg.role.has_script_timing()
                || arg.also_role.is_some_and(ArgRole::has_script_timing);
            let variable = matches!(arg.role, ArgRole::VarRead | ArgRole::VarWrite)
                || matches!(arg.also_role, Some(ArgRole::VarRead | ArgRole::VarWrite));
            let variable_write =
                arg.role == ArgRole::VarWrite || arg.also_role == Some(ArgRole::VarWrite);
            assert!(
                executable || arg.script_timing == ScriptTiming::SameInvocation,
                "{path} {} gives timing to a non-executable option",
                option.name
            );
            assert!(
                arg.callback_taint_inputs.is_empty()
                    || executable && arg.script_timing == ScriptTiming::Deferred,
                "{path} {} gives callback inputs to a non-deferred executable",
                option.name
            );
            assert!(
                variable || arg.variable_scope == VariableScope::CurrentFrame,
                "{path} {} gives variable scope to a non-variable option",
                option.name
            );
            assert!(
                !arg.taints_var_write || variable_write,
                "{path} {} taints writes without a VarWrite role",
                option.name
            );
        }
    }

    fn check_callbacks(
        path: &str,
        roles: &[(u8, ArgRole)],
        callbacks: &[(u8, &'static [tcl_registry::CallbackTaintInput])],
        has_role_resolver: bool,
        can_defer: bool,
    ) {
        for (index, inputs) in callbacks {
            assert!(
                !inputs.is_empty(),
                "{path} argument {index} has an empty callback-input row"
            );
            assert!(
                has_role_resolver
                    || roles.iter().any(|(candidate, role)| {
                        candidate == index && role.has_script_timing()
                    }),
                "{path} argument {index} has callback inputs but is not executable"
            );
            assert!(
                can_defer,
                "{path} argument {index} has callback inputs but cannot be deferred"
            );
        }
    }

    for dialect in LOADABLE_DIALECTS {
        let registry = registry_for_dialect(dialect);
        let names: Vec<String> = registry.command_names().map(str::to_owned).collect();
        for name in names {
            let Some(spec) = registry.get(&name) else {
                continue;
            };
            check_options(spec.name, spec.options);
            check_callbacks(
                spec.name,
                spec.arg_roles,
                spec.callback_taint_inputs,
                spec.has_dynamic_argument_roles() || spec.command_prefix_resolver.is_some(),
                spec.script_timing_resolver.is_some() || spec.traits.contains(Traits::DEFERS_BODY),
            );
            for sub in spec.subcommands.iter().chain(
                spec.object_class
                    .into_iter()
                    .flat_map(|class| class.instance_methods),
            ) {
                let path = format!("{} {}", spec.name, sub.name);
                check_options(&path, sub.options);
                check_callbacks(
                    &path,
                    sub.arg_roles,
                    sub.callback_taint_inputs,
                    sub.has_dynamic_argument_roles() || sub.command_prefix_resolver.is_some(),
                    sub.script_timing_resolver.is_some()
                        || sub.traits.contains(Traits::DEFERS_BODY),
                );
            }
        }
    }
}

/// A conditionally-bound local must never carry an `ArgRole` SSA models as
/// an unconditional definition.
///
/// Declaring `dict update`'s pair locals `ArgRole::VarWrite` reproduces a
/// W210 false positive: `VarWrite` at a
/// `repeated_args` position feeds every generic "does this argument write a
/// variable" site the compiler has — established by reading each one's own
/// `arg_indices_for_role` call: `tcl-compiler`'s `ssa::defs_of_with_registry`
/// (the `Statement::Barrier` arm `dict update`/`dict with` lower to),
/// `var_scoping` (twice — the `global`/`variable`/`upvar`-family scope-alias
/// walk), `cfg_builder::builtin_write_defs_from_text`, `lowering`'s
/// registry-driven default-call def extractor and its `trace`/`vwait`
/// resolvers, and `ir_helpers`'s `defs_from_expr` / `defs_from_body_script`
/// / deletion-safety scans. Every one of those asks for `ArgRole::VarWrite`
/// specifically; none asks for `LoopVarList` or any other role. So
/// `VarWrite` is, by construction of the compiler's own def-use code (not by
/// assumption here), the one role SSA treats as an **unconditional**
/// definition of the position it is declared at — a def with `version > 0`
/// that `build_undef_suppression`
/// (`tcl-compiler/src/analyser/diagnostics/helpers.rs`) puts straight into
/// `explicitly_defined`, which `suppresses_strict` checks *before* it
/// honours the key-aware `dict_with_known_keys` suppression
/// `harvest_dict_with_suppression` computed for the very same read — so an
/// absent-key read that should be suppressed draws W210 instead.
///
/// [`RepeatedArgLayout::conditional_binding`] is the registry's declared
/// counterpart: `true` marks a position whose local is bound only when a
/// runtime *data* condition holds (a dict/array key actually being present),
/// not merely whenever the statement executes — `dict update dictVar key
/// varName` binds `varName` only if `key` is in the dict (tclsh: an absent
/// key leaves it unset). This sweep asserts the invariant over **every**
/// `repeated_args` declaration in every dialect's registry: no command name
/// appears in the check, only the two declared fields, so the next command
/// whose layout gets this wrong fails here — pointing at the offending
/// declaration — rather than surfacing four layers away as a diagnostic
/// false positive. `dict update` (`conditional_binding: true`, `LoopVarList`)
/// is the one positive control that currently exercises the `true` arm;
/// `foreach`/`lmap`'s loop-var lists, `global`/`variable`'s repeated names,
/// and the `upvar`/`namespace upvar` local layouts are positive controls for
/// the untouched (`conditional_binding: false`) arm — they use `VarWrite` /
/// `LoopVarList` freely because their bindings really are unconditional
/// (aliasing/assignment that depends on the statement running, never on a
/// key or element being present).
#[test]
fn repeated_arg_layouts_never_pair_conditional_binding_with_an_ssa_def_role() {
    use tcl_registry::RepeatedArgLayout;

    // Every `ArgRole` the compiler's own def-use code queries when deciding
    // "does this argument position define a variable" — see the doc comment
    // above for the exhaustive list of call sites this was read from. Only
    // `VarWrite` is ever asked for; extending this list requires reading a
    // new unconditional-def call site into existence first, not guessing.
    const SSA_UNCONDITIONAL_DEF_ROLES: &[ArgRole] = &[ArgRole::VarWrite];

    let mut conditional_layouts_checked = 0usize;

    for &dialect in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dialect);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };

            let mut layouts: Vec<(String, &'static [RepeatedArgLayout])> =
                vec![(spec.name.to_owned(), spec.repeated_args)];
            for sub in spec.subcommands {
                layouts.push((format!("{} {}", spec.name, sub.name), sub.repeated_args));
            }

            for (path, repeated) in layouts {
                for layout in repeated {
                    if !layout.conditional_binding {
                        continue;
                    }
                    conditional_layouts_checked += 1;
                    assert!(
                        !SSA_UNCONDITIONAL_DEF_ROLES.contains(&layout.role),
                        "{dialect} {path}: repeated_args layout at start {} declares \
                         conditional_binding: true (its local is bound only when a runtime \
                         data condition holds) but role {:?} is one SSA models as an \
                         unconditional def — this reproduces the issue #1247 W210 \
                         false-positive mechanism (issue #1278); use a role SSA does not \
                         treat as a definite def instead",
                        layout.start,
                        layout.role,
                    );
                }
            }
        }
    }

    assert!(
        conditional_layouts_checked > 0,
        "the sweep must actually reach a `conditional_binding: true` layout \
         (dict update's pair locals) — otherwise the assertion above is vacuous"
    );
}

/// One clause-grammar owner — a command or a subcommand — with the layouts
/// its group rows cite and its own static role table.
struct ClauseGrammarOwner {
    pub(super) path: String,
    grammar: &'static tcl_registry::ClauseGrammarSpec,
    layouts: &'static [tcl_registry::RepeatedArgLayout],
    pub(super) arg_roles: &'static [(u8, ArgRole)],
}

/// Every clause grammar of every loadable dialect's specs.
fn clause_grammar_owners() -> Vec<ClauseGrammarOwner> {
    let mut out = Vec::new();
    for &dialect in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dialect);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = reg.get(name) else { continue };
            if let Some(grammar) = spec.clause_grammar {
                out.push(ClauseGrammarOwner {
                    path: format!("{dialect} {}", spec.name),
                    grammar,
                    layouts: spec.repeated_args,
                    arg_roles: spec.arg_roles,
                });
            }
            for sub in spec.subcommands {
                if let Some(grammar) = sub.clause_grammar {
                    out.push(ClauseGrammarOwner {
                        path: format!("{dialect} {} {}", spec.name, sub.name),
                        grammar,
                        layouts: sub.repeated_args,
                        arg_roles: sub.arg_roles,
                    });
                }
            }
        }
    }
    out
}

/// Every slot of every row of `grammar`, with the row it sits in.
fn clause_slots(
    grammar: &tcl_registry::ClauseGrammarSpec,
) -> Vec<(tcl_registry::ClauseRowId, tcl_registry::ClauseSlot)> {
    grammar
        .all_rows()
        .flat_map(|(id, row)| row.slots().iter().map(move |slot| (id, *slot)))
        .collect()
}

/// A group row cites a `repeated_args` layout by index; the stride and the
/// trailing exclusion are that layout's facts, so the index must name one.
#[test]
fn clause_grammar_group_rows_cite_a_real_layout() {
    let owners = clause_grammar_owners();
    let mut groups = 0usize;
    for owner in &owners {
        for (_, row) in owner.grammar.all_rows() {
            if let tcl_registry::ClauseRowShape::Group { layout } = row.shape {
                groups += 1;
                assert!(
                    usize::from(layout) < owner.layouts.len(),
                    "{}: a group row cites repeated_args[{layout}], but the spec declares {} layout(s)",
                    owner.path,
                    owner.layouts.len()
                );
                assert!(
                    row.keyword.is_none(),
                    "{}: a group row is keywordless by construction",
                    owner.path
                );
            }
        }
    }
    assert!(
        groups > 0,
        "the sweep must reach a group row (foreach's binder groups)"
    );
}

/// The clause-slot sibling of
/// `repeated_arg_layouts_never_pair_conditional_binding_with_an_ssa_def_role`:
/// a slot whose names are bound only under a runtime data condition never
/// carries `VarWrite`, which every def-use site reads as an unconditional
/// definition.
#[test]
fn conditional_binding_clause_slots_never_carry_var_write() {
    for owner in clause_grammar_owners() {
        for (row, slot) in clause_slots(owner.grammar) {
            assert!(
                !(slot.conditional_binding && slot.role == ArgRole::VarWrite),
                "{} {row:?}: a conditional_binding clause slot declares VarWrite",
                owner.path
            );
        }
    }
}

/// A clause slot speaks the six roles a clause uses — `Expr`, `Body`,
/// `LoopVarList`, `Pattern`, `Keyword` (a `?noise?` word), `Value` — so a
/// consumer reading the plan learns no seventh; a noise word is a `Keyword`
/// slot and a handler rides a `Pattern` slot.
#[test]
fn clause_slots_use_only_the_six_roles() {
    const CLAUSE_ROLES: &[ArgRole] = &[
        ArgRole::Expr,
        ArgRole::Body,
        ArgRole::LoopVarList,
        ArgRole::Pattern,
        ArgRole::Keyword,
        ArgRole::Value,
    ];
    let owners = clause_grammar_owners();
    assert!(
        !owners.is_empty(),
        "the shipped clause grammars must be reached"
    );
    for owner in owners {
        for (row, slot) in clause_slots(owner.grammar) {
            assert!(
                CLAUSE_ROLES.contains(&slot.role),
                "{} {row:?}: clause slot role {:?} is not one of the six",
                owner.path,
                slot.role
            );
            assert_eq!(
                slot.noise.is_some(),
                slot.role == ArgRole::Keyword,
                "{} {row:?}: a noise word is exactly a Keyword slot",
                owner.path
            );
            assert!(
                slot.handler.is_none() || slot.role == ArgRole::Pattern,
                "{} {row:?}: a handler vocabulary rides a Pattern slot",
                owner.path
            );
        }
    }
}

/// Where a grammar-carrying command also keeps a static role table (`catch`,
/// `for`, `while`, `dict for`, `array for`), the two describe one call: every
/// flat role the walk assigns on a representative call is the table's role at
/// that position, so folding both never gives a word two readings.
#[test]
fn clause_grammars_agree_with_their_static_role_tables() {
    for owner in clause_grammar_owners() {
        if owner.arg_roles.is_empty() {
            continue;
        }
        let width = owner
            .arg_roles
            .iter()
            .map(|(index, _)| usize::from(*index) + 1)
            .max()
            .unwrap_or(0);
        let words: Vec<String> = (0..width).map(|index| format!("w{index}")).collect();
        let args: Vec<&str> = words.iter().map(String::as_str).collect();
        let plan = owner.grammar.walk(&args, owner.layouts);
        assert_eq!(
            plan.defect, None,
            "{}: the representative call is well formed",
            owner.path
        );
        for (index, role) in plan.roles {
            assert!(
                owner
                    .arg_roles
                    .iter()
                    .any(|(at, found)| usize::from(*at) == index && *found == role),
                "{}: the grammar gives word {index} {role:?}, which its arg_roles do not",
                owner.path
            );
        }
    }
}

/// One option-effect scope — a command's options (and its forms'), or one
/// subcommand's — with the families declared beside them.
struct OptionEffectScope {
    pub(super) path: String,
    pub(super) options: Vec<&'static tcl_registry::hover::OptionSpec>,
    families: &'static [tcl_registry::OptionEffectFamily],
}

/// Every option-effect scope of every loadable dialect's specs.
fn option_effect_scopes() -> Vec<OptionEffectScope> {
    let mut scopes = Vec::new();
    for &dialect in LOADABLE_DIALECTS {
        let reg = registry_for_dialect(dialect);
        let names: Vec<String> = reg.command_names().map(str::to_owned).collect();
        for name in &names {
            for spec in reg.specs(name) {
                let mut options: Vec<&'static tcl_registry::hover::OptionSpec> =
                    spec.options.iter().collect();
                for form in spec.command_forms {
                    options.extend(form.options.iter());
                }
                scopes.push(OptionEffectScope {
                    path: format!("{dialect} {}", spec.name),
                    options,
                    families: spec.option_effect_families,
                });
                for sub in spec.subcommands {
                    scopes.push(OptionEffectScope {
                        path: format!("{dialect} {} {}", spec.name, sub.name),
                        options: sub.options.iter().collect(),
                        families: sub.option_effect_families,
                    });
                }
            }
        }
    }
    scopes
}

/// Every option that declares an effect names a family its own scope
/// declares (`docs/design/compiler/registry-consumer-contracts.md` § *Options
/// with semantic effects*): a family is where an axis starts and how its
/// options combine, so an effect citing an undeclared one would be read
/// against no base at all.
#[test]
fn every_option_effect_names_a_declared_family() {
    let mut effects_checked = 0usize;
    for scope in option_effect_scopes() {
        for option in &scope.options {
            let Some(effect) = option.effect else {
                continue;
            };
            effects_checked += 1;
            assert!(
                scope
                    .families
                    .iter()
                    .any(|family| family.name == effect.family),
                "{}: option `{}` cites family `{}`, which the scope does not declare",
                scope.path,
                option.name,
                effect.family,
            );
        }
        let mut names: Vec<&str> = scope.families.iter().map(|family| family.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            scope.families.len(),
            "{}: a family is declared twice",
            scope.path
        );
    }
    assert!(
        effects_checked > 0,
        "the sweep must reach the shipped option effects (`subst`, `lsearch`, `regexp`, `switch`)"
    );
}

/// A family's base mentions only axis values its own options move: an
/// `Only(axis)` base that no option of the family selects or disables would
/// start an axis no call can reach, which is a mis-declared family rather
/// than a default.
#[test]
fn a_family_base_mentions_only_axes_its_options_mention() {
    use tcl_registry::FamilyBase;
    let mut bases_checked = 0usize;
    for scope in option_effect_scopes() {
        for family in scope.families {
            let FamilyBase::Only(axis) = family.base else {
                continue;
            };
            bases_checked += 1;
            assert!(
                scope
                    .options
                    .iter()
                    .any(|option| option.effect.is_some_and(|effect| {
                        effect.family == family.name && effect.kind.axis() == Some(axis)
                    })),
                "{}: family `{}` starts from {axis:?}, which none of its options moves",
                scope.path,
                family.name,
            );
        }
    }
    assert!(
        bases_checked > 0,
        "the sweep must reach an `Only` base (`lsearch`'s glob, `switch`'s exact)"
    );
}
