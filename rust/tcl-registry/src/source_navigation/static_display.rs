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

//! Ordered static source display under real authored Registry grammars.

use crate::definer::{MemberEffect, MemberKind, MemberRefKind, TCLOO_GRAMMAR};
use crate::{ArgRole, CommandRegistry, PatternType};

/// The ordering policy of the cross-editor static source display, which a
/// surface registers in its hook table. Rows stay conditional on the actual
/// authored descriptors; the recipe selects no runtime command or definition
/// member. New option rows append after the historical prefix so existing
/// alternatives keep their precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticDisplayRecipe {
    /// Names followed by one symbolic name and a parameter list.
    pub named_callables: &'static [&'static str],
    /// Nameless lifecycle keywords followed by a parameter list.
    pub constructors: &'static [&'static str],
    /// Class source heads and optional fixed subcommand words.
    pub classes: &'static [(&'static str, Option<&'static str>)],
    /// Definition-member words whose operands name classes.
    pub class_references: &'static [&'static str],
    /// Commands with a declared regular-expression pattern language.
    pub regex_commands: &'static [&'static str],
    /// The option-run terminator, left out of the option union.
    pub regex_terminator: &'static str,
    /// The leading regular-expression options, in display order.
    pub regex_options: &'static [&'static str],
}

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

/// Reborrow the installed surfaces' static display recipe only while every row still
/// has its actual descriptor role. A missing/changed prerequisite refuses the
/// complete recipe, rather than silently publishing a shorter grammar. The
/// authored union includes package-gated class spelling as a display policy;
/// it supplies no package population or current-document availability.
#[must_use]
pub fn static_tcl_source_display(registry: &CommandRegistry) -> Option<StaticTclSourceDisplay> {
    let recipe = crate::installed().static_display()?;
    let named_callables = recipe.named_callables.to_vec();
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
    let constructors = recipe.constructors.to_vec();
    if constructors
        .iter()
        .any(|name| !flat_callable(name, None, 0))
    {
        return None;
    }
    let classes = recipe.classes.to_vec();
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
    let class_references = recipe.class_references.to_vec();
    if class_references.iter().any(|name| {
        TCLOO_GRAMMAR.member(name).is_none_or(|member| {
            member.kind != MemberKind::Flat || member.all_args_ref != Some(MemberRefKind::Class)
        })
    }) {
        return None;
    }
    let regex_commands = recipe.regex_commands.to_vec();
    let mut available_options = Vec::new();
    for &command in &regex_commands {
        let mut found = false;
        for spec in registry.specs(command) {
            if spec.pattern_type != Some(PatternType::Regex) {
                continue;
            }
            found = true;
            for option in spec.options {
                if option.name == recipe.regex_terminator {
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
    if recipe
        .regex_options
        .iter()
        .any(|name| !available_options.contains(name))
    {
        return None;
    }
    let mut regex_options = recipe.regex_options.to_vec();
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
        // The union follows the regex commands' order, and `regsub` (the
        // last) brings its own `-command` row (Tcl 9.0), so a row is added to
        // `regsub` for the extension to land after the whole existing union.
        registry.insert(CommandSpec {
            name: "regsub",
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
