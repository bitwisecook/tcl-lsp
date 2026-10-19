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

//! Generate the configuration the editors without a manifest of their own
//! copy out of a README: Zed's `extension.toml` language table, Helix
//! `[[language]]` blocks, Emacs derived modes and eglot rows, Neovim file
//! types, and the dialect lists and defaults every editor guide states.
//!
//! Every list is read from the compiled environment registry
//! ([`tcl_dialect::model::EnvironmentRegistry::compiled_selectable`]) and the
//! editor language model [`crate::editor_extensions`] builds from it, so a
//! new environment reaches each editor without a hand edit. Generated text
//! sits between `<generated: name>` and `</generated>` marker lines written
//! in the file's own comment syntax; the prose around a region is kept.
//! A one-line list or default the guides show inline (a `filetypes = { … }`
//! table, a `dialect = '…'` value) is rewritten in place.
//!
//! Run `cargo xtask gen-editor-configs`; `--check` makes every region a drift
//! gate.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use tcl_dialect::model::{DEFAULT_ENVIRONMENT_ID, EnvironmentDefinition, EnvironmentRegistry};

use crate::editor_extensions::{
    Language, ZED_LANGUAGES, ZedLanguage, all_extensions, language_of_environment, languages,
    shebang_stem, zed_config_path,
};
use crate::util::source_literals::{elisp_string, lua_string, toml_array, toml_string};
use crate::util::{replace_generated_region, repo_root, write_if_changed};

const ZED_EXTENSION: &str = "editors/zed/extension.toml";
const ZED_README: &str = "editors/zed/README.md";
const HELIX_README: &str = "editors/helix/README.md";
const EMACS_README: &str = "editors/emacs/README.md";
const NEOVIM_LUA: &str = "editors/neovim/tcl_lsp.lua";
const NEOVIM_README: &str = "editors/neovim/README.md";
const SUBLIME_README: &str = "editors/sublime-text/README.md";
const INSTALL_EDITORS: &str = "INSTALL-editors.md";

/// The mode names that are not the environment id with its dots dropped
/// (`tcl8.6` is `tcl86-mode`). Jim's derives from `tcl-mode` and says so, which
/// leaves `jim-mode` for a mode written for Jim alone.
const EMACS_MODE_NAMES: &[(&str, &str)] = &[("jim", "jim-tcl-mode")];

/// Reserved words a Lua table key cannot be written bare as.
const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// The width comment lists and extension tables wrap at.
const WRAP_WIDTH: usize = 78;

fn contains_source_line_separator(value: &str) -> bool {
    value
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}'))
}

/// What every render reads: the editor language model and the selectable
/// environments.
pub struct Model {
    pub langs: Vec<Language>,
    pub environments: &'static [Arc<EnvironmentDefinition>],
}

/// What an editor can key an environment's files on.
pub struct Registration {
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub shebang_words: Vec<String>,
}

impl Model {
    pub fn load() -> Result<Self> {
        Ok(Self {
            langs: languages()?,
            environments: EnvironmentRegistry::compiled_selectable(),
        })
    }

    /// The file extensions and shebang words of `environment`: its own
    /// language's, plus the extra languages that select it (the iApp
    /// presentation language rides the iApps environment).
    pub fn registration(&self, environment: &EnvironmentDefinition) -> Registration {
        let id = environment.id.as_str();
        let own = language_of_environment(&self.langs, id);
        let mut extensions = own.map_or_else(
            || {
                environment
                    .server_detection
                    .file_extensions
                    .iter()
                    .map(|claim| claim.extension.to_string())
                    .collect()
            },
            |lang| lang.extensions.clone(),
        );
        let mut filenames = own.map_or_else(
            || {
                environment
                    .server_detection
                    .filenames
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            },
            |lang| lang.filenames.clone(),
        );
        for extra in self
            .langs
            .iter()
            .filter(|l| l.is_extra && l.dialect.as_deref() == Some(id))
        {
            extensions.extend(extra.extensions.iter().cloned());
            filenames.extend(extra.filenames.iter().cloned());
        }
        let shebang_words = environment
            .server_detection
            .shebang_words
            .iter()
            .map(ToString::to_string)
            .collect();
        Registration {
            extensions,
            filenames,
            shebang_words,
        }
    }

    fn environment(&self, id: &str) -> Result<&Arc<EnvironmentDefinition>> {
        self.environments
            .iter()
            .find(|environment| environment.id.as_str() == id)
            .ok_or_else(|| anyhow!("{id} is not a selectable environment"))
    }

    fn default_environment(&self) -> Result<&Arc<EnvironmentDefinition>> {
        self.environment(DEFAULT_ENVIRONMENT_ID)
    }

    /// Every extension no derived mode of `modes` owns, in language order.
    fn extensions_outside(&self, modes: &[&str]) -> Vec<String> {
        let owned: BTreeSet<String> = self
            .environments
            .iter()
            .filter(|environment| modes.contains(&environment.id.as_str()))
            .flat_map(|environment| self.registration(environment).extensions)
            .collect();
        all_extensions(&self.langs)
            .into_iter()
            .filter(|ext| !owned.contains(ext))
            .collect()
    }

    /// Every whole file name no derived mode of `modes` owns.
    fn filenames_outside(&self, modes: &[&str]) -> Vec<String> {
        let owned: BTreeSet<String> = self
            .environments
            .iter()
            .filter(|environment| modes.contains(&environment.id.as_str()))
            .flat_map(|environment| self.registration(environment).filenames)
            .collect();
        let mut names: Vec<String> = Vec::new();
        for lang in self.langs.iter().filter(|lang| !lang.is_extra) {
            for name in &lang.filenames {
                if !owned.contains(name) && !names.contains(name) {
                    names.push(name.clone());
                }
            }
        }
        names
    }

    /// The interpreter names any environment (or the plain `tcl` language)
    /// selects by, without a version suffix: `tclsh8.6` is covered by
    /// `tclsh`, since editors match the word as a prefix.
    fn shebang_stems(&self) -> Vec<String> {
        let mut stems: BTreeSet<String> = self
            .environments
            .iter()
            .flat_map(|environment| environment.server_detection.shebang_words.iter())
            .map(|word| shebang_stem(word).to_owned())
            .collect();
        stems.extend(
            self.langs
                .iter()
                .flat_map(|lang| lang.shebang_words.iter())
                .map(|word| shebang_stem(word).to_owned()),
        );
        stems.into_iter().collect()
    }
}

/// `words` laid out greedily, one space apart, on lines that stay within
/// [`WRAP_WIDTH`] counting the prefix; the first line starts with
/// `first_prefix`.
pub fn wrap_words(first_prefix: &str, prefix: &str, words: &[String]) -> String {
    let mut out = String::new();
    let mut line = first_prefix.to_owned();
    let mut fresh = true;
    for word in words {
        if !fresh && line.len() + 1 + word.len() > WRAP_WIDTH {
            out.push_str(line.trim_end());
            out.push('\n');
            prefix.clone_into(&mut line);
            fresh = true;
        }
        if !fresh {
            line.push(' ');
        }
        line.push_str(word);
        fresh = false;
    }
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

/// Prose wrapped to [`WRAP_WIDTH`]; a word never splits, so an inline code
/// span without a space in it stays whole.
pub fn wrap_text(text: &str) -> String {
    let words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    wrap_words("", "", &words)
}

/// `items` with a comma after each, and after the last too when `trailing`.
fn comma_separated(items: &[String], trailing: bool) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            if trailing || index + 1 < items.len() {
                format!("{item},")
            } else {
                item.clone()
            }
        })
        .collect()
}

/// `` `a`, `b`, and `c` ``: names as inline code in a sentence, with the
/// Oxford comma.
pub fn code_sentence_list<S: AsRef<str>>(names: &[S]) -> String {
    let coded: Vec<String> = names
        .iter()
        .map(|name| format!("`{}`", name.as_ref()))
        .collect();
    match coded.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
    }
}

fn dialect_ids(model: &Model) -> Vec<String> {
    model
        .environments
        .iter()
        .map(|environment| environment.id.to_string())
        .collect()
}

/// The `Valid dialects:` comment a settings snippet carries, in `comment`
/// syntax.
fn valid_dialects_comment(model: &Model, comment: &str, indent: &str) -> String {
    let prefix = format!("{indent}{comment} ");
    let mut words = vec!["Valid".to_owned(), "dialects:".to_owned()];
    words.extend(comma_separated(&dialect_ids(model), false));
    wrap_words(&prefix, &prefix, &words)
}

fn default_dialect_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"((?::dialect|\bdialect)["']?\s*[:=]?\s*["'])tcl\d\.\d(["'])"#)
            .expect("the default-dialect pattern is a valid regex")
    })
}

/// Rewrite every `dialect = 'tclN.N'` (or `"dialect": "tclN.N"`,
/// `:dialect "tclN.N"`) value to the default environment.
fn with_default_dialect(text: &str) -> String {
    default_dialect_regex()
        .replace_all(text, |captures: &regex::Captures| {
            format!("{}{DEFAULT_ENVIRONMENT_ID}{}", &captures[1], &captures[2])
        })
        .into_owned()
}

#[derive(Clone, Copy)]
enum LuaFiletypesOwner {
    ReturnedRootTable,
    LspconfigDefaultConfig,
}

#[derive(Clone, Copy)]
struct LuaToken<'a> {
    text: &'a str,
    start: usize,
}

fn lua_long_bracket_open(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    if bytes.get(start) != Some(&b'[') {
        return None;
    }
    let mut cursor = start + 1;
    while bytes.get(cursor) == Some(&b'=') {
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'[')).then_some((cursor - start - 1, cursor + 1))
}

fn skip_lua_long_bracket(bytes: &[u8], body: usize, equals: usize) -> Result<usize> {
    let close = format!("]{}]", "=".repeat(equals));
    let relative = bytes[body..]
        .windows(close.len())
        .position(|window| window == close.as_bytes())
        .ok_or_else(|| anyhow!("unterminated Lua long string or comment"))?;
    Ok(body + relative + close.len())
}

/// Tokenize the lexical surface needed to identify Lua table fields. Strings
/// and comments are skipped, so marker-looking text in them is never a field.
fn lua_tokens(text: &str) -> Result<Vec<LuaToken<'_>>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'-' && bytes.get(cursor + 1) == Some(&b'-') {
            cursor += 2;
            if let Some((equals, body)) = lua_long_bracket_open(bytes, cursor) {
                cursor = skip_lua_long_bracket(bytes, body, equals)?;
            } else {
                while cursor < bytes.len() && bytes[cursor] != b'\n' {
                    cursor += 1;
                }
            }
            continue;
        }
        if let Some((equals, body)) = lua_long_bracket_open(bytes, cursor) {
            let start = cursor;
            cursor = skip_lua_long_bracket(bytes, body, equals)?;
            tokens.push(LuaToken {
                text: "<string>",
                start,
            });
            continue;
        }
        if matches!(bytes[cursor], b'\'' | b'"') {
            let start = cursor;
            let quote = bytes[cursor];
            cursor += 1;
            let mut closed = false;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'\\' => cursor = (cursor + 2).min(bytes.len()),
                    byte if byte == quote => {
                        cursor += 1;
                        closed = true;
                        break;
                    }
                    b'\n' | b'\r' => bail!("newline in quoted Lua string"),
                    _ => cursor += 1,
                }
            }
            if !closed {
                bail!("unterminated quoted Lua string");
            }
            tokens.push(LuaToken {
                text: "<string>",
                start,
            });
            continue;
        }
        let start = cursor;
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
        } else if bytes[cursor].is_ascii() {
            cursor += 1;
        } else {
            let width = text[cursor..].chars().next().map_or(1, char::len_utf8);
            cursor += width;
        }
        tokens.push(LuaToken {
            text: &text[start..cursor],
            start,
        });
    }
    Ok(tokens)
}

fn lua_matching_brace(tokens: &[LuaToken<'_>], open: usize) -> Result<usize> {
    anyhow::ensure!(
        tokens.get(open).is_some_and(|token| token.text == "{"),
        "expected Lua table opener"
    );
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.text {
            "{" => depth += 1,
            "}" => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("unbalanced Lua table close"))?;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    bail!("unterminated Lua table")
}

fn lua_direct_table_fields(
    tokens: &[LuaToken<'_>],
    parent: usize,
    name: &str,
) -> Result<Vec<usize>> {
    let end = lua_matching_brace(tokens, parent)?;
    let mut fields = Vec::new();
    let mut nested = 0usize;
    for index in parent + 1..end {
        let token = tokens[index].text;
        if nested == 0
            && matches!(tokens[index - 1].text, "{" | "," | ";")
            && token == name
            && tokens.get(index + 1).is_some_and(|token| token.text == "=")
            && tokens.get(index + 2).is_some_and(|token| token.text == "{")
        {
            fields.push(index);
        }
        match token {
            "{" => nested += 1,
            "}" => nested -= 1,
            _ => {}
        }
    }
    Ok(fields)
}

/// This generator accepts a bounded authored Lua table chunk, not arbitrary
/// executing Lua. Block/function expressions require a real Lua parser and are
/// refused instead of donating fields across a different lexical scope.
fn lua_table_chunk_delimiters(tokens: &[LuaToken<'_>]) -> Result<()> {
    let mut stack = Vec::new();
    for token in tokens {
        match token.text {
            "function" | "local" | "if" | "then" | "else" | "elseif" | "end" | "for" | "while"
            | "repeat" | "until" | "do" | "return" | "goto" => {
                bail!("unsupported Lua block in authored table chunk")
            }
            "{" => stack.push("}"),
            "(" => stack.push(")"),
            "[" => bail!("computed/quoted Lua keys and indexing require an unsupported key census"),
            "}" | ")" | "]" => anyhow::ensure!(
                stack.pop() == Some(token.text),
                "unbalanced Lua table chunk delimiters"
            ),
            _ => {}
        }
    }
    anyhow::ensure!(stack.is_empty(), "unterminated Lua table chunk delimiter");
    Ok(())
}

fn lua_tokens_equal(tokens: &[LuaToken<'_>], expected: &[&str]) -> bool {
    tokens.len() == expected.len()
        && tokens
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.text == *expected)
}

/// Admit the exact supported chunk ancestry before selecting any table field.
fn lua_filetypes_owner_open(
    text: &str,
    tokens: &[LuaToken<'_>],
    owner: LuaFiletypesOwner,
) -> Result<usize> {
    match owner {
        LuaFiletypesOwner::ReturnedRootTable => {
            anyhow::ensure!(
                tokens.first().is_some_and(|token| token.text == "return"),
                "expected a root return statement, not a nested function return"
            );
            lua_table_chunk_delimiters(&tokens[1..])?;
            let end = lua_matching_brace(tokens, 1)?;
            anyhow::ensure!(
                end + 1 == tokens.len(),
                "unsupported statements after root return table"
            );
            Ok(1)
        }
        LuaFiletypesOwner::LspconfigDefaultConfig => {
            let prefix = [
                "local",
                "lspconfig",
                "=",
                "require",
                "(",
                "<string>",
                ")",
                "local",
                "configs",
                "=",
                "require",
                "(",
                "<string>",
                ")",
                "if",
                "not",
                "configs",
                ".",
                "tcl_lsp",
                "then",
                "configs",
                ".",
                "tcl_lsp",
                "=",
                "{",
            ];
            anyhow::ensure!(
                tokens.len() >= prefix.len() && lua_tokens_equal(&tokens[..prefix.len()], &prefix),
                "unsupported lspconfig binding/guard/assignment ancestry"
            );
            for (index, module) in [(5, "lspconfig"), (12, "lspconfig.configs")] {
                let start = tokens[index].start;
                anyhow::ensure!(
                    text[start..].starts_with(&format!("'{module}'"))
                        || text[start..].starts_with(&format!("\"{module}\"")),
                    "lspconfig owner is not bound to the supported required module"
                );
            }
            let open = prefix.len() - 1;
            let end = lua_matching_brace(tokens, open)?;
            lua_table_chunk_delimiters(&tokens[open..=end])?;
            let setup = ["end", "lspconfig", ".", "tcl_lsp", ".", "setup", "(", "{"];
            anyhow::ensure!(
                tokens.len() >= end + 1 + setup.len()
                    && lua_tokens_equal(&tokens[end + 1..end + 1 + setup.len()], &setup),
                "unsupported lspconfig root continuation"
            );
            let setup_open = end + setup.len();
            let setup_end = lua_matching_brace(tokens, setup_open)?;
            lua_table_chunk_delimiters(&tokens[setup_open..=setup_end])?;
            anyhow::ensure!(
                lua_tokens_equal(&tokens[setup_end + 1..], &[")"]),
                "unsupported statements after lspconfig setup"
            );
            let defaults = lua_direct_table_fields(tokens, open, "default_config")?;
            anyhow::ensure!(defaults.len() == 1, "expected one default_config table");
            Ok(defaults[0] + 2)
        }
    }
}

/// Select the one exact Lua fence belonging to the supported Markdown section.
/// Markdown prose is never lexed as Lua; offsets are retained in the original
/// document and only that source region is replaced.
fn neovim_lspconfig_fence(text: &str) -> Result<std::ops::Range<usize>> {
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
    let mut ancestry = Vec::new();
    let mut heading_text = None::<String>;
    let mut in_section = false;
    let mut section_count = 0;
    let mut selected_code = false;
    let mut selected_range = None::<std::ops::Range<usize>>;
    let mut result = None;
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if ancestry.is_empty() {
                    match &tag {
                        Tag::Heading {
                            level: HeadingLevel::H1 | HeadingLevel::H2,
                            ..
                        } => {
                            in_section = false;
                            if matches!(
                                &tag,
                                Tag::Heading {
                                    level: HeadingLevel::H2,
                                    ..
                                }
                            ) {
                                heading_text = Some(String::new());
                            }
                        }
                        Tag::CodeBlock(kind) if in_section => {
                            anyhow::ensure!(
                                matches!(kind, CodeBlockKind::Fenced(info) if info.as_ref() == "lua"),
                                "unsupported lspconfig code block kind"
                            );
                            anyhow::ensure!(result.is_none(), "multiple lspconfig Lua fences");
                            selected_code = true;
                            selected_range = None;
                        }
                        _ => {}
                    }
                }
                ancestry.push(tag.to_end());
            }
            Event::End(end) => {
                anyhow::ensure!(
                    ancestry.pop() == Some(end),
                    "inconsistent Markdown parser ancestry"
                );
                if ancestry.is_empty() {
                    if end == TagEnd::Heading(HeadingLevel::H2) {
                        if heading_text.take().as_deref() == Some("nvim-lspconfig (Neovim 0.8+)") {
                            section_count += 1;
                            anyhow::ensure!(
                                section_count == 1,
                                "duplicate lspconfig Markdown section"
                            );
                            in_section = true;
                        }
                    } else if end == TagEnd::CodeBlock && selected_code {
                        result = Some(
                            selected_range
                                .take()
                                .ok_or_else(|| anyhow!("empty lspconfig Lua fence"))?,
                        );
                        selected_code = false;
                    }
                }
            }
            Event::Text(value) => {
                if let Some(heading) = &mut heading_text {
                    heading.push_str(&value);
                }
                if selected_code {
                    anyhow::ensure!(
                        text.get(range.clone()) == Some(value.as_ref()),
                        "Lua fence content requires a transformed Markdown source mapping"
                    );
                    if let Some(previous) = &mut selected_range {
                        anyhow::ensure!(
                            previous.end == range.start,
                            "discontinuous Lua fence source mapping"
                        );
                        previous.end = range.end;
                    } else {
                        selected_range = Some(range);
                    }
                }
            }
            _ => {}
        }
    }
    result.ok_or_else(|| anyhow!("missing lspconfig Lua fence"))
}

/// Select a real field from one of the supported table owners, then verify its
/// canonical source row and neighboring fields before replacing that row.
fn set_lua_list(
    text: &str,
    owner: LuaFiletypesOwner,
    indent: &str,
    before: &str,
    after: &str,
    items: &[String],
) -> Result<String> {
    let tokens = lua_tokens(text)?;
    let owner_open = lua_filetypes_owner_open(text, &tokens, owner)?;
    let fields = lua_direct_table_fields(&tokens, owner_open, "filetypes")?;
    anyhow::ensure!(
        fields.len() == 1,
        "expected one filetypes field in the selected Lua table"
    );
    let start = tokens[fields[0]].start;
    let line_index = text[..start].bytes().filter(|byte| *byte == b'\n').count();
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let line = *lines
        .get(line_index)
        .ok_or_else(|| anyhow!("filetypes field has no source line"))?;
    let body = line.strip_suffix('\n').unwrap_or(line);
    let body = body.strip_suffix('\r').unwrap_or(body);
    anyhow::ensure!(
        body.starts_with(indent)
            && body[indent.len()..].starts_with("filetypes = { ")
            && body.ends_with(" },"),
        "filetypes field is outside the supported canonical source row"
    );
    let previous = lines[..line_index]
        .iter()
        .rev()
        .map(|line| line.trim_end_matches(['\r', '\n']))
        .find(|line| !line.trim().is_empty());
    let next = lines[line_index + 1..]
        .iter()
        .map(|line| line.trim_end_matches(['\r', '\n']))
        .find(|line| !line.trim().is_empty());
    anyhow::ensure!(
        previous.is_some_and(|line| line.trim() == before)
            && next.is_some_and(|line| line.trim() == after),
        "filetypes field does not have the supported neighboring owner fields"
    );
    let quoted: Vec<String> = items
        .iter()
        .map(|item| lua_string(item))
        .collect::<Result<_>>()?;
    let replacement = format!("{indent}filetypes = {{ {} }},", quoted.join(", "));
    let mut output = String::with_capacity(text.len() + replacement.len());
    for (index, original) in lines.iter().enumerate() {
        if index == line_index {
            output.push_str(&replacement);
            if original.ends_with("\r\n") {
                output.push_str("\r\n");
            } else if original.ends_with('\n') {
                output.push('\n');
            }
        } else {
            output.push_str(original);
        }
    }
    Ok(output)
}

/// A Lua table key, bare when it is an identifier that is not a reserved
/// word.
fn lua_key(key: &str) -> Result<String> {
    let identifier = key
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if identifier && !LUA_KEYWORDS.contains(&key) {
        Ok(key.to_owned())
    } else {
        Ok(format!("[{}]", lua_string(key)?))
    }
}

// Zed

/// The `name` a Zed language directory's `config.toml` declares.
fn zed_language_name(zed: &ZedLanguage) -> Result<String> {
    let path = repo_root().join(zed_config_path(zed));
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    text.lines()
        .find_map(|line| line.strip_prefix("name = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{} declares no name", path.display()))
}

fn render_zed_extension(original: &str, _model: &Model) -> Result<String> {
    let names: Vec<String> = ZED_LANGUAGES
        .iter()
        .map(zed_language_name)
        .collect::<Result<_>>()?;
    let quoted: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
    let text = replace_generated_region(
        original,
        "zed-languages",
        &format!("languages = [{}]\n", quoted.join(", ")),
    )?;
    let mut ids = String::new();
    for (zed, name) in ZED_LANGUAGES.iter().zip(&names) {
        let _ = writeln!(ids, "\"{name}\" = \"{}\"", zed.language_id);
    }
    replace_generated_region(&text, "zed-language-ids", &ids)
}

fn render_zed_readme(original: &str, model: &Model) -> Result<String> {
    let body = wrap_text(&format!(
        "`tclLsp.dialect` takes any of {}.",
        code_sentence_list(&dialect_ids(model))
    ));
    let text = replace_generated_region(original, "zed-dialects", &body)?;
    Ok(with_default_dialect(&text))
}

// Helix

/// The language id an editor sends for `environment`: its editor identity
/// when it has one, otherwise its canonical id. The server resolves either.
fn language_id_of(model: &Model, environment: &EnvironmentDefinition) -> String {
    language_of_environment(&model.langs, environment.id.as_str())
        .map_or_else(|| environment.id.to_string(), |lang| lang.id.clone())
}

/// One Helix `[[language]]` block.
fn helix_block(
    comment: &str,
    name: &str,
    language_id: Option<&str>,
    file_types: &[String],
    shebangs: &[String],
) -> Result<String> {
    if contains_source_line_separator(comment) {
        bail!("Helix comment contains a line or control separator");
    }
    let mut out = format!("# {comment}\n[[language]]\nname = {}\n", toml_string(name)?);
    if let Some(id) = language_id {
        let _ = writeln!(out, "language-id = {}", toml_string(id)?);
    }
    out.push_str("scope = \"source.tcl\"\n");
    if !file_types.is_empty() {
        let _ = writeln!(out, "file-types = {}", toml_array(file_types)?);
    }
    if !shebangs.is_empty() {
        let _ = writeln!(out, "shebangs = {}", toml_array(shebangs)?);
    }
    out.push_str(concat!(
        "comment-tokens = [\"#\"]\n",
        "indent = { tab-width = 4, unit = \"    \" }\n",
        "language-servers = [\"tcl-lsp\"]\n",
        "auto-pairs = { \"{\" = \"}\", \"[\" = \"]\", \"(\" = \")\", \"\\\"\" = \"\\\"\" }\n",
    ));
    Ok(out)
}

/// The environments Helix gets a `[[language]]` block for: each has a file
/// extension or shebang word to key on. The rest can only be selected by a
/// `# tcl-dialect:` comment or the `dialect` setting.
fn helix_environments(model: &Model) -> Vec<(&Arc<EnvironmentDefinition>, Registration)> {
    model
        .environments
        .iter()
        .map(|environment| (environment, model.registration(environment)))
        .filter(|(_, reg)| !reg.extensions.is_empty() || !reg.shebang_words.is_empty())
        .collect()
}

fn render_helix_readme(original: &str, model: &Model) -> Result<String> {
    let plain = model
        .langs
        .iter()
        .find(|l| l.id == "tcl")
        .context("no plain tcl language")?;
    let mut blocks = vec![
        helix_block(
            "Plain Tcl and Tk. Sends languageId \"tcl\": the server detects the dialect.",
            "tcl",
            None,
            &plain.extensions,
            &plain.shebang_words,
        )?,
        concat!(
            "# Each environment that owns a file type or a shebang interpreter gets a\n",
            "# language entry of its own, so Helix sends a distinct `language-id`: the id\n",
            "# the server resolves to that environment.\n",
        )
        .to_owned(),
    ];
    for (environment, reg) in helix_environments(model) {
        blocks.push(helix_block(
            &environment.display_name,
            environment.id.as_str(),
            Some(&language_id_of(model, environment)),
            &reg.extensions,
            &reg.shebang_words,
        )?);
    }
    let text = replace_generated_region(original, "helix-languages", &blocks.join("\n"))?;

    let listed = helix_environments(model);
    let unlisted: Vec<&str> = model
        .environments
        .iter()
        .map(|environment| environment.id.as_str())
        .filter(|id| {
            !listed
                .iter()
                .any(|(environment, _)| environment.id.as_str() == *id)
        })
        .collect();
    let unlisted_text = match unlisted.as_slice() {
        [] => String::new(),
        [one] => wrap_text(&format!(
            "{} has no file extension or shebang word of its own, so it has no entry. Select \
             it per file with a `# tcl-dialect:` comment or per workspace with the `dialect` \
             setting below.",
            code_sentence_list(&[one])
        )),
        many => wrap_text(&format!(
            "{} have no file extension or shebang word of their own, so they have no entry. \
             Select them per file with a `# tcl-dialect:` comment or per workspace with the \
             `dialect` setting below.",
            code_sentence_list(many)
        )),
    };
    let text = replace_generated_region(&text, "helix-unlisted", &unlisted_text)?;
    let text = replace_generated_region(
        &text,
        "helix-dialects",
        &valid_dialects_comment(model, "#", ""),
    )?;
    Ok(with_default_dialect(&text))
}

// Emacs

/// Escape one string for literal use in an Emacs regular expression.
fn elisp_regex_literal(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        if matches!(ch, '\\' | '.' | '*' | '+' | '?' | '^' | '$' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// An Emacs regexp matching the file names ending in one of `extensions`,
/// emitted as a safely quoted Elisp string while preserving the established
/// grouped pattern form.
fn elisp_extension_regexp(extensions: &[String]) -> Result<String> {
    let escaped: Vec<String> = extensions
        .iter()
        .map(|extension| elisp_regex_literal(extension))
        .collect();
    let pattern = match escaped.as_slice() {
        [one] => format!(r"\.{one}\'"),
        many => format!(r"\.\({}\)\'", many.join(r"\|")),
    };
    elisp_string(&pattern)
}

/// An Emacs regexp matching a path ending in one of the whole basenames, as
/// an Elisp string.
fn elisp_basename_regexp(names: &[String]) -> Result<String> {
    let escaped: Vec<String> = names.iter().map(|name| elisp_regex_literal(name)).collect();
    let pattern = match escaped.as_slice() {
        [one] => format!(r"/{one}\'"),
        many => format!(r"/\({}\)\'", many.join(r"\|")),
    };
    elisp_string(&pattern)
}

/// An environment with its own Emacs derived mode, and the mode's name.
type DerivedMode<'a> = (&'a Arc<EnvironmentDefinition>, String);

/// The Emacs derived mode's name for `environment`.
fn emacs_mode_name(environment: &EnvironmentDefinition) -> String {
    let id = environment.id.as_str();
    EMACS_MODE_NAMES
        .iter()
        .find(|(named, _)| *named == id)
        .map_or_else(
            || format!("{}-mode", id.replace('.', "")),
            |(_, mode)| (*mode).to_owned(),
        )
}

/// The derived modes in selectable order: every environment with an editor
/// identity to send as the `languageId` and an extension, file name or
/// shebang word to key on. The rest of the family opens in plain `tcl-mode`:
/// the server routes those files from their own name or content, so sending
/// the plain language id costs nothing but the mode line.
fn derived_modes(model: &Model) -> Vec<DerivedMode<'_>> {
    model
        .environments
        .iter()
        .filter(|environment| environment.editor_identity.is_some())
        .filter(|environment| {
            let reg = model.registration(environment);
            !reg.extensions.is_empty() || !reg.filenames.is_empty() || !reg.shebang_words.is_empty()
        })
        .map(|environment| (environment, emacs_mode_name(environment)))
        .collect()
}

/// The derived-mode definitions and every `auto-mode-alist` /
/// `interpreter-mode-alist` row: what each mode owns, then plain `tcl-mode`
/// for the rest of the family.
fn emacs_mode_forms(model: &Model, owners: &[DerivedMode<'_>]) -> Result<String> {
    let mut body = String::new();
    for (environment, mode) in owners {
        let _ = writeln!(
            body,
            "(define-derived-mode {mode} tcl-mode {})",
            elisp_string(&environment.short_name)?
        );
    }
    body.push('\n');
    body.push_str(";; The extensions, file names and interpreters each derived mode owns.\n");
    for (environment, mode) in owners {
        let reg = model.registration(environment);
        if !reg.extensions.is_empty() {
            let _ = writeln!(
                body,
                "(add-to-list 'auto-mode-alist '({} . {mode}))",
                elisp_extension_regexp(&reg.extensions)?
            );
        }
        if !reg.filenames.is_empty() {
            let _ = writeln!(
                body,
                "(add-to-list 'auto-mode-alist '({} . {mode}))",
                elisp_basename_regexp(&reg.filenames)?
            );
        }
        for word in &reg.shebang_words {
            let _ = writeln!(
                body,
                "(add-to-list 'interpreter-mode-alist '({} . {mode}))",
                elisp_string(word)?
            );
        }
    }
    let derived: Vec<&str> = owners.iter().map(|(e, _)| e.id.as_str()).collect();
    body.push('\n');
    body.push_str(concat!(
        ";; Plain `tcl-mode` for the rest of the family. The server routes these from\n",
        ";; their own name or content, so sending languageId \"tcl\" costs nothing but\n",
        ";; the mode line.\n",
    ));
    let plain_extensions = model.extensions_outside(&derived);
    if !plain_extensions.is_empty() {
        let _ = writeln!(
            body,
            "(add-to-list 'auto-mode-alist '({} . tcl-mode))",
            elisp_extension_regexp(&plain_extensions)?
        );
    }
    let names = model.filenames_outside(&derived);
    if !names.is_empty() {
        let _ = writeln!(
            body,
            "(add-to-list 'auto-mode-alist '({} . tcl-mode))",
            elisp_basename_regexp(&names)?
        );
    }
    Ok(body)
}

/// The `eglot-server-programs` rows: plain `tcl-mode`, then each derived
/// mode with the language id it sends.
fn emacs_eglot_forms(model: &Model, owners: &[DerivedMode<'_>]) -> Result<String> {
    let mut eglot = String::from(concat!(
        "(with-eval-after-load 'eglot\n",
        "  (add-to-list 'eglot-server-programs\n",
        "               '(tcl-mode . (\"/path/to/tcl-lsp-server\")))\n",
    ));
    for (index, (environment, mode)) in owners.iter().enumerate() {
        let id = elisp_string(&language_id_of(model, environment))?;
        let close = if index + 1 == owners.len() { ")" } else { "" };
        let _ = writeln!(
            eglot,
            "  (add-to-list 'eglot-server-programs\n               '(({mode} :language-id {id}) . (\"/path/to/tcl-lsp-server\"))){close}"
        );
    }
    Ok(eglot)
}

/// The `eglot-ensure` hooks: plain `tcl-mode` and each derived mode.
fn emacs_hook_forms(owners: &[DerivedMode<'_>]) -> String {
    let mut hooks: Vec<String> = vec!["tcl-mode-hook".to_owned()];
    hooks.extend(owners.iter().map(|(_, mode)| format!("{mode}-hook")));
    if let Some(last) = hooks.last_mut() {
        last.push_str("))");
    }
    let first = "(dolist (h '(";
    let mut forms = String::from(";; Auto-start on Tcl and the dialect modes\n");
    forms.push_str(&wrap_words(first, &" ".repeat(first.len()), &hooks));
    forms.push_str("  (add-hook h #'eglot-ensure))\n");
    forms
}

/// The prose after the settings snippet: which files ride a derived mode and
/// which ride plain `tcl-mode`.
fn emacs_notes(model: &Model, owners: &[DerivedMode<'_>]) -> Result<String> {
    let derived: Vec<&str> = owners.iter().map(|(e, _)| e.id.as_str()).collect();
    let owned: Vec<String> = owners
        .iter()
        .flat_map(|(environment, _)| model.registration(environment).extensions)
        .map(|ext| format!(".{ext}"))
        .collect();
    let plain: Vec<String> = model
        .extensions_outside(&derived)
        .iter()
        .map(|ext| format!(".{ext}"))
        .collect();
    let mut notes = wrap_text(&format!(
        "{} files are handled by the derived modes in the eglot setup above, which send the \
         correct `languageId` — do **not** also map them to plain `tcl-mode`, or they would \
         analyse as `{}`.",
        code_sentence_list(&owned),
        model.default_environment()?.id
    ));
    notes.push('\n');
    let names = model.filenames_outside(&derived);
    let file_names = if names.is_empty() {
        String::new()
    } else {
        format!(", and the file names {}", code_sentence_list(&names))
    };
    notes.push_str(&wrap_text(&format!(
        "Everything else the registry owns rides plain `tcl-mode`: {}{file_names}. The \
         server detects those from their own content or file name, so the `languageId` has \
         no ambiguity to resolve.",
        code_sentence_list(&plain)
    )));
    notes.push('\n');
    notes.push_str(&wrap_text(
        "The `auto-mode-alist`, `interpreter-mode-alist` and `eglot-server-programs` forms \
         above are generated from the environment registry by `cargo xtask gen-editor-configs`; \
         CI fails if they drift.",
    ));
    Ok(notes)
}

fn render_emacs_readme(original: &str, model: &Model) -> Result<String> {
    let owners = derived_modes(model);
    let text =
        replace_generated_region(original, "emacs-modes", &emacs_mode_forms(model, &owners)?)?;
    let text = replace_generated_region(&text, "emacs-eglot", &emacs_eglot_forms(model, &owners)?)?;
    let text = replace_generated_region(&text, "emacs-hooks", &emacs_hook_forms(&owners))?;
    let text = replace_generated_region(&text, "emacs-notes", &emacs_notes(model, &owners)?)?;
    let text = replace_generated_region(
        &text,
        "emacs-dialects",
        &wrap_text(&format!(
            "`:dialect` takes any of {}.",
            code_sentence_list(&dialect_ids(model))
        )),
    )?;
    Ok(with_default_dialect(&text))
}

// Neovim

/// The `extension = { … }` entries: a comment and a wrapped run of
/// `ext = 'filetype'` per language. An extra language keeps its own file
/// type.
fn neovim_extensions(model: &Model, indent: &str) -> Result<String> {
    let mut out = String::new();
    for lang in &model.langs {
        if lang.extensions.is_empty() {
            continue;
        }
        let file_type = if lang.is_extra {
            lang.id.as_str()
        } else {
            "tcl"
        };
        let items: Vec<String> = lang
            .extensions
            .iter()
            .map(|ext| Ok(format!("{} = {}", lua_key(ext)?, lua_string(file_type)?)))
            .collect::<Result<_>>()?;
        anyhow::ensure!(
            !contains_source_line_separator(&lang.aliases[0]),
            "Neovim language alias contains a line or control separator"
        );
        let _ = writeln!(out, "{indent}-- {}", lang.aliases[0]);
        out.push_str(&wrap_words(indent, indent, &comma_separated(&items, true)));
    }
    Ok(out)
}

/// A `pattern` entry that gives a script with no extension the `tcl` file
/// type from the interpreter its first line names.
fn neovim_shebangs(model: &Model, indent: &str) -> Result<String> {
    let words: Vec<String> = model
        .shebang_stems()
        .iter()
        .map(|word| lua_string(word))
        .collect::<Result<_>>()?;
    Ok(format!(
        "{indent}-- Scripts named by their interpreter rather than an extension.\n\
         {indent}['.*'] = {{\n\
         {indent}  function(_, bufnr)\n\
         {indent}    local first = vim.api.nvim_buf_get_lines(bufnr, 0, 1, false)[1] or ''\n\
         {indent}    for _, word in ipairs({{ {} }}) do\n\
         {indent}      if first:find('^#!.-%f[%w]' .. vim.pesc(word)) then\n\
         {indent}        return 'tcl'\n\
         {indent}      end\n\
         {indent}    end\n\
         {indent}  end,\n\
         {indent}  {{ priority = -math.huge }},\n\
         {indent}}},\n",
        words.join(", ")
    ))
}

/// The file types a Neovim server entry attaches to: plain `tcl`, and the
/// file type of each extra language.
fn neovim_filetypes(model: &Model) -> Vec<String> {
    let mut types = vec!["tcl".to_owned()];
    types.extend(
        model
            .langs
            .iter()
            .filter(|lang| lang.is_extra)
            .map(|lang| lang.id.clone()),
    );
    types
}

fn default_dialect_row_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(\| `dialect` \| string \| `)tcl\d\.\d(`)")
            .expect("the settings-table pattern is a valid regex")
    })
}

fn render_neovim_lua(original: &str, model: &Model) -> Result<String> {
    let text = set_lua_list(
        original,
        LuaFiletypesOwner::ReturnedRootTable,
        "  ",
        "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },",
        "root_markers = { '.git' },",
        &neovim_filetypes(model),
    )?;
    let text = replace_generated_region(
        &text,
        "neovim-dialects",
        &valid_dialects_comment(model, "--", "      "),
    )?;
    Ok(with_default_dialect(&text))
}

fn render_neovim_readme(original: &str, model: &Model) -> Result<String> {
    let region = neovim_lspconfig_fence(original)?;
    let rendered = set_lua_list(
        &original[region.clone()],
        LuaFiletypesOwner::LspconfigDefaultConfig,
        "      ",
        "cmd = { '/path/to/tcl-lsp-server' },",
        "root_dir = lspconfig.util.root_pattern('.git'),",
        &neovim_filetypes(model),
    )?;
    let mut text = original.to_owned();
    text.replace_range(region, &rendered);
    let text = replace_generated_region(
        &text,
        "neovim-extensions",
        &neovim_extensions(model, "    ")?,
    )?;
    let text =
        replace_generated_region(&text, "neovim-shebangs", &neovim_shebangs(model, "    ")?)?;
    let text = replace_generated_region(
        &text,
        "neovim-dialects",
        &wrap_text(&format!(
            "`dialect` takes any of {}.",
            code_sentence_list(&dialect_ids(model))
        )),
    )?;
    let text = default_dialect_row_regex()
        .replace_all(&text, |captures: &regex::Captures| {
            format!("{}{DEFAULT_ENVIRONMENT_ID}{}", &captures[1], &captures[2])
        })
        .into_owned();
    Ok(with_default_dialect(&text))
}

// Sublime

fn render_sublime_readme(original: &str, model: &Model) -> Result<String> {
    let default = model.default_environment()?;
    let body = wrap_text(&format!(
        "The default dialect is {}. Change `settings.tclLsp.dialect` in LSP-Tcl's settings to \
         one of {}.",
        default.display_name,
        code_sentence_list(&dialect_ids(model))
    ));
    replace_generated_region(original, "sublime-dialects", &body)
}

// Installation guide

/// The generic-client extension lists in the installation guide — Vim/Neovim
/// `au BufRead`, coc-settings' `fileExtensions`, the Lua `file_patterns`, and
/// the `file-types` of a single Helix `tcl` language — and the Neovim
/// `vim.filetype.add` table.
///
/// They are configuration users paste, so a missing entry is a client that
/// never attaches, not a documentation nit.
///
/// These name [`tcl_registry::dialects::TCL_SOURCE_EXTENSIONS`] rather than
/// the full registered union, and deliberately: each attaches **one** filetype
/// or language to everything it lists, which is the "project source we index"
/// question, not the "which dialect owns this suffix" one. That is also what
/// keeps the vendor suffixes that collide with foreign files (`.do`,
/// `.globals`, `.sdc`) out of a blanket `set filetype=tcl`.
fn render_install_editors(original: &str, _model: &Model) -> Result<String> {
    let extensions = tcl_registry::dialects::TCL_SOURCE_EXTENSIONS;
    let mut out = original.to_owned();
    let renders: [(&str, &str, String); 4] = [
        (
            "au BufRead,BufNewFile ",
            " set filetype=tcl",
            extensions
                .iter()
                .map(|e| format!("*.{e}"))
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "      \"fileExtensions\": [",
            "],",
            extensions
                .iter()
                .map(|e| format!("\".{e}\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "  file_patterns = { ",
            " },",
            extensions
                .iter()
                .map(|e| format!("\"%.{e}$\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "file-types = [",
            "]\nlanguage-servers",
            extensions
                .iter()
                .map(|e| format!("\"{e}\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ];
    for (prefix, suffix, body) in &renders {
        let mut at = 0;
        let mut rewrote = false;
        while let Some(found) = out[at..].find(prefix) {
            let list_start = at + found + prefix.len();
            let Some(list_end) = out[list_start..]
                .find(suffix)
                .map(|n| list_start + n)
                .filter(|end| !out[list_start..*end].contains('\n'))
            else {
                at = list_start;
                continue;
            };
            out.replace_range(list_start..list_end, body);
            at = list_start + body.len();
            rewrote = true;
        }
        if !rewrote {
            bail!("{INSTALL_EDITORS} has no `{prefix}…{suffix}` extension list to generate");
        }
    }
    let table: Vec<String> = extensions
        .iter()
        .map(|ext| Ok(format!("{} = {}", lua_key(ext)?, lua_string("tcl")?)))
        .collect::<Result<_>>()?;
    let out = replace_generated_region(
        &out,
        "install-neovim-extensions",
        &wrap_words("  ", "  ", &comma_separated(&table, true)),
    )?;
    Ok(with_default_dialect(&out))
}

type Render = fn(&str, &Model) -> Result<String>;

/// Every file this generator owns, paired with the render that rebuilds it.
const TARGETS: &[(&str, Render)] = &[
    (ZED_EXTENSION, render_zed_extension),
    (ZED_README, render_zed_readme),
    (HELIX_README, render_helix_readme),
    (EMACS_README, render_emacs_readme),
    (NEOVIM_LUA, render_neovim_lua),
    (NEOVIM_README, render_neovim_readme),
    (SUBLIME_README, render_sublime_readme),
    (INSTALL_EDITORS, render_install_editors),
];

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let model = Model::load()?;
    let mut rendered_files = Vec::with_capacity(TARGETS.len());
    for &(rel, render) in TARGETS {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let rendered = render(&original, &model).with_context(|| format!("rendering {rel}"))?;
        rendered_files.push((rel, path, original, rendered));
    }
    let mut drift = Vec::new();
    for (rel, _path, original, rendered) in &rendered_files {
        if rendered == original {
            continue;
        }
        if check {
            drift.push(*rel);
        }
    }
    if check && !drift.is_empty() {
        eprintln!(
            "{} editor configuration file(s) are stale — run `cargo xtask gen-editor-configs`:",
            drift.len()
        );
        for rel in drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: editor configuration regions match the environment registry.");
        return Ok(ExitCode::SUCCESS);
    }
    for (rel, path, original, rendered) in rendered_files {
        if rendered != original && write_if_changed(&path, &rendered)? {
            eprintln!("wrote {rel}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed(rel: &str) -> String {
        fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
    }

    #[test]
    fn committed_files_match_generated_regions() {
        let model = Model::load().unwrap();
        for &(rel, render) in TARGETS {
            let original = committed(rel);
            assert_eq!(
                render(&original, &model).unwrap(),
                original,
                "{rel} is stale"
            );
        }
    }

    /// A render must repair a broken region, not merely reproduce a file that
    /// already matches.
    #[test]
    fn a_stale_dialect_list_is_regenerated() {
        let model = Model::load().unwrap();
        let original = committed(NEOVIM_README);
        let broken = original.replace("`jim`", "`bogus`");
        assert_ne!(broken, original, "the README lists jim");
        assert_eq!(render_neovim_readme(&broken, &model).unwrap(), original);
    }

    /// Every region a target file declares is one its render writes: blank
    /// each in turn and the render must restore the committed file.
    #[test]
    fn every_declared_region_is_written_by_its_render() {
        let model = Model::load().unwrap();
        let marker = Regex::new(r"<generated: ([a-z0-9-]+)>").unwrap();
        let mut seen = 0;
        for &(rel, render) in TARGETS {
            let original = committed(rel);
            for name in marker.captures_iter(&original).map(|c| c[1].to_owned()) {
                let blanked = replace_generated_region(&original, &name, "").unwrap();
                assert_eq!(
                    render(&blanked, &model).unwrap(),
                    original,
                    "{rel}: region {name} is not restored"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 17, "the target files declare 17 regions");
    }

    /// Every environment with an editor identity and something to key on gets a
    /// derived mode with a name of its own, and a new one would need no edit
    /// here.
    #[test]
    fn every_environment_with_an_identity_and_a_claim_has_a_derived_mode() {
        let model = Model::load().unwrap();
        let owners = derived_modes(&model);
        let names: Vec<&str> = owners.iter().map(|(_, mode)| mode.as_str()).collect();
        for mode in &names {
            assert!(
                mode.ends_with("-mode") && !mode.contains('.') && *mode != "tcl-mode",
                "{mode}"
            );
            assert_eq!(
                names.iter().filter(|other| *other == mode).count(),
                1,
                "{mode}"
            );
        }
        for environment in model.environments {
            let reg = model.registration(environment);
            let claims = !reg.extensions.is_empty()
                || !reg.filenames.is_empty()
                || !reg.shebang_words.is_empty();
            let has_mode = owners
                .iter()
                .any(|(owner, _)| owner.id.as_str() == environment.id.as_str());
            assert_eq!(
                has_mode,
                environment.editor_identity.is_some() && claims,
                "{}",
                environment.id
            );
        }
        for (id, mode) in [
            ("expect", "expect-mode"),
            ("f5-iapps", "f5-iapps-mode"),
            ("f5-irules", "f5-irules-mode"),
            ("f5-tmsh", "f5-tmsh-mode"),
            ("jim", "jim-tcl-mode"),
            ("tcl8.6", "tcl86-mode"),
            ("xilinx-eda-tcl", "xilinx-eda-tcl-mode"),
        ] {
            assert!(
                owners
                    .iter()
                    .any(|(owner, name)| owner.id.as_str() == id && name == mode),
                "{id} has {mode}"
            );
        }
    }

    /// The Emacs forms are read by Emacs, which this generator cannot run: each
    /// top-level form must close its parentheses before the next begins, and
    /// the region as a whole must balance. Parentheses inside strings (regexp
    /// groups) and comments do not count.
    #[test]
    fn the_emacs_forms_balance_their_parentheses() {
        let model = Model::load().unwrap();
        let owners = derived_modes(&model);
        for (name, forms) in [
            ("modes", emacs_mode_forms(&model, &owners).unwrap()),
            ("eglot", emacs_eglot_forms(&model, &owners).unwrap()),
            ("hooks", emacs_hook_forms(&owners)),
        ] {
            let mut depth: i32 = 0;
            for line in forms.lines() {
                let starts_form = line.starts_with('(');
                if starts_form {
                    assert_eq!(depth, 0, "{name}: a form starts inside another: {line}");
                }
                let mut in_string = false;
                let mut chars = line.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' if in_string => {
                            chars.next();
                        }
                        '"' => in_string = !in_string,
                        ';' if !in_string => break,
                        '(' if !in_string => depth += 1,
                        ')' if !in_string => depth -= 1,
                        _ => {}
                    }
                    assert!(depth >= 0, "{name}: an unmatched ')' in {line}");
                }
                assert!(!in_string, "{name}: an unterminated string in {line}");
            }
            assert_eq!(depth, 0, "{name}: the region does not balance");
        }
    }

    #[test]
    fn the_iapp_presentation_file_name_rides_the_iapps_mode() {
        let model = Model::load().unwrap();
        let emacs = render_emacs_readme(&committed(EMACS_README), &model).unwrap();
        assert!(
            emacs.contains(
                "(add-to-list 'auto-mode-alist '(\"/presentation\\\\'\" . f5-iapps-mode))"
            )
        );
        assert!(
            !emacs.contains("\"presentation\""),
            "the plain-mode file names exclude it"
        );
    }

    #[test]
    fn jim_reaches_helix_emacs_and_neovim() {
        let model = Model::load().unwrap();
        let helix = render_helix_readme(&committed(HELIX_README), &model).unwrap();
        assert!(
            helix.contains(
                "name = \"jim\"\nlanguage-id = \"tcl-jim\"\nscope = \"source.tcl\"\nshebangs = [\"jimsh\"]"
            ),
            "{helix}"
        );
        let emacs = render_emacs_readme(&committed(EMACS_README), &model).unwrap();
        assert!(emacs.contains("(define-derived-mode jim-tcl-mode tcl-mode \"Jim\")"));
        assert!(emacs.contains("(jim-tcl-mode :language-id \"tcl-jim\")"));
        assert!(
            emacs.contains("(add-to-list 'interpreter-mode-alist '(\"jimsh\" . jim-tcl-mode))")
        );
        let neovim = render_neovim_readme(&committed(NEOVIM_README), &model).unwrap();
        assert!(neovim.contains("'jimsh'"));
    }

    #[test]
    fn the_zed_language_table_names_every_directory() {
        let model = Model::load().unwrap();
        let toml = render_zed_extension(&committed(ZED_EXTENSION), &model).unwrap();
        for zed in ZED_LANGUAGES {
            let name = zed_language_name(zed).unwrap();
            assert!(
                toml.contains(&format!("\"{name}\" = \"{}\"", zed.language_id)),
                "{name}"
            );
        }
    }

    #[test]
    fn every_default_dialect_is_the_default_environment() {
        let stale = "dialect = 'tcl9.0'\n\"dialect\": \"tcl8.4\"\n:dialect \"tcl8.5\"\n'dialect': 'tcl9.1'\n";
        let fresh = with_default_dialect(stale);
        assert_eq!(fresh.matches(DEFAULT_ENVIRONMENT_ID).count(), 4, "{fresh}");
        assert_eq!(
            with_default_dialect("dialect = 'f5-irules'"),
            "dialect = 'f5-irules'",
            "a named dialect is not a default"
        );
    }

    #[test]
    fn lua_keys_are_bare_unless_reserved() {
        assert_eq!(lua_key("tcl").unwrap(), "tcl");
        assert_eq!(lua_key("do").unwrap(), "['do']");
        assert_eq!(lua_key("3d").unwrap(), "['3d']");
    }

    #[test]
    fn target_literals_keep_hostile_extension_text_inside_source_values() {
        let extension = "odd\"\n\u{2028}".to_owned();
        let helix = helix_block(
            "safe",
            "dialect\"name",
            Some("lang\n-id"),
            std::slice::from_ref(&extension),
            &[],
        )
        .unwrap();
        assert!(helix_block("bad\u{2028}comment", "name", None, &[], &[]).is_err());
        let parsed: toml::Value = toml::from_str(&helix).unwrap();
        let row = &parsed["language"][0];
        assert_eq!(row["name"].as_str(), Some("dialect\"name"));
        assert_eq!(row["language-id"].as_str(), Some("lang\n-id"));
        assert_eq!(row["file-types"][0].as_str(), Some(extension.as_str()));

        let lua = set_lua_list("return {\n  cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },\n\n  filetypes = { 'old' },\n  root_markers = { '.git' },\n}", LuaFiletypesOwner::ReturnedRootTable, "  ", "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },", "root_markers = { '.git' },", std::slice::from_ref(&extension)).unwrap();
        assert!(lua.contains(&lua_string(&extension).unwrap()));
        assert!(!lua.contains("odd\"\n"));
        assert!(
            set_lua_list(
                "return { otherfiletypes = { 'old' }, }\n",
                LuaFiletypesOwner::ReturnedRootTable,
                "  ",
                "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },",
                "root_markers = { '.git' },",
                &["tcl".into()]
            )
            .is_err()
        );
        assert!(
            set_lua_list(
                "return { -- filetypes = { 'old' },\n}\n",
                LuaFiletypesOwner::ReturnedRootTable,
                "  ",
                "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },",
                "root_markers = { '.git' },",
                &["tcl".into()]
            )
            .is_err()
        );
        let long_string_decoy = "local decoy = [[\ncmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },\n  filetypes = { 'old' },\nroot_markers = { '.git' },\n]]\nreturn {}\n";
        assert!(
            set_lua_list(
                long_string_decoy,
                LuaFiletypesOwner::ReturnedRootTable,
                "  ",
                "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },",
                "root_markers = { '.git' },",
                &["tcl".into()]
            )
            .is_err()
        );
        let long_comment_decoy = "--[[\ncmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },\n  filetypes = { 'old' },\nroot_markers = { '.git' },\n]]\nreturn {}\n";
        assert!(
            set_lua_list(
                long_comment_decoy,
                LuaFiletypesOwner::ReturnedRootTable,
                "  ",
                "cmd = { '/path/to/tcl-lsp/target/release/tcl-lsp-server' },",
                "root_markers = { '.git' },",
                &["tcl".into()]
            )
            .is_err()
        );
        let key = "x'] = os.execute('bad')";
        assert_eq!(
            lua_key(key).unwrap(),
            format!("[{}]", lua_string(key).unwrap())
        );
        assert!(!lua_key(key).unwrap().contains("'] ="));

        let elisp = elisp_extension_regexp(std::slice::from_ref(&extension)).unwrap();
        let expected_pattern =
            elisp_string(&format!(r"\.{}\'", elisp_regex_literal(&extension))).unwrap();
        assert_eq!(elisp, expected_pattern);
        assert!(!elisp.contains("odd\"\n"));
    }

    #[test]
    fn wrapped_lists_stay_within_the_width() {
        let items: Vec<String> = (0..40).map(|i| format!("item{i}")).collect();
        for line in wrap_words("  ", "  ", &comma_separated(&items, true)).lines() {
            assert!(line.len() <= WRAP_WIDTH, "{line}");
        }
    }
}

#[cfg(test)]
mod lua_chunk_owner_v4_tests {
    use super::*;

    #[test]
    fn actual_readme_prose_is_outside_the_selected_lua_source() {
        let original = fs::read_to_string(repo_root().join(NEOVIM_README)).unwrap();
        assert!(original.contains("Neovim's"));
        let root = fs::read_to_string(repo_root().join(NEOVIM_LUA)).unwrap();
        assert!(
            lua_filetypes_owner_open(
                &root,
                &lua_tokens(&root).unwrap(),
                LuaFiletypesOwner::ReturnedRootTable
            )
            .is_ok()
        );
        let region = neovim_lspconfig_fence(&original).unwrap();
        let source = &original[region];
        let tokens = lua_tokens(source).unwrap();
        assert!(
            lua_filetypes_owner_open(source, &tokens, LuaFiletypesOwner::LspconfigDefaultConfig)
                .is_ok()
        );
        let bad = original.replace("```lua\nlocal lspconfig", "```text\nlocal lspconfig");
        assert!(neovim_lspconfig_fence(&bad).is_err());
        let decoy = "````text\n## nvim-lspconfig (Neovim 0.8+)\n```lua\nreturn {}\n```\n````\n";
        assert!(neovim_lspconfig_fence(decoy).is_err());
        let duplicate = format!("{original}\n## nvim-lspconfig (Neovim 0.8+)\n");
        assert!(neovim_lspconfig_fence(&duplicate).is_err());
    }

    #[test]
    fn nested_function_or_rebound_config_cannot_donate_the_owner() {
        let decoy = "local function decoy() return { filetypes = {} } end";
        assert!(
            lua_filetypes_owner_open(
                decoy,
                &lua_tokens(decoy).unwrap(),
                LuaFiletypesOwner::ReturnedRootTable
            )
            .is_err()
        );
        let decoy = "return { callback = function() filetypes = {} end }";
        assert!(
            lua_filetypes_owner_open(
                decoy,
                &lua_tokens(decoy).unwrap(),
                LuaFiletypesOwner::ReturnedRootTable
            )
            .is_err()
        );
        let readme = fs::read_to_string(repo_root().join(NEOVIM_README)).unwrap();
        let region = neovim_lspconfig_fence(&readme).unwrap();
        let source = &readme[region];
        let rebound = source.replace(
            "local configs   = require('lspconfig.configs')",
            "local configs = require('unrelated')",
        );
        assert_ne!(rebound, source);
        assert!(
            lua_filetypes_owner_open(
                &rebound,
                &lua_tokens(&rebound).unwrap(),
                LuaFiletypesOwner::LspconfigDefaultConfig
            )
            .is_err()
        );
        let wrapped = format!("local function decoy()\n{source}\nend");
        assert!(
            lua_filetypes_owner_open(
                &wrapped,
                &lua_tokens(&wrapped).unwrap(),
                LuaFiletypesOwner::LspconfigDefaultConfig
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod markdown_and_lua_key_v5_tests {
    use super::*;

    #[test]
    fn inert_markdown_containers_cannot_donate_a_lua_fence() {
        let readme = fs::read_to_string(repo_root().join(NEOVIM_README)).unwrap();
        let region = neovim_lspconfig_fence(&readme).unwrap();
        let source = &readme[region];
        for wrapper in [
            format!("<!--\n## nvim-lspconfig (Neovim 0.8+)\n```lua\n{source}```\n-->\n"),
            format!("<script>\n## nvim-lspconfig (Neovim 0.8+)\n```lua\n{source}```\n</script>\n"),
            "## nvim-lspconfig (Neovim 0.8+)\n```lua\nreturn {}\n```\n"
                .lines()
                .map(|line| ["> ", line, "\n"].concat())
                .collect(),
        ] {
            assert!(neovim_lspconfig_fence(&wrapper).is_err());
        }
        assert!(neovim_lspconfig_fence(&readme).is_ok());
    }

    #[test]
    fn quoted_or_computed_duplicate_keys_refuse_the_whole_owner() {
        let source = fs::read_to_string(repo_root().join(NEOVIM_LUA)).unwrap();
        for key in [
            "['filetypes']",
            "[\"filetypes\"]",
            "[name]",
            "['file' .. 'types']",
        ] {
            let changed = source.replace(
                "  root_markers =",
                &format!("  {key} = {{ 'shadow' }},\n  root_markers ="),
            );
            assert_ne!(changed, source);
            assert!(
                lua_filetypes_owner_open(
                    &changed,
                    &lua_tokens(&changed).unwrap(),
                    LuaFiletypesOwner::ReturnedRootTable
                )
                .is_err()
            );
        }
    }
}
