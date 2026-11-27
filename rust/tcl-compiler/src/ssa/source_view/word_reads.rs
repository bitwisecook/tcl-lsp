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

//! One retained read of a word or spelling: its reference, completion, presence, produced value, representation, place and contents.

use super::super::{SsaReadReference, Symbol, ValueKey, Version};
use super::SsaSourceView;
use crate::cfg::BlockId;
use crate::var_resolve::ContentsOrigin;
use tcl_registry::CommandRegistry;
use tcl_source::words::SourceSite;
use tcl_source::words::WordExpr;

/// Closed contents-presence alternatives for a potential missing-read diagnostic.
/// This evidence supplies neither an SSA value nor a guaranteed read failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SsaReadPresenceAlternatives {
    contains_undefined: bool,
    contains_defined: bool,
}

impl SsaReadPresenceAlternatives {
    /// At least one retained physical continuation has undefined contents.
    /// Every other continuation is also closed and has known presence.
    #[must_use]
    pub const fn may_be_undefined(self) -> bool {
        self.contains_undefined
    }

    /// At least one retained continuation may hold defined contents, so the
    /// read is not a proved absence on every path.
    #[must_use]
    pub const fn may_be_defined(self) -> bool {
        self.contains_defined
    }
}

/// Captured physical representation evidence for conversion-cost advice.
/// Numeric categories come from actual current native production, independently
/// of semantic contents. Container alternatives never establish one current intrep.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SsaReadRepresentationAdvice {
    pub representation: Option<tcl_syntax::value::ValueRepresentation>,
    pub container_alternatives: Option<crate::native_numeric::ClosedContainerRepresentations>,
    pub already_numeric: bool,
    /// Unanimous current subtype, or Numeric for closed differing subtypes.
    /// Missing evidence cannot borrow a semantic type or replayed commitment.
    pub numeric_category: Option<tcl_registry::TclType>,
}

/// Retained contents provenance for a read whose possible writes need not be
/// separate scalar definitions (for example an unknown array-element index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsaReadContents {
    /// Exact physical binding read.
    pub reference: SsaReadReference,
    /// Source-reaching contents provenance, including explicit unknown effects.
    pub origin: ContentsOrigin,
    /// Whether entry contents are one possible source of this value.
    pub includes_incoming: bool,
    /// Represented write statements to this exact root and lifetime.
    pub writes: Vec<(BlockId, usize)>,
    /// Some possible contents are not represented by these write points.
    pub unknown_residual: bool,
}

impl<'a> SsaSourceView<'a> {
    /// Query a substitution's canonical read without borrowing another read's context.
    #[must_use]
    pub fn read_reference(self, source: &SourceSite, spelling: &str) -> Option<SsaReadReference> {
        let (block, index) = self.point?;
        self.ssa.read_reference_at(block, index, source, spelling)
    }

    /// Exact attempted-read cell and its operation's SSA version. This can
    /// identify an undefined version for diagnostics; it proves neither a
    /// completed read nor a produced value. Missing source or cell provenance
    /// never falls back to a written variable label.
    #[must_use]
    pub(crate) fn read_occurrence_reference(
        self,
        source: &SourceSite,
        spelling: &str,
    ) -> Option<(Symbol, Version)> {
        let (block, index) = self.point?;
        let read = self.read_reference(source, spelling)?;
        let block = self.ssa.blocks.get(&block)?;
        let version = if index == usize::MAX {
            block.exit_versions.get(&read.symbol)?
        } else {
            block.statements.get(index)?.uses.get(&read.symbol)?
        };
        Some((read.symbol, *version))
    }

    /// Completion of an exact reached substitution, independently of SSA values.
    /// Every retained physical context must prove the same variable-read error.
    /// An observer, unknown residual or absent source carrier cannot supply that
    /// proof; an undefined read never needs a fabricated incoming SSA version.
    #[must_use]
    pub fn read_completion_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<crate::var_resolve::VariableReadCompletion> {
        self.read_properties_at(source, spelling, registry)
            .map(|(completion, _)| completion)
    }

    /// Definedness of one exact reached read, independently of value versions.
    /// All closed physical alternatives must agree. Undefined contents are
    /// distinct from an existing array root rejected by a scalar read.
    #[must_use]
    pub fn read_contents_presence_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<crate::var_resolve::ContentsPresence> {
        self.read_properties_at(source, spelling, registry)
            .map(|(_, presence)| presence)
    }

    /// Closed presence alternatives at one original reached read. Mixed
    /// Defined/Undefined alternatives can support a potential missing-read
    /// diagnostic; they cannot supply a constant or a definite execution error.
    /// Missing provenance, unknown worlds and read observers decline.
    #[must_use]
    pub fn read_contents_presence_alternatives_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadPresenceAlternatives> {
        use crate::var_resolve::ContentsPresence;
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        if access.context_alternatives().is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return None;
        }
        let mut contains_undefined = false;
        let mut contains_defined = false;
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, registry);
            if place.observed {
                return None;
            }
            match context.closed_contents_presence(&place)? {
                ContentsPresence::Defined => contains_defined = true,
                ContentsPresence::Undefined => contains_undefined = true,
                ContentsPresence::DefinedOrUndefined => {
                    contains_undefined = true;
                    contains_defined = true;
                }
                ContentsPresence::Unknown => return None,
            }
        }
        Some(SsaReadPresenceAlternatives {
            contains_undefined,
            contains_defined,
        })
    }

    fn read_properties_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<(
        crate::var_resolve::VariableReadCompletion,
        crate::var_resolve::ContentsPresence,
    )> {
        use crate::var_resolve::{ContentsPresence, VariableReadCompletion};

        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        let contexts = access.context_alternatives();
        if contexts.is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            return Some((VariableReadCompletion::Unknown, ContentsPresence::Unknown));
        }
        let mut errors = true;
        let mut presence = None;
        for context in contexts {
            let place = access.place_in_context(context, registry);
            errors &= context.read_completion(&place) == VariableReadCompletion::Error;
            let alternative = context.contents_presence(&place);
            presence = Some(match presence {
                None => alternative,
                Some(previous) if previous == alternative => previous,
                Some(_) => ContentsPresence::Unknown,
            });
        }
        Some((
            if errors {
                VariableReadCompletion::Error
            } else {
                VariableReadCompletion::Unknown
            },
            presence.unwrap_or(ContentsPresence::Unknown),
        ))
    }

    /// Resolve a word consisting solely of one substitution using its exact source site.
    #[must_use]
    pub fn read_word(self, word: &WordExpr) -> Option<SsaReadReference> {
        let access = self.word_variable_access(word)?;
        self.read_reference(&access.source, &access.original_spelling)
    }

    /// Definition supplying one captured original argv value before a nested
    /// handler runs. This contents dependency leaves the physical SSA read and
    /// every later barrier unchanged; it grants no representation or erasure.
    pub(crate) fn captured_argument_value_definition(
        self,
        word: &WordExpr,
        invocation: &tcl_names::command::CommandAllocationSite,
        registry: &CommandRegistry,
    ) -> Option<ValueKey> {
        let access = self.word_variable_access(word)?;
        if !exclusive_argument_owner(&access.owner, invocation)
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
            || !self.read_word_produces_value(word, registry)
        {
            return None;
        }
        let place = source_read_place(access, registry);
        let cell = place.cell.as_ref()?;
        if place.dynamic
            || place.observed
            || place.kind != tcl_names::place::PlaceKind::Scalar
            || cell.generation == tcl_names::place::CellGeneration::Unknown
            || access.context_alternatives().iter().any(|context| {
                let alternative = access.place_in_context(context, registry);
                alternative.cell.as_ref() != Some(cell)
                    || alternative.dynamic
                    || alternative.observed
                    || context.read_contents_origin(&alternative, registry)
                        != access
                            .variable_context
                            .read_contents_origin(&place, registry)
            })
        {
            return None;
        }
        let contents = self.read_word_contents(word, registry)?;
        if contents.unknown_residual || contents.includes_incoming {
            return None;
        }
        let [(write_block, write_index)] = contents.writes.as_slice() else {
            return None;
        };
        let (read_block, read_index) = self.point?;
        if *write_block != read_block || *write_index >= read_index {
            return None;
        }
        let statement = self
            .ssa
            .blocks
            .get(write_block)?
            .statements
            .get(*write_index)?;
        let writer = SsaSourceView::at_statement(self.ssa, *write_block, *write_index)
            .source_tokens()?
            .source_binding
            .as_ref()?
            .invocation_site()?;
        if writer.source != invocation.source {
            return None;
        }
        let version = *statement.defs.get(&contents.reference.symbol)?;
        Some((contents.reference.symbol, version))
    }

    fn word_variable_access(
        self,
        word: &WordExpr,
    ) -> Option<&'a crate::command_binding::SourceVariableAccess> {
        let (spelling, source) = word.sole_variable_substitution()?;
        self.source_tokens()?
            .variable_access_for_site(source)
            .filter(|access| access.original_spelling == spelling)
    }

    /// Every retained native context for this exact substitution proves that
    /// its read produces a value. This is separate from address, SSA version,
    /// representation and command completion evidence.
    #[must_use]
    pub fn read_word_produces_value(self, word: &WordExpr, registry: &CommandRegistry) -> bool {
        let Some(access) = self.word_variable_access(word) else {
            return false;
        };
        Self::access_produces_value(access, registry)
    }

    /// Completed value production at one retained original substitution site,
    /// including a variable component inside an interpolated word. Every
    /// physical alternative must prove the read; a label cannot supply a site.
    #[must_use]
    pub fn read_produces_value_at(
        self,
        source: &SourceSite,
        original_spelling: &str,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(tokens) = self.source_tokens() else {
            return false;
        };
        let Some(access) = crate::command_binding::SourceVariableAccess::find_at_source(
            &tokens.variable_accesses,
            source,
            original_spelling,
        ) else {
            return false;
        };
        Self::access_produces_value(access, registry)
    }

    fn access_produces_value(
        access: &crate::command_binding::SourceVariableAccess,
        registry: &CommandRegistry,
    ) -> bool {
        !access.context_alternatives().is_empty()
            && access.context_residual()
                == crate::command_binding::SourceVariableReadResidual::Closed
            && access.context_alternatives().iter().all(|context| {
                let place = access.place_in_context(context, registry);
                context.read_produces_value(&place, registry)
            })
    }

    /// One represented value read which can be replaced without removing a
    /// variable observer. Every original physical context must select the same
    /// bounded scalar cell and produce a value. This grants no source extent,
    /// store deletion, alias continuity or movement of the producing expression.
    #[must_use]
    pub fn replaceable_read_at(
        self,
        source: &SourceSite,
        original_spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadReference> {
        let tokens = self.source_tokens()?;
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &tokens.variable_accesses,
            source,
            original_spelling,
        )?;
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_REPLACEABLE_READ offset={} alternatives={} residual={:?} produces={}",
                source.span.start(),
                access.context_alternatives().len(),
                access.context_residual(),
                Self::access_produces_value(access, registry)
            );
            for context in access.context_alternatives() {
                let place = access.place_in_context(context, registry);
                eprintln!(
                    "ORIGINAL_REPLACEABLE_READ offset={} place_kind={:?} observed={} dynamic={} presence={:?} generation_known={}",
                    source.span.start(),
                    place.kind,
                    place.observed,
                    place.dynamic,
                    context.contents_presence(&place),
                    place.cell.as_ref().is_some_and(
                        |cell| cell.generation != tcl_names::place::CellGeneration::Unknown
                    )
                );
            }
        }
        if !Self::access_produces_value(access, registry) {
            return None;
        }
        let reference = self.read_reference(source, original_spelling)?;
        reference.version?;
        let key = self.ssa.cell_key(reference.symbol)?;
        access
            .context_alternatives()
            .iter()
            .all(|context| {
                let place = access.place_in_context(context, registry);
                !place.observed
                    && !place.dynamic
                    && place.kind == tcl_names::place::PlaceKind::Scalar
                    && place.cell.as_ref().is_some_and(|cell| {
                        cell.generation != tcl_names::place::CellGeneration::Unknown
                    })
                    && crate::var_resolve::canonical_binding_value_key(&place).as_ref() == Some(key)
            })
            .then_some(reference)
    }

    /// Actual representation at this captured read, independently of a
    /// producer's semantic type. Unknown, observed or disagreeing native
    /// contexts cannot donate a representation from an earlier constructor.
    #[must_use]
    pub fn read_word_representation(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<tcl_syntax::value::ValueRepresentation> {
        self.read_word_representation_advice(word, registry)
            .representation
    }

    /// Closed ordinary container possibilities for conversion-cost advice.
    /// Every original read must succeed unobserved in its actual live cell;
    /// this grants no single representation, constant or erasure proof.
    #[must_use]
    pub fn read_word_representation_alternatives(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        self.read_word_representation_advice(word, registry)
            .container_alternatives
    }

    pub(crate) fn read_word_representation_advice(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        self.word_variable_access(word)
            .map_or_else(SsaReadRepresentationAdvice::default, |access| {
                source_read_representations(access, registry)
            })
    }

    /// Representation advice for an actually reached native name-operand
    /// read of this physical SSA cell. This uses the retained post-argv read
    /// inventory, never the context before argument evaluation. All matching
    /// reads must succeed unobserved and agree; absent or opaque inventories
    /// donate neither a representation nor closed container possibilities.
    pub(crate) fn native_read_representation_advice(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> SsaReadRepresentationAdvice {
        use crate::command_binding::SourceVariableReadResidual;

        let Some(reads) = self
            .source_tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.invocation_variable_reads.as_ref())
            .filter(|reads| reads.residual == SourceVariableReadResidual::Closed)
        else {
            return SsaReadRepresentationAdvice::default();
        };
        let Some(key) = self.ssa.cell_key(symbol) else {
            return SsaReadRepresentationAdvice::default();
        };
        captured_read_representations(
            reads
                .native_reads
                .iter()
                .filter(|read| {
                    crate::var_resolve::canonical_binding_value_key(&read.place).as_ref()
                        == Some(key)
                })
                .map(|read| (read.variable_context.as_ref(), read.place.clone())),
            registry,
        )
    }

    /// Physical address selected by one retained sole variable substitution.
    /// This reports address and observer facts independently of represented
    /// scalar contents; it does not license a value or representation fold.
    #[must_use]
    pub fn read_word_place(
        self,
        word: &WordExpr,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<tcl_names::place::Place> {
        let access = self.word_variable_access(word)?;
        Some(access.place_in_context(&access.variable_context, registry))
    }

    /// Project the contents alternatives of one exact retained read.
    /// Writes must retain the same physical root and lifetime; source offsets
    /// alone never make an unrelated store a candidate.
    #[must_use]
    pub fn read_contents_at(
        self,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = crate::command_binding::SourceVariableAccess::find_at_source(
            &self.source_tokens()?.variable_accesses,
            source,
            spelling,
        )?;
        let reference = self.read_reference(source, spelling)?;
        let target = source_read_place(access, registry);
        let origin = access
            .variable_context
            .read_contents_origin(&target, registry);
        let (includes_incoming, offsets) = match &origin {
            ContentsOrigin::Incoming => (true, &[][..]),
            ContentsOrigin::WrittenAt(offset) => (false, std::slice::from_ref(offset)),
            ContentsOrigin::Alternatives { incoming, writes } => (*incoming, writes.as_slice()),
            ContentsOrigin::Unknown => (false, &[][..]),
        };
        let mut contents = SsaReadContents {
            reference,
            origin: origin.clone(),
            includes_incoming,
            writes: Vec::new(),
            unknown_residual: origin == ContentsOrigin::Unknown || target.observed,
        };
        let points = self.ssa.point_contexts.as_ref()?;
        for offset in offsets {
            let mut found = false;
            for (&block, body) in &self.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    if statement.statement.span().start() != *offset {
                        continue;
                    }
                    let writes = crate::place_bridge::def_places_at(
                        &statement.statement,
                        block,
                        index,
                        points,
                        registry,
                    );
                    if writes.iter().any(|write| {
                        write.cell.is_some()
                            && write.cell == target.cell
                            && tcl_names::place::overlap(write, &target)
                    }) {
                        contents.writes.push((block, index));
                        found = true;
                    }
                }
            }
            contents.unknown_residual |= !found;
        }
        contents.writes.sort_unstable();
        contents.writes.dedup();
        Some(contents)
    }

    /// Contents alternatives for a sole variable value word, using its original
    /// captured read rather than the current value of a cached input binding.
    #[must_use]
    pub fn read_word_contents(
        self,
        word: &WordExpr,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let access = self.word_variable_access(word)?;
        self.read_contents_at(&access.source, &access.original_spelling, registry)
    }

    /// Project a name-only query only when every retained read agrees on its contents.
    /// No reached read, a missing native grammar, or conflicting read identities
    /// leaves the projection unresolved. Exact source queries remain preferable.
    #[must_use]
    pub fn read_spelling(self, name: &str) -> Option<SsaReadReference> {
        let mut selected = None;
        for access in &self.source_tokens()?.variable_accesses {
            let dialect = access.variable_context.invocation_dialect?;
            let reference = tcl_syntax::naming::var_reference_for_style(
                &access.original_spelling,
                dialect.lexer_grammar.braced_var,
            );
            if reference != name {
                continue;
            }
            let read = self.read_reference(&access.source, &access.original_spelling)?;
            if selected.is_some_and(|previous| previous != read) {
                return None;
            }
            selected = Some(read);
        }
        selected
    }

    /// Consensus contents alternatives for evaluators accepting only a name.
    /// Differing occurrences remain unresolved; exact per-reference queries
    /// preserve more information and are preferred.
    #[must_use]
    pub fn read_spelling_contents(
        self,
        name: &str,
        registry: &CommandRegistry,
    ) -> Option<SsaReadContents> {
        let mut selected = None;
        for access in &self.source_tokens()?.variable_accesses {
            let dialect = access.variable_context.invocation_dialect?;
            if tcl_syntax::naming::var_reference_for_style(
                &access.original_spelling,
                dialect.lexer_grammar.braced_var,
            ) != name
            {
                continue;
            }
            let contents =
                self.read_contents_at(&access.source, &access.original_spelling, registry)?;
            if selected
                .as_ref()
                .is_some_and(|previous| *previous != contents)
            {
                return None;
            }
            selected = Some(contents);
        }
        selected
    }
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
///
/// This function is inherently long because the rename walk couples
/// version counters, stacks, phi versions, incoming edges, and
/// per-statement use/def maps — splitting it would just scatter the
/// state across many parameters.
// Long renumbering pass with sequential block-walk phases.
/// Intern names that appear *only* in a terminator (a `return $x` value /
/// expr, or a branch condition). The rename walk interns statement and phi
/// names but not terminator reads, so a parameter read solely in the return
/// (`proc p {x} { return $x }`) would otherwise be absent from the interner —
/// leaving `var_symbol` unable to resolve it, which breaks any consumer that
/// resolves such a name (e.g. the interprocedural O103 seed for an
/// argument-sensitive passthrough). Interning is map-membership only; no SSA
/// statement / version map changes, so every analysis result is unaffected.
pub(super) fn source_read_representations(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> SsaReadRepresentationAdvice {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
        return SsaReadRepresentationAdvice::default();
    }
    captured_read_representations(
        access.context_alternatives().iter().map(|context| {
            let place = access.place_in_context(context, registry);
            (context.as_ref(), place)
        }),
        registry,
    )
}

/// Common projection for substitutions, expression operands and native named
/// reads. An empty or opaque inventory cannot supply any positive evidence.
fn captured_read_representations<'a>(
    reads: impl Iterator<
        Item = (
            &'a crate::var_resolve::ResolveContext,
            tcl_names::place::Place,
        ),
    >,
    registry: &CommandRegistry,
) -> SsaReadRepresentationAdvice {
    use tcl_syntax::value::ValueRepresentation;
    let mut seen = false;
    let mut representation = None;
    let mut representation_closed = true;
    let mut container_alternatives = None;
    let mut containers_closed = true;
    let mut already_numeric = true;
    let mut numeric_category = None;
    let mut numeric_categories_closed = true;
    for (context, place) in reads {
        seen = true;
        if !context.read_produces_value(&place, registry) {
            return SsaReadRepresentationAdvice::default();
        }
        let current = context.contents_representation_at(&place);
        representation_closed &= current != ValueRepresentation::Unknown
            && representation.is_none_or(|previous| previous == current);
        representation = Some(current);
        if let Some(current) = context.container_representation_alternatives_at(&place) {
            container_alternatives = Some(container_alternatives.map_or(
                current,
                |previous: crate::native_numeric::ClosedContainerRepresentations| {
                    previous.joined(current)
                },
            ));
        } else {
            containers_closed = false;
        }
        already_numeric &= context.contents_already_native_numeric_at(&place, registry);
        if let Some(current) = context.contents_native_numeric_category_at(&place, registry) {
            numeric_category = Some(numeric_category.map_or(current, |previous| {
                if previous == current {
                    current
                } else {
                    tcl_registry::TclType::Numeric
                }
            }));
        } else {
            numeric_categories_closed = false;
        }
    }
    SsaReadRepresentationAdvice {
        representation: (seen && representation_closed)
            .then_some(representation)
            .flatten(),
        container_alternatives: (seen && containers_closed)
            .then_some(container_alternatives)
            .flatten(),
        already_numeric: seen && already_numeric,
        numeric_category: (seen && numeric_categories_closed)
            .then_some(numeric_category)
            .flatten(),
    }
}

fn exclusive_argument_owner(
    owner: &crate::command_binding::SourceVariableEvaluationOwner,
    invocation: &tcl_names::command::CommandAllocationSite,
) -> bool {
    use crate::command_binding::SourceVariableEvaluationOwner;
    match owner {
        SourceVariableEvaluationOwner::InvocationArguments {
            invocation: original,
            ..
        } => original == invocation,
        SourceVariableEvaluationOwner::Alternatives(owners) => {
            !owners.is_empty()
                && owners
                    .iter()
                    .all(|owner| exclusive_argument_owner(owner, invocation))
        }
        _ => false,
    }
}

pub(in super::super) fn source_read_place(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> tcl_names::place::Place {
    access.place_in_context(&access.variable_context, registry)
}
