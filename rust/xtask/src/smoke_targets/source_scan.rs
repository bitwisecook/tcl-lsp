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

//! The scan of target sources for smoke tests: module trees, include macros, smoke markers and the source inventory.

use super::manifest::{best_owners, is_smoke_named_target, target_identity};
use super::{
    LexedToken, ManifestRow, SmokeInventory, SmokeMarkers, SourceIncludes, Target, TargetIdentity,
    command_output,
};
use anyhow::{Context, Result, bail};
use rustc_lexer::{LiteralKind, TokenKind};
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::ext::IdentExt;

fn tracked_rust_sources(root: &Path) -> Result<Vec<String>> {
    let mut command = Command::new("git");
    command
        .args(["ls-files", "--", ":(glob)rust/**/*.rs"])
        .current_dir(root);
    let output = command_output(&mut command)?;
    Ok(String::from_utf8(output.stdout)
        .context("git ls-files output is not UTF-8")?
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

fn meta_applies_test_attribute(meta: &syn::Meta) -> bool {
    if meta
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .is_ok_and(|nested| nested.iter().skip(1).any(meta_applies_test_attribute))
}

fn has_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| meta_applies_test_attribute(&attribute.meta))
}

fn module_directory(source: &Path) -> Result<PathBuf> {
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let stem = source
        .file_stem()
        .and_then(OsStr::to_str)
        .with_context(|| format!("Rust source has no UTF-8 stem: {}", source.display()))?;
    if matches!(stem, "lib" | "main" | "mod") {
        Ok(parent.to_path_buf())
    } else {
        Ok(parent.join(stem))
    }
}

pub(super) fn lexically_normalise(path: &Path) -> PathBuf {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalised.pop() {
                    normalised.push(component.as_os_str());
                }
            }
            _ => normalised.push(component.as_os_str()),
        }
    }
    normalised
}

fn collect_module_paths(meta: &syn::Meta, paths: &mut Vec<String>) {
    if meta.path().is_ident("path") {
        if let syn::Meta::NameValue(value) = meta
            && let syn::Expr::Lit(value) = &value.value
            && let syn::Lit::Str(value) = &value.lit
        {
            paths.push(value.value());
        }
        return;
    }
    let syn::Meta::List(list) = meta else {
        return;
    };
    if !list.path.is_ident("cfg_attr") {
        return;
    }
    if let Ok(nested) = list
        .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
    {
        for attribute in nested.iter().skip(1) {
            collect_module_paths(attribute, paths);
        }
    }
}

fn module_sources(module: &syn::ItemMod, directory: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut configured = Vec::new();
    for attribute in &module.attrs {
        collect_module_paths(&attribute.meta, &mut configured);
    }
    let has_direct_path = module
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("path"));
    let mut sources: Vec<(PathBuf, PathBuf)> = configured
        .into_iter()
        .map(|path| {
            let source = lexically_normalise(&directory.join(path));
            let child_directory = source
                .parent()
                .map_or_else(|| directory.to_path_buf(), Path::to_path_buf);
            (source, child_directory)
        })
        .filter(|(path, _)| path.is_file())
        .collect();
    if !has_direct_path {
        let name = module.ident.unraw().to_string();
        sources.extend(
            [
                directory.join(format!("{name}.rs")),
                directory.join(&name).join("mod.rs"),
            ]
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| (path, directory.join(&name))),
        );
    }
    sources.sort();
    sources.dedup();
    sources
}

fn inline_module_directories(module: &syn::ItemMod, directory: &Path) -> Vec<PathBuf> {
    let mut configured = Vec::new();
    for attribute in &module.attrs {
        collect_module_paths(&attribute.meta, &mut configured);
    }
    let has_direct_path = module
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("path"));
    let mut directories: Vec<PathBuf> = configured
        .into_iter()
        .map(|path| lexically_normalise(&directory.join(path)))
        .collect();
    if !has_direct_path {
        directories.push(directory.join(module.ident.unraw().to_string()));
    }
    directories.sort();
    directories.dedup();
    directories
}

fn is_builtin_include_macro(path: &syn::Path) -> bool {
    let mut segments = path.segments.iter();
    let Some(first) = segments.next() else {
        return false;
    };
    let Some(second) = segments.next() else {
        return first.ident.unraw() == "include";
    };
    let namespace = first.ident.unraw();
    segments.next().is_none()
        && (namespace == "std" || namespace == "core")
        && second.ident.unraw() == "include"
}

fn lexed_ident_is(token: LexedToken, text: &str, expected: &str) -> bool {
    matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent)
        && text[token.start..token.end]
            .strip_prefix("r#")
            .unwrap_or(&text[token.start..token.end])
            == expected
}

fn is_builtin_include_invocation(tokens: &[LexedToken], text: &str, index: usize) -> bool {
    let Some(token) = tokens.get(index) else {
        return false;
    };
    if !lexed_ident_is(*token, text, "include")
        || tokens
            .get(index + 1)
            .is_none_or(|next| next.kind != TokenKind::Not)
    {
        return false;
    }
    if index >= 2
        && tokens[index - 1].kind == TokenKind::Colon
        && tokens[index - 2].kind == TokenKind::Colon
    {
        let namespace = index - 3;
        return tokens.get(namespace).is_some_and(|token| {
            (lexed_ident_is(*token, text, "std") || lexed_ident_is(*token, text, "core"))
                && if namespace >= 2
                    && tokens[namespace - 1].kind == TokenKind::Colon
                    && tokens[namespace - 2].kind == TokenKind::Colon
                {
                    namespace.checked_sub(3).is_none_or(|prefix| {
                        !matches!(
                            tokens[prefix].kind,
                            TokenKind::Ident | TokenKind::RawIdent | TokenKind::Dollar
                        )
                    })
                } else {
                    namespace.checked_sub(1).is_none_or(|prefix| {
                        !matches!(
                            tokens[prefix].kind,
                            TokenKind::Ident
                                | TokenKind::RawIdent
                                | TokenKind::Dollar
                                | TokenKind::Dot
                                | TokenKind::Colon
                        )
                    })
                }
        });
    }
    index
        .checked_sub(1)
        .is_none_or(|previous| tokens[previous].kind != TokenKind::Colon)
}

pub(super) fn collect_smoke_test_sources(
    source: &Path,
    directory: &Path,
    items: &[syn::Item],
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                if has_test_attribute(&function.attrs)
                    && (inside_smoke_module
                        || function.sig.ident.unraw().to_string().starts_with("smoke"))
                {
                    found.insert(source.to_path_buf());
                }
            }
            syn::Item::Mod(module) => {
                let module_is_smoke =
                    inside_smoke_module || module.ident.unraw().to_string().starts_with("smoke");
                if let Some((_, items)) = &module.content {
                    for child_directory in inline_module_directories(module, directory) {
                        collect_smoke_test_sources(
                            source,
                            &child_directory,
                            items,
                            module_is_smoke,
                            visited,
                            found,
                        )?;
                    }
                } else {
                    for (child, child_directory) in module_sources(module, directory) {
                        collect_source_smoke_tests_at(
                            &child,
                            &child_directory,
                            module_is_smoke,
                            visited,
                            found,
                        )?;
                    }
                }
            }
            syn::Item::Macro(item_macro) if is_builtin_include_macro(&item_macro.mac.path) => {
                if let Ok(literal) = syn::parse2::<syn::LitStr>(item_macro.mac.tokens.clone()) {
                    collect_literal_include(
                        source,
                        &literal.value(),
                        inside_smoke_module,
                        visited,
                        found,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_literal_include(
    source: &Path,
    path: &str,
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let included_path = lexically_normalise(&parent.join(path));
    if !included_path.is_file() {
        return Ok(());
    }
    let included_text = fs::read_to_string(&included_path)
        .with_context(|| format!("reading Rust include {}", included_path.display()))?;
    if syn::parse_file(&included_text).is_err() {
        // An expression- or statement-only include cannot add a top-level
        // libtest item.
        return Ok(());
    }
    let child_directory = included_path
        .parent()
        .map_or_else(|| parent.to_path_buf(), Path::to_path_buf);
    collect_source_smoke_tests_at(
        &included_path,
        &child_directory,
        inside_smoke_module,
        visited,
        found,
    )
}

pub(super) fn collect_source_smoke_tests(
    source: &Path,
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    collect_source_smoke_tests_at(
        source,
        &module_directory(source)?,
        inside_smoke_module,
        visited,
        found,
    )
}

fn collect_source_smoke_tests_at(
    source: &Path,
    directory: &Path,
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if !visited.insert((
        source.to_path_buf(),
        directory.to_path_buf(),
        inside_smoke_module,
    )) {
        return Ok(());
    }
    let text = fs::read_to_string(source)
        .with_context(|| format!("reading Rust module {}", source.display()))?;
    let parsed = syn::parse_file(&text)
        .with_context(|| format!("parsing Rust module {}", source.display()))?;
    let (markers, _source_includes) = source_smoke_contract(source, &text)
        .with_context(|| format!("checking smoke markers in {}", source.display()))?;
    if markers.target {
        found.insert(source.to_path_buf());
    }
    collect_smoke_test_sources(
        source,
        directory,
        &parsed.items,
        inside_smoke_module,
        visited,
        found,
    )?;
    // Direct includes are handled by the AST walk above. Opaque macro-body
    // includes are intentionally not traversed: a macro can be defined here
    // and invoked from a different namespace.
    Ok(())
}

pub(super) fn source_smoke_markers(text: &str) -> Result<SmokeMarkers> {
    let mut offset = 0;
    let mut line_start = 0;
    let mut line_number = 1;
    let mut markers = SmokeMarkers::default();
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        let lexeme = &text[offset..end];
        if token.kind == TokenKind::LineComment {
            let body = lexeme
                .strip_prefix("//")
                .context("Rust lexer returned a malformed line comment")?
                .trim();
            let standalone = text[line_start..offset]
                .chars()
                .all(|character| matches!(character, ' ' | '\t'));
            if body == "tcl-lsp-smoke-target" && standalone {
                markers.target = true;
            } else if body == "tcl-lsp-no-smoke-include" && standalone {
                markers.no_smoke_include = true;
            } else if body.starts_with("tcl-lsp-") && body.contains("smoke") {
                bail!(
                    "line {line_number}: invalid smoke marker; expected exactly '// tcl-lsp-smoke-target' or '// tcl-lsp-no-smoke-include' on a standalone line"
                );
            }
        }
        line_number += lexeme.bytes().filter(|byte| *byte == b'\n').count();
        if let Some(last_newline) = lexeme.rfind('\n') {
            line_start = offset + last_newline + 1;
        }
        offset = end;
    }
    if markers.target && markers.no_smoke_include {
        bail!("source has contradictory smoke-target and no-smoke-include markers");
    }
    Ok(markers)
}

fn delimiter_close(kind: TokenKind) -> Option<TokenKind> {
    match kind {
        TokenKind::OpenParen => Some(TokenKind::CloseParen),
        TokenKind::OpenBrace => Some(TokenKind::CloseBrace),
        TokenKind::OpenBracket => Some(TokenKind::CloseBracket),
        _ => None,
    }
}

pub(super) fn source_include_macros(text: &str) -> Result<SourceIncludes> {
    let mut offset = 0;
    let mut tokens = Vec::new();
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        if !matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
        ) {
            tokens.push(LexedToken {
                kind: token.kind,
                start: offset,
                end,
            });
        }
        offset = end;
    }

    let mut includes = SourceIncludes::default();
    for (index, _token) in tokens.iter().enumerate() {
        if !is_builtin_include_invocation(&tokens, text, index)
            || index
                .checked_sub(1)
                .is_some_and(|previous| matches!(tokens[previous].kind, TokenKind::Dollar))
        {
            continue;
        }
        let Some(open) = tokens.get(index + 2) else {
            includes.non_literal_count += 1;
            continue;
        };
        let Some(close) = delimiter_close(open.kind) else {
            includes.non_literal_count += 1;
            continue;
        };
        let mut closes = vec![close];
        let mut close_index = None;
        for (candidate_index, candidate) in tokens.iter().enumerate().skip(index + 3) {
            if let Some(expected) = delimiter_close(candidate.kind) {
                closes.push(expected);
            } else if matches!(
                candidate.kind,
                TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket
            ) {
                let Some(expected) = closes.pop() else {
                    bail!("unbalanced delimiter while scanning include! invocation");
                };
                if candidate.kind != expected {
                    bail!("mismatched delimiter while scanning include! invocation");
                }
                if closes.is_empty() {
                    close_index = Some(candidate_index);
                    break;
                }
            }
        }
        let close_index = close_index.context("unterminated include! invocation")?;
        let body = &tokens[index + 3..close_index];
        let literal = body.first().filter(|_| body.len() == 1).and_then(|body| {
            if matches!(
                body.kind,
                TokenKind::Literal {
                    kind: LiteralKind::Str { .. } | LiteralKind::RawStr { .. },
                    ..
                }
            ) {
                syn::parse_str::<syn::LitStr>(&text[body.start..body.end]).ok()
            } else {
                None
            }
        });
        if let Some(literal) = literal {
            includes.literal_paths.push(literal.value());
        } else {
            includes.non_literal_count += 1;
        }
    }
    Ok(includes)
}

fn count_opaque_literal_includes(source: &Path, items: &[syn::Item]) -> Result<usize> {
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let mut count = 0;
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    count += count_opaque_literal_includes(source, items)?;
                }
            }
            syn::Item::Macro(item_macro) if !is_builtin_include_macro(&item_macro.mac.path) => {
                count += source_include_macros(&item_macro.mac.tokens.to_string())?
                    .literal_paths
                    .into_iter()
                    .filter(|path| lexically_normalise(&parent.join(path)).is_file())
                    .count();
            }
            _ => {}
        }
    }
    Ok(count)
}

fn source_smoke_contract(source: &Path, text: &str) -> Result<(SmokeMarkers, SourceIncludes)> {
    let markers = source_smoke_markers(text)?;
    let mut includes = source_include_macros(text)?;
    if let Ok(parsed) = syn::parse_file(text) {
        includes.ambiguous_literal_count = count_opaque_literal_includes(source, &parsed.items)?;
    }
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let missing_literal_count = includes
        .literal_paths
        .iter()
        .filter(|path| !lexically_normalise(&parent.join(path)).is_file())
        .count();
    let unresolved_count =
        includes.non_literal_count + missing_literal_count + includes.ambiguous_literal_count;
    if markers.no_smoke_include && unresolved_count != 1 {
        bail!(
            "a no-smoke-include marker must classify exactly one unresolved include! invocation; found {unresolved_count}"
        );
    }
    if unresolved_count > 0 && !markers.target && !markers.no_smoke_include {
        bail!(
            "{unresolved_count} unresolved include! invocation(s) require an exact standalone '// tcl-lsp-smoke-target' or '// tcl-lsp-no-smoke-include' comment"
        );
    }
    Ok((markers, includes))
}

fn relative_source(root: &Path, source: &Path) -> Result<String> {
    Ok(source
        .strip_prefix(root)
        .with_context(|| {
            format!(
                "discovered smoke source is outside workspace: {}",
                source.display()
            )
        })?
        .to_string_lossy()
        .replace('\\', "/"))
}

pub(super) fn record_target_owner(
    inventory: &mut SmokeInventory,
    relative: String,
    target: &Target,
) {
    inventory
        .owners
        .entry(relative)
        .or_default()
        .insert(target_identity(target));
}

pub(super) fn record_target_source(
    inventory: &mut SmokeInventory,
    relative: String,
    target: &Target,
) {
    inventory.sources.insert(relative.clone());
    record_target_owner(inventory, relative, target);
}

pub(super) fn scan_target_smoke_sources(
    root: &Path,
    target: &Target,
    inventory: &mut SmokeInventory,
) -> Result<()> {
    let smoke_named = is_smoke_named_target(target);
    if smoke_named {
        record_target_source(inventory, relative_source(root, &target.source)?, target);
    }
    let target_directory = target.source.parent().with_context(|| {
        format!(
            "Cargo target source has no parent: {}",
            target.source.display()
        )
    })?;
    let mut visited = HashSet::new();
    let mut found = BTreeSet::new();
    collect_source_smoke_tests_at(
        &target.source,
        target_directory,
        smoke_named,
        &mut visited,
        &mut found,
    )?;
    for (source, _, _) in visited {
        if source.starts_with(root) {
            record_target_owner(inventory, relative_source(root, &source)?, target);
        }
    }
    for source in found {
        record_target_source(inventory, relative_source(root, &source)?, target);
    }
    Ok(())
}

pub(super) fn scan_smoke_sources(root: &Path, targets: &[Target]) -> Result<SmokeInventory> {
    let mut inventory = SmokeInventory::default();
    for relative in tracked_rust_sources(root)? {
        let source = root.join(&relative);
        let text = fs::read_to_string(&source).with_context(|| format!("reading {relative}"))?;
        if source_smoke_contract(&source, &text)
            .with_context(|| format!("checking {relative}"))?
            .0
            .target
        {
            inventory.sources.insert(relative);
        }
    }
    for target in targets.iter().filter(|target| target.testable) {
        scan_target_smoke_sources(root, target, &mut inventory)?;
    }
    Ok(inventory)
}

pub(super) fn inventory_entries(
    root: &Path,
    inventory: &SmokeInventory,
    targets: &[Target],
) -> BTreeSet<(String, TargetIdentity)> {
    inventory
        .sources
        .iter()
        .flat_map(|source| {
            let owners: Vec<TargetIdentity> = inventory.owners.get(source).map_or_else(
                || {
                    best_owners(&root.join(source), targets)
                        .into_iter()
                        .map(target_identity)
                        .collect()
                },
                |owners| owners.iter().cloned().collect(),
            );
            owners.into_iter().map(|owner| (source.clone(), owner))
        })
        .collect()
}

fn inventory_entry_text(entry: &(String, TargetIdentity)) -> String {
    let (source, (package, kind, target)) = entry;
    format!("{source} [{package} {kind}:{target}]")
}

pub(super) fn check_source_inventory(
    root: &Path,
    rows: &[ManifestRow],
    inventory: &SmokeInventory,
    targets: &[Target],
) -> Result<()> {
    let expected_sources: BTreeSet<String> =
        rows.iter().map(|row| row.source_text.clone()).collect();
    let expected_entries: BTreeSet<(String, TargetIdentity)> = rows
        .iter()
        .map(|row| {
            (
                row.source_text.clone(),
                (
                    row.package.clone(),
                    row.kind.clone(),
                    row.target_name.clone(),
                ),
            )
        })
        .collect();
    let actual_entries = inventory_entries(root, inventory, targets);
    if expected_sources == inventory.sources && expected_entries == actual_entries {
        return Ok(());
    }
    let missing_sources: Vec<_> = inventory
        .sources
        .difference(&expected_sources)
        .cloned()
        .collect();
    let stale_sources: Vec<_> = expected_sources
        .difference(&inventory.sources)
        .cloned()
        .collect();
    let missing_entries: Vec<_> = actual_entries
        .difference(&expected_entries)
        .map(inventory_entry_text)
        .collect();
    let stale_entries: Vec<_> = expected_entries
        .difference(&actual_entries)
        .map(inventory_entry_text)
        .collect();
    bail!(
        "smoke-targets.tsv inventory drift; missing sources: [{}]; stale sources: [{}]; missing target rows: [{}]; stale target rows: [{}]",
        missing_sources.join(", "),
        stale_sources.join(", "),
        missing_entries.join(", "),
        stale_entries.join(", ")
    )
}
