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

//! `object-recipe-gate` — one selector for list and dictionary recipes in
//! `tcl-vm`.
//!
//! Inside the iRules simulation a list or dictionary is a Tcl 8.4 object
//! while the physical engine is Tcl 9, so a producer and a consumer that
//! pick their recipe from different selectors seal one object twice and the
//! second seal is refused. Every list or dictionary constructor, conversion
//! or seal therefore takes its protocol from `ObjectMaterialization` (or an
//! `OriginalAuthoredOperandScope`), never from the physical engine.
//!
//! The gate is lexical. A production function that both calls a list or
//! dictionary sink and reads a physical string protocol (a physical dialect
//! selector together with `native_string_protocol`) fails, unless it is in
//! [`PHYSICAL_ONLY`]: a site the authored realm cannot reach, or one whose
//! objects never cross into it. A [`PHYSICAL_ONLY`] entry that no longer
//! matches also fails, so the list cannot go stale. Protocols passed between
//! functions are out of the gate's reach; the selector is what review holds.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rustc_lexer::TokenKind;

/// The crate the gate covers.
const SCOPE: &str = "rust/tcl-vm/src";

/// Calls that construct, convert or seal a list or dictionary under a
/// supplied protocol.
const SINKS: &[&str] = &[
    "duplicate_native_object_checked",
    "into_native_dictionary_search",
    "into_native_dictionary_search_with_strings",
    "list_with_native_canonical_in",
    "native_dictionary_constructor",
    "native_dictionary_set_member",
    "native_full_list_range",
    "native_list_append_elements",
    "native_list_append_list_elements",
    "native_list_append_prepared_elements",
    "native_list_append_prepared_elements_checked",
    "native_list_backing_in",
    "native_list_command_range",
    "native_list_command_range_checked",
    "native_list_concatenate",
    "native_list_constructor",
    "native_list_copy",
    "native_list_copy_checked",
    "native_list_range",
    "native_list_range_validate",
    "native_list_replace_elements",
    "native_list_replace_prepared_range_checked",
    "native_object_dict_pairs",
    "native_object_dict_pairs_in",
    "native_object_dict_pairs_with_strings",
    "native_object_list_elements",
    "native_object_list_elements_checked",
    "native_object_list_elements_in",
    "native_object_list_elements_with_strings",
    "prepare_native_dictionary",
    "prepare_native_dictionary_after_conversion",
    "prepare_native_dictionary_after_conversion_checked",
    "prepare_native_dictionary_for_increment",
    "seal_compound_string_protocol",
];

/// Selectors that name the physical engine rather than the object purpose.
const PHYSICAL: &[&str] = &[
    "actual_native_invocation_dialect",
    "native_scalar_carrier_dialect",
];

/// The protocol a physical selector yields for strings and compounds.
const STRING_PROTOCOL: &str = "native_string_protocol";

const COROUTINE: &str =
    "coroutines are absent from the Tcl 8.4 surface; their relay lists stay in the engine";
const TCLOO: &str = "TclOO is absent from the Tcl 8.4 surface";

/// `(file under SCOPE, function, reason)` for each reviewed physical site.
const PHYSICAL_ONLY: &[(&str, &str, &str)] = &[
    ("cmd_coro.rs", "cmd_coroutine", COROUTINE),
    ("cmd_coro.rs", "invoke_original_relay", COROUTINE),
    ("cmd_coro.rs", "request_yieldto", COROUTINE),
    ("cmd_coro.rs", "request_yieldto_original", COROUTINE),
    ("cmd_coro.rs", "resume_delivery_value", COROUTINE),
    ("cmd_oo/native_context.rs", "helper_context_error", TCLOO),
    ("exec.rs", "tcloo_next_list", TCLOO),
    ("exec.rs", "tick_original_invocation", COROUTINE),
    (
        "interp.rs",
        "error_stack_log_value",
        "the error stack is the physical engine's own record",
    ),
];

/// Run the gate; `check` is accepted for symmetry, the gate never rewrites.
pub fn run(_check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let scope = root.join(SCOPE);
    let mut files = Vec::new();
    collect_rs_files(&scope, &mut files);
    let test_only = test_only_files(&files);
    let mut flagged = BTreeSet::new();
    for path in &files {
        if test_only.contains(path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let relative = path
            .strip_prefix(&scope)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for function in physical_recipe_functions(&text) {
            flagged.insert((relative.clone(), function));
        }
    }
    let mut report = String::new();
    for (file, function) in &flagged {
        if !PHYSICAL_ONLY
            .iter()
            .any(|(allowed, name, _)| allowed == file && name == function)
        {
            let _ = writeln!(report, "  {SCOPE}/{file}: fn {function}");
        }
    }
    let mut stale = String::new();
    for (file, function, _) in PHYSICAL_ONLY {
        if !flagged.contains(&((*file).to_owned(), (*function).to_owned())) {
            let _ = writeln!(stale, "  {SCOPE}/{file}: fn {function}");
        }
    }
    if !report.is_empty() {
        eprintln!(
            "object-recipe-gate: a list or dictionary recipe read from the physical \
             engine; take it from `object_protocol()` / `ObjectMaterialization` \
             instead, or add a reviewed physical-only entry to \
             rust/xtask/src/object_recipe_gate.rs:\n{report}"
        );
    }
    if !stale.is_empty() {
        eprintln!("object-recipe-gate: physical-only entries that no longer match:\n{stale}");
    }
    if report.is_empty() && stale.is_empty() {
        println!("object-recipe-gate: OK (every list and dictionary recipe has one selector)");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// A token's kind and text, trivia dropped.
struct Tok<'a> {
    kind: TokenKind,
    text: &'a str,
}

fn tokens(text: &str) -> Vec<Tok<'_>> {
    let mut offset = 0;
    rustc_lexer::tokenize(text)
        .filter_map(|token| {
            let lexeme = &text[offset..offset + token.len];
            offset += token.len;
            (!matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
            ))
            .then_some(Tok {
                kind: token.kind,
                text: lexeme,
            })
        })
        .collect()
}

/// The length of a `#[cfg(…)]` attribute at `index` that holds only under
/// test: it names `test` other than as `not(test)`.
fn cfg_test_len(tokens: &[Tok<'_>], index: usize) -> Option<usize> {
    let head = tokens.get(index..index + 4)?;
    if !head
        .iter()
        .map(|token| token.text)
        .eq(["#", "[", "cfg", "("])
    {
        return None;
    }
    let mut depth = 0usize;
    let mut test = false;
    for position in index + 1..tokens.len() {
        match tokens[position].text {
            "[" | "(" => depth += 1,
            ")" => depth = depth.saturating_sub(1),
            "]" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return test.then_some(position + 1 - index);
                }
            }
            "test" => {
                let negated = position >= 2
                    && tokens[position - 1].text == "("
                    && tokens[position - 2].text == "not";
                test |= !negated;
            }
            _ => {}
        }
    }
    None
}

/// The module files a `#[cfg(test)] mod name;` declaration compiles only
/// under test, through its `#[path]` when it has one.
fn test_only_files(files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let tokens = tokens(&text);
        let directory = path.parent().unwrap_or(path);
        let children = if path
            .file_name()
            .is_some_and(|name| name == "mod.rs" || name == "lib.rs")
        {
            directory.to_path_buf()
        } else {
            path.with_extension("")
        };
        for index in 0..tokens.len() {
            let Some(length) = cfg_test_len(&tokens, index) else {
                continue;
            };
            let mut rest = &tokens[index + length..];
            let mut explicit = None;
            while let [hash, open, ..] = rest
                && hash.text == "#"
                && open.text == "["
            {
                if let [_, _, key, eq, literal, ..] = rest
                    && key.text == "path"
                    && eq.text == "="
                {
                    explicit = Some(directory.join(literal.text.trim_matches('"')));
                }
                let close = rest.iter().position(|token| token.text == "]");
                rest = &rest[close.map_or(rest.len(), |close| close + 1)..];
            }
            while rest
                .first()
                .is_some_and(|token| matches!(token.text, "pub" | "(" | "crate" | "super" | ")"))
            {
                rest = &rest[1..];
            }
            if let [module, name, semi, ..] = rest
                && module.text == "mod"
                && semi.text == ";"
            {
                out.insert(explicit.unwrap_or_else(|| children.join(format!("{}.rs", name.text))));
                out.insert(children.join(name.text).join("mod.rs"));
            }
        }
    }
    out
}

/// A physical protocol bound to names, live until its block closes.
struct Binding {
    names: Vec<String>,
    depth: usize,
}

/// Whether `tokens` read a physical engine's string protocol.
fn reads_physical_protocol(tokens: &[Tok<'_>]) -> bool {
    let called = |name: &str| {
        tokens.windows(2).any(|pair| {
            pair[0].kind == TokenKind::Ident
                && pair[0].text == name
                && pair[1].kind == TokenKind::OpenParen
        })
    };
    PHYSICAL.iter().any(|selector| called(selector)) && called(STRING_PROTOCOL)
}

/// The index just past the bracket that closes the one opening at `open`.
fn closing(tokens: &[Tok<'_>], open: usize) -> usize {
    let mut depth = 0usize;
    for (position, token) in tokens.iter().enumerate().skip(open) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace => depth += 1,
            TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseBrace => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return position + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// The end of a `let` binding's initialiser starting at `start`: its `;`, its
/// `let … else`, or the block an `if let` / `while let` opens.
fn initialiser_end(tokens: &[Tok<'_>], start: usize, conditional: bool) -> usize {
    let mut position = start;
    while let Some(token) = tokens.get(position) {
        match token.kind {
            TokenKind::OpenBrace if conditional => return position,
            TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace => {
                position = closing(tokens, position);
            }
            TokenKind::Semi | TokenKind::CloseBrace => return position,
            TokenKind::Ident if token.text == "else" => return position,
            _ => position += 1,
        }
    }
    position
}

/// The end of the item a `#[cfg(test)]` guards, from `start`, and whether it
/// is a block: a bodiless item (`mod name;`) or a field (`name: value,`) has
/// none.
fn guarded_item_end(tokens: &[Tok<'_>], start: usize) -> (usize, bool) {
    let mut position = start;
    while let Some(token) = tokens.get(position) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::OpenBracket => position = closing(tokens, position),
            TokenKind::OpenBrace => return (position, true),
            TokenKind::Semi | TokenKind::Comma | TokenKind::CloseBrace | TokenKind::CloseParen => {
                return (position, false);
            }
            _ => position += 1,
        }
    }
    (position, false)
}

/// The end of a signature from `start`: its body's `{` or its `;`, past an
/// array type's own `;`.
fn signature_end(tokens: &[Tok<'_>], start: usize) -> usize {
    let mut position = start;
    while let Some(token) = tokens.get(position) {
        match token.kind {
            TokenKind::OpenParen | TokenKind::OpenBracket => position = closing(tokens, position),
            TokenKind::OpenBrace | TokenKind::Semi => break,
            _ => position += 1,
        }
    }
    position
}

/// The names a `let` at `index` binds and whether its initialiser reads a
/// physical protocol.
fn let_binding(tokens: &[Tok<'_>], index: usize, conditional: bool) -> (Vec<String>, bool) {
    let equals = tokens[index..]
        .iter()
        .position(|next| next.kind == TokenKind::Eq)
        .map_or(tokens.len(), |offset| index + offset);
    let end = initialiser_end(tokens, equals + 1, conditional);
    let names = tokens[index + 1..equals.min(tokens.len())]
        .iter()
        .filter(|next| next.kind == TokenKind::Ident)
        .map(|next| next.text.to_owned())
        .collect();
    (
        names,
        equals < end && reads_physical_protocol(&tokens[equals + 1..end]),
    )
}

/// The production functions, by their outermost `fn`, in which a list or
/// dictionary sink receives a physical string protocol, directly or through
/// a `let` binding still in scope.
fn physical_recipe_functions(text: &str) -> Vec<String> {
    let tokens = tokens(text);
    let mut out: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut functions: Vec<(String, usize)> = Vec::new();
    let mut pending: Option<String> = None;
    let mut bindings: Vec<Binding> = Vec::new();
    let mut scoped: Option<Binding> = None;
    let mut skip_from: Option<usize> = None;
    let mut index = 0;
    while index < tokens.len() {
        if skip_from.is_none()
            && let Some(length) = cfg_test_len(&tokens, index)
        {
            let (end, block) = guarded_item_end(&tokens, index + length);
            if block {
                skip_from = Some(depth);
            }
            index = end;
            continue;
        }
        let token = &tokens[index];
        match token.kind {
            TokenKind::OpenBrace => {
                depth += 1;
                if let Some(name) = pending.take() {
                    functions.push((name, depth));
                }
                if let Some(mut binding) = scoped.take() {
                    binding.depth = depth;
                    bindings.push(binding);
                }
            }
            TokenKind::CloseBrace => {
                bindings.retain(|binding| binding.depth < depth);
                if functions.last().is_some_and(|(_, body)| *body == depth) {
                    functions.pop();
                }
                depth = depth.saturating_sub(1);
                if skip_from == Some(depth) {
                    skip_from = None;
                }
            }
            TokenKind::Semi => pending = None,
            TokenKind::Ident if token.text == "fn" => {
                pending = tokens
                    .get(index + 1)
                    .filter(|next| next.kind == TokenKind::Ident)
                    .map(|next| next.text.to_owned());
                if pending.is_some() {
                    index = signature_end(&tokens, index + 2);
                    continue;
                }
            }
            TokenKind::Ident if token.text == "let" && skip_from.is_none() => {
                let conditional = index > 0 && matches!(tokens[index - 1].text, "if" | "while");
                let (names, physical) = let_binding(&tokens, index, conditional);
                if physical {
                    if conditional {
                        scoped = Some(Binding { names, depth: 0 });
                    } else {
                        bindings.push(Binding { names, depth });
                    }
                } else if !conditional {
                    // A later `let` in the same block shadows the name.
                    for binding in bindings.iter_mut().filter(|binding| binding.depth == depth) {
                        binding.names.retain(|name| !names.contains(name));
                    }
                }
            }
            TokenKind::Ident
                if skip_from.is_none()
                    && SINKS.contains(&token.text)
                    && tokens
                        .get(index + 1)
                        .is_some_and(|next| next.kind == TokenKind::OpenParen) =>
            {
                let arguments = &tokens[index + 1..closing(&tokens, index + 1)];
                let tainted = reads_physical_protocol(arguments)
                    || arguments.iter().any(|argument| {
                        argument.kind == TokenKind::Ident
                            && bindings.iter().any(|binding| {
                                binding.names.iter().any(|name| name == argument.text)
                            })
                    });
                if tainted
                    && let Some((name, _)) = functions.first()
                    && !out.contains(name)
                {
                    out.push(name.clone());
                }
            }
            _ => {}
        }
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::physical_recipe_functions;

    #[test]
    fn a_physical_protocol_beside_a_sink_is_flagged_and_a_selected_one_is_not() {
        let source = r"
            fn physical(vm: &Vm, items: Vec<Value>) -> Value {
                let protocol = vm.actual_native_invocation_dialect().native_string_protocol().unwrap();
                Value::native_list_constructor(items, protocol)
            }
            fn selected(vm: &Vm, items: Vec<Value>) -> Value {
                Value::native_list_constructor(items, vm.object_protocol().unwrap())
            }
            fn other(vm: &Vm) -> bool {
                vm.actual_native_invocation_dialect().native_string_protocol().is_some()
            }
            #[cfg(test)]
            mod tests {
                fn ignored(vm: &Vm, items: Vec<Value>) -> Value {
                    let p = vm.actual_native_invocation_dialect().native_string_protocol().unwrap();
                    Value::native_list_constructor(items, p)
                }
            }
        ";
        assert_eq!(
            physical_recipe_functions(source),
            vec!["physical".to_owned()]
        );
    }

    #[test]
    fn a_binding_taints_its_scope_only() {
        let source = r"
            fn outer(vm: &Vm, original: &Value) -> Result<Value, E> {
                let Some(protocol) = vm.native_scalar_carrier_dialect().native_string_protocol() else {
                    return Err(E);
                };
                let copy = || original.native_list_copy(protocol);
                copy()
            }
            fn scoped(vm: &Vm, items: Vec<Value>) -> Value {
                if let Some(protocol) = vm.actual_native_invocation_dialect().native_string_protocol() {
                    let _ = protocol;
                }
                let protocol = vm.object_protocol().unwrap();
                Value::native_list_constructor(items, protocol)
            }
            fn shadowed(vm: &Vm, items: Vec<Value>) -> Value {
                let protocol = vm.actual_native_invocation_dialect().native_string_protocol();
                let protocol = vm.object_protocol().unwrap();
                Value::native_list_constructor(items, protocol)
            }
            fn signature(bytes: [u8; 4]) -> usize { bytes.len() }
            fn fields() -> State {
                State {
                    #[cfg(test)]
                    probe: 0,
                    other: 1,
                }
            }
            fn after(vm: &Vm, items: Vec<Value>) -> Value {
                let p = vm.actual_native_invocation_dialect().native_string_protocol().unwrap();
                Value::native_list_constructor(items, p)
            }
        ";
        assert_eq!(
            physical_recipe_functions(source),
            vec!["outer".to_owned(), "after".to_owned()]
        );
    }
}
