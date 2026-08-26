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

//! Formatter configuration.
//!
//! Defaults follow the F5 iRules Style Guide.  Every configurable
//! formatter field is modelled here as an editor-settings/config knob.
//!
//! The docstring knobs split by consumer: `docstring_tag_style` and the
//! `docstring_decoration*` fields drive the docstring-stub generator's
//! *content* ([`generate_stub_for_proc`](super::docstring::generate_stub_for_proc) —
//! the "generate docstring" code action and the MCP docstring tool).
//! `docstring_style` drives the code action's *placement*: the LSP server's
//! `code_action` handler resolves it from `tclLsp.formatting.docstringStyle`
//! and threads it into `code_actions_in_program`, which inserts the
//! generated stub above the `proc` (`Preceding`) or as the first line of its
//! body (`Body`), or offers no stub-generation action at all (`None`, the
//! documented default — "do not generate or reformat docstrings"). The
//! formatter itself never rewrites an *existing* docstring, so none of the
//! docstring knobs affect a plain format pass — only the explicit
//! generate-docstring action.

use tcl_dialect::DialectProfile;
use tcl_lexer::LexerConfig;
use tcl_registry::prelude::DialectSet;

/// Where to place opening braces.  Only K&R is supported (the F5
/// style-guide default); the enum exists so the field can grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceStyle {
    /// Opening brace at the end of the line.
    KAndR,
}

/// What characters to use for indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    /// Indent with spaces (`indent_size` per level).
    Spaces,
    /// Indent with tabs (one tab per level).
    Tabs,
}

/// Where a docstring comment block is placed relative to a `proc`.
///
/// Consumed by the "Generate docstring" code action
/// (`tcl_lsp_core::code_actions::code_actions_in_program`): `Preceding` and
/// `Body` choose the insertion point for a newly-generated stub; `None`
/// suppresses the action entirely (no stub is generated). The formatter
/// never moves or rewrites an *existing* docstring — this only governs where
/// a new one is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocstringStyle {
    /// Comment block above the `proc` statement.
    Preceding,
    /// Comment block at the start of the `proc` body.
    Body,
    /// Do not generate or reformat docstrings.
    None,
}

impl DocstringStyle {
    /// Parse the editor-settings spelling (`"preceding"`, `"body"`,
    /// `"none"`, case-insensitive). An unrecognised value falls back to
    /// [`Self::None`] — the same default `FormatterConfig` and every editor
    /// catalogue ship.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "preceding" => Self::Preceding,
            "body" => Self::Body,
            _ => Self::None,
        }
    }
}

/// Tag format used inside docstrings for parameter/return documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocstringTagStyle {
    /// Doxygen-style tags: `@param`, `@return`, `@brief`.
    Doxygen,
    /// Plain prose with an `Arguments:` section.
    Plain,
    /// Leave the tag format as-is.
    None,
}

/// Which spelling pair every boolean-consumed word is normalised to
/// (#1233).
///
/// Tcl accepts `true`/`false`, `yes`/`no`, `on`/`off`, `0`/`1`, and any
/// unique prefix of the word forms, wherever a value is consumed as a
/// boolean. One global choice is applied everywhere, with no per-context
/// carve-outs, so a file reads with one convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BooleanForm {
    /// `true` / `false` — the default.
    #[default]
    TrueFalse,
    /// `yes` / `no`.
    YesNo,
    /// `on` / `off`.
    OnOff,
    /// `1` / `0`.
    ZeroOne,
    /// Leave every boolean word's bytes untouched.
    Preserve,
}

impl BooleanForm {
    /// The `(true-spelling, false-spelling)` pair, or `None` for
    /// [`Self::Preserve`].
    #[must_use]
    pub const fn pair(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::TrueFalse => Some(("true", "false")),
            Self::YesNo => Some(("yes", "no")),
            Self::OnOff => Some(("on", "off")),
            Self::ZeroOne => Some(("1", "0")),
            Self::Preserve => None,
        }
    }

    /// Parse the editor-settings spelling (`"true/false"`, `"yes/no"`,
    /// `"on/off"`, `"0/1"`, `"preserve"`). An unrecognised value falls back
    /// to the default.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "yes/no" => Self::YesNo,
            "on/off" => Self::OnOff,
            "0/1" => Self::ZeroOne,
            "preserve" => Self::Preserve,
            _ => Self::TrueFalse,
        }
    }
}

/// All configurable formatting options.
///
/// This is a flat configuration DTO of *style* knobs: every boolean is an
/// independent, user-facing on/off setting (not a state machine). The one
/// non-style member is [`Self::profile`], the resolved dialect every
/// dialect-derived fact is projected from — build it with
/// [`Self::for_profile`] / [`Self::for_dialect`] and set the style knobs on
/// top.
///
/// `struct_excessive_bools` is kept here deliberately. The only fix the
/// lint accepts is grouping the bools into sub-structs, but the public
/// flat-field API is consumed cross-crate by `tcl-lsp-server`
/// (`formatter_config_from` sets the style knobs field-by-field, and
/// several `.field` reads in `lib.rs` depend on the flat
/// shape). Regrouping cannot be done without editing `tcl-lsp-server`,
/// which is out of scope for this crate, so the allow stays until that
/// cross-crate change is made together.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatterConfig {
    /// Spaces per indentation level (when `indent_style == Spaces`).
    pub indent_size: usize,
    /// Spaces vs tabs.
    pub indent_style: IndentStyle,
    /// Extra indentation for continuation lines.
    pub continuation_indent: usize,
    /// Brace placement style.
    pub brace_style: BraceStyle,
    /// Insert spaces inside single-line braces: `{ body }`.
    pub space_between_braces: bool,
    /// Hard limit for line length; longer lines are wrapped.
    pub max_line_length: usize,
    /// Soft target for line length.
    pub goal_line_length: usize,
    /// Ensure a space after `#` in comments.
    pub space_after_comment_hash: bool,
    /// Remove trailing whitespace from lines.
    pub trim_trailing_whitespace: bool,
    /// Rewrite `$var` as `${var}` for consistency.
    pub enforce_braced_variables: bool,
    /// Rewrite a bare expression argument as a braced `{ … }` expr. Registry
    /// `ArgRole::Expr` marks the expression argument(s); a command flagged
    /// `EXPR_CONCATENATES_ARGS` (`expr`) has its whole tail braced as one
    /// group, while a bounded condition (`if`/`while`/`for`) is braced in
    /// place.
    pub enforce_braced_expr: bool,
    /// Align comments to the surrounding code's indentation. When `true` (the
    /// default) a standalone comment is re-indented to the code column; when
    /// `false` the comment keeps its original column.
    pub align_comments_to_code: bool,
    /// Line ending for formatted output: `"\n"`, `"\r\n"`, `"\r"`, or the
    /// sentinel [`LINE_ENDING_AUTO`] (the default) meaning "keep whatever the
    /// document already uses". Resolve it against the text being formatted
    /// with [`FormatterConfig::resolved_line_ending`] — never read this field
    /// directly at a substitution point, or an `auto` config will write the
    /// literal string `auto` into the output.
    pub line_ending: String,
    /// Ensure the file ends with a newline.
    pub ensure_final_newline: bool,
    /// Expand single-line command bodies onto multiple lines.
    pub expand_single_line_bodies: bool,
    /// Minimum number of commands a body must contain before
    /// `expand_single_line_bodies` forces it onto multiple lines. A body with
    /// fewer commands stays inline even when expansion is enabled.
    pub min_body_commands_for_expansion: usize,
    /// Split `;`-separated commands onto their own lines. When `true` (the
    /// default) `a; b` becomes two lines; when `false` such commands are kept
    /// on one line joined by `; `.
    pub replace_semicolons_with_newlines: bool,
    /// Blank lines between proc definitions.
    pub blank_lines_between_procs: usize,
    /// Blank lines between top-level blocks.
    pub blank_lines_between_blocks: usize,
    /// Maximum consecutive blank lines to keep.
    pub max_consecutive_blank_lines: usize,
    /// Where docstrings are placed relative to `proc` definitions, or
    /// whether stub generation is offered at all (see [`DocstringStyle`]).
    /// Round-tripped here for config-consumer parity; the LSP server's
    /// `code_action` handler resolves the same setting independently and
    /// passes it to `code_actions_in_program`, which is the actual consumer.
    pub docstring_style: DocstringStyle,
    /// Tag format the docstring-stub generator emits (`@param` vs an
    /// `Arguments:` prose block). Consumed by `generate_stub_for_proc`.
    pub docstring_tag_style: DocstringTagStyle,
    /// Whether the stub generator wraps a docstring in decoration border lines
    /// (e.g. `# ......`). Consumed by `generate_stub_for_proc`.
    pub docstring_decoration: bool,
    /// Character used for docstring decoration borders (stub generator).
    pub docstring_decoration_char: char,
    /// Width of docstring decoration border lines (stub generator).
    pub docstring_decoration_width: usize,
    /// Expand unique-prefix keyword abbreviations to their canonical
    /// spellings (`string le` → `string length`, `lsearch -noc` →
    /// `lsearch -nocase`).  Default `true`.  Ambiguous and unknown words are
    /// always left alone — the formatter never guesses.  See
    /// [`super::keywords`].
    pub expand_abbreviations: bool,
    /// Which spelling every boolean-consumed word is normalised to.  Default
    /// [`BooleanForm::TrueFalse`]; [`BooleanForm::Preserve`] turns the
    /// rewrite off.  Only *consumption* sites the registry proves boolean are
    /// rewritten — never a value-definition site.
    pub boolean_form: BooleanForm,
    /// The document's resolved dialect profile — the **single** dialect fact
    /// the formatter carries (issue #1465).
    ///
    /// The formatter needs three dialect-derived facts: the lexer preset it
    /// tokenises with ([`Self::lexer_config`]), the availability mask that
    /// filters per-release rewrite candidates ([`Self::dialect_bits`]), and
    /// the release range a rewrite must stay correct across
    /// ([`Self::target_range`]). All three are pure projections of this
    /// profile and are derived from it here, so a caller can no longer set a
    /// strict subset and format an iRule with the Tcl 9 lexer.
    ///
    /// Defaults to the lenient environment's profile — the permissive
    /// modern-Tcl one: the 9.x lexing grammar, no candidate filter, and no
    /// forward range. Resolve a real one with [`Self::for_profile`] or
    /// [`Self::for_dialect`] (which alias-normalises, so both iRules
    /// spellings land on the same profile).
    pub profile: &'static DialectProfile,
    /// An explicit **range of releases** the document must stay correct
    /// across, replacing the profile's own forward range.
    ///
    /// `None` — the default — means [`Self::target_range`] answers with the
    /// profile's forward-compatibility range
    /// ([`tcl_registry::version_range::forward_range`]): the document's own
    /// release and every later one, which is what a document written for one
    /// release and run on a later one needs. Set this only for a document
    /// that must *also* keep working on a release the profile does not
    /// imply — a backward range no single profile can express.
    pub target_range_override: Option<DialectSet>,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            indent_style: IndentStyle::Spaces,
            continuation_indent: 4,
            brace_style: BraceStyle::KAndR,
            space_between_braces: true,
            max_line_length: 120,
            goal_line_length: 100,
            space_after_comment_hash: true,
            trim_trailing_whitespace: true,
            enforce_braced_variables: false,
            enforce_braced_expr: false,
            align_comments_to_code: true,
            line_ending: LINE_ENDING_AUTO.to_owned(),
            ensure_final_newline: true,
            expand_single_line_bodies: false,
            min_body_commands_for_expansion: 2,
            replace_semicolons_with_newlines: true,
            blank_lines_between_procs: 1,
            blank_lines_between_blocks: 1,
            max_consecutive_blank_lines: 2,
            docstring_style: DocstringStyle::None,
            docstring_tag_style: DocstringTagStyle::Doxygen,
            docstring_decoration: false,
            docstring_decoration_char: '.',
            docstring_decoration_width: 70,
            expand_abbreviations: true,
            boolean_form: BooleanForm::TrueFalse,
            // The "no dialect stated" ingress, through the one seam.
            profile: crate::profile_for_dialect(""),
            target_range_override: None,
        }
    }
}

/// Sentinel value of [`FormatterConfig::line_ending`] meaning "use the line
/// ending the document already has" — the default, and what the editors'
/// `tclLsp.formatting.lineEnding: "auto"` maps to.
pub const LINE_ENDING_AUTO: &str = "auto";

/// The line ending `source` already uses: whichever of `\r\n`, a lone `\r`, or
/// `\n` occurs first. A document with no line break at all (a single line, no
/// trailing newline) has no evidence either way and yields `"\n"`.
#[must_use]
pub fn detect_line_ending(source: &str) -> &'static str {
    let bytes = source.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'\n' => return "\n",
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => return "\r\n",
            b'\r' => return "\r",
            _ => {}
        }
    }
    "\n"
}

impl FormatterConfig {
    /// Default formatting knobs, targeting `profile`.
    ///
    /// The one way to aim the formatter at a dialect: the lexer preset, the
    /// rewrite-candidate filter, and the forward range all follow from this
    /// one resolved profile, so none of them can be forgotten (issue #1465).
    #[must_use]
    pub fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            profile,
            ..Self::default()
        }
    }

    /// [`Self::for_profile`] from a dialect *name*, resolved through the
    /// one environment ingress ([`crate::profile_for_dialect`]) — so an
    /// alias spelling (`irules`, `tcl-irule`), an editor language id, and
    /// the canonical name all select the same environment, and an unknown
    /// name lands on the permissive modern-Tcl fallback rather than a
    /// mismatched default.
    #[must_use]
    pub fn for_dialect(dialect: &str) -> Self {
        Self::for_profile(crate::profile_for_dialect(dialect))
    }

    /// The lexer preset the formatter tokenises with, from the profile's
    /// grammar.
    ///
    /// Carries the dialect's parsing rules — notably
    /// `irules_brace_separator`, which makes an iRule's `}{` (valid in TMM,
    /// e.g. `if {expr}{body}`) parse as two words so the formatter re-emits
    /// it as `} {`, and `expand_syntax`, which is off for the 8.4-based
    /// dialects so a `{*}` there stays a literal braced word.
    #[must_use]
    pub fn lexer_config(&self) -> LexerConfig {
        LexerConfig::from_grammar(self.profile.grammar)
    }

    /// The availability mask the profile's own release(s) contribute, or
    /// `None` when the profile is the permissive fallback — i.e. no dialect
    /// was declared, so every keyword the handed registry declares stays a
    /// rewrite candidate (the pre-#1257 conservative direction).
    #[must_use]
    pub fn dialect_bits(&self) -> Option<DialectSet> {
        if self.profile.is_fallback() {
            None
        } else {
            Some(crate::document_context_for_profile(self.profile).authoring_mask())
        }
    }

    /// The range of releases a rewrite must stay correct across: the explicit
    /// [`Self::target_range_override`], else the profile's
    /// forward-compatibility range.
    ///
    /// An abbreviation is expanded only when it resolves to the *same*
    /// keyword in every release of this range, so a prefix a later Tcl makes
    /// ambiguous is left alone rather than expanded into one of its
    /// candidates. A vendor dialect that names no core release (`f5-irules`)
    /// yields the empty range — the handed registry is then the whole story.
    #[must_use]
    pub fn target_range(&self) -> DialectSet {
        self.target_range_override
            .unwrap_or_else(|| tcl_registry::version_range::forward_range(self.profile.name))
    }

    /// The concrete line ending to emit when formatting `source`: the
    /// configured one, or — for the [`LINE_ENDING_AUTO`] default — the one
    /// `source` already uses ([`detect_line_ending`]).
    ///
    /// Every newline a formatter or code action writes into a document must go
    /// through here, so an edit never mixes `\n` into a CRLF (or old-Mac) file.
    #[must_use]
    pub fn resolved_line_ending<'a>(&'a self, source: &str) -> &'a str {
        if self.line_ending == LINE_ENDING_AUTO {
            detect_line_ending(source)
        } else {
            &self.line_ending
        }
    }

    /// Build the indentation string for nesting `level`.
    #[must_use]
    pub fn make_indent(&self, level: usize) -> String {
        match self.indent_style {
            IndentStyle::Tabs => "\t".repeat(level),
            IndentStyle::Spaces => " ".repeat(self.indent_size * level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatterConfig, LINE_ENDING_AUTO};
    use tcl_dialect::DialectProfile;
    use tcl_registry::prelude::DialectSet;

    #[test]
    fn every_dialect_fact_follows_from_the_one_profile() {
        // Issue #1465: the lexer preset, the candidate-filter mask, and the
        // forward range are projections of the resolved profile, so aiming
        // the formatter at iRules is one decision, not three.
        let cfg = FormatterConfig::for_profile(DialectProfile::irules());
        assert!(cfg.profile.is_irules());
        // The iRules grammar: the `}{` ghost separator on, and — an 8.4 core
        // — `{*}` expansion off, so `{*}` stays a literal braced word.
        assert!(cfg.lexer_config().irules_brace_separator);
        assert!(!cfg.lexer_config().expand_syntax);
        assert_eq!(cfg.dialect_bits(), Some(DialectSet::IRULES));
        // `f5-irules` names its own runtime, not a core release: no forward
        // range to widen an abbreviation over.
        assert!(cfg.target_range().is_empty());
    }

    #[test]
    fn both_irules_spellings_resolve_the_same_profile() {
        // The canonical name and its legacy aliases are one dialect; a caller
        // passing either gets the iRules lexer.
        for spelling in ["f5-irules", "irules", "tcl-irule"] {
            let cfg = FormatterConfig::for_profile(tcl_dialect::DialectProfile::by_name(spelling));
            assert!(cfg.profile.is_irules(), "{spelling}");
            assert!(cfg.lexer_config().irules_brace_separator, "{spelling}");
        }
    }

    #[test]
    fn a_core_release_projects_its_mask_and_forward_range() {
        let cfg = FormatterConfig::for_profile(tcl_dialect::DialectProfile::by_name("tcl8.6"));
        assert_eq!(cfg.dialect_bits(), Some(DialectSet::TCL86));
        assert_eq!(
            tcl_registry::version_range::core_releases_in(cfg.target_range()),
            vec!["tcl8.6", "tcl9.0", "tcl9.1"]
        );
        assert!(cfg.lexer_config().expand_syntax);
        assert!(!cfg.lexer_config().irules_brace_separator);
    }

    #[test]
    fn the_default_is_the_modern_tcl_profile() {
        let cfg = FormatterConfig::default();
        assert!(cfg.profile.is_fallback());
        // Modern lexing: `{*}` expansion and Tcl 9's nesting `${…}` rule.
        assert!(cfg.lexer_config().expand_syntax);
        assert!(!cfg.lexer_config().irules_brace_separator);
        // No dialect declared, so no candidate filter and no range — the
        // conservative direction, unchanged from before #1465.
        assert_eq!(cfg.dialect_bits(), None);
        assert!(cfg.target_range().is_empty());
        assert_eq!(cfg.line_ending, LINE_ENDING_AUTO);
    }

    #[test]
    fn an_explicit_range_overrides_the_profiles_forward_one() {
        // A document that must also keep working on an *older* release than
        // its own names a range no profile implies.
        let cfg = FormatterConfig {
            target_range_override: Some(DialectSet::TCL86 | DialectSet::TCL90),
            ..FormatterConfig::for_profile(tcl_dialect::DialectProfile::by_name("tcl9.0"))
        };
        assert_eq!(
            tcl_registry::version_range::core_releases_in(cfg.target_range()),
            vec!["tcl8.6", "tcl9.0"]
        );
        // The rest of the dialect still follows the profile.
        assert_eq!(cfg.dialect_bits(), Some(DialectSet::TCL90));
    }

    #[test]
    fn an_unknown_dialect_name_lands_on_the_permissive_fallback() {
        let cfg = FormatterConfig::for_profile(tcl_dialect::DialectProfile::by_name("tcl-9000"));
        assert!(cfg.profile.is_fallback());
        assert_eq!(cfg, FormatterConfig::default());
    }
}
