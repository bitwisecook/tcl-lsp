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

//! When a script argument runs: script timing per argument, callback script indices and callback taint inputs.

use super::CommandRegistry;
use super::argument_roles::push_option_value_roles;
use crate::InvocationWords;
use crate::arg_role::ArgRole;
use crate::hover::CallbackTaintInput;
use crate::resolved_invocation::{ResolvedInvocation, SubcommandResolution};
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;
use tcl_dialect::model::SurfaceQuery;

/// Return the timing of the script-valued option that consumes `index`.
///
/// The scan deliberately mirrors [`push_option_value_roles`]: aliases and
/// unique abbreviations resolve through the same table, and value arity / `--`
/// handling comes from [`OptionSpec::value_indices`].  `None` means either that
/// `index` is not an option value or that the consuming value is not a script.
fn option_script_timing_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<crate::hover::ScriptTiming> {
    option_script_timing_at_selected(
        &options.iter().collect::<Vec<_>>(),
        args,
        scan_start,
        index,
        crate::abbrev::PrefixMatching::Enabled,
    )
}

fn option_script_timing_at_selected(
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
    prefix_matching: crate::abbrev::PrefixMatching,
) -> Option<crate::hover::ScriptTiming> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return opt.value_script_timing();
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

/// Return the external callback substitutions declared by the option value
/// that consumes `index`. The empty slice is a meaningful answer: the option
/// is a callback, but it carries no user-controlled replacement value.
fn option_callback_taint_inputs_at(
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
) -> Option<&'static [CallbackTaintInput]> {
    option_callback_taint_inputs_at_selected(
        &options.iter().collect::<Vec<_>>(),
        args,
        scan_start,
        index,
        crate::abbrev::PrefixMatching::Enabled,
    )
}

fn option_callback_taint_inputs_at_selected(
    options: &[&crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    index: usize,
    prefix_matching: crate::abbrev::PrefixMatching,
) -> Option<&'static [CallbackTaintInput]> {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) =
            crate::spec::resolve_available_option_prefix_with(options, args[i], prefix_matching)
        {
            let vals = opt.value_indices(args, i);
            if vals.contains(&index) {
                return Some(opt.value_callback_taint_inputs());
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
    None
}

impl CommandRegistry {
    /// When the script at 0-based argument `index` runs relative to this
    /// concrete invocation.
    ///
    /// Script-valued options carry an exact per-value [`ScriptTiming`] fact, so
    /// one invocation may safely mix immediate, deferred, and reference-only
    /// executable text. A command or subcommand [`ScriptTimingResolver`]
    /// supplies the same exact fact for positional forms whose timing depends
    /// on their written arguments.
    /// Invocation-wide [`Traits::DEFERS_BODY`] remains the compatibility
    /// fallback. `None` means `index` is not a supplied executable position.
    ///
    /// [`ScriptTiming`]: crate::hover::ScriptTiming
    /// [`ScriptTimingResolver`]: crate::spec::ScriptTimingResolver
    #[must_use]
    pub fn script_timing(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::ScriptTiming> {
        let resolved = self.resolve_invocation(name, args, dialect)?;
        let spec = if dialect.is_none() {
            self.get(resolved.canonical_command)?
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)?
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        if !spec.subcommands.is_empty()
            && sub.is_none()
            && resolved.subcommand != SubcommandResolution::NotApplicable
        {
            return None;
        }
        let prefix = self
            .command_prefixes(name, args)
            .iter()
            .any(|(at, _)| *at == index);
        Self::script_timing_for_resolved(&resolved, spec, sub, args, index, prefix)
    }

    /// Script timing of an explicitly selected instance method. The caller
    /// supplies its receiver class; this metadata query does not prove the
    /// receiver's physical class or live command identity.
    #[must_use]
    pub fn instance_script_timing(
        &self,
        class_name: &str,
        receiver: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::ScriptTiming> {
        let resolved = self.resolve_instance_invocation(class_name, receiver, args, dialect)?;
        let spec =
            self.get_for_surface(class_name, dialect.or_else(|| self.own_surface_query()))?;
        let selected = resolved.subcommand.resolved()?;
        let method = self.instance_method_at(
            class_name,
            selected.canonical_name,
            self.package_floor_for_spec(spec),
            dialect.or_else(|| self.own_surface_query()),
        )?;
        let prefix = self
            .instance_method_command_prefixes(
                class_name,
                selected.canonical_name,
                args.get(1..).unwrap_or_default(),
            )
            .iter()
            .any(|(at, _)| at.checked_add(1) == Some(index));
        Self::script_timing_for_resolved(&resolved, spec, Some(method), args, index, prefix)
    }

    fn script_timing_for_resolved(
        resolved: &ResolvedInvocation<'_, '_>,
        spec: &CommandSpec,
        sub: Option<&SubCommand>,
        args: &[&str],
        index: usize,
        command_prefix: bool,
    ) -> Option<crate::hover::ScriptTiming> {
        let prefixes = if command_prefix {
            vec![index]
        } else {
            Vec::new()
        };
        Self::script_timings_for_resolved(resolved, spec, sub, args, &[index], &prefixes)
            .pop()
            .flatten()
    }

    /// [`Self::script_timing_for_resolved`] for each of `indices`, resolving
    /// the command's roles, timing resolver and options once: per index they
    /// cost a whole command's work, so a long literal command was quadratic
    /// (#2422). `prefixes` are the indices that hold a command prefix.
    fn script_timings_for_resolved(
        resolved: &ResolvedInvocation<'_, '_>,
        spec: &CommandSpec,
        sub: Option<&SubCommand>,
        args: &[&str],
        indices: &[usize],
        prefixes: &[usize],
    ) -> Vec<Option<crate::hover::ScriptTiming>> {
        let Some(scan_start) = resolved
            .semantics
            .argument_offset
            .checked_add(if sub.is_none() {
                spec.constructor_prefix_words().unwrap_or(0)
            } else {
                0
            })
        else {
            return vec![None; indices.len()];
        };
        let mut roles: Vec<_> = resolved
            .argument_roles()
            .0
            .into_iter()
            .map(|(position, role)| {
                (
                    resolved.semantics.argument_offset + usize::from(position),
                    role,
                )
            })
            .collect();
        push_option_value_roles(
            &mut roles,
            &resolved.semantics.options.available().collect::<Vec<_>>(),
            args,
            scan_start,
            &|_| true,
            resolved.semantics.options.prefix_matching,
        );
        let executable: std::collections::HashSet<usize> = roles
            .iter()
            .filter(|(_, role)| role.has_script_timing())
            .map(|&(position, _)| position)
            .chain(prefixes.iter().copied())
            .collect();
        let (timing_resolver, offset) = sub.map_or((spec.script_timing_resolver, 0usize), |sub| {
            (sub.script_timing_resolver, 1usize)
        });
        let mut resolved_timings: Option<Vec<(usize, crate::hover::ScriptTiming)>> = None;
        let options = resolved.semantics.options.available().collect::<Vec<_>>();
        indices
            .iter()
            .map(|&index| {
                if !executable.contains(&index) {
                    return None;
                }
                if let Some(resolve) = timing_resolver
                    && let Some(relative) = index.checked_sub(offset)
                    && let Some(&(_, timing)) = resolved_timings
                        .get_or_insert_with(|| {
                            resolve(crate::InvocationArguments::literals(
                                args.get(offset..).unwrap_or(&[]),
                            ))
                            .into_iter()
                            .map(|(position, timing)| (usize::from(position), timing))
                            .collect()
                        })
                        .iter()
                        .find(|(position, _)| *position == relative)
                {
                    return Some(timing);
                }
                // An invocation-sensitive resolver is the exact fact and
                // therefore precedes an option's static default. This matters
                // for APIs such as `bibtex::parse`: the same callback option
                // runs synchronously for an in-memory input but is stored when
                // `-channel` selects background parsing. Resolver silence
                // deliberately falls through to the option.
                if let Some(timing) = option_script_timing_at_selected(
                    &options,
                    args,
                    scan_start,
                    index,
                    resolved.semantics.options.prefix_matching,
                ) {
                    return Some(timing);
                }
                Some(if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
                    crate::hover::ScriptTiming::Deferred
                } else {
                    crate::hover::ScriptTiming::SameInvocation
                })
            })
            .collect()
    }

    /// The 0-based argument positions of this invocation that hold a script,
    /// command prefix or lambda the command stores as a **callback**: to run
    /// after it returns, at the global level or in the frame of whatever
    /// fires it, where a plain variable name can be one the registering code
    /// holds.
    ///
    /// The positions [`Self::script_timing`] calls
    /// [`Deferred`](crate::hover::ScriptTiming::Deferred), less those of a
    /// command that stores a definition
    /// ([`Traits::BODY_RUNS_IN_OWN_FRAME`]): a `proc` body is dormant too, but
    /// runs in a frame of its own. A command that runs the script now
    /// ([`SameInvocation`](crate::hover::ScriptTiming::SameInvocation)) or
    /// only names a registration (`trace remove`,
    /// [`ReferenceOnly`](crate::hover::ScriptTiming::ReferenceOnly)) has none.
    /// Indices count as [`Self::script_timing`] counts them, the resolved
    /// subcommand word included.
    #[must_use]
    pub fn callback_script_indices(
        &self,
        name: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Vec<usize> {
        if self
            .invocation_traits(name, args, dialect)
            .contains(Traits::BODY_RUNS_IN_OWN_FRAME)
        {
            return Vec::new();
        }
        // One resolution for every word, as `script_timing` would resolve it
        // per word (#2422).
        let Some(resolved) = self.resolve_invocation(name, args, dialect) else {
            return Vec::new();
        };
        let spec = if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        };
        let Some(spec) = spec else {
            return Vec::new();
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        if !spec.subcommands.is_empty()
            && sub.is_none()
            && resolved.subcommand != SubcommandResolution::NotApplicable
        {
            return Vec::new();
        }
        let prefixes: Vec<usize> = self
            .command_prefixes(name, args)
            .iter()
            .map(|(at, _)| *at)
            .collect();
        let indices: Vec<usize> = (0..args.len()).collect();
        Self::script_timings_for_resolved(&resolved, spec, sub, args, &indices, &prefixes)
            .into_iter()
            .zip(indices)
            .filter(|(timing, _)| *timing == Some(crate::hover::ScriptTiming::Deferred))
            .map(|(_, index)| index)
            .collect()
    }

    /// User-controlled values that this concrete deferred callback host
    /// substitutes into argument `index` before Tcl evaluates it.
    ///
    /// This is deliberately a registry query rather than a Tk-specific
    /// compiler table. It unifies positional callback bodies (`bind`) and
    /// option values (`entry -validatecommand`), and only returns a non-empty
    /// list when the selected executable position is declared to receive an
    /// external value. Framework metadata is not inferred as taint.
    #[must_use]
    pub fn callback_taint_inputs(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_invocation(name, args, dialect) else {
            return &[];
        };
        let Some(spec) = (if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        }) else {
            return &[];
        };
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let (table, offset) = sub.map_or((spec.callback_taint_inputs, 0usize), |sub| {
            (sub.callback_taint_inputs, 1usize)
        });
        let options = resolved.semantics.options.base;
        let scan_start = resolved.semantics.argument_offset;
        if let Some(inputs) = option_callback_taint_inputs_at(options, args, scan_start, index) {
            return if option_script_timing_at(options, args, scan_start, index)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }
        if self
            .script_timing(name, args, index, dialect)
            .is_none_or(|timing| timing != crate::hover::ScriptTiming::Deferred)
        {
            return &[];
        }
        let Some(relative) = index.checked_sub(offset) else {
            return &[];
        };
        table
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// Callback inputs selected from evaluated word knowledge. An unknown
    /// captured value preserves its argv slot; it is never an empty literal.
    /// Positional callbacks use the complete structured role selection. Option
    /// callbacks retain the existing exact layout query when all values are
    /// available; unresolved option layout abstains.
    #[must_use]
    pub fn callback_taint_inputs_words(
        &self,
        words: InvocationWords<'_>,
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let words = words.with_profile(self.profile());
        let Some(name) = words.head_literal() else {
            return &[];
        };
        let arguments = words.arguments();
        if !arguments.has_exact_argv_len() {
            return &[];
        }
        let Some(resolved) = self
            .resolve_structured_invocation(words, dialect)
            .resolved()
        else {
            return &[];
        };
        let Some(spec) = (if dialect.is_none() {
            self.get(resolved.canonical_command)
        } else {
            self.get_for_surface(resolved.canonical_command, dialect)
        }) else {
            return &[];
        };
        if resolved
            .argument_count_for_arity()
            .is_none_or(|count| !resolved.semantics.arity.accepts(count))
        {
            return &[];
        }
        let (roles, complete) = resolved.argument_roles();
        let positional_body = complete
            && roles.iter().any(|(at, role)| {
                resolved.semantics.argument_offset + usize::from(*at) == index
                    && *role == ArgRole::Body
            });
        let sub = resolved
            .subcommand
            .resolved()
            .and_then(|subcommand| spec.subcommand(subcommand.canonical_name));
        let timing_resolver = sub.map_or(spec.script_timing_resolver, |sub| {
            sub.script_timing_resolver
        });
        if positional_body
            && resolved.semantics.traits.contains(Traits::DEFERS_BODY)
            && timing_resolver.is_none()
            && resolved.semantics.options.base.is_empty()
        {
            let (table, offset) = sub.map_or((spec.callback_taint_inputs, 0usize), |sub| {
                (sub.callback_taint_inputs, 1usize)
            });
            if let Some(relative) = index.checked_sub(offset)
                && let Some((_, inputs)) = table.iter().find(|(at, _)| usize::from(*at) == relative)
            {
                return inputs;
            }
        }
        let Some(literals) = (0..arguments.len())
            .map(|at| arguments.literal_at(at))
            .collect::<Option<Vec<_>>>()
        else {
            return &[];
        };
        self.callback_taint_inputs(name, &literals, index, dialect)
    }

    /// User-controlled values that a resolved instance method substitutes
    /// into argument `index` before evaluating its deferred callback.
    ///
    /// `args` starts with the instance method word. The query is the
    /// object-dispatch counterpart of [`Self::callback_taint_inputs`]: it
    /// resolves the receiver class and method through the same registry table,
    /// then consults the method's positional callback table or its declared
    /// options. A method form carrying [`Traits::CONFIGURES_INSTANCE_OPTIONS`]
    /// inherits the owning class's option table, which is how widget
    /// `configure` setters expose constructor options without a command-name
    /// branch. Unknown/dynamic methods and non-deferred positions abstain.
    #[must_use]
    pub fn instance_callback_taint_inputs(
        &self,
        class_name: &str,
        receiver: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let Some(resolved) = self.resolve_instance_invocation(class_name, receiver, args, dialect)
        else {
            return &[];
        };
        let Some(method_spelling) = args.first() else {
            return &[];
        };
        let Some(method) = self.instance_method(class_name, method_spelling) else {
            return &[];
        };
        let Some(relative) = index.checked_sub(1) else {
            return &[];
        };

        let options = resolved.semantics.options.available().collect::<Vec<_>>();
        let matching = resolved.semantics.options.prefix_matching;
        let option_inputs =
            option_callback_taint_inputs_at_selected(&options, args, 1, index, matching);
        if let Some(inputs) = option_inputs {
            return if option_script_timing_at_selected(&options, args, 1, index, matching)
                == Some(crate::hover::ScriptTiming::Deferred)
            {
                inputs
            } else {
                &[]
            };
        }

        let timing = method
            .script_timing_resolver
            .and_then(|resolve| {
                resolve(crate::InvocationArguments::literals(
                    args.get(1..).unwrap_or(&[]),
                ))
                .into_iter()
                .find_map(|(at, timing)| (usize::from(at) == relative).then_some(timing))
            })
            .unwrap_or_else(|| {
                if resolved.semantics.traits.contains(Traits::DEFERS_BODY) {
                    crate::hover::ScriptTiming::Deferred
                } else {
                    crate::hover::ScriptTiming::SameInvocation
                }
            });
        if timing != crate::hover::ScriptTiming::Deferred {
            return &[];
        }
        method
            .callback_taint_inputs
            .iter()
            .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
            .unwrap_or(&[])
    }

    /// Deferred inputs for an explicitly selected instance with frozen argv.
    /// Unknown value bytes retain their operand slots; unknown method/option
    /// selection or expansion supplies no callback input contract.
    #[must_use]
    pub fn instance_callback_taint_inputs_words(
        &self,
        class_name: &str,
        words: InvocationWords<'_>,
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> &'static [CallbackTaintInput] {
        let words = words.with_profile(self.profile());
        let Some(resolved) =
            self.resolve_structured_instance_invocation(class_name, words, dialect)
        else {
            return &[];
        };
        if resolved
            .argument_count_for_arity()
            .is_none_or(|count| !resolved.semantics.arity.accepts(count))
        {
            return &[];
        }
        let Some(selected) = resolved.subcommand.resolved() else {
            return &[];
        };
        let query = dialect.or_else(|| {
            words
                .arguments()
                .dialect()
                .and_then(tcl_native_model::InvocationDialect::authoring_query)
        });
        let Some(spec) = self.get_for_surface(class_name, query) else {
            return &[];
        };
        let Some(method) = self.instance_method_at(
            class_name,
            selected.canonical_name,
            self.package_floor_for_spec(spec),
            query,
        ) else {
            return &[];
        };
        let (roles, complete) = resolved.argument_roles();
        if complete
            && method.script_timing_resolver.is_none()
            && resolved.semantics.options.base.is_empty()
            && resolved.semantics.traits.contains(Traits::DEFERS_BODY)
            && roles.iter().any(|(at, role)| {
                *role == ArgRole::Body
                    && usize::from(*at) + resolved.semantics.argument_offset == index
            })
        {
            return index
                .checked_sub(resolved.semantics.argument_offset)
                .and_then(|relative| {
                    method
                        .callback_taint_inputs
                        .iter()
                        .find_map(|(at, inputs)| (usize::from(*at) == relative).then_some(*inputs))
                })
                .unwrap_or_default();
        }
        let Some(args) = (0..words.arguments().len())
            .map(|at| words.arguments().literal_at(at))
            .collect::<Option<Vec<_>>>()
        else {
            return &[];
        };
        self.instance_callback_taint_inputs(
            class_name,
            words.head_literal().unwrap_or_default(),
            &args,
            index,
            dialect,
        )
    }

    /// Variable frame of the option value at 0-based argument `index`.
    ///
    /// `None` means that `index` is not a variable-name option value. The
    /// returned scope is registry data and therefore applies uniformly to
    /// lowering, SSA, and taint analysis without command-name branches.
    #[must_use]
    pub fn option_variable_scope(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::hover::VariableScope> {
        let words = crate::InvocationWords::literals(name, args).with_profile(self.profile());
        self.resolve_structured_invocation(words, dialect)
            .resolved()?
            .authored_source_option_variable_scope_at(index)
    }
}
