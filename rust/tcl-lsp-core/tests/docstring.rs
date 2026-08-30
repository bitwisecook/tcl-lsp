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

//! Tests for docstring handling on the `tcl-lsp-core` / `tcl-compiler`
//! public surface.
//!
//! Docstring handling spans three behaviours, exposed through a narrow
//! public API split across the tree:
//!
//!   * EXTRACTION — `tcl_compiler::analyser::utils::extract_body_docstring`
//!     pulls the leading `#`-comment block out of a proc *body*, stripping
//!     decoration lines and stopping at the first blank line / code line.
//!   * PARSE + RENDER — the `@brief`/`@param`/`@return` parser and its
//!     Markdown renderer live in `tcl-lsp-core/src/hover.rs` as the
//!     PRIVATE `format_docstring`.  It is reachable only through the
//!     public `hover()` entry, which harvests a proc's leading
//!     doc-comment into `ProcDef.doc` and renders it.  The parse / render
//!     tests reach it through `hover()` by planting the docstring as
//!     leading `#` comments above a `proc` and asserting the rendered
//!     hover Markdown.
//!   * GENERATE — the "generate docstring stub" producer lives in
//!     `tcl-lsp-core/src/code_actions.rs` as the PRIVATE
//!     `docstring_actions`, reached through public `code_actions()`.
//!     It emits a DOXYGEN-style block: a
//!     `# @brief TODO: describe <proc>` header, then `# @param <name>`
//!     lines with `- (default: <value>)` for defaulted params and
//!     `- Additional arguments` for the `args` varargs sentinel.  The
//!     PLAIN tag-style and decoration variants of the renderer are public
//!     (`formatting::render_comment_block`) and unit-tested in
//!     `formatting::docstring`; the code action just always picks DOXYGEN.
//!
//! ## Proof split
//!
//! Every behaviour asserted here is a doc-text STRUCTURE fact — what the
//! extractor keeps/drops, which tag the parser recognises, the Markdown
//! framing the renderer emits, the skeleton the generator produces.
//! None of these are Tcl *runtime* values, so they are asserted directly
//! (no tclsh needed).  The single underlying Tcl fact — that
//! `proc name {params} body` binds those params — is what lets the
//! generate/parse fixtures use a real `proc` as the carrier.

use tcl_compiler::analyser::utils::extract_body_docstring;
use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::code_actions::{ActionKind, CodeAction, code_actions, code_actions_in_program};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::formatting::DocstringStyle;
use tcl_lsp_core::hover::hover;

// Harness — mirrors hover_residual.rs / code_actions_depth.rs.

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

/// A single-position (empty) `LspRange` at `(line, character)`.
fn cursor(line: u32, character: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: character,
        end_line: line,
        end_character: character,
    }
}

/// The first action whose title contains `needle`.
fn find<'a>(actions: &'a [CodeAction], needle: &str) -> Option<&'a CodeAction> {
    actions.iter().find(|a| a.title.contains(needle))
}

/// Render the proc-hover Markdown for `proc <name>` declared in `source`.
///
/// The cursor is placed on the proc-name token.  This drives the private
/// `format_docstring` renderer through the public `hover` entry: `hover`
/// harvests the leading `#`-comment block into `ProcDef.doc`, then
/// renders `@brief`/`@param`/`@return` as structured Markdown.  Returns
/// the full hover body so the caller can assert on the rendered sections.
fn proc_hover_markdown(source: &str, name: &str) -> String {
    let analysis = analyse(source);
    // Find the line + column of the proc name token (`proc <name>`).
    for (lineno, line) in source.lines().enumerate() {
        if let Some(pos) = line.find(&format!("proc {name}")) {
            let col = pos + "proc ".len() + 1; // land inside the name
            let h = hover(
                source,
                u32::try_from(lineno).unwrap(),
                u32::try_from(col).unwrap(),
                &analysis,
                None,
            )
            .expect("hover on proc name");
            return h.value;
        }
    }
    panic!("proc {name} not found in source");
}

/// Build a one-proc document whose leading `#`-comment block is `doc`
/// (each line already `#`-prefixed by the caller), declaring
/// `proc carrier {<params>} {}`.  Returns the rendered hover Markdown for
/// the `carrier` proc — i.e. `format_docstring` applied to `doc`.
fn render_doc_via_hover(doc_comment_lines: &str, params: &str) -> String {
    let src = format!("{doc_comment_lines}\nproc carrier {{{params}}} {{}}\n");
    proc_hover_markdown(&src, "carrier")
}

// extract_body_docstring — analyser::utils::extract_body_docstring

#[test]
fn extract_empty_body() {
    assert_eq!(extract_body_docstring(""), "");
}

#[test]
fn extract_no_comments() {
    // a code line at the top yields nothing.
    assert_eq!(extract_body_docstring("puts hello"), "");
}

#[test]
fn extract_simple_comment() {
    assert_eq!(extract_body_docstring("# Hello\nputs hello"), "Hello");
}

#[test]
fn extract_multiline_comment() {
    // consecutive `#` lines join with `\n`.
    let body = "# Line one\n# Line two\nputs hello";
    assert_eq!(extract_body_docstring(body), "Line one\nLine two");
}

#[test]
fn extract_decoration_stripped() {
    // a `# ....` decoration line is dropped,
    // the real text survives.
    let body = "# ....\n# Hello\n# ....\nputs hello";
    assert_eq!(extract_body_docstring(body), "Hello");
}

#[test]
fn extract_hash_decoration_stripped() {
    // pure `########` rules are dropped.
    let body = "########\n# Hello\n########\nputs hello";
    assert_eq!(extract_body_docstring(body), "Hello");
}

#[test]
fn extract_blank_line_stops() {
    // the first blank line ends the block, so the
    // second comment paragraph is not harvested.
    let body = "# First block\n\n# Second block\nputs hello";
    assert_eq!(extract_body_docstring(body), "First block");
}

#[test]
fn extract_leading_blank_lines_skipped() {
    // blank lines BEFORE the first
    // comment are skipped (they don't terminate an empty block).
    let body = "\n\n# Hello\nputs hello";
    assert_eq!(extract_body_docstring(body), "Hello");
}

#[test]
fn extract_osvvm_style() {
    // an OSVVM-style banner: dotted rules top & bottom,
    // a description line, a blank `#`, and a `@param` tag.  The rules are
    // dropped; the description and the `@param` survive (the bare `#`
    // line becomes empty text and is kept as a separator — inner blanks are
    // kept once the block has started).
    let body = concat!(
        "# ....................................................................\n",
        "# Update build date and time in pkg\n",
        "#\n",
        "# @param filename - Filename to pkg file\n",
        "# ....................................................................\n",
        "puts $filename\n",
    );
    let result = extract_body_docstring(body);
    assert!(
        result.contains("Update build date"),
        "expected description kept: {result:?}",
    );
    assert!(
        result.contains("@param filename"),
        "expected @param tag kept: {result:?}",
    );
}

// Docstring parse — format_docstring via public hover()
//
// The parser/renderer is private; we drive it by planting the docstring
// as leading `#` comments above a proc and reading the rendered hover.
// The Markdown framing is the renderer's contract:
//   * `@brief` / plain description text appears verbatim,
//   * `@param name - desc` → `- **name** — desc` under `**Parameters:**`,
//   * `@param name` (no desc) → `- **name**`,
//   * `@return` / `@returns` → `**Returns:** <text>`.

#[test]
fn parse_plain_text_is_description() {
    // a plain comment with no tags renders as the bare
    // description (no Parameters / Returns sections).
    let md = render_doc_via_hover("# Hello world", "");
    assert!(md.contains("Hello world"), "{md}");
    assert!(!md.contains("**Parameters:**"), "no params expected: {md}");
    assert!(!md.contains("**Returns:**"), "no returns expected: {md}");
}

#[test]
fn parse_multiline_description() {
    // two plain lines both land in the
    // description block.
    let md = render_doc_via_hover("# Line one\n# Line two", "");
    assert!(md.contains("Line one"), "{md}");
    assert!(md.contains("Line two"), "{md}");
}

#[test]
fn parse_brief_tag() {
    // `@brief <text>` surfaces the brief text.
    let md = render_doc_via_hover("# @brief Calculate the sum", "a b");
    assert!(md.contains("Calculate the sum"), "{md}");
}

#[test]
fn parse_param_with_dash_separator() {
    // `@param name - desc` parses name + description, dash stripped.
    let md = render_doc_via_hover("# @param filename - Filename to pkg file", "filename");
    assert!(md.contains("**Parameters:**"), "{md}");
    assert!(
        md.contains("**filename**") && md.contains("Filename to pkg file"),
        "expected param name + desc: {md}",
    );
    // The dash separator is consumed, not rendered into the description.
    assert!(
        !md.contains("- Filename to pkg"),
        "dash separator should be stripped: {md}",
    );
}

#[test]
fn parse_param_without_description() {
    // `@param x` with no trailing text
    // renders the bare name (no em-dash, no description).
    let md = render_doc_via_hover("# @param x", "x");
    assert!(md.contains("**Parameters:**"), "{md}");
    assert!(md.contains("**x**"), "{md}");
    // No em-dash separator when there is no description.
    assert!(
        !md.contains("**x** \u{2014}"),
        "bare param must not emit an em-dash: {md}",
    );
}

#[test]
fn parse_return_tag() {
    // `@return <text>` → **Returns:** line.
    let md = render_doc_via_hover("# @return The sum of a and b", "a b");
    assert!(md.contains("**Returns:** The sum of a and b"), "{md}");
}

#[test]
fn parse_returns_tag_alias() {
    // `@returns` is accepted as an alias of `@return`.
    let md = render_doc_via_hover("# @returns The result", "");
    assert!(md.contains("**Returns:** The result"), "{md}");
}

#[test]
fn parse_full_doxygen() {
    // brief + two params + return, all recognised and
    // rendered into their sections.
    let doc = concat!(
        "# @brief Calculate the sum\n",
        "# @param a - First number\n",
        "# @param b - Second number\n",
        "# @return The sum of a and b",
    );
    let md = render_doc_via_hover(doc, "a b");
    assert!(md.contains("Calculate the sum"), "brief: {md}");
    assert!(md.contains("**Parameters:**"), "params header: {md}");
    assert!(
        md.contains("**a**") && md.contains("First number"),
        "param a: {md}"
    );
    assert!(
        md.contains("**b**") && md.contains("Second number"),
        "param b: {md}"
    );
    assert!(
        md.contains("**Returns:** The sum of a and b"),
        "return: {md}"
    );
}

#[test]
fn parse_mixed_description_and_tags() {
    // a free-form description line plus a
    // `@param` tag: both the prose and the parameter survive.
    let doc = concat!(
        "# Update build date and time in pkg\n",
        "#\n",
        "# @param filename - Filename to pkg file",
    );
    let md = render_doc_via_hover(doc, "filename");
    assert!(md.contains("Update build date"), "description: {md}");
    assert!(md.contains("**filename**"), "param: {md}");
}

// Render Markdown / format docstring — format_docstring via hover()

#[test]
fn render_markdown_params_and_returns_sections() {
    // brief + params + return render the `**Parameters:**` header, the
    // em-dash-joined param rows, and the `**Returns:**` line. The param rows
    // are joined with a U+2014 em-dash (`**a** — First`).
    let doc = concat!(
        "# @brief Add numbers\n",
        "# @param a - First\n",
        "# @param b - Second\n",
        "# @return The sum",
    );
    let md = render_doc_via_hover(doc, "a b");
    assert!(md.contains("**Parameters:**"), "{md}");
    assert!(
        md.contains("**a** \u{2014} First"),
        "em-dash param row: {md}"
    );
    assert!(md.contains("**Returns:** The sum"), "{md}");
}

#[test]
fn format_docstring_roundtrip() {
    // brief + param + return all appear in the formatted Markdown.
    let doc = "# @brief Hello\n# @param x - A number\n# @return The result";
    let md = render_doc_via_hover(doc, "x");
    assert!(md.contains("Hello"), "{md}");
    assert!(md.contains("**x**"), "{md}");
    assert!(md.contains("**Returns:**"), "{md}");
}

// Parse edge cases — format_docstring via hover()

#[test]
fn parse_multi_return_accumulated() {
    // two `@return` tags accumulate into a
    // single space-joined Returns line ("First part Second part").
    let doc = "# @return First part\n# @return Second part";
    let md = render_doc_via_hover(doc, "");
    assert!(
        md.contains("**Returns:** First part Second part"),
        "multi-return should space-join: {md}",
    );
}

#[test]
fn parse_decoration_lines_stripped_in_parse() {
    // pure-decoration lines
    // (`......`, `------`) are dropped from the description, leaving only
    // the real prose.
    let doc = "# ......\n# Hello world\n# ------";
    let md = render_doc_via_hover(doc, "");
    assert!(md.contains("Hello world"), "{md}");
    assert!(!md.contains("......"), "dotted decoration dropped: {md}");
    assert!(!md.contains("------"), "dash decoration dropped: {md}");
}

// GAP: the bare-tag guards (`@param` with no text → no param, `@brief` with
// no text → empty brief) exercise the standalone parser.  Through the hover
// carrier these degenerate tags can't be planted in isolation without the
// renderer treating the surrounding `proc`/comment shape as the description;
// the bare-tag-no-crash guard is therefore covered by hover.rs's own in-crate
// unit test `format_docstring_handles_param_without_description`, not
// re-asserted here.

// Generate docstring stub — docstring_actions via public code_actions()
//
// The generator emits a `# @param <name>` skeleton (one line per
// parameter) under a leading `# ` comment, as a `Source` code action
// titled "Generate docstring for '<proc>'".  It fires only when the
// cursor sits on the proc's declaration LINE and the proc has no existing
// doc-comment.

/// Run `code_actions` with the cursor on the declaration line of `proc`
/// and return the "Generate docstring" action's inserted text, if any.
fn generate_stub_text(source: &str, proc_name: &str) -> Option<String> {
    let analysis = analyse(source);
    // Locate the proc declaration line.
    let line = source
        .lines()
        .position(|l| l.contains(&format!("proc {proc_name}")))?;
    let actions = code_actions(
        source,
        cursor(u32::try_from(line).unwrap(), 0),
        Some(&analysis),
        &analysis.diagnostics,
    );
    find(&actions, &format!("Generate docstring for '{proc_name}'"))
        .map(|a| a.edits[0].new_text.clone())
}

#[test]
fn generate_stub_basic_emits_param_line() {
    // a one-param proc generates a `# @brief TODO: describe greet` header
    // followed by a `# @param name` line (DOXYGEN output).
    let src = "proc greet {name} { puts $name }\n";
    let stub = generate_stub_text(src, "greet").expect("a generate-docstring action");
    assert!(
        stub.contains("# @brief TODO: describe greet"),
        "expected @brief header: {stub:?}",
    );
    assert!(
        stub.contains("# @param name"),
        "expected param stub line: {stub:?}",
    );
    // The brief header is emitted FIRST, before the param lines.
    let ib = stub.find("@brief").unwrap();
    let ip = stub.find("@param name").unwrap();
    assert!(ib < ip, "brief precedes params: {stub:?}");
    // It is inserted as a leading comment block (starts with `# `).
    assert!(stub.starts_with("# "), "stub is a comment block: {stub:?}");
    // A plain param (no default) carries no `-` description / annotation.
    assert!(
        !stub.contains("# @param name -"),
        "plain param must not carry a dash-description: {stub:?}",
    );
}

#[test]
fn generate_stub_action_is_source_kind() {
    // The generate-docstring action is a `source` action (it is an
    // editor-level producer, not a quick-fix).
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    let act = find(&actions, "Generate docstring for 'greet'").expect("generate action");
    assert_eq!(act.kind, ActionKind::Source, "{act:?}");
}

#[test]
fn generate_stub_one_param_line_per_param() {
    // a multi-param proc emits one `# @param <name>` line per parameter,
    // in declaration order.
    let src = "proc add {a b c} { expr {$a + $b + $c} }\n";
    let stub = generate_stub_text(src, "add").expect("a generate action");
    assert!(stub.contains("# @param a"), "param a: {stub:?}");
    assert!(stub.contains("# @param b"), "param b: {stub:?}");
    assert!(stub.contains("# @param c"), "param c: {stub:?}");
    // a before b before c.
    let ia = stub.find("@param a").unwrap();
    let ib = stub.find("@param b").unwrap();
    let ic = stub.find("@param c").unwrap();
    assert!(ia < ib && ib < ic, "params in declaration order: {stub:?}");
}

#[test]
fn generate_stub_param_named_args_is_included() {
    // the `args` varargs sentinel gets an "Additional arguments"
    // description: `# @param args - Additional arguments`.
    let src = "proc variadic {args} { puts $args }\n";
    let stub = generate_stub_text(src, "variadic").expect("a generate action");
    assert!(
        stub.contains("# @param args - Additional arguments"),
        "args param carries the prose description: {stub:?}",
    );
}

#[test]
fn generate_stub_defaulted_param_carries_default_annotation() {
    // a parameter with a default value gets a `- (default: <value>)`
    // annotation on its `@param` line.
    let src = "proc greet {name {greeting Hello}} { puts \"$greeting $name\" }\n";
    let stub = generate_stub_text(src, "greet").expect("a generate action");
    // The defaulted param shows its default value.
    assert!(
        stub.contains("# @param greeting - (default: Hello)"),
        "defaulted param carries `(default: …)`: {stub:?}",
    );
    // The non-defaulted param stays bare (no annotation).
    assert!(
        stub.contains("# @param name") && !stub.contains("# @param name -"),
        "non-defaulted param stays bare: {stub:?}",
    );
}

#[test]
fn generate_stub_emits_no_return_line() {
    // The stub never sets a return description, so it emits no
    // `# @return` line.
    let src = "proc add {a b} { expr {$a + $b} }\n";
    let stub = generate_stub_text(src, "add").expect("a generate action");
    assert!(
        !stub.contains("@return"),
        "stub must not emit a @return line: {stub:?}",
    );
}

#[test]
fn generate_stub_skipped_for_documented_proc() {
    // a proc that already carries a leading doc-comment is NOT offered a
    // generate-docstring action (its `ProcDef.doc` is non-empty, so the
    // generator skips it).
    let src = "# Existing doc\nproc foo {} { puts hi }\n";
    let analysis = analyse(src);
    // Cursor on the proc declaration line (line 1).
    let actions = code_actions(src, cursor(1, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "Generate docstring for 'foo'").is_none(),
        "documented proc must not be offered a stub; got titles {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn generate_stub_offered_for_undocumented_proc() {
    // The companion positive case to the skip: an undocumented proc IS
    // offered the generate action.  Together these reproduce the
    // documented/undocumented partition of `insert_docstring_stubs`.
    let src = "proc bar {} { puts bye }\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "Generate docstring for 'bar'").is_some(),
        "undocumented proc should be offered a stub; got titles {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn generate_stub_multiple_procs_each_offered_on_their_line() {
    // two undocumented procs each get their own generate action when the
    // cursor is on the matching declaration line.  (The generator is
    // per-cursor-line, so each proc is offered a stub on its own line.)
    let src = "proc foo {} { puts hi }\nproc bar {} { puts bye }\n";
    let analysis = analyse(src);
    let foo = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&foo, "Generate docstring for 'foo'").is_some(),
        "foo offered on its line: {:?}",
        foo.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
    let bar = code_actions(src, cursor(1, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&bar, "Generate docstring for 'bar'").is_some(),
        "bar offered on its line: {:?}",
        bar.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn generate_stub_absent_when_cursor_off_declaration_line() {
    // The generator gates on the cursor being on the proc DECLARATION line
    // (`decl.line == range.start_line`).  A cursor on the body line of a
    // single-line proc still matches; a cursor on an unrelated later line
    // does not.  This pins the line-gate (an LSP-structural fact).
    let src = "set marker 1\nproc qux {z} { puts $z }\n";
    let analysis = analyse(src);
    // Cursor on line 0 (the `set marker` line), NOT the proc line.
    let actions = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    assert!(
        find(&actions, "Generate docstring for 'qux'").is_none(),
        "off-declaration-line cursor must not offer the stub; got {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

// `DocstringStyle` placement (#1314) — code_actions_in_program's
// docstring_style parameter, the resolved `tclLsp.formatting.docstringStyle`
// setting's actual consumer.

#[test]
fn docstring_style_none_offers_no_action() {
    // FN-shape: `None` — "do not generate or reformat docstrings", the
    // documented default — suppresses the source action entirely, even for
    // an undocumented proc with the cursor on its declaration line.
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let actions = code_actions_in_program(
        src,
        cursor(0, 0),
        Some(&analysis),
        &analysis.diagnostics,
        None,
        DocstringStyle::None,
    );
    assert!(
        find(&actions, "Generate docstring for 'greet'").is_none(),
        "DocstringStyle::None must suppress the generate action; got {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn docstring_style_preceding_matches_plain_code_actions_default() {
    // TP-shape: passing `Preceding` explicitly through
    // `code_actions_in_program` reproduces exactly what the plain
    // `code_actions()` entry point has always done (its only placement).
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let explicit = code_actions_in_program(
        src,
        cursor(0, 0),
        Some(&analysis),
        &analysis.diagnostics,
        None,
        DocstringStyle::Preceding,
    );
    let default = code_actions(src, cursor(0, 0), Some(&analysis), &analysis.diagnostics);
    let explicit_edit = &find(&explicit, "Generate docstring for 'greet'")
        .expect("explicit Preceding action")
        .edits[0];
    let default_edit = &find(&default, "Generate docstring for 'greet'")
        .expect("default action")
        .edits[0];
    assert_eq!(explicit_edit, default_edit);
    // Preceding lands on the declaration line itself, column 0.
    assert_eq!(explicit_edit.range.start_line, 0);
    assert_eq!(explicit_edit.range.start_character, 0);
}

#[test]
fn docstring_style_body_inserts_after_the_opening_brace_not_on_decl_line() {
    // TP-shape: `Body` placement inserts inside the proc body — not on the
    // declaration line the way `Preceding` does.
    let src = "proc greet {name} {\n    puts $name\n}\n";
    let analysis = analyse(src);
    let actions = code_actions_in_program(
        src,
        cursor(0, 0),
        Some(&analysis),
        &analysis.diagnostics,
        None,
        DocstringStyle::Body,
    );
    let edit = &find(&actions, "Generate docstring for 'greet'")
        .expect("a Body-placement generate action")
        .edits[0];
    // The opening brace is on line 0; the insertion point is just after it,
    // not at column 0 of line 0 (which is what Preceding would use).
    assert_eq!(edit.range.start_line, 0);
    assert!(
        edit.range.start_character > 0,
        "Body placement must not land at column 0 of the decl line: {edit:?}",
    );
    assert!(
        edit.new_text.contains("@brief TODO: describe greet"),
        "stub content unaffected by placement: {:?}",
        edit.new_text,
    );
}

#[test]
fn docstring_style_body_matches_the_bodys_existing_indent() {
    // TP-shape: the inserted stub's comment lines pick up the same
    // indentation as the body's existing (2-space) content, not the
    // formatter's 4-space default.
    let src = "proc greet {name} {\n  puts $name\n}\n";
    let analysis = analyse(src);
    let actions = code_actions_in_program(
        src,
        cursor(0, 0),
        Some(&analysis),
        &analysis.diagnostics,
        None,
        DocstringStyle::Body,
    );
    let edit = &find(&actions, "Generate docstring for 'greet'")
        .expect("a Body-placement generate action")
        .edits[0];
    assert!(
        edit.new_text.contains("\n  # @brief"),
        "stub should be indented to match the body's 2-space content: {:?}",
        edit.new_text,
    );
}

#[test]
fn docstring_style_body_falls_back_to_four_spaces_for_a_single_line_proc() {
    // FN-shape: a single-line body has no other line to read an indent
    // from, so the fallback (four spaces, the formatter's default indent
    // size) applies, and the edit still splices in cleanly around the
    // existing body text.
    let src = "proc greet {name} { puts $name }\n";
    let analysis = analyse(src);
    let actions = code_actions_in_program(
        src,
        cursor(0, 0),
        Some(&analysis),
        &analysis.diagnostics,
        None,
        DocstringStyle::Body,
    );
    let edit = &find(&actions, "Generate docstring for 'greet'")
        .expect("a Body-placement generate action")
        .edits[0];
    assert!(
        edit.new_text.contains("\n    # @brief"),
        "fallback four-space indent expected: {:?}",
        edit.new_text,
    );
}

// Covered elsewhere — the structured `DocstringInfo` surface.
//
// `formatting::docstring` exposes the public `DocstringInfo` value type and
// its helpers directly, so these behaviours are unit-tested there rather than
// through this file's LSP-surface fixtures:
//
//   * `render_comment_block` re-emits a `DocstringInfo` into a `#`-comment
//     block in Doxygen (`# @brief …` / `# @param name - …` / `# @return …`)
//     or PLAIN (`# Arguments:` / `#   name - desc` / `# Returns: …`) style,
//     with optional decoration borders and indent.  (The "generate docstring"
//     code action always picks the DOXYGEN skeleton, which is what the
//     fixtures above assert; the styled variants are exercised as unit tests.)
//   * `DocstringInfo::to_json` serialises to the AI-tool dict shape, omitting
//     empty fields.
//   * `resolve_tag_style` maps a case-insensitive style string to the enum.
//
// GAP catalogue — docstring behaviours with NO `tcl-lsp-core` equivalent
// (no public surface to assert against).  Recorded here, not tested.
//
// GAP: a bare/qualified proc lookup (`AnalysisResult.find_proc`) —
//   `AnalysisResult` exposes the `all_procs` map publicly but provides no
//   `find_proc(name)` convenience that resolves a bare name to a
//   namespace-qualified proc; callers index `all_procs` by the fully
//   qualified `::name` key directly.  No public lookup method.
//
// GAP: whole-document doc harvest (`collect_proc_docs`) — gathering EVERY
//   proc's `{name, doc_raw, doc, params}` record across a document in one
//   call has no counterpart; the hover/code-action surface is
//   per-cursor-position, not a whole-document doc harvest.  (The
//   underlying per-proc data — `ProcDef.doc`, `ProcDef.params` — is
//   public, but the aggregating helper is not.)
//
// GAP: batch stub insertion (`insert_docstring_stubs`) — the BATCH
//   "insert a stub for every undocumented proc and return (modified,
//   count)" operation has no single entry point.  Its per-proc
//   decisions (skip documented, stub undocumented) are reproduced above
//   via repeated `code_actions` calls, but the whole-document rewrite +
//   count is not exposed.
