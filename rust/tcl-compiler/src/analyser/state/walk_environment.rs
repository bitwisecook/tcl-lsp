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

//! The dialect environment of one walk: grammar and lexer configuration, the command surface, the source map and the declared version targets.

use super::super::types::AnalysisResult;
use super::Analyser;

impl Analyser {
    /// The grammar this document is lexed under: the ingress-resolved one
    /// when the walk came through [`Self::resolve_walk_environment`], else
    /// the profile's. Every re-segmentation and every grammar-axis read in
    /// the handlers goes through here, never through `profile.grammar`
    /// directly, so the analyser cannot lex a document under a rule the
    /// ingress did not resolve.
    pub(in super::super) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.ingress_grammar.unwrap_or(self.profile.grammar)
    }

    /// Complete source invocation axes from the retained entry or actual
    /// authoring environment. This supplies no entered handler or name lookup.
    pub(in super::super) fn original_source_invocation_dialect(
        &self,
    ) -> Option<tcl_registry::InvocationDialect> {
        match self.source_analysis_entry.as_deref() {
            Some(entry) => entry
                .options()
                .source_invocation_dialect(self.lexer_config()),
            None => self.registry.as_deref().map(|registry| {
                crate::environment_ingress::authoring_invocation_dialect(
                    registry,
                    None,
                    self.lexer_config(),
                )
            }),
        }
    }

    /// The command surface this document analyses against: `registry` plus
    /// the document's own `# tcl-lsp: stub` declarations.
    ///
    /// A stub states the same kind of fact a `CommandSpec` does, so every
    /// role query in the walk goes through here rather than reaching for the
    /// bare catalogue. The registry is passed in because a caller has already
    /// resolved which one this document analyses under, and sometimes holds
    /// it as a clone so a `&mut self` step can run beside it.
    pub(in super::super) fn command_surface<'a>(
        &'a self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> tcl_registry::model::DocumentCommandSurface<'a> {
        tcl_registry::model::DocumentCommandSurface::new(registry, self.declared_commands.as_ref())
    }

    /// The body-lexing config for this document: [`Self::grammar`] — the
    /// ingress-resolved grammar, else the profile's — carrying the
    /// dialect-dependent tokenisation flags (`{*}` expansion, the iRules
    /// `}{` word break, numerals, escapes, Jim's axes). Threaded into every
    /// analyser re-segmentation so a body is read under the document's
    /// grammar, never the default's. [`Self::file_lexer_config`] is the
    /// whole-file form.
    pub(in super::super) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.resolved_input.as_ref().map_or_else(
            || tcl_lexer::LexerConfig::from_grammar(self.grammar()),
            |input| input.config,
        )
    }

    /// This document dialect's word-value rules: how a braced word's
    /// `\<newline>` folds and how list text divides. The [`Self::grammar`]
    /// twin for the re-parses that split a *word* rather than re-lex a
    /// script — a proc's parameter list, an OO member's — so they cannot
    /// answer C Tcl's question about a `JimTcl` document.
    pub(in super::super) fn word_rules(&self) -> tcl_syntax::word_rules::WordValueRules {
        tcl_syntax::word_rules::WordValueRules::from_grammar(&self.grammar())
    }

    /// The release-dependent facts the index-bounds proofs read for this
    /// document: its character model, its numeral grammar, and whether `lset`
    /// appends at a list's length — `Some` when the profile names a measured
    /// release, or every release agrees, and `None` where they split
    /// ([`tcl_dialect::TclVersion::lset_appends_at_length_for`]).
    pub(in super::super) fn bounds_release(&self) -> crate::interval_bounds::BoundsRelease {
        let release = (!self.result.dialect.is_empty())
            .then(|| self.profile.const_fold_version())
            .flatten();
        crate::interval_bounds::BoundsRelease {
            characters: self.profile.character_model(),
            numbers: crate::intervals::numbers_for_dialect(Some(self.profile)),
            lset_appends_at_length: tcl_dialect::TclVersion::lset_appends_at_length_for(release),
        }
    }

    /// [`Self::lexer_config`] for the **whole-file** segmentation at the top of
    /// [`Self::analyse`] — the one place that stands where a Tcl runtime's
    /// `source` stands, and therefore the only place that may skip a leading
    /// byte-order mark. An independently supplied full input retains its
    /// own BOM treatment. Default ingress uses the selected file grammar:
    /// Tcl 9's `source` strips a leading U+FEFF; Tcl 8.x's does not.
    pub(in super::super) fn file_lexer_config(&self) -> tcl_lexer::LexerConfig {
        // Implementation contract: naming.consumer.original-ilx-method-source-candidates
        // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
        // A supplied whole configuration owns the document boundary as well
        // as its retained body/Realm input. Only default ingress selects the
        // profile's source-file BOM treatment.
        self.resolved_input.as_ref().map_or_else(
            || tcl_lexer::LexerConfig {
                leading_bom: tcl_lexer::LexerConfig::for_file_grammar(self.grammar()).leading_bom,
                ..self.lexer_config()
            },
            |input| input.config,
        )
    }

    /// A [`tcl_lexer::SourceMap`] over [`Self::source`], built from the
    /// cached [`Self::cached_line_index`] rather than rescanning the
    /// document. Use this instead of `SourceMap::new(&self.source)` in any
    /// handler that runs once per command (i.e. potentially once per
    /// nesting level) — see the doc comment on [`Self::cached_line_index`]
    /// for why the naive rescan is a real `DoS`, not just an inefficiency.
    ///
    /// Falls back to a fresh `SourceMap::new(source)` scan when
    /// `cached_line_index_len != source.len()` — see
    /// [`Self::cached_line_index`]'s doc comment for why the cache can be
    /// stale and why a length mismatch is how this detects it.
    ///
    /// A free function taking the fields explicitly, **not** a `&self`
    /// method: a method call always borrows all of `self`, which would tie
    /// up `self.result` for the returned `SourceMap`'s whole lifetime and
    /// break the many call sites that build a map and later push a
    /// diagnostic in the same scope. Called as
    /// `Analyser::source_map(&self.source, &self.cached_line_index,
    /// self.cached_line_index_source_len)`.
    pub(in super::super) fn source_map<'src>(
        source: &'src str,
        cached_line_index: &tcl_lexer::LineIndex,
        cached_line_index_len: usize,
    ) -> tcl_lexer::SourceMap<'src> {
        if source.len() == cached_line_index_len {
            tcl_lexer::SourceMap::with_line_index(source, cached_line_index.clone())
        } else {
            tcl_lexer::SourceMap::new(source)
        }
    }

    /// [`Self::source_map`] as a `&self` method, for the call sites that can
    /// confine the map to a scope that ends before they need `&mut self`
    /// again (typically: compute the spans into locals, then push the
    /// diagnostic). Borrowing all of `self` is why this cannot replace the
    /// free function — see its doc comment — but where the borrow *is*
    /// scoped this spelling keeps the three-argument call out of the way.
    pub(in crate::analyser) fn cached_source_map(&self) -> tcl_lexer::SourceMap<'_> {
        Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        )
    }

    /// [`Self::source`] sliced by absolute byte offsets, or `None` when the
    /// range is inverted, out of bounds, **or lands inside a multi-byte UTF-8
    /// sequence**.
    ///
    /// The one sanctioned way for a handler to turn a token span back into
    /// source text.  A bare `&self.source[start..end]` panics on a span whose
    /// end is not a `char` boundary, which is reachable from real-world input:
    /// a body word of the shape `{…}x` (Tcl's "extra characters after
    /// close-brace", which this analyser accepts leniently) has its closing
    /// `}` dropped from the segmenter's word *value*, so rebasing that value
    /// by a single offset shifts every token after the brace one byte left —
    /// harmless-looking on ASCII, mid-character on anything else.  An iRule
    /// carrying zero-width spaces (`U+200B`, the usual residue of a
    /// copy-paste from a web page) hit exactly that and aborted `fp-sweep`
    /// outright, while the LSP lost **every** diagnostic for the file and
    /// showed it as clean.
    ///
    /// [`crate::analyser::utils::contiguous_prefix`] fixes that producer, but
    /// spans reach these slices from the lexer, the segmenter, the CST, and
    /// the analyser's own arithmetic, so the consumer stays defensive: a span
    /// this rejects means "no text here", never an abort.
    ///
    /// A free function taking `source` explicitly, **not** a `&self` method,
    /// for the same reason as [`Self::source_map`]: a method call borrows all
    /// of `self`, which would tie up `self.result` for the returned slice's
    /// lifetime and break every call site that slices and then pushes a
    /// diagnostic in the same scope.
    pub(in crate::analyser) fn source_slice(
        source: &str,
        start: usize,
        end: usize,
    ) -> Option<&str> {
        source.get(start..end)
    }

    /// The active dialect's canonical name — the string that round-trips
    /// through configuration and the providers (`self.profile.name`).
    #[must_use]
    pub fn dialect(&self) -> &'static str {
        self.profile.name
    }

    /// The active dialect's `${…}` close rule — the input to the shared owner
    /// [`tcl_lexer::braced_var_name_end`].
    ///
    /// Every analyser scan that recovers a variable name out of free word text
    /// must resolve the closer through the owner under *this* style. Anything
    /// else contradicts the spans the document was lexed with: at 9.x the
    /// lexer makes `${a{b}c}` one `Var` naming `a{b}c`, while a hard-coded
    /// first-`}` scan reads `a{b` and reports on a variable the source never
    /// mentions.
    #[must_use]
    pub fn braced_var(&self) -> tcl_dialect::BracedVarStyle {
        self.lexer_config().braced_var
    }

    /// Split a `${…}`-headed `Var` token's text (as
    /// [`tcl_lexer::SourceMap::token_text`] returns it, i.e. already past the
    /// `${`) into the dispatched variable name and the word suffix glued to
    /// it.
    ///
    /// The lexer merges a braced substitution and everything concatenated to
    /// it into **one** `Var` token, so `${ns}::setdef` arrives as the raw text
    /// `ns}::setdef` — the name is `ns` and `::setdef` is an ordinary suffix.
    /// A *pure* `${x}` reference arrives as `x`, its closer already excluded
    /// from the span, and is wholly the name.
    ///
    /// Both shapes fall out of the shared owner
    /// [`tcl_lexer::braced_var_name_end`] under this dialect's rule:
    /// `Closed` is the composite case, `Unterminated` the pure one. The
    /// first-`}` split this replaces split `${a{b}c}::setdef` after `a{b`,
    /// naming a variable the source never mentions and handing the rest to
    /// the suffix fold. The three call sites — the const
    /// dispatch record, the W307 head reading, and `resolve_dynamic_word` —
    /// share it so they cannot disagree about which bytes name the variable.
    #[must_use]
    pub(in crate::analyser) fn split_braced_head<'t>(&self, raw: &'t str) -> (&'t str, &'t str) {
        match tcl_lexer::braced_var_name_end(raw.as_bytes(), 0, self.braced_var()) {
            tcl_lexer::BracedVarEnd::Closed(end) => (&raw[..end], &raw[end + 1..]),
            tcl_lexer::BracedVarEnd::Unterminated => (raw, ""),
        }
    }

    /// Resolve the actual generation without replacing an unavailable overlay.
    pub(crate) fn try_analysis_context(
        &self,
    ) -> Result<
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
        tcl_registry::model::OverlayMiss,
    > {
        if let Some(input) = &self.resolved_input {
            return Ok(std::sync::Arc::clone(&input.context));
        }
        if let Some(miss) = &self.result.analysis_context_unavailable {
            return Err(miss.clone());
        }
        if let Some(context) = &self.context {
            return Ok(std::sync::Arc::clone(context));
        }
        let environment = crate::environment_ingress::resolve_environment(self.profile.name);
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        crate::environment_ingress::analysis_registry(&environment, &keyed, self.pack_overlay)
    }

    /// Internal generation access after an entry has checked availability.
    #[must_use]
    pub(crate) fn analysis_context(&self) -> std::sync::Arc<tcl_registry::model::ContextRegistry> {
        self.try_analysis_context()
            .expect("analysis consumers require a checked available ingress")
    }

    /// An unavailable generation is a typed incomplete analysis, not a clean
    /// successful result. No source or declaration walk is performed.
    pub(in super::super) fn unavailable_analysis_result(
        &mut self,
        dialect: &str,
    ) -> Option<AnalysisResult> {
        let miss = self.result.analysis_context_unavailable.clone()?;
        let result = AnalysisResult {
            dialect: dialect.to_owned(),
            analysis_context_unavailable: Some(miss),
            ..AnalysisResult::default()
        };
        self.result = result.clone();
        Some(result)
    }

    /// Resolve `dialect` at a walk ingress: stash
    /// the environment and this walk's registry generation, derive the
    /// interop [`Self::profile`], and return whether this environment
    /// ships `Tk` **ambient** ([`Self::tk_ambient`] — the fact
    /// carried as a placement query on the walk's own context, not as an
    /// `availability_for_name` `TK`-bit union).
    pub(in super::super) fn resolve_walk_environment(&mut self, dialect: &str) -> bool {
        if let Some(input) = &self.resolved_input {
            self.result.analysis_context_unavailable = None;
            self.profile = input.profile;
            self.ingress_grammar = Some(input.config.grammar_over(input.profile.grammar));
            self.unit_profile = Some(input.unit_profile);
            self.context = Some(std::sync::Arc::clone(&input.context));
            self.environment = None;
            return input.context.context().ambient_package("Tk");
        }
        let environment = crate::environment_ingress::resolve_environment(dialect);
        self.profile = environment.analyser_profile();
        self.ingress_grammar = Some(environment.grammar());
        self.unit_profile = Some(environment.unit_profile());
        let keyed =
            crate::environment_ingress::DocumentEnvironment::keyed_versions(&self.library_versions);
        let generation = match crate::environment_ingress::analysis_registry(
            &environment,
            &keyed,
            self.pack_overlay,
        ) {
            Ok(generation) => generation,
            Err(miss) => {
                self.context = None;
                self.registry = None;
                self.result.analysis_context_unavailable = Some(miss);
                self.environment = Some(environment);
                return false;
            }
        };
        self.result.analysis_context_unavailable = None;
        let tk_ambient = generation.context().ambient_package("Tk");
        self.context = Some(generation);
        self.environment = Some(environment);
        tk_ambient
    }

    /// Resolve the §5.4 declared version targets for this walk: the
    /// configured [`Self::declared_targets`] pairs plus the source's
    /// `# tcl-lsp: supports NAME RANGE` directives (the directive wins
    /// per provider — most specific source, §5.4), parsed under the
    /// targets grammar ([`tcl_registry::model::targets_from_clauses`])
    /// and recorded on a document-level clone of this walk's resolved
    /// context. A malformed or empty declaration is dropped rather than
    /// guessed at. A provider named after a **family** declares targets
    /// on that family's core axis and is honoured only when the
    /// document's own core is that family — the Tcl, iRules and Jim
    /// ladders are separate axes, and a declaration on one says nothing
    /// about another (invariant I2).
    ///
    /// Also derives the numeral-grammar era set the W151 cross-target
    /// numeral check walks under; fewer than two represented grammars
    /// switches that check off.
    pub(in super::super) fn resolve_declared_targets(&mut self, source: &str) {
        use tcl_dialect::model::{Family, VersionAxisId};
        self.range_context = None;
        self.range_numeral_grammars = Vec::new();
        let directives = super::super::utils::parse_supports_directives(source);
        if self.declared_targets.is_empty() && directives.is_empty() {
            return;
        }
        let mut merged: Vec<(String, String)> = self.declared_targets.clone();
        for (name, clauses) in directives {
            merged.retain(|(existing, _)| *existing != name);
            merged.push((name, clauses));
        }
        let generation = self.analysis_context();
        let mut context = generation.context().clone();
        let core_family = context.environment.core.map(|core| core.family);
        let mut declared_any = false;
        for (name, clauses) in &merged {
            // A **family name** (`tcl`, `jim`, `f5-irules`, …) declares
            // targets on that family's own core axis, and only when the
            // document's core is that family: a `supports tcl 8.5-9.0`
            // under a jim core, or a `supports jim 0.81-` under a Tcl
            // one, is a declaration about a ladder this document is not
            // on, and invariant I2 says it must be dropped rather than
            // coerced. Recognising only `tcl` as a family here would mint a
            // fictitious *package* axis named `jim` for `supports jim 0.81-`
            // and switch range mode on against it.
            let family = Family::ALL
                .into_iter()
                .find(|family| name.eq_ignore_ascii_case(family.name()));
            let axis = match family {
                Some(family) if core_family == Some(family) => VersionAxisId::core(family),
                Some(_) => continue,
                None => VersionAxisId::package(name),
            };
            let clause_list: Vec<&str> = clauses.split_whitespace().collect();
            let Ok(targets) = tcl_registry::model::targets_from_clauses(&axis, &clause_list) else {
                continue;
            };
            if targets.is_empty() {
                continue;
            }
            context.declare_targets(targets);
            declared_any = true;
        }
        if !declared_any {
            return;
        }
        let core_axis = VersionAxisId::core(Family::Tcl);
        if let Some(declared) = context.declared_targets(&core_axis) {
            for &grammar in tcl_dialect::NumberSyntax::ALL {
                // Exhaustive on purpose: a future grammar variant must
                // name its interval here before it compiles.
                let requirement = match grammar {
                    tcl_dialect::NumberSyntax::Tcl84 => "0-8.5",
                    tcl_dialect::NumberSyntax::Tcl85 => "8.5-9.0",
                    tcl_dialect::NumberSyntax::Tcl90 => "9.0-",
                    // JimTcl's numeral grammars are not intervals on the
                    // Tcl core axis — Jim is a reimplementation, not a
                    // release of it — so no declared Tcl range can select
                    // one. They reach this diagnostic through the Jim
                    // ladder instead.
                    tcl_dialect::NumberSyntax::Jim | tcl_dialect::NumberSyntax::Jim080 => {
                        continue;
                    }
                };
                let Ok(era) = tcl_dialect::model::VersionSet::from_requirements(
                    core_axis.clone(),
                    &[requirement],
                ) else {
                    continue;
                };
                if declared
                    .intersect(&era)
                    .is_ok_and(|overlap| !overlap.is_empty())
                {
                    self.range_numeral_grammars.push(grammar);
                }
            }
        }
        self.range_context = Some(context);
    }
}
