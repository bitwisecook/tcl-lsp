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

//! Every dynamic role resolver declares the roles it can produce, swept over each argument shape.

use super::*;

/// Dynamic-role capability declarations are a closed, conservative
/// registry contract.  Source-aware callers cannot run a resolver after
/// expansion, so every emitted role must be declared here instead of
/// assuming a particular command's fallback shape.
/// Every shipped surface, not just the always-loaded Tcl, stdlib, tcllib,
/// Itcl, and Tk catalogue.  Resolver capability metadata is equally
/// load-bearing for optional dialect/package overlays.
fn registry_with_every_resolver_surface() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    for layer in [
        SurfaceLayer::Package("bpf"),
        SurfaceLayer::Core(Family::F5Irules, ""),
        SurfaceLayer::Package("iapps"),
        SurfaceLayer::Package("tmsh"),
        SurfaceLayer::Package("expect"),
        SurfaceLayer::Package("spectcl"),
    ] {
        registry.load_surface(layer);
    }
    registry
}

fn assert_role_inputs_exclusive(registry: &CommandRegistry) {
    for specs in registry.by_name.values() {
        for spec in specs {
            for inputs in std::iter::once([
                spec.arg_role_resolver.is_some(),
                spec.arg_role_count_resolver.is_some(),
                spec.arg_role_layout_resolver.is_some(),
            ])
            .chain(spec.subcommands.iter().map(|sub| {
                [
                    sub.arg_role_resolver.is_some(),
                    sub.arg_role_count_resolver.is_some(),
                    sub.arg_role_layout_resolver.is_some(),
                ]
            })) {
                assert!(
                    inputs.into_iter().filter(|present| *present).count() <= 1,
                    "{} has conflicting role input contracts",
                    spec.name
                );
            }
        }
    }
}

fn assert_count_role_capabilities(registry: &CommandRegistry) {
    for specs in registry.by_name.values() {
        for spec in specs {
            for (resolver, declared) in
                std::iter::once((spec.arg_role_count_resolver, spec.arg_role_resolver_roles)).chain(
                    spec.subcommands
                        .iter()
                        .map(|sub| (sub.arg_role_count_resolver, sub.arg_role_resolver_roles)),
                )
            {
                if let Some(resolver) = resolver {
                    for count in [0, 3, 5, 256] {
                        for (_, role) in resolver(count) {
                            assert!(
                                declared.contains(&role),
                                "count resolver emitted {role:?} for argc {count}, outside {declared:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

fn assert_command_role_capabilities(registry: &CommandRegistry, name: &str, args: &[&str]) {
    let spec = registry.get(name).expect("registered command");
    let roles = if let Some(resolver) = spec.arg_role_count_resolver {
        resolver(args.len())
    } else if let Some(resolver) = spec.arg_role_layout_resolver {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let options = crate::resolved_invocation::invocation_options(
            spec,
            None,
            None,
            crate::resolved_invocation::InvocationAvailability {
                query: Some(profile.surface_query()),
                package_version: None,
            },
        );
        resolver(
            crate::InvocationArguments::literals(args)
                .with_dialect(crate::InvocationDialect::of_profile(profile)),
            options,
        )
        .expect("literal native layout")
    } else if let Some(resolver) = spec.arg_role_resolver {
        resolver(args)
    } else {
        assert!(!spec.has_dynamic_argument_roles());
        return;
    };
    for (_, role) in roles {
        assert!(
            spec.arg_role_resolver_roles.contains(&role),
            "resolver emitted {role:?} for {args:?}, outside {:?}",
            spec.arg_role_resolver_roles,
        );
    }
}

fn assert_subcommand_role_capabilities(
    registry: &CommandRegistry,
    command_name: &str,
    sub_name: &str,
    args: &[&str],
) {
    let spec = registry.get(command_name).expect("registered command");
    let sub = spec.subcommand(sub_name).expect("registered subcommand");
    let roles = if let Some(resolver) = sub.arg_role_count_resolver {
        resolver(args.len())
    } else if let Some(resolver) = sub.arg_role_layout_resolver {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let options = crate::resolved_invocation::invocation_options(
            spec,
            Some(sub),
            None,
            crate::resolved_invocation::InvocationAvailability {
                query: Some(profile.surface_query()),
                package_version: None,
            },
        );
        resolver(
            crate::InvocationArguments::literals(args)
                .with_dialect(crate::InvocationDialect::of_profile(profile)),
            options,
        )
        .expect("literal native layout")
    } else {
        sub.arg_role_resolver.expect("dynamic role resolver")(args)
    };
    for (_, role) in roles {
        assert!(sub.arg_role_resolver_roles.contains(&role));
    }
}

#[test]
fn dynamic_role_capabilities_cover_every_resolver_and_representative_output() {
    let registry = registry_with_every_resolver_surface();
    let mut resolver_count = 0;
    for specs in registry.by_name.values() {
        for spec in specs {
            if spec.has_dynamic_argument_roles() {
                resolver_count += 1;
                assert!(
                    !spec.arg_role_resolver_roles.is_empty(),
                    "{} has a dynamic role resolver without its closed capability set",
                    spec.name
                );
            }
            for sub in spec.subcommands {
                if sub.has_dynamic_argument_roles() {
                    resolver_count += 1;
                    assert!(
                        !sub.arg_role_resolver_roles.is_empty(),
                        "{} {} has a dynamic role resolver without its closed capability set",
                        spec.name,
                        sub.name
                    );
                }
            }
        }
    }
    assert!(
        resolver_count >= 52,
        "the resolver catalogue unexpectedly shrank"
    );

    assert_role_inputs_exclusive(&registry);

    assert_count_role_capabilities(&registry);

    // Representative resolver families, including the multi-role and
    // discriminator-dependent rows most likely to drift.
    assert_command_role_capabilities(&registry, "set", &["name"]);
    assert_command_role_capabilities(&registry, "set", &["name", "value"]);
    assert_command_role_capabilities(
        &registry,
        "regexp",
        &["pattern", "text", "whole", "capture"],
    );
    assert_command_role_capabilities(
        &registry,
        "regsub",
        &["pattern", "text", "replacement", "out"],
    );
    assert_command_role_capabilities(
        &registry,
        "if",
        &["expr", "then", "body", "else", "fallback"],
    );
    assert_command_role_capabilities(&registry, "try", &["body", "on", "0", "result", "handler"]);
    assert_subcommand_role_capabilities(&registry, "binary", "scan", &["bytes", "a*", "out"]);
    assert_subcommand_role_capabilities(
        &registry,
        "dict",
        "update",
        &["d", "key", "local", "body"],
    );
    assert_subcommand_role_capabilities(&registry, "namespace", "which", &["-variable", "name"]);
    assert_subcommand_role_capabilities(&registry, "namespace", "which", &["-command", "name"]);
    assert_subcommand_role_capabilities(
        &registry,
        "trace",
        "add",
        &["variable", "name", "write", "callback"],
    );
    // A clause grammar took the retired `if` / `try` resolvers' place; its
    // walk's flat roles stay inside the same closed capability set.
    let check_grammar = |name: &str, args: &[&str]| {
        let spec = registry.get(name).expect("command");
        let plan = spec.clause_plan(args, None).expect("a clause grammar");
        for (_, role) in plan.roles {
            assert!(
                spec.arg_role_resolver_roles.contains(&role)
                    || spec.arg_roles.iter().any(|(_, found)| *found == role),
                "{name}'s grammar emitted {role:?} for {args:?}, outside its declared roles"
            );
        }
    };
    check_grammar("if", &["expr", "then", "body", "else", "fallback"]);
    check_grammar("try", &["body", "on", "0", "result", "handler"]);
}

/// A clause grammar's flat roles are part of `may_have_arg_role`'s answer,
/// so every role a grammar can assign is one its spec already declares —
/// in the resolver's closed set (the set a grammar that replaced a
/// resolver keeps) or in the static table. Otherwise an expansion-blocked
/// call would abstain differently from a literal one.
#[test]
fn clause_grammar_roles_are_declared_capabilities() {
    let registry = registry_with_every_resolver_surface();
    let mut grammars = 0;
    let declared = |roles: &[ArgRole], table: &[(u8, ArgRole)], role: ArgRole| {
        roles.contains(&role) || table.iter().any(|(_, found)| *found == role)
    };
    for specs in registry.by_name.values() {
        for spec in specs {
            let owners = std::iter::once((
                spec.name.to_owned(),
                spec.clause_grammar,
                spec.arg_role_resolver_roles,
                spec.arg_roles,
            ))
            .chain(spec.subcommands.iter().map(|sub| {
                (
                    format!("{} {}", spec.name, sub.name),
                    sub.clause_grammar,
                    sub.arg_role_resolver_roles,
                    sub.arg_roles,
                )
            }));
            for (owner, grammar, roles, table) in owners {
                let Some(grammar) = grammar else { continue };
                grammars += 1;
                for &role in ArgRole::ALL {
                    assert!(
                        !grammar.may_assign(role) || declared(roles, table, role),
                        "{owner}'s clause grammar can assign {role:?}, which it does not declare"
                    );
                }
            }
        }
    }
    assert!(
        grammars >= 11,
        "the clause-grammar catalogue unexpectedly shrank"
    );
}

/// The representative rows in the sibling test are hand-picked, so a
/// resolver nobody thought to list stays unchecked — `control::do` emitted
/// `ArgRole::Expr` without declaring it for exactly that reason (#2068).
/// Sweep every resolver instead: feed each one the literals its own spec
/// knows about (option names, `arg_values` words, sibling subcommand
/// names), at every position of every arity up to the widest form a
/// resolver in this tree inspects, and require the closed capability set
/// to cover the result.
#[test]
fn dynamic_role_capabilities_cover_every_resolver_argument_shape() {
    let registry = registry_with_every_resolver_surface();
    let mut swept = 0;
    for specs in registry.by_name.values() {
        for spec in specs {
            if let Some(resolver) = spec.arg_role_count_resolver {
                swept += 1;
                for count in 0..=6 {
                    assert!(
                        resolver(count)
                            .iter()
                            .all(|(_, role)| spec.arg_role_resolver_roles.contains(role))
                    );
                }
            }
            if let Some(resolver) = spec.arg_role_layout_resolver {
                swept += 1;
                sweep_layout_resolver(spec, None, resolver);
            }
            if let Some(resolver) = spec.arg_role_resolver {
                swept += 1;
                sweep_resolver(
                    resolver,
                    spec.arg_role_resolver_roles,
                    &resolver_literals(spec, None),
                    spec.name,
                );
            }
            for sub in spec.subcommands {
                if let Some(resolver) = sub.arg_role_count_resolver {
                    swept += 1;
                    for count in 0..=6 {
                        assert!(
                            resolver(count)
                                .iter()
                                .all(|(_, role)| sub.arg_role_resolver_roles.contains(role))
                        );
                    }
                }
                if let Some(resolver) = sub.arg_role_layout_resolver {
                    swept += 1;
                    sweep_layout_resolver(spec, Some(sub), resolver);
                }
                if let Some(resolver) = sub.arg_role_resolver {
                    swept += 1;
                    sweep_resolver(
                        resolver,
                        sub.arg_role_resolver_roles,
                        &resolver_literals(spec, Some(sub)),
                        &format!("{} {}", spec.name, sub.name),
                    );
                }
            }
        }
    }
    // The same floor the capability test asserts, so the two stay in
    // step: a resolver added without a capability set fails there, and
    // one whose emitted roles drift fails here.
    assert!(swept >= 52, "the resolver catalogue unexpectedly shrank");
}

fn sweep_layout_resolver(
    spec: &'static crate::CommandSpec,
    sub: Option<&'static crate::spec::SubCommand>,
    resolver: crate::spec::ArgRoleLayoutResolver,
) {
    let declared = sub.map_or(spec.arg_role_resolver_roles, |sub| {
        sub.arg_role_resolver_roles
    });
    let options = crate::resolved_invocation::invocation_options(
        spec,
        sub,
        None,
        crate::resolved_invocation::InvocationAvailability::default(),
    );
    let literals = resolver_literals(spec, sub);
    for len in 0..=6 {
        let mut words = vec![crate::InvocationWord::Dynamic; len];
        for position in 0..len {
            for literal in &literals {
                words[position] = crate::InvocationWord::Literal(literal);
                if let Some(roles) =
                    resolver(crate::InvocationArguments::structured(&words), options)
                {
                    assert!(
                        roles.iter().all(|(_, role)| declared.contains(role)),
                        "{}: {words:?} -> {roles:?}",
                        spec.name
                    );
                }
            }
            words[position] = crate::InvocationWord::Dynamic;
        }
    }
}

/// The literal words a resolver is plausibly handed: the option names and
/// `arg_values` of the spec it belongs to, plus its sibling subcommand
/// names, which discriminator-dependent resolvers (`trace add`,
/// `namespace which`) branch on.  Deduplicated and bounded so the sweep
/// stays a fast unit test.
fn resolver_literals(
    spec: &'static crate::CommandSpec,
    sub: Option<&'static crate::spec::SubCommand>,
) -> Vec<&'static str> {
    let mut out: BTreeSet<&'static str> = BTreeSet::new();
    let collect = |options: &'static [crate::hover::OptionSpec],
                   arg_values: &'static [(u8, &'static [crate::hover::ArgValue])],
                   out: &mut BTreeSet<&'static str>| {
        for option in options {
            out.insert(option.name);
        }
        for (_, values) in arg_values {
            for value in *values {
                out.insert(value.value);
            }
        }
    };
    collect(spec.options, spec.arg_values, &mut out);
    if let Some(sub) = sub {
        collect(sub.options, sub.arg_values, &mut out);
    }
    for sibling in spec.subcommands {
        out.insert(sibling.name);
    }
    // A bare word and a lone dash stand in for "some value" and "an
    // option-shaped word the table does not know".
    out.insert("x");
    out.insert("-");
    out.into_iter().take(48).collect()
}

/// Call `resolver` with every one-literal substitution into an
/// all-placeholder argument vector, for each arity up to `MAX_WORDS`, and
/// assert each emitted role is declared.
fn sweep_resolver(
    resolver: ArgRoleResolver,
    declared: &[ArgRole],
    literals: &[&'static str],
    label: &str,
) {
    // The widest form any resolver in this tree inspects is `trace add
    // variable name ops callback` plus a trailing word; six covers it
    // with room to spare, and a resolver that only reads a prefix is
    // exercised by the shorter arities in the same loop.
    const MAX_WORDS: usize = 6;
    let check = |args: &[&str]| {
        for (_, role) in resolver(args) {
            assert!(
                declared.contains(&role),
                "`{label}`'s resolver emitted {role:?} for {args:?}, outside its declared {declared:?}"
            );
        }
    };
    for len in 0..=MAX_WORDS {
        let base = vec!["x"; len];
        check(&base);
        for position in 0..len {
            for literal in literals {
                let mut args = base.clone();
                args[position] = literal;
                check(&args);
            }
        }
    }
}
