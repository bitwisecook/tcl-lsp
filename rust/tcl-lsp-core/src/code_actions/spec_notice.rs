// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed source subjects for closed-vocabulary loader notices.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// The loader's actual rejected vocabulary position, independent of prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecPackNoticeKind {
    /// A rejected property of the current definition row.
    Property,
    /// A rejected flag in a particular definition row vocabulary.
    Flag {
        /// The loader-selected row whose flag vocabulary applies.
        row: String,
    },
}

/// A unique literal source word of an actual loader rejection.
/// The complete original pack image and producer grammar remain retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecPackNoticeSubject {
    image: SourceImage,
    config: LexerConfig,
    kind: SpecPackNoticeKind,
    word: String,
    span: Span,
    line: u32,
}
impl SpecPackNoticeSubject {
    /// Bind a loader-supplied kind/word to one unchanged lexical source site.
    /// Computed, escaped, ambiguous and absent sites cannot issue an edit subject.
    #[must_use]
    pub fn from_loader(
        source: &str,
        config: LexerConfig,
        line: u32,
        word: &str,
        kind: SpecPackNoticeKind,
    ) -> Option<Self> {
        if word.is_empty() || !word.is_ascii() || word.contains(char::is_whitespace) {
            return None;
        }
        let image = SourceImage::document(source);
        let mut sites = Vec::new();
        fn plain(word: &NativeWord) -> Option<&str> {
            if word.group().expand {
                return None;
            }
            let bytes = word
                .image()
                .bytes()
                .get(word.content_span().ok()?.as_range())?;
            if bytes
                .iter()
                .any(|b| matches!(b, b'\\' | b'$' | b'[' | b']'))
            {
                return None;
            }
            std::str::from_utf8(bytes).ok()
        }
        fn walk(
            image: &SourceImage,
            region: Span,
            config: LexerConfig,
            depth: u32,
            line: u32,
            wanted: &str,
            kind: &SpecPackNoticeKind,
            sites: &mut Vec<Span>,
        ) -> Option<()> {
            if depth > 32 {
                return None;
            }
            let plan =
                tcl_lexer::native_script_words_in(image.clone(), region, config.at_depth(depth))
                    .ok()?;
            if plan.fatal_tail.is_some() {
                return None;
            }
            for command in plan.commands {
                let head = command.words.first()?;
                let head_text = plain(head);
                for (ordinal, word) in command.words.iter().enumerate() {
                    let word_line = 1 + image
                        .bytes()
                        .get(..word.span().start() as usize)?
                        .iter()
                        .filter(|&&b| b == b'\n')
                        .count() as u32;
                    let position = match kind {
                        SpecPackNoticeKind::Property => ordinal == 0,
                        SpecPackNoticeKind::Flag { row } => {
                            ordinal > 0
                                && head_text == Some(row.as_str())
                                && wanted.starts_with('-')
                        }
                    };
                    if position
                        && word_line == line
                        && plain(word) == Some(wanted)
                        && word.group().kind != tcl_lexer::WordKind::Braced
                    {
                        sites.push(word.content_span().ok()?);
                    }
                    if word.group().kind == tcl_lexer::WordKind::Braced {
                        let _ = walk(
                            image,
                            word.content_span().ok()?,
                            config,
                            depth + 1,
                            line,
                            wanted,
                            kind,
                            sites,
                        );
                    }
                }
            }
            Some(())
        }
        walk(
            &image,
            Span::new(0, u32::try_from(image.len()).ok()?),
            config,
            0,
            line,
            word,
            &kind,
            &mut sites,
        )?;
        let [span] = sites.as_slice() else {
            return None;
        };
        Some(Self {
            image,
            config,
            kind,
            word: word.into(),
            span: *span,
            line,
        })
    }
    /// The complete pack image retained at loader rejection.
    #[must_use]
    pub fn image(&self) -> &SourceImage {
        &self.image
    }
    /// The typed vocabulary position selected by the loader.
    #[must_use]
    pub fn kind(&self) -> &SpecPackNoticeKind {
        &self.kind
    }
    /// The rejected literal source word, independently of its notice text.
    #[must_use]
    pub fn word(&self) -> &str {
        &self.word
    }
    /// The unique original word extent in the retained image.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// The original one-based source line selected by the loader.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SpecPackNoticeData {
    pub subject: Arc<SpecPackNoticeSubject>,
    registry: RegistrySemanticKey,
    consumer_config: LexerConfig,
}
#[derive(Default)]
struct Subjects {
    next: u64,
    bytes: usize,
    rows: BTreeMap<u64, SpecPackNoticeData>,
    order: VecDeque<u64>,
}
fn subjects() -> &'static Mutex<Subjects> {
    static SUBJECTS: OnceLock<Mutex<Subjects>> = OnceLock::new();
    SUBJECTS.get_or_init(|| Mutex::new(Subjects::default()))
}
impl SpecPackNoticeData {
    pub fn new(subject: &SpecPackNoticeSubject, analysis: &AnalysisResult) -> Option<Self> {
        let registry = analysis.resolved_registry()?;
        registry.document_grammar()?;
        let config = analysis.body_lexer_config?;
        if !analysis.matches_original_source_image(&subject.image, config) {
            return None;
        }
        Some(Self {
            subject: Arc::new(subject.clone()),
            registry: registry.snapshot().semantic_key(),
            consumer_config: config,
        })
    }
    pub fn to_value(&self) -> serde_json::Value {
        const MAX: usize = 512;
        const MAX_BYTES: usize = 16 * 1024 * 1024;
        let size = self.subject.image.len();
        if size > MAX_BYTES {
            return serde_json::Value::Null;
        }
        let mut s = subjects()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while s.rows.len() >= MAX || s.bytes.saturating_add(size) > MAX_BYTES {
            let Some(old) = s.order.pop_front() else {
                return serde_json::Value::Null;
            };
            if let Some(old) = s.rows.remove(&old) {
                s.bytes = s.bytes.saturating_sub(old.subject.image.len());
            }
        }
        let Some(token) = s.next.checked_add(1) else {
            return serde_json::Value::Null;
        };
        s.next = token;
        s.bytes += size;
        s.rows.insert(token, self.clone());
        s.order.push_back(token);
        serde_json::json!({"kind":"spectcl-source-notice","version":1,"token":token.to_string()})
    }
    pub fn from_value(value: &serde_json::Value, code: &str) -> Option<Self> {
        if code != super::SPEC_PACK_DIAGNOSTIC_CODE
            || value.get("kind")?.as_str()? != "spectcl-source-notice"
            || value.get("version")?.as_u64()? != 1
        {
            return None;
        }
        let token = value.get("token")?.as_str()?.parse::<u64>().ok()?;
        subjects()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rows
            .get(&token)
            .cloned()
    }
    pub fn matches(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        registry: &CommandRegistry,
        diagnostic: &super::ContextDiagnostic,
    ) -> bool {
        let Some(config) = analysis.body_lexer_config else {
            return false;
        };
        let Some(current) = analysis.resolved_registry() else {
            return false;
        };
        let image = SourceImage::document(source);
        if config != self.consumer_config
            || image != self.subject.image
            || !analysis.matches_original_source_image(&image, config)
            || registry.document_grammar().is_none()
            || self.registry != registry.snapshot().semantic_key()
            || self.registry != current.snapshot().semantic_key()
            || diagnostic.code != super::SPEC_PACK_DIAGNOSTIC_CODE
            || diagnostic.range.start_line != self.subject.line.saturating_sub(1)
            || diagnostic.range.end_line != diagnostic.range.start_line
        {
            return false;
        }
        // Producer and consumer syntax configurations are independent. Both
        // must select the same exact literal extent in the unchanged image.
        let Some(current) = SpecPackNoticeSubject::from_loader(
            source,
            config,
            self.subject.line,
            &self.subject.word,
            self.subject.kind.clone(),
        ) else {
            return false;
        };
        current.span == self.subject.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_actions::{
        ContextDiagnostic, ContextDiagnosticData, SPEC_PACK_DIAGNOSTIC_CODE,
    };
    use tcl_compiler::analyser::Analyser;
    #[test]
    fn original_spec_notice_actions_use_typed_subjects_and_current_owners_instead_of_prose() {
        // Implementation contract: naming.consumer.typed-spec-pack-notice-actions
        // docs/design/analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md
        let source = "speclib demo 1 {\n command demo::x {\n arty 1\n }\n}\n";
        let mut analysis = Analyser::new().analyse(source, "spectcl");
        let config = analysis.body_lexer_config.unwrap();
        let subject = SpecPackNoticeSubject::from_loader(
            source,
            config,
            3,
            "arty",
            SpecPackNoticeKind::Property,
        )
        .unwrap();
        let data = ContextDiagnosticData::from_spec_pack_notice(&subject, &analysis).unwrap();
        let wire = data.to_value();
        let mut diagnostic = ContextDiagnostic {
            data: ContextDiagnosticData::from_value(&wire, SPEC_PACK_DIAGNOSTIC_CODE),
            code: SPEC_PACK_DIAGNOSTIC_CODE.into(),
            message: "unrelated `-rle` prose".into(),
            range: crate::definition::LspRange {
                start_line: 2,
                start_character: 0,
                end_line: 2,
                end_character: 7,
            },
        };
        let registry = analysis.resolved_registry().unwrap();
        let document = registry
            .document_grammar()
            .expect("actual selected pack source vocabulary");
        let pack = registry
            .authored_document_member_grammar(document, "speclib")
            .expect("actual admitted document member source vocabulary");
        assert!(
            registry
                .authored_document_member_grammar(pack, "command")
                .is_some()
        );
        assert!(
            data.notice()
                .unwrap()
                .matches(source, &analysis, registry, &diagnostic),
            "the independently issued subject must retain current source/configuration/Registry"
        );
        assert_eq!(
            crate::code_actions::statement_head_at(source, subject.span().start(), config)
                .as_deref(),
            Some("arty"),
            "the exact subject must select its own statement rather than an enclosing declaration"
        );
        let grammar = crate::oo_body::definition_grammar_at(
            source,
            subject.span().start(),
            registry,
            config,
            analysis
                .resolved_profile()
                .map(tcl_dialect::DialectProfile::surface_query),
        )
        .expect("the actual selected SpecTcl declaration context must retain its grammar");
        assert!(
            grammar
                .members
                .iter()
                .any(|member| member.keyword == "arity"),
            "the selected command-body vocabulary must contain the actual replacement"
        );
        let actions = crate::code_actions::spec_pack_quick_fixes(
            source,
            &analysis,
            std::slice::from_ref(&diagnostic),
        );
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].edits[0].new_text, "arity");
        diagnostic.message = "display-only notice with no quoted word".into();
        assert_eq!(
            crate::code_actions::spec_pack_quick_fixes(
                source,
                &analysis,
                std::slice::from_ref(&diagnostic)
            ),
            actions
        );
        diagnostic.data = None;
        assert!(
            crate::code_actions::spec_pack_quick_fixes(
                source,
                &analysis,
                std::slice::from_ref(&diagnostic)
            )
            .is_empty()
        );
        diagnostic.data = Some(data);
        let changed = format!("# changed owner\n{source}");
        let changed_analysis = Analyser::new().analyse(&changed, "spectcl");
        assert!(
            crate::code_actions::spec_pack_quick_fixes(
                &changed,
                &changed_analysis,
                std::slice::from_ref(&diagnostic)
            )
            .is_empty()
        );
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(
            crate::code_actions::spec_pack_quick_fixes(
                source,
                &analysis,
                std::slice::from_ref(&diagnostic)
            )
            .is_empty()
        );
        assert!(
            ContextDiagnosticData::from_value(
                &serde_json::json!({"kind":"spectcl-source-notice","version":1,"token":"0"}),
                SPEC_PACK_DIAGNOSTIC_CODE
            )
            .is_none()
        );
        let duplicate = "speclib demo 1 {command demo::x {arty 1; arty 2}}";
        assert!(
            SpecPackNoticeSubject::from_loader(
                duplicate,
                config,
                1,
                "arty",
                SpecPackNoticeKind::Property
            )
            .is_none()
        );
    }
}
