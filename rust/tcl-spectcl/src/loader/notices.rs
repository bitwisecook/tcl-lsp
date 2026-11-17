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

//! The notices a load reports: what was dropped, words newer than the declared vocabulary, and the log that accumulates them.

use super::statement_reader::{Stmt, Word};
use super::vocabulary_class::VocabularyClass;
use super::{Log, RejectedWordSubject, vocabulary_class};
use std::fmt;

// Notices

/// One thing the loader dropped, with enough context to fix the pack.
///
/// Every notice is a *degradation*, never a failure: the declaration it names
/// is discarded and the rest of the pack loads. That is what makes "new server
/// + old pack" and "old server + new pack" both work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Where in the pack the notice arose (`"command lsort"`,
    /// `"command string / subcommand is"`, `"pack"`).
    pub context: String,
    /// 1-based source line of the offending statement.
    pub line: u32,
    /// What was dropped, and why.
    pub message: String,
    /// The §6.1 compatibility class of the word this notice is about.
    ///
    /// [`Presentation`](VocabularyClass::Presentation) for everything the
    /// loader has always warned-and-dropped; the stronger classes are only
    /// ever reached in the forward direction (a pack declaring a vocabulary
    /// this build postdates) and inside the `dialect` / `environment`
    /// blocks, where every word is semantic by construction.
    pub class: VocabularyClass,
    /// Typed literal source position of a rejected property/flag, when known.
    pub subject: Option<tcl_lsp_core::code_actions::SpecPackNoticeSubject>,
    /// Present exactly when the notice reports a word newer than the pack's
    /// declared vocabulary.
    pub newer_vocabulary: Option<NewerVocabulary>,
}

/// A word the pack uses that its declared vocabulary does not have yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewerVocabulary {
    /// The word as written.
    pub word: String,
    /// The first vocabulary that has the word.
    pub since: &'static str,
    /// The vocabulary the pack declares.
    pub declared: String,
}

impl fmt::Display for Notice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.context, self.line, self.message)
    }
}

impl Log {
    /// Record a use of `word`, which no vocabulary before `since` has.
    pub(super) fn since(&mut self, line: u32, word: &str, since: &'static str) {
        self.newer_words
            .push((self.context.clone(), line, word.to_owned(), since));
    }

    pub(super) fn v11(&mut self, line: u32, word: &str) {
        self.since(line, word, "1.1");
    }

    pub(super) fn v12(&mut self, line: u32, word: &str) {
        self.since(line, word, "1.2");
    }

    pub(super) fn v20(&mut self, line: u32, word: &str) {
        self.since(line, word, "2.0");
    }

    pub(super) fn v21(&mut self, line: u32, word: &str) {
        self.since(line, word, "2.1");
    }

    pub(super) fn say(&mut self, line: u32, message: impl Into<String>) {
        self.notices.push(Notice {
            subject: None,
            newer_vocabulary: None,
            context: self.context.clone(),
            line,
            message: message.into(),
            class: VocabularyClass::Presentation,
        });
    }

    /// Say something in a stronger §6.1 class, and remember that the
    /// current spec was degraded by it.
    pub(super) fn say_classified(
        &mut self,
        line: u32,
        class: VocabularyClass,
        message: impl Into<String>,
    ) {
        self.notices.push(Notice {
            subject: None,
            newer_vocabulary: None,
            context: self.context.clone(),
            line,
            message: message.into(),
            class,
        });
        match class {
            VocabularyClass::Presentation => {}
            VocabularyClass::Assistance => self.assistance_unknown = true,
            VocabularyClass::Semantic => self.semantic_unknown = true,
        }
    }

    /// Classify an unknown `word` and report it.
    ///
    /// The escalation past `Presentation` happens **only** when the pack
    /// declares a vocabulary this build postdates — §6.1's forward
    /// direction, "an older loader meeting newer vocabulary". An unknown
    /// word in a pack whose vocabulary this build knows in full is an
    /// author's typo, not a word with a meaning being dropped, and it keeps
    /// today's warn-and-drop treatment exactly.
    fn unknown_word(&mut self, line: u32, word: &str, message: String) {
        let class = if self.forward_vocabulary {
            vocabulary_class::classify(word)
        } else {
            VocabularyClass::Presentation
        };
        match class {
            VocabularyClass::Presentation => self.say(line, message),
            VocabularyClass::Assistance => self.say_classified(
                line,
                class,
                format!(
                    "{message}; it is assistance-class vocabulary this build does not \
                     speak, so the command loads with the affected capability degraded \
                     (design §6.1)"
                ),
            ),
            VocabularyClass::Semantic => self.say_classified(
                line,
                class,
                format!(
                    "{message}; it is semantic-class vocabulary this build does not \
                     speak, so the command is excluded from strong analysis rather than \
                     analysed without it (design §6.1)"
                ),
            ),
        }
    }

    /// Forget the per-spec degradation state before reading a new spec.
    pub(super) fn begin_spec(&mut self) {
        self.assistance_unknown = false;
        self.semantic_unknown = false;
    }

    /// Run `body` with `context` pushed, restoring the previous label after.
    pub(super) fn scoped<T>(&mut self, context: String, body: impl FnOnce(&mut Self) -> T) -> T {
        let previous = std::mem::replace(&mut self.context, context);
        let out = body(self);
        self.context = previous;
        out
    }

    pub(super) fn unknown_property(&mut self, stmt: &Stmt) {
        // `word_text(0)` on a braced word is *every byte between the braces*,
        // so interpolating it raw would put whole multi-line bodies into a
        // Problems-panel message. A diagnostic has to stay one
        // readable line, and a block that reached here is an orphan — the
        // useful thing to say is that it has no preceding declaration, not to
        // quote it back.
        let stmt_is_block = stmt.words.first().is_some_and(|w| w.braced);
        if stmt_is_block {
            self.say(
                stmt.line,
                "a `{ … }` block with no preceding declaration was dropped \
                 (an opening brace must be on the same line as the word it \
                 belongs to)",
            );
            return;
        }
        // `quotable` is the existing helper for exactly this — `unknown_flag`
        // has always used it; `unknown_property` simply never did, which is
        // the whole of the defect.
        let raw = stmt.word_text(0);
        let word = quotable(raw);
        let message = format!("unknown property `{word}` dropped");
        self.unknown_word(stmt.line, raw, message);
        self.attach_subject(RejectedWordSubject {
            original: stmt
                .words
                .first()
                .and_then(|word| word.original_source.clone()),
            word: raw.to_owned(),
            kind: tcl_lsp_core::code_actions::SpecPackNoticeKind::Property,
        });
    }

    /// The subject of a notice about the row that starts on `line`.
    pub(super) fn row_subject(
        &mut self,
        line: u32,
    ) -> Option<tcl_lsp_core::code_actions::SpecPackNoticeSubject> {
        let source = self.source.as_deref()?;
        self.rows
            .get_or_insert_with(|| {
                tcl_lsp_core::code_actions::SpecPackNoticeSubject::for_rows(
                    source,
                    super::source_config(super::FileBom::Skip),
                )
            })
            .get(&line)
            .cloned()
    }

    fn attach_subject(&mut self, subject: RejectedWordSubject) {
        let Some(source) = &self.source else {
            return;
        };
        let Some(original) = subject
            .original
            .filter(|original| original.image().bytes() == source.as_bytes())
        else {
            return;
        };
        let geometry = tcl_lsp_core::code_actions::SpecPackNoticeSubject::from_original_word(
            original,
            &subject.word,
            subject.kind,
        );
        if let Some(notice) = self.notices.last_mut() {
            notice.subject = geometry.filter(|geometry| geometry.line() == notice.line);
        }
    }

    // Detached evaluated/list values report notices without a source subject.
    pub(super) fn unknown_flag_value(&mut self, row: &str, line: u32, flag: &str) {
        let message = format!("unknown flag `{}` on `{row}` dropped", quotable(flag));
        self.unknown_word(line, flag, message);
    }

    pub(super) fn unknown_flag(&mut self, row: &str, line: u32, flag: &Word) {
        let message = format!("unknown flag `{}` on `{row}` dropped", quotable(&flag.text));
        self.unknown_word(line, &flag.text, message);
        self.attach_subject(RejectedWordSubject {
            original: flag.original_source.clone(),
            word: flag.text.clone(),
            kind: tcl_lsp_core::code_actions::SpecPackNoticeKind::Flag {
                row: row.to_owned(),
            },
        });
    }
}

/// The longest a word quoted inside a notice may be before it is elided.
const QUOTABLE_WIDTH: usize = 40;

/// A word as a notice can quote it: one physical line, bounded length.
///
/// An unknown flag is *usually* a short word, but a flag the loader does not
/// know consumes no value, so the braced word that followed it is read as a
/// flag in its own right on the next turn — and that word can be a twenty-line
/// block (`apave.tclspec`'s invented `-repeats { row { … } }`). Quoting it
/// verbatim would put newlines inside a notice message, which the notice's
/// consumers cannot carry: `tests/spec_corpus_baseline.txt` is one
/// tab-separated record per line, and an editor's problems pane is one line
/// per diagnostic.
pub(super) fn quotable(word: &str) -> String {
    // The first line of a braced word is usually empty — the brace is followed
    // by a newline — so the first line with content is what identifies it.
    let head = word
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let elided = head.len() < word.trim().len();
    if head.chars().count() <= QUOTABLE_WIDTH {
        return if elided {
            format!("{head}…")
        } else {
            head.to_owned()
        };
    }
    let cut: String = head.chars().take(QUOTABLE_WIDTH).collect();
    format!("{cut}…")
}
