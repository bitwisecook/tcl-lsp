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

//! Syntactic expansion boundaries. An invocation can have multiple definition
//! candidates; expansion, feature selection, and build output are separate
//! evidence from the source inventory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use syn::visit::{self, Visit};

#[derive(Default)]
pub(super) struct Expansions {
    scope: Vec<String>,
    rows: Vec<(String, &'static str, String)>,
}

impl Expansions {
    fn add(&mut self, kind: &'static str, target: &str) {
        self.rows.push((
            self.scope.join("::"),
            kind,
            target.replace(['\t', '\n', '\r'], " "),
        ));
    }
}

/// Macro input is a token tree until the compiler expands it. Report scanner
/// operations found there as candidates even when a metavariable prevents
/// parsing its containing expression as a Rust AST.
#[derive(Default)]
struct MacroFacts {
    traversal: Vec<String>,
    delimiter: Vec<String>,
    entry: Vec<String>,
}

impl MacroFacts {
    fn scan(&mut self, mut input: syn::buffer::Cursor<'_>) {
        while !input.eof() {
            if let Some((inside, _, _, next)) = input.any_group() {
                self.scan(inside);
                input = next;
            } else if let Some((ident, next)) = input.ident() {
                let ident = ident.to_string();
                match ident.as_str() {
                    "for" | "while" | "loop" | "iter" | "position" | "find" | "split" | "chars"
                    | "bytes" | "char_indices" | "as_bytes" => self.traversal.push(ident),
                    "split_list"
                    | "join_list"
                    | "group_commands"
                    | "parse_expr"
                    | "split_whitespace"
                    | "split_ascii_whitespace"
                    | "LexerConfig" => self.entry.push(ident),
                    _ => {}
                }
                input = next;
            } else if let Some((literal, next)) = input.literal() {
                match syn::Lit::new(literal) {
                    syn::Lit::Byte(b)
                        if b.value().is_ascii_punctuation() || b.value().is_ascii_whitespace() =>
                    {
                        self.delimiter.push(format!("byte {:#04x}", b.value()));
                    }
                    syn::Lit::Char(c)
                        if c.value().is_ascii_punctuation() || c.value().is_ascii_whitespace() =>
                    {
                        self.delimiter.push(format!("char {:?}", c.value()));
                    }
                    _ => {}
                }
                input = next;
            } else if let Some((_, next)) = input.token_tree() {
                input = next;
            } else {
                break;
            }
        }
    }

    fn structural(mut self) -> Vec<String> {
        if !self.traversal.is_empty() && !self.delimiter.is_empty() {
            self.entry.append(&mut self.traversal);
            self.entry.append(&mut self.delimiter);
        }
        self.entry.sort();
        self.entry
    }
}

impl<'ast> Visit<'ast> for Expansions {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.scope.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scope.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let name = item.sig.ident.to_string();
        self.scope.push(name.clone());
        if item
            .attrs
            .iter()
            .any(|a| a.path().is_ident("proc_macro") || a.path().is_ident("proc_macro_attribute"))
        {
            self.add("proc-macro definition", &name);
        }
        for attr in &item.attrs {
            if attr.path().is_ident("proc_macro_derive")
                && let syn::Meta::List(list) = &attr.meta
            {
                self.add(
                    "proc-macro definition",
                    list.tokens
                        .to_string()
                        .split(',')
                        .next()
                        .unwrap_or_default()
                        .trim(),
                );
            }
        }
        visit::visit_item_fn(self, item);
        self.scope.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.scope.push(item.sig.ident.to_string());
        visit::visit_impl_item_fn(self, item);
        self.scope.pop();
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.mac.path.is_ident("macro_rules")
            && let Some(name) = &item.ident
        {
            self.add("macro_rules definition", &name.to_string());
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if let syn::Meta::List(list) = &attr.meta {
            if attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr") {
                self.add("conditional compilation", &list.tokens.to_string());
            } else if attr.path().is_ident("derive") {
                self.add("derive expansion", &list.tokens.to_string());
            }
        }
        if attr.path().is_ident("path")
            && let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(lit) = &nv.value
            && let syn::Lit::Str(value) = &lit.lit
        {
            self.add("path module", &value.value());
        }
        let path = super::path_text(attr.path());
        if !matches!(
            path.as_str(),
            "cfg"
                | "cfg_attr"
                | "derive"
                | "path"
                | "doc"
                | "allow"
                | "expect"
                | "warn"
                | "deny"
                | "forbid"
                | "test"
                | "ignore"
                | "should_panic"
                | "inline"
                | "cold"
                | "must_use"
                | "deprecated"
                | "repr"
                | "non_exhaustive"
                | "proc_macro"
                | "proc_macro_attribute"
                | "proc_macro_derive"
        ) {
            self.add("attribute expansion candidate", &path);
        }
        visit::visit_attribute(self, attr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = super::path_text(&mac.path);
        let mut facts = MacroFacts::default();
        facts.scan(syn::buffer::TokenBuffer::new2(mac.tokens.clone()).begin());
        for operation in facts.structural() {
            self.add("macro token operation", &format!("{name}: {operation}"));
        }
        if matches!(name.as_str(), "include" | "include_bytes" | "include_str") {
            if let Ok(path) = syn::parse2::<syn::LitStr>(mac.tokens.clone()) {
                self.add("literal include", &path.value());
            } else {
                self.add("computed include", &mac.tokens.to_string());
            }
        } else if name != "macro_rules" {
            self.add("macro invocation", &name);
        }
        // Token trees may contain Rust, a literal or a separate domain language.
        // Its entire root remains visible even when Syn cannot expand it.
    }
}

#[derive(Default)]
pub(super) struct Inventory {
    definitions: BTreeMap<String, BTreeSet<String>>,
    rows: Vec<(String, String, &'static str, String)>,
}

impl Inventory {
    pub(super) fn add(&mut self, path: &str, tree: &syn::File) -> Vec<String> {
        let mut scanner = Expansions::default();
        scanner.visit_file(tree);
        let mut signed = Vec::new();
        for (scope, kind, target) in scanner.rows {
            if matches!(kind, "macro_rules definition" | "proc-macro definition") {
                self.definitions
                    .entry(target.clone())
                    .or_default()
                    .insert(path.into());
            }
            if kind != "macro invocation"
                || !matches!(
                    target.as_str(),
                    "println" | "eprintln" | "format" | "write" | "writeln"
                )
            {
                let entry = if scope.is_empty() { "<module>" } else { &scope };
                signed.push(format!("{entry} | {kind} {target}"));
            }
            self.rows.push((path.into(), scope, kind, target));
        }
        signed
    }

    pub(super) fn report(
        &self,
        root: &Path,
        tracked: &BTreeSet<&str>,
        targets: &str,
        verbose: bool,
    ) {
        let build_scripts: BTreeMap<_, _> = targets
            .lines()
            .skip(1)
            .filter_map(|row| {
                let fields: Vec<_> = row.split('\t').collect();
                (fields.len() == 7 && fields[3].split(',').any(|k| k == "custom-build")).then(
                    || {
                        (
                            Path::new(fields[0]).parent().unwrap_or(Path::new("")),
                            fields[4],
                        )
                    },
                )
            })
            .collect();
        let mut counts = BTreeMap::new();
        for (file, scope, kind, target) in &self.rows {
            *counts.entry(kind).or_insert(0) += 1;
            if !verbose {
                continue;
            }
            let input = Path::new(file);
            let owner = match *kind {
                "literal include" | "path module" => {
                    let path = input.parent().unwrap_or(Path::new("")).join(target);
                    match root
                        .join(&path)
                        .canonicalize()
                        .ok()
                        .and_then(|p| p.strip_prefix(root).ok().map(Path::to_owned))
                    {
                        Some(rel) if rel.to_str().is_some_and(|p| tracked.contains(p)) => {
                            format!("tracked input {}", rel.display())
                        }
                        _ => "untracked or generated input; inspect build".into(),
                    }
                }
                "computed include" => input
                    .ancestors()
                    .find_map(|p| build_scripts.get(p))
                    .map_or_else(
                        || "unresolved expansion or external build output".into(),
                        |path| {
                            format!("Cargo build entry {path}; generated bytes require inspection")
                        },
                    ),
                "macro invocation" | "attribute expansion candidate" => self
                    .definitions
                    .get(target.rsplit("::").next().unwrap_or(target))
                    .map_or_else(
                        || "builtin, imported or external macro; expansion unverified".into(),
                        |paths| {
                            format!(
                                "definition candidates {}; binding/expansion unverified",
                                paths.iter().cloned().collect::<Vec<_>>().join(", ")
                            )
                        },
                    ),
                "derive expansion" => "inspect selected derive implementation and output".into(),
                "conditional compilation" => "inspect target features and cfg selection".into(),
                _ => "inspect definition and generated consumer".into(),
            };
            println!("{file}::{scope}\t{kind}\t{target}\t{owner}");
        }
        for (kind, count) in counts {
            println!("parser-ownership: {count} {kind} boundaries");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_reader_and_generated_adapter_stay_visible_without_expansion() {
        let src = r#"macro_rules! transform { () => { fn hidden(b: &[u8]) { for &v in b { if v == b']' { break; } } } }; }
            #[cfg(feature="adapter")] mod component {
                transform!();
                include!(concat!(env!("OUT_DIR"), "/foreign.rs"));
                static SOURCE: &[u8] = include_bytes!("outer.grammar");
            }
            #[proc_macro_attribute] fn project() {}
            #[project] #[derive(Project)] struct Host;"#;
        let mut inventory = Inventory::default();
        inventory.add("new/host.rs", &syn::parse_file(src).unwrap());
        let known = |kind, needle: &str| {
            inventory
                .rows
                .iter()
                .any(|(_, _, actual, value)| *actual == kind && value.contains(needle))
        };
        assert!(known("macro_rules definition", "transform"));
        assert!(known("macro invocation", "transform"));
        assert!(known("macro token operation", "macro_rules: for"));
        assert!(known("macro token operation", "macro_rules: byte 0x5d"));
        assert!(known("computed include", "OUT_DIR") && known("literal include", "outer.grammar"));
        assert!(
            known("conditional compilation", "adapter") && known("derive expansion", "Project")
        );
        assert!(known("proc-macro definition", "project"));
        assert!(known("attribute expansion candidate", "project"));
    }
}
