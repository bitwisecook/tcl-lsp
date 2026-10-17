// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ordered static source display under real authored Registry grammars.

use crate::commands::tcl::{
    STATIC_DISPLAY_CLASS_REFERENCES, STATIC_DISPLAY_CLASSES, STATIC_DISPLAY_CONSTRUCTORS,
    STATIC_DISPLAY_NAMED_CALLABLES, STATIC_DISPLAY_REGEX_COMMANDS, STATIC_DISPLAY_REGEX_OPTIONS,
    STATIC_DISPLAY_REGEX_TERMINATOR,
};
use crate::definer::{MemberEffect, MemberKind, MemberRefKind, TCLOO_GRAMMAR};
use crate::{ArgRole, CommandRegistry, PatternType};

/// Ordered cross-editor display candidates. These are static source shapes,
/// without document availability, selected handlers, members, frames or cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticTclSourceDisplay {
    /// Names immediately followed by one symbolic name and a parameter list.
    pub named_callables: Vec<&'static str>,
    /// Nameless lifecycle keywords immediately followed by a parameter list.
    pub constructors: Vec<&'static str>,
    /// Class source heads and optional fixed subcommand words.
    pub classes: Vec<(&'static str, Option<&'static str>)>,
    /// Definition-member words whose operands name classes.
    pub class_references: Vec<&'static str>,
    /// Commands with a declared regular-expression pattern language.
    pub regex_commands: Vec<&'static str>,
    /// Union of their actual option descriptors, in authored display order.
    pub regex_options: Vec<&'static str>,
}

fn flat_callable(keyword: &str, name: Option<u8>, params: u8) -> bool {
    TCLOO_GRAMMAR.member(keyword).is_some_and(|member| {
        member.kind == MemberKind::Flat
            && matches!(member.effect, MemberEffect::Callable {
                name_slot, params_slot: Some(slot), ..
            } if name_slot == name && slot == params)
    })
}

/// Reborrow the existing Tcl static display recipe only while every row still
/// has its actual descriptor role. A missing/changed prerequisite refuses the
/// complete recipe, rather than silently publishing a shorter grammar. The
/// authored union includes package-gated class spelling as a display policy;
/// it supplies no package population or current-document availability.
#[must_use]
pub fn static_tcl_source_display(registry: &CommandRegistry) -> Option<StaticTclSourceDisplay> {
    let named_callables = STATIC_DISPLAY_NAMED_CALLABLES.to_vec();
    for &name in &named_callables {
        let procedure = registry.specs(name).iter().any(|spec| {
            spec.arg_roles.contains(&(0, ArgRole::Name))
                && spec.arg_roles.contains(&(1, ArgRole::ParamList))
                && spec.arg_roles.contains(&(2, ArgRole::Body))
        });
        if !procedure && !flat_callable(name, Some(0), 1) {
            return None;
        }
    }
    let constructors = STATIC_DISPLAY_CONSTRUCTORS.to_vec();
    if constructors
        .iter()
        .any(|name| !flat_callable(name, None, 0))
    {
        return None;
    }
    let classes = STATIC_DISPLAY_CLASSES.to_vec();
    for &(command, subcommand) in &classes {
        if !registry.specs(command).iter().any(|spec| {
            spec.definition_body.is_some()
                && subcommand.map_or_else(
                    || spec.arg_roles.contains(&(0, ArgRole::Name)),
                    |name| {
                        spec.subcommands.iter().any(|sub| {
                            sub.name == name
                                && sub.arg_roles.contains(&(0, ArgRole::Name))
                                && sub.arg_roles.contains(&(1, ArgRole::Body))
                        })
                    },
                )
        }) {
            return None;
        }
    }
    let class_references = STATIC_DISPLAY_CLASS_REFERENCES.to_vec();
    if class_references.iter().any(|name| {
        TCLOO_GRAMMAR.member(name).is_none_or(|member| {
            member.kind != MemberKind::Flat || member.all_args_ref != Some(MemberRefKind::Class)
        })
    }) {
        return None;
    }
    let regex_commands = STATIC_DISPLAY_REGEX_COMMANDS.to_vec();
    let mut available_options = Vec::new();
    for &command in &regex_commands {
        let mut found = false;
        for spec in registry.specs(command) {
            if spec.pattern_type != Some(PatternType::Regex) {
                continue;
            }
            found = true;
            for option in spec.options {
                if option.name == STATIC_DISPLAY_REGEX_TERMINATOR {
                    continue;
                }
                if !available_options.contains(&option.name) {
                    available_options.push(option.name);
                }
            }
        }
        if !found {
            return None;
        }
    }
    if STATIC_DISPLAY_REGEX_OPTIONS
        .iter()
        .any(|name| !available_options.contains(name))
    {
        return None;
    }
    let mut regex_options = STATIC_DISPLAY_REGEX_OPTIONS.to_vec();
    for option in available_options {
        if !regex_options.contains(&option) {
            regex_options.push(option);
        }
    }
    Some(StaticTclSourceDisplay {
        named_callables,
        constructors,
        classes,
        class_references,
        regex_commands,
        regex_options,
    })
}

#[cfg(test)]
mod tests {
    use super::static_tcl_source_display;
    use crate::{CommandRegistry, CommandSpec, PatternType};

    #[test]
    fn static_display_extends_real_regex_option_rows_without_reordering_existing_union() {
        let mut registry = CommandRegistry::build_default();
        let before = static_tcl_source_display(&registry).expect("authored source display");
        registry.insert(CommandSpec {
            name: "regexp",
            pattern_type: Some(PatternType::Regex),
            options: &[crate::hover::OptionSpec {
                name: "-sourcefuture",
                ..crate::hover::OptionSpec::DEFAULT
            }],
            ..CommandSpec::DEFAULT
        });
        let after = static_tcl_source_display(&registry).unwrap();
        assert_eq!(
            &after.regex_options[..before.regex_options.len()],
            &before.regex_options
        );
        assert_eq!(after.regex_options.last(), Some(&"-sourcefuture"));
        registry.insert(CommandSpec {
            name: "unrelated_source_pattern",
            pattern_type: Some(PatternType::Regex),
            options: &[crate::hover::OptionSpec {
                name: "-foreignsource",
                ..crate::hover::OptionSpec::DEFAULT
            }],
            ..CommandSpec::DEFAULT
        });
        assert_eq!(static_tcl_source_display(&registry), Some(after));
    }
}
