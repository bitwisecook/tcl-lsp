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

//! Source reachability from actual Cargo targets. A conditional or macro
//! generated edge is evidence to inspect, never proof the feature shipped.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use syn::visit::{self, Visit};

#[derive(Default)]
pub(super) struct Sources {
    entries: BTreeMap<String, Vec<String>>,
    roots: BTreeSet<String>,
    edges: BTreeMap<String, Vec<(String, &'static str)>>,
}

impl Sources {
    pub(super) fn from_targets(targets: &str) -> Self {
        let mut graph = Self::default();
        for row in targets.lines().skip(1) {
            let cols: Vec<_> = row.split('\t').collect();
            if cols.len() != 7 {
                continue;
            }
            let path = cols[4];
            graph.entries.entry(path.into()).or_default().push(format!(
                "{}::{} [{}] features {}",
                cols[1], cols[2], cols[3], cols[5]
            ));
            if cols[3]
                .split(',')
                .any(|kind| !matches!(kind, "test" | "bench" | "example"))
            {
                graph.roots.insert(path.into());
            }
        }
        graph
    }

    pub(super) fn add(&mut self, file: &str, tree: &syn::File) {
        let path = Path::new(file);
        let parent = path.parent().unwrap_or(Path::new(""));
        let module_dir = if self.entries.contains_key(file)
            || path.file_name().is_some_and(|name| name == "mod.rs")
        {
            parent.to_owned()
        } else {
            parent.join(path.file_stem().unwrap_or_default())
        };
        let mut scan = Links {
            file_dir: parent,
            module_dir,
            inline: false,
            out: Vec::new(),
        };
        scan.visit_file(tree);
        self.edges.insert(file.into(), scan.out);
    }

    fn reached(&self, tracked: &BTreeSet<&str>) -> BTreeSet<String> {
        let mut reached = self.roots.clone();
        let mut todo: Vec<_> = reached.iter().cloned().collect();
        while let Some(file) = todo.pop() {
            for (target, _) in self.edges.get(&file).into_iter().flatten() {
                if tracked.contains(target.as_str()) && reached.insert(target.clone()) {
                    todo.push(target.clone());
                }
            }
        }
        reached
    }

    pub(super) fn report(
        &self,
        tracked: &BTreeSet<&str>,
        fragments: &BTreeSet<String>,
        verbose: bool,
    ) {
        let reached = self.reached(tracked);
        let mut named = 0;
        for path in &reached {
            if path.split('/').any(|p| p == "tests")
                || crate::util::test_sources::is_test_module_file(path)
            {
                named += 1;
                if verbose {
                    println!(
                        "{path}\tpossible shipping source under test naming\tregistered Cargo or transitive module/include edge"
                    );
                }
            }
            if fragments.contains(path) {
                println!(
                    "{path}\tpossible shipping Rust fragment\tfull-file AST unavailable; inspect inclusion context"
                );
            }
        }
        if verbose {
            for (path, targets) in &self.entries {
                for target in targets {
                    println!("{path}\tregistered Cargo source\t{target}");
                }
            }
            for (issuer, destinations) in &self.edges {
                for (target, kind) in destinations {
                    if tracked.contains(target.as_str()) {
                        println!(
                            "{issuer}\t{kind}\t{target}\t{}",
                            if reached.contains(issuer) {
                                "from a possible shipping source"
                            } else {
                                "source edge; shipping reach unverified"
                            }
                        );
                    }
                }
            }
        }
        println!(
            "parser-ownership: {} possible shipping Rust sources from Cargo/module/literal include edges, {named} test-named; computed generation and macro expansion remain separate",
            reached.len()
        );
    }
}

struct Links<'a> {
    file_dir: &'a Path,
    module_dir: PathBuf,
    inline: bool,
    out: Vec<(String, &'static str)>,
}

fn lexical_path(path: &Path) -> Option<String> {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::ParentDir if normalized.pop() => {}
            _ => return None,
        }
    }
    Some(normalized.to_str()?.to_owned())
}

fn literal_path(meta: &syn::Meta) -> Option<String> {
    let syn::Meta::NameValue(nv) = meta else {
        return None;
    };
    if !nv.path.is_ident("path") {
        return None;
    }
    let syn::Expr::Lit(lit) = &nv.value else {
        return None;
    };
    let syn::Lit::Str(path) = &lit.lit else {
        return None;
    };
    Some(path.value())
}

impl Links<'_> {
    fn edge(&mut self, path: &Path, kind: &'static str) {
        if let Some(target) = lexical_path(path) {
            self.out.push((target, kind));
        }
    }
}

impl<'ast> Visit<'ast> for Links<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if module.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg")
                && attr.parse_args::<syn::Ident>().is_ok_and(|i| i == "test")
        }) {
            return;
        }
        let direct = module.attrs.iter().find_map(|a| literal_path(&a.meta));
        let mut selected = Vec::new();
        if let Some(ref path) = direct {
            selected.push(path.clone());
        }
        // cfg_attr(.., path = "...") adds a separate possible file.
        for attr in module
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg_attr"))
        {
            if let Ok(parts) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) {
                selected.extend(parts.iter().skip(1).filter_map(literal_path));
            }
        }
        let name = module.ident.to_string();
        let base = if self.inline {
            &self.module_dir
        } else {
            self.file_dir
        };
        let mut candidates: Vec<_> = selected.iter().map(|p| base.join(p)).collect();
        if let Some((_, contents)) = &module.content {
            if direct.is_none() {
                candidates.push(self.module_dir.join(&name));
            }
            for dir in candidates {
                let previous = std::mem::replace(&mut self.module_dir, dir);
                let old = std::mem::replace(&mut self.inline, true);
                for item in contents {
                    self.visit_item(item);
                }
                self.inline = old;
                self.module_dir = previous;
            }
        } else {
            for path in candidates {
                self.edge(&path, "module path attribute");
            }
            if direct.is_none() {
                self.edge(&self.module_dir.join(format!("{name}.rs")), "Rust module");
                self.edge(&self.module_dir.join(name).join("mod.rs"), "Rust module");
            }
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("include")
            && let Ok(path) = syn::parse2::<syn::LitStr>(mac.tokens.clone())
        {
            self.edge(&self.file_dir.join(path.value()), "Rust source include");
        }
        visit::visit_macro(self, mac);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipping_targets_include_test_named_modules_and_literal_inclusions() {
        let targets = "manifest\tpackage\ttarget\tkind\tentry\tfeatures\tall\nCargo.toml\tp\tlive\tbin\tp/tests/start.rs\t\t\nCargo.toml\tp\ttests\ttest\tp/tests/client.rs\t\t";
        let mut sources = Sources::from_targets(targets);
        sources.add("p/tests/start.rs", &syn::parse_file(r#"#[path = "../src/included_tests.rs"] mod rules; #[cfg(feature="adapter")] mod generated; #[cfg(test)] mod fixture;"#).unwrap());
        sources.add(
            "p/src/included_tests.rs",
            &syn::parse_file(r#"include!("../tests/hidden.rs");"#).unwrap(),
        );
        let paths = [
            "p/tests/start.rs",
            "p/tests/client.rs",
            "p/src/included_tests.rs",
            "p/tests/hidden.rs",
            "p/tests/generated.rs",
            "p/tests/fixture.rs",
        ]
        .into_iter()
        .collect();
        let reached = sources.reached(&paths);
        assert!(reached.contains("p/tests/hidden.rs"));
        assert!(reached.contains("p/tests/generated.rs"));
        assert!(!reached.contains("p/tests/client.rs"));
        assert!(!reached.contains("p/tests/fixture.rs"));
    }
}
