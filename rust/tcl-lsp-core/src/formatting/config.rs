//! Formatter configuration.
//!
//! Defaults follow the F5 iRules Style Guide.  Every configurable
//! formatter field is modelled here as an editor-settings/config knob;
//! the docstring-related knobs (`docstring_*`) are carried but the
//! docstring rewriter itself is not yet implemented, so they are not yet
//! consulted by the engine.

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
/// Carried as an editor-settings/config knob; the docstring rewriter itself
/// is not yet implemented, so the value is not yet consumed by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocstringStyle {
    /// Comment block above the `proc` statement.
    Preceding,
    /// Comment block at the start of the `proc` body.
    Body,
    /// Do not generate or reformat docstrings.
    None,
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

/// All configurable formatting options.
///
/// This is a flat configuration DTO: every boolean is an independent,
/// user-facing on/off setting (not a state machine).
///
/// `struct_excessive_bools` is kept here deliberately. The only fix the
/// lint accepts is grouping the bools into sub-structs, but the public
/// flat-field API is consumed cross-crate by `tcl-lsp-server`
/// (`formatter_config_from` builds it field-by-field, and a struct
/// literal in `lib.rs` plus several `.field` reads depend on the flat
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
    /// Rewrite a bare expression argument as a braced `{ … }` expr.
    /// Declared and serialised, but not yet consumed by the engine —
    /// the behaviour is hardcoded.
    pub enforce_braced_expr: bool,
    /// Align trailing inline comments to a consistent column.
    /// Declared and serialised, not yet engine-consumed.
    pub align_comments_to_code: bool,
    /// Line ending for formatted output.
    pub line_ending: String,
    /// Ensure the file ends with a newline.
    pub ensure_final_newline: bool,
    /// Expand single-line command bodies onto multiple lines.
    pub expand_single_line_bodies: bool,
    /// Minimum number of commands a body must contain before
    /// `expand_single_line_bodies` forces it onto multiple lines.
    /// Declared and serialised, not yet engine-consumed.
    pub min_body_commands_for_expansion: usize,
    /// Split `;`-separated commands onto their own lines.
    /// Declared and serialised, but semicolon-splitting is
    /// hardcoded-on, so the flag is not yet consumed.
    pub replace_semicolons_with_newlines: bool,
    /// Blank lines between proc definitions.
    pub blank_lines_between_procs: usize,
    /// Blank lines between top-level blocks.
    pub blank_lines_between_blocks: usize,
    /// Maximum consecutive blank lines to keep.
    pub max_consecutive_blank_lines: usize,
    /// Where docstrings are placed relative to `proc` definitions.
    /// Carried as an editor-settings/config knob; not yet engine-consumed
    /// (the docstring rewriter is unimplemented).
    pub docstring_style: DocstringStyle,
    /// Tag format used in docstrings. Carried as a config knob, not yet consumed.
    pub docstring_tag_style: DocstringTagStyle,
    /// Add decoration border lines (e.g. `# ......`) around docstrings.
    /// Carried as a config knob, not yet consumed.
    pub docstring_decoration: bool,
    /// Character used for docstring decoration borders. Carried as a config knob.
    pub docstring_decoration_char: char,
    /// Width of docstring decoration border lines. Carried as a config knob.
    pub docstring_decoration_width: usize,
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
            docstring_style: DocstringStyle::None,
            docstring_tag_style: DocstringTagStyle::Doxygen,
            docstring_decoration: false,
            docstring_decoration_char: '.',
            docstring_decoration_width: 70,
        }
    }
}

impl FormatterConfig {
    /// Build the indentation string for nesting `level`.
    #[must_use]
    pub fn make_indent(&self, level: usize) -> String {
        match self.indent_style {
            IndentStyle::Tabs => "\t".repeat(level),
            IndentStyle::Spaces => " ".repeat(self.indent_size * level),
        }
    }
}
