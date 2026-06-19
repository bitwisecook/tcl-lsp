//! Formatter configuration — Rust port of
//! `core/formatting/config.py`.
//!
//! Defaults follow the F5 iRules Style Guide.  Only the fields the
//! engine consults today are modelled; the docstring-related knobs
//! are carried for parity but the docstring rewriter itself is a
//! later sub-strip.

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

/// All configurable formatting options.  Mirrors Python's
/// `FormatterConfig`; `Default` reproduces its dataclass defaults.
///
/// This is a configuration DTO — the boolean toggles are
/// independent user-facing settings, not a state machine, so the
/// `struct_excessive_bools` allow follows the same convention the
/// registry's `spec.rs` / `events.rs` config structs use.
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
    /// Rewrite a bare expression argument as a braced `{ … }` expr.
    /// Config-surface parity with Python `enforce_braced_expr`
    /// (GAP-C4): declared and serialised, but — like the Python
    /// field — not yet consumed by the engine (the behaviour is
    /// hardcoded on both sides).
    pub enforce_braced_expr: bool,
    /// Align trailing inline comments to a consistent column.
    /// Config-surface parity with Python `align_comments_to_code`
    /// (GAP-C4): declared and serialised, not yet engine-consumed on
    /// either side.
    pub align_comments_to_code: bool,
    /// Line ending for formatted output.
    pub line_ending: String,
    /// Ensure the file ends with a newline.
    pub ensure_final_newline: bool,
    /// Expand single-line command bodies onto multiple lines.
    pub expand_single_line_bodies: bool,
    /// Minimum number of commands a body must contain before
    /// `expand_single_line_bodies` forces it onto multiple lines.
    /// Config-surface parity with Python
    /// `min_body_commands_for_expansion` (GAP-C4): declared and
    /// serialised, not yet engine-consumed on either side.
    pub min_body_commands_for_expansion: usize,
    /// Split `;`-separated commands onto their own lines.
    /// Config-surface parity with Python
    /// `replace_semicolons_with_newlines` (GAP-C4): declared and
    /// serialised, but semicolon-splitting is hardcoded-on in both
    /// the Python and Rust engines, so the flag is not yet consumed.
    pub replace_semicolons_with_newlines: bool,
    /// Blank lines between proc definitions.
    pub blank_lines_between_procs: usize,
    /// Blank lines between top-level blocks.
    pub blank_lines_between_blocks: usize,
    /// Maximum consecutive blank lines to keep.
    pub max_consecutive_blank_lines: usize,
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
            line_ending: "\n".to_owned(),
            ensure_final_newline: true,
            expand_single_line_bodies: false,
            min_body_commands_for_expansion: 2,
            replace_semicolons_with_newlines: true,
            blank_lines_between_procs: 1,
            blank_lines_between_blocks: 1,
            max_consecutive_blank_lines: 2,
        }
    }
}

impl FormatterConfig {
    /// Build the indentation string for nesting `level`.  Mirrors
    /// Python's `_make_indent`.
    #[must_use]
    pub fn make_indent(&self, level: usize) -> String {
        match self.indent_style {
            IndentStyle::Tabs => "\t".repeat(level),
            IndentStyle::Spaces => " ".repeat(self.indent_size * level),
        }
    }
}
