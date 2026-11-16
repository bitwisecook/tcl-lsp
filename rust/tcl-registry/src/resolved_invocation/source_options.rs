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

//! Options in the authored source: diagnostics, teardown and destructive-path targets, credentials, files and option relationships.

use super::ResolvedInvocation;
use crate::hover::OptionSpec;
use crate::traits::Traits;
use tcl_dialect::model::SpecSurface;

/// Selected original credential metadata, independent of runtime values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredSourceCredentialPosition {
    /// Canonical available option whose single value carries a credential.
    Option(&'static str),
    /// Effective post-head ordinal of the original sensitive header name.
    Header(usize),
}

/// One original operand selected by an existing credential descriptor.
/// It supplies source advice, not exposure, evaluation or rewrite authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredSourceCredentialArgument {
    /// Effective post-head value ordinal, including captured alias operands.
    pub argument: usize,
    /// Descriptor-selected option or original sensitive header position.
    pub position: AuthoredSourceCredentialPosition,
}

/// One declared option selected for readonly source diagnostics.
/// Availability remains separate so excluded exact spellings can be explained.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOption<'r> {
    /// Effective post-head ordinal, including captured selector operands.
    pub argument: usize,
    /// Descriptor selected by the shared exact/prefix vocabulary.
    pub option: &'r OptionSpec,
    /// Actual retained surface and package floor admit this descriptor.
    pub available: bool,
    /// Inherited surface for lifecycle and declared-target diagnostics.
    pub surface: Option<&'static [SpecSurface]>,
    /// Effective ordinals of present value words; unknown width or missing
    /// values retain `None` and stop further option interpretation.
    pub values: Option<std::ops::Range<usize>>,
}

/// The first boundary where a selected source option scan stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredSourceOptionBoundary {
    /// An ordinary data word, whose value can remain dynamic.
    Positional(usize),
    /// The next data-or-option word has no original literal value.
    Dynamic(usize),
    /// An explicitly written or captured terminator.
    Terminator(usize),
    /// A literal option spelling outside the selected vocabulary.
    Unknown(usize),
    /// An ambiguous spelling in the actual admitted option vocabulary.
    Ambiguous {
        /// Effective post-head ordinal of the ambiguous spelling.
        argument: usize,
        /// Canonical candidates from the shared keyword resolver.
        candidates: Vec<&'static str>,
    },
    /// No data operand follows the known option prefix.
    End,
    /// Dynamic selection or value width prevents further interpretation.
    Indeterminate,
}

/// Possible teardown targets in the already selected Registry source schema.
/// These ordinals identify effective arguments, not successful effects or
/// runtime targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceTeardownArguments {
    /// Effective post-head ordinals under the selected descriptor's layout.
    pub arguments: std::ops::Range<usize>,
}

/// Possible destructive path operands in an authored source schema.
/// These coordinates describe warning candidates, not accepted handler argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceDestructivePathArguments {
    /// Effective post-head ordinals, including captured alias operands.
    pub arguments: std::ops::Range<usize>,
    /// Selected selector path for diagnostic presentation only.
    pub subcommands: Vec<&'static str>,
    /// A dynamic boundary can be data or an option of unknown width. Only
    /// that first candidate is retained; later operands are not classified.
    pub selection_unknown: bool,
}

/// Source option topology from one selected descriptor and actual argv.
/// This supplies neither handler acceptance nor data value or edit authority.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOptionScan<'r> {
    /// Known prefix observations before any unresolved boundary.
    pub options: Vec<AuthoredSourceOption<'r>>,
    /// The selected vocabulary declares an admitted `--` terminator.
    pub accepts_terminator: bool,
    /// Already selected canonical selector path used in reporting.
    pub subcommands: Vec<&'static str>,
    /// Exact reason and coordinate where option interpretation stops.
    pub boundary: AuthoredSourceOptionBoundary,
}

/// Possible option operands from one selected source descriptor and actual
/// argument layout. These ordinals grant no evaluated value, accepted handler,
/// successful completion or editable source correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSourceOptionArguments {
    /// Effective post-head ordinals that can be parsed as options.
    pub arguments: Vec<usize>,
    /// Already selected canonical selector path, for presentation only.
    pub subcommands: Vec<&'static str>,
}

/// Relationship grammar and source facts from the same selected option scan.
/// Effective ordinals are distinct from original written anchors. Incomplete
/// source facts cannot prove missing options or actual runtime values.
#[derive(Debug, Clone)]
pub struct AuthoredSourceOptionRelationships<'r> {
    /// Available option topology under the unchanged full context.
    pub scan: AuthoredSourceOptionScan<'r>,
    /// Effective ordinals classified as positional data by this grammar.
    pub positionals: Vec<usize>,
    /// All source positions and values were statically classified.
    pub complete: bool,
    /// Selected descriptor and matching-form relation metadata.
    pub relations: Vec<&'static crate::OptionRelation>,
    /// Optional authored constraints callback, with no execution authority.
    pub constraints: Option<crate::ConstraintsHook>,
}

impl<'r> ResolvedInvocation<'r, '_> {
    /// Declared option diagnostics from the already selected source schema.
    /// Exact excluded rows remain explainable; prefixes use the actual admitted
    /// vocabulary. Dynamic selection, unknown widths and reserved data cannot
    /// be skipped to discover a later option. No execution or edit is implied.
    #[must_use]
    pub fn authored_source_diagnostic_options(&self) -> Option<AuthoredSourceOptionScan<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let subcommands = self.authored_source_option_selector_path()?;
        let selected = self.semantics.script_metadata;
        let options = self.semantics.options;
        let mut scan = options.diagnostic_scan(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
            self.semantics.argument_offset,
            selected
                .subcommand
                .map_or(selected.command.option_placement, |sub| {
                    sub.option_placement
                }),
        )?;
        scan.subcommands = subcommands;
        Some(scan)
    }

    /// Teardown target positions from the selected descriptor's declared
    /// layout. The caller must still join each ordinal to its actual original
    /// source word; this does not prove that any target was affected.
    #[must_use]
    pub fn authored_source_teardown_arguments(&self) -> Option<AuthoredSourceTeardownArguments> {
        if !self
            .selected
            .spec
            .traits
            .contains(Traits::FIRE_AND_FORGET_TEARDOWN)
        {
            return None;
        }
        let layout = self.selected.spec.teardown_argument_layout?;
        let count = self.words.arguments().exact_argv_len()?;
        let arguments = match layout {
            crate::spec::TeardownArgumentLayout::AllEffectivePostHeadArguments => {
                self.semantics.argument_offset..count
            }
        };
        (arguments.start <= arguments.end).then_some(AuthoredSourceTeardownArguments { arguments })
    }

    /// Possible path data for the selected destructive source descriptor.
    /// Dynamic data retains MAY uncertainty; unknown/ambiguous option widths
    /// refuse. This supplies no filesystem effect, accepted argv or Native fact.
    #[must_use]
    pub fn authored_source_destructive_path_arguments(
        &self,
    ) -> Option<AuthoredSourceDestructivePathArguments> {
        let subcommand = self.authored_source_descriptors().subcommand?;
        if !subcommand.destructive {
            return None;
        }
        let count = self.words.arguments().exact_argv_len()?;
        let scan = self.authored_source_diagnostic_options()?;
        if scan
            .options
            .iter()
            .any(|option| !option.available || option.values.is_none())
        {
            return None;
        }
        let options = self.semantics.options;
        let (arguments, selection_unknown) = if options.base.is_empty() && options.form.is_empty() {
            (self.semantics.argument_offset..count, false)
        } else {
            match scan.boundary {
                AuthoredSourceOptionBoundary::Positional(index) => (index..count, false),
                AuthoredSourceOptionBoundary::Terminator(index) => {
                    (index.checked_add(1)?..count, false)
                }
                AuthoredSourceOptionBoundary::Dynamic(index) => {
                    (index..index.checked_add(1)?, true)
                }
                AuthoredSourceOptionBoundary::End => (count..count, false),
                AuthoredSourceOptionBoundary::Unknown(_)
                | AuthoredSourceOptionBoundary::Ambiguous { .. }
                | AuthoredSourceOptionBoundary::Indeterminate => return None,
            }
        };
        Some(AuthoredSourceDestructivePathArguments {
            arguments,
            subcommands: scan.subcommands,
            selection_unknown,
        })
    }

    /// Original option-value positions under the selected channel relationship
    /// descriptor. A closed admitted option vector retains the final written or
    /// captured value occurrence, independently of evaluation or channel state.
    #[must_use]
    pub fn authored_source_channel_configuration(
        &self,
    ) -> Option<crate::AuthoredSourceChannelConfiguration> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let selected = self.authored_source_descriptors();
        let descriptor = selected
            .subcommand
            .map_or(selected.command.channel_configuration, |subcommand| {
                subcommand.channel_configuration
            })?;
        descriptor.project(self.authored_source_diagnostic_options()?)
    }

    /// Existing credential descriptors projected through this invocation's
    /// admitted option grammar and selected subcommand. Coordinates remain
    /// effective post-head ordinals; no literal value, exposure or edit follows.
    #[must_use]
    pub fn authored_source_credential_arguments(&self) -> Vec<AuthoredSourceCredentialArgument> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let selected = self.authored_source_descriptors();
        let mut arguments = Vec::new();
        if !selected.command.credential_options.is_empty()
            && let Some(scan) = self.authored_source_diagnostic_options()
        {
            for option in scan.options {
                if !option.available
                    || !selected
                        .command
                        .credential_options
                        .contains(&option.option.name)
                {
                    continue;
                }
                let Some(values) = option.values else {
                    continue;
                };
                if values.len() == 1 {
                    arguments.push(AuthoredSourceCredentialArgument {
                        argument: values.start,
                        position: AuthoredSourceCredentialPosition::Option(option.option.name),
                    });
                }
            }
        }
        if let Some(subcommand) = selected.subcommand
            && let Some(argument) = subcommand.credential_arg.map(usize::from)
            && let Some(count) = self.words.arguments().exact_argv_len()
            && count >= 3
            && argument < count
        {
            // credential_arg deliberately counts the selected subcommand at
            // index zero, unlike the other subcommand-relative argument fields.
            let header = 1;
            if self
                .words
                .arguments()
                .literal_at(header)
                .is_some_and(|name| {
                    subcommand
                        .sensitive_headers
                        .iter()
                        .any(|sensitive| name.eq_ignore_ascii_case(sensitive))
                })
            {
                arguments.push(AuthoredSourceCredentialArgument {
                    argument,
                    position: AuthoredSourceCredentialPosition::Header(header),
                });
            }
        }
        arguments
    }

    /// Filename position of the already selected source-file form. Exact
    /// cardinality and the form's available literal prefix describe authored
    /// source syntax only; they prove no native file handler, file contents or
    /// completed evaluation. Unknown selectors/expansions decline.
    #[must_use]
    pub fn authored_source_file_argument(&self) -> Option<usize> {
        if self.semantics.body_execution
            != Some(crate::body_execution::BodyExecutionSpec::SourceFile)
            || !self.semantics.traits.contains(crate::Traits::SOURCES_FILE)
        {
            return None;
        }
        let count = self
            .words
            .arguments()
            .slice_from(self.semantics.argument_offset)
            .exact_argv_len()?;
        // The one-filename layout is shared by every SourceFile grammar,
        // even when the filename's value cannot select a literal form.
        if count == 1 {
            return Some(self.semantics.argument_offset);
        }
        let form = self.form?;
        if !form.arity.accepts(u16::try_from(count).ok()?) {
            return None;
        }
        // Every form of the selected SourceFile grammar ends with filename.
        // Form selection, including option availability, precedes this query.
        self.semantics
            .argument_offset
            .checked_add(count.checked_sub(1)?)
    }

    /// Possible option positions for conditional source advice. Selection,
    /// prefixes, value widths, reserved data and terminator availability all
    /// use this invocation's retained context. Dynamic operands preserve every
    /// admitted continuation; no Normal completion or option value is proved.
    #[must_use]
    pub fn authored_source_possible_option_arguments(
        &self,
    ) -> Option<AuthoredSourceOptionArguments> {
        let scan = self.authored_source_diagnostic_options()?;
        if !scan.accepts_terminator {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let arguments = self.semantics.options.possible_option_arguments(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
            selected
                .subcommand
                .map_or(selected.command.option_placement, |sub| {
                    sub.option_placement
                }),
        )?;
        Some(AuthoredSourceOptionArguments {
            arguments: arguments
                .into_iter()
                .map(|index| self.semantics.argument_offset + index)
                .collect(),
            subcommands: scan.subcommands,
        })
    }

    /// Relationship facts from the shared actual source option topology.
    /// Unavailable/unknown/dynamic option boundaries preserve only established
    /// positive observations. Only a complete classified vector can establish
    /// an absent term. No runtime substitution, handler or repair follows.
    #[must_use]
    pub fn authored_source_option_relationships(
        &self,
    ) -> Option<AuthoredSourceOptionRelationships<'r>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let scan = self.authored_source_diagnostic_options()?;
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        let selected = self.semantics.script_metadata;
        let (base, forms, constraints, surface) = selected.subcommand.map_or(
            (
                selected.command.option_relations,
                selected.command.command_forms,
                selected.command.constraints,
                selected.command.surface,
            ),
            |sub| {
                (
                    sub.option_relations,
                    sub.subcommand_forms,
                    sub.constraints,
                    sub.surface.or(self
                        .instance_method_provider
                        .map_or(selected.command.surface, |(owner, _)| owner.surface)),
                )
            },
        );
        let relations = base
            .iter()
            .chain(
                forms
                    .iter()
                    .filter(|form| self.form.is_some_and(|selected| selected.name == form.name))
                    .flat_map(|form| form.option_relations.iter()),
            )
            .filter(|relation| {
                relation.supports_dialect(self.semantics.options.availability.query, surface)
            })
            .collect();
        let mut consumed = std::collections::BTreeSet::new();
        let mut complete = true;
        for option in &scan.options {
            if !option.available {
                complete = false;
            }
            consumed.insert(option.argument);
            if let Some(values) = &option.values {
                consumed.extend(values.clone());
            } else {
                complete = false;
            }
        }
        let bound = match scan.boundary {
            AuthoredSourceOptionBoundary::Positional(_) | AuthoredSourceOptionBoundary::End => {
                count
            }
            AuthoredSourceOptionBoundary::Terminator(index) => {
                consumed.insert(index);
                count
            }
            AuthoredSourceOptionBoundary::Dynamic(index)
            | AuthoredSourceOptionBoundary::Unknown(index)
            | AuthoredSourceOptionBoundary::Ambiguous {
                argument: index, ..
            } => {
                complete = false;
                index
            }
            AuthoredSourceOptionBoundary::Indeterminate => {
                complete = false;
                0
            }
        };
        let positionals = (self.semantics.argument_offset..bound)
            .filter(|index| !consumed.contains(index))
            .collect::<Vec<_>>();
        complete &= (self.semantics.argument_offset..count)
            .all(|index| arguments.literal_at(index).is_some());
        Some(AuthoredSourceOptionRelationships {
            scan,
            positionals,
            complete,
            relations,
            constraints,
        })
    }

    /// Available options of the actual selected source form and selector.
    /// Unresolved selectors withdraw the vocabulary; this grants no accepted
    /// operand, handler execution or source edit authority.
    #[must_use]
    pub fn authored_source_available_options(&self) -> Option<Vec<&'r OptionSpec>> {
        self.authored_source_option_selector_path()?;
        Some(self.semantics.options.available().collect())
    }
}
