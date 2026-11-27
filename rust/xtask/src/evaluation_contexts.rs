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

//! Exact ratchet of authored evaluation-context gaps across native and bundled specs.

use std::collections::BTreeSet;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use tcl_registry::evaluation_context::EvaluationContextSpec;
use tcl_registry::{ArgRole, CommandSpec, SubCommand, Traits};

const PATH: &str = "docs/references/command-spec/evaluation-context-gaps.txt";

pub fn run(check: bool) -> Result<ExitCode> {
    let mut gaps = BTreeSet::new();
    for pack in tcl_registry::installed()
        .surfaces()
        .iter()
        .filter(|pack| pack.is_built())
    {
        for spec in pack.specs() {
            collect_spec(&mut gaps, pack.id, spec);
        }
    }
    let packs = tcl_spectcl::bundled::load_from_required(&crate::util::repo_root().join("specs"))
        .map_err(anyhow::Error::msg)?;
    for pack in &packs.packs {
        for command in &pack.commands {
            collect_spec(&mut gaps, &pack.name, command.spec);
        }
    }
    let expected = format!(
        "# Authored evaluation-context gaps; every row remains unknown.\n# Regenerate with cargo xtask evaluation-contexts after reviewing additions.\n{}\n",
        gaps.into_iter().collect::<Vec<_>>().join("\n")
    );
    let path = crate::util::repo_root().join(PATH);
    if check {
        let actual =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        if actual != expected {
            bail!(
                "evaluation-context declaration gaps changed; review new gaps or remove retired rows with cargo xtask evaluation-contexts"
            );
        }
    } else {
        fs::write(&path, &expected).with_context(|| format!("writing {}", path.display()))?;
    }
    eprintln!(
        "evaluation contexts: {} undeclared rows remain explicit",
        expected.lines().count().saturating_sub(2)
    );
    Ok(ExitCode::SUCCESS)
}

fn collect_spec(gaps: &mut BTreeSet<String>, pack: &str, spec: &CommandSpec) {
    let owner = format!("{pack}/{}", spec.name);
    collect_roles(gaps, &owner, spec.arg_roles, spec.evaluation_contexts);
    collect_capabilities(
        gaps,
        &owner,
        spec.arg_role_resolver_roles,
        spec.evaluation_contexts,
    );
    collect_options(gaps, &owner, spec.options, spec.evaluation_contexts);
    for &(at, _) in spec.command_prefixes {
        check_role(
            gaps,
            &owner,
            "prefix",
            Some(at),
            ArgRole::CommandPrefix,
            spec.evaluation_contexts,
        );
    }
    if spec.command_prefix_resolver.is_some() {
        check_role(
            gaps,
            &owner,
            "prefix-resolver",
            None,
            ArgRole::CommandPrefix,
            spec.evaluation_contexts,
        );
    }
    for role in ArgRole::ALL
        .iter()
        .copied()
        .filter(|role| role.needs_evaluation_context())
    {
        if spec
            .clause_grammar
            .is_some_and(|grammar| grammar.may_assign(role))
        {
            check_role(gaps, &owner, "clause", None, role, spec.evaluation_contexts);
        }
    }
    for layout in spec.repeated_args {
        check_role(
            gaps,
            &owner,
            "repeated",
            None,
            layout.role,
            spec.evaluation_contexts,
        );
    }
    check_deferral(gaps, &owner, spec.traits, spec.body_execution);
    for form in spec.command_forms {
        collect_form(
            gaps,
            &format!("{owner}/form:{}", form.name),
            form,
            spec.evaluation_contexts,
        );
    }
    for sub in spec.subcommands {
        collect_sub(
            gaps,
            &owner,
            sub,
            spec.evaluation_contexts,
            spec.body_execution,
        );
    }
    if let Some(class) = spec.object_class {
        for sub in class.instance_methods {
            collect_sub(gaps, &format!("{owner}/instance"), sub, None, None);
        }
    }
}

fn collect_sub(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    sub: &SubCommand,
    inherited: Option<EvaluationContextSpec>,
    inherited_execution: Option<tcl_registry::body_execution::BodyExecutionSpec>,
) {
    let owner = format!("{owner}/{}", sub.name);
    let contexts = sub.evaluation_contexts.or(inherited);
    collect_roles(gaps, &owner, sub.arg_roles, contexts);
    collect_capabilities(gaps, &owner, sub.arg_role_resolver_roles, contexts);
    collect_options(gaps, &owner, sub.options, contexts);
    for &(at, _) in sub.command_prefixes {
        check_role(
            gaps,
            &owner,
            "prefix",
            Some(at),
            ArgRole::CommandPrefix,
            contexts,
        );
    }
    if sub.command_prefix_resolver.is_some() {
        check_role(
            gaps,
            &owner,
            "prefix-resolver",
            None,
            ArgRole::CommandPrefix,
            contexts,
        );
    }
    for role in ArgRole::ALL
        .iter()
        .copied()
        .filter(|role| role.needs_evaluation_context())
    {
        if sub
            .clause_grammar
            .is_some_and(|grammar| grammar.may_assign(role))
        {
            check_role(gaps, &owner, "clause", None, role, contexts);
        }
    }
    for layout in sub.repeated_args {
        check_role(gaps, &owner, "repeated", None, layout.role, contexts);
    }
    for nested in sub.sub_subcommands {
        if let Some(options) = nested.options {
            collect_options(gaps, &format!("{owner}/{}", nested.name), options, contexts);
        }
    }
    check_deferral(
        gaps,
        &owner,
        sub.traits,
        sub.body_execution.or(inherited_execution),
    );
    for form in sub.subcommand_forms {
        collect_form(gaps, &format!("{owner}/form:{}", form.name), form, contexts);
    }
}

fn collect_form(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    form: &tcl_registry::forms::CommandForm,
    inherited: Option<EvaluationContextSpec>,
) {
    let contexts = form.evaluation_contexts.or(inherited);
    collect_roles(gaps, owner, form.arg_roles, contexts);
    collect_options(gaps, owner, form.options, contexts);
}

fn collect_roles(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    roles: &[(u8, ArgRole)],
    contexts: Option<EvaluationContextSpec>,
) {
    for &(at, role) in roles {
        check_role(gaps, owner, "argument", Some(at), role, contexts);
    }
}

fn collect_capabilities(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    roles: &[ArgRole],
    contexts: Option<EvaluationContextSpec>,
) {
    for &role in roles {
        check_role(gaps, owner, "resolver", None, role, contexts);
    }
}

fn collect_options(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    options: &[tcl_registry::hover::OptionSpec],
    contexts: Option<EvaluationContextSpec>,
) {
    for option in options {
        for role in [option.value_role(), option.value_also_role()]
            .into_iter()
            .flatten()
        {
            check_role(gaps, owner, option.name, None, role, contexts);
        }
    }
}

fn check_role(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    source: &str,
    index: Option<u8>,
    role: ArgRole,
    contexts: Option<EvaluationContextSpec>,
) {
    if !role.needs_evaluation_context() {
        return;
    }
    let declared = contexts.is_some_and(|descriptor| {
        (descriptor.resolver.is_some() && descriptor.resolver_roles.contains(&role))
            || descriptor
                .roles
                .iter()
                .chain(descriptor.fallback_roles.iter())
                .any(|row| row.roles.contains(&role))
            || index.is_some_and(|index| descriptor.arguments.iter().any(|(at, _)| *at == index))
    });
    if !declared {
        gaps.insert(format!(
            "{owner} {source} {} {role:?}",
            index.map_or_else(|| "*".to_owned(), |index| index.to_string())
        ));
    }
}

fn check_deferral(
    gaps: &mut BTreeSet<String>,
    owner: &str,
    traits: Traits,
    execution: Option<tcl_registry::body_execution::BodyExecutionSpec>,
) {
    if traits.contains(Traits::DEFERS_BODY)
        && execution
            .and_then(tcl_registry::body_execution::BodyExecutionSpec::deferred_contract)
            .is_none()
    {
        gaps.insert(format!("{owner} deferred-entry-trigger"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_capabilities_do_not_discharge_unrelated_name_contexts() {
        fn body_only(
            _: tcl_registry::InvocationArguments<'_>,
            _: usize,
        ) -> Option<tcl_registry::evaluation_context::EvaluationContext> {
            None
        }
        let mut gaps = BTreeSet::new();
        let descriptor = EvaluationContextSpec {
            resolver: Some(body_only),
            resolver_roles: &[ArgRole::Body],
            ..EvaluationContextSpec::EMPTY
        };
        let spec = CommandSpec {
            name: "probe",
            arg_roles: &[(0, ArgRole::Body), (1, ArgRole::Name)],
            evaluation_contexts: Some(descriptor),
            ..CommandSpec::DEFAULT
        };
        collect_spec(&mut gaps, "probe", &spec);
        assert_eq!(
            gaps,
            BTreeSet::from(["probe/probe argument 1 Name".to_owned()])
        );
    }

    #[test]
    fn nested_callback_option_is_an_explicit_context_gap() {
        use tcl_registry::hover::{OptionSpec, OptionValue};
        const OPTIONS: &[OptionSpec] = &[OptionSpec {
            name: "-command",
            value: OptionValue::deferred_script(),
            ..OptionSpec::DEFAULT
        }];
        const NESTED: &[tcl_registry::SubSubCommand] = &[tcl_registry::SubSubCommand {
            name: "create",
            options: Some(OPTIONS),
            ..tcl_registry::SubSubCommand::DEFAULT
        }];
        let spec = CommandSpec {
            name: "probe",
            subcommands: &[SubCommand {
                name: "ensemble",
                sub_subcommands: NESTED,
                ..SubCommand::DEFAULT
            }],
            ..CommandSpec::DEFAULT
        };
        let mut gaps = BTreeSet::new();
        collect_spec(&mut gaps, "probe", &spec);
        assert_eq!(
            gaps,
            BTreeSet::from(["probe/probe/ensemble/create -command * Body".to_owned()])
        );
    }

    #[test]
    fn new_unknown_context_is_a_gap_and_authored_context_removes_it() {
        let mut gaps = BTreeSet::new();
        let spec = CommandSpec {
            name: "probe",
            arg_roles: &[(0, ArgRole::Body)],
            ..CommandSpec::DEFAULT
        };
        collect_spec(&mut gaps, "probe", &spec);
        assert_eq!(
            gaps,
            BTreeSet::from(["probe/probe argument 0 Body".to_owned()])
        );
        gaps.clear();
        collect_spec(
            &mut gaps,
            "probe",
            &CommandSpec {
                evaluation_contexts: Some(EvaluationContextSpec::CALLER),
                ..spec
            },
        );
        assert!(gaps.is_empty());
    }
}
