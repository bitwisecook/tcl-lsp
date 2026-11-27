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

//! Inventory candidate tests of private implementation state for review.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use syn::parse::Parser;
use syn::visit::{self, Visit};

const LEDGER: &str = "docs/work/name-handling/ownership/test-cleanup.tsv";

#[derive(Default)]
struct Names {
    fields: BTreeMap<String, bool>,
    items: BTreeMap<String, bool>,
    methods: BTreeMap<String, bool>,
}

fn record(names: &mut BTreeMap<String, bool>, name: &syn::Ident, vis: &syn::Visibility) {
    *names.entry(name.to_string()).or_default() |= matches!(vis, syn::Visibility::Public(_));
}

fn test_gate(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && matches!(&attr.meta, syn::Meta::List(list) if list.tokens.to_string()
                .split(|c: char| !c.is_alphanumeric() && c != '-')
                .any(|word| matches!(word, "test" | "test-support")))
    })
}

impl<'ast> Visit<'ast> for Names {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !test_gate(&node.attrs) {
            record(&mut self.items, &node.ident, &node.vis);
            visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        record(&mut self.items, &node.ident, &node.vis);
        for field in &node.fields {
            if let Some(name) = &field.ident {
                record(&mut self.fields, name, &field.vis);
            }
        }
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        record(&mut self.items, &node.ident, &node.vis);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !test_gate(&node.attrs) {
            record(&mut self.items, &node.sig.ident, &node.vis);
        }
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        record(&mut self.items, &node.ident, &node.vis);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.trait_.is_none() && !test_gate(&node.attrs) {
            for item in &node.items {
                if let syn::ImplItem::Fn(method) = item {
                    record(&mut self.methods, &method.sig.ident, &method.vis);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    bodies: usize,
    fields: usize,
    items: usize,
    methods: usize,
}

impl Counts {
    fn total(self) -> usize {
        self.fields + self.items + self.methods
    }

    fn grows(self, old: Self) -> bool {
        self.bodies > old.bodies
            || self.fields > old.fields
            || self.items > old.items
            || self.methods > old.methods
    }
}

struct Survey<'a> {
    names: &'a Names,
    test_only: bool,
    function: bool,
    modules: Vec<String>,
    found: BTreeMap<String, Counts>,
    current: Counts,
}

fn private(names: &BTreeMap<String, bool>, ident: &syn::Ident) -> bool {
    names.get(&ident.to_string()) == Some(&false)
}

impl<'ast> Visit<'ast> for Survey<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let saved = self.test_only;
        self.test_only |= test_gate(&node.attrs);
        self.modules.push(node.ident.to_string());
        visit::visit_item_mod(self, node);
        self.modules.pop();
        self.test_only = saved;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let is_test = node.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|part| part.ident == "test")
        });
        if !(self.test_only || is_test || test_gate(&node.attrs)) {
            return;
        }
        let saved = self.function;
        let outer = self.current;
        self.current = Counts::default();
        self.function = true;
        visit::visit_block(self, &node.block);
        if self.current.total() != 0 {
            let row = self.found.entry(self.modules.join("::")).or_default();
            row.bodies += 1;
            row.fields += self.current.fields;
            row.items += self.current.items;
            row.methods += self.current.methods;
        }
        self.function = saved;
        self.current = outer;
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        if self.function
            && let syn::Member::Named(name) = &node.member
            && private(&self.names.fields, name)
        {
            self.current.fields += 1;
        }
        visit::visit_expr_field(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.function
            && node.path.segments.first().is_some_and(|part| {
                matches!(part.ident.to_string().as_str(), "crate" | "super" | "self")
            })
            && node
                .path
                .segments
                .iter()
                .skip(1)
                .any(|part| private(&self.names.items, &part.ident))
        {
            self.current.items += 1;
        }
        visit::visit_expr_path(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.function && private(&self.names.methods, &node.method) {
            self.current.methods += 1;
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // Parse arguments as expressions, so quoted Rust/Tcl is never scanned.
        // Pattern and statement macros require manual review.
        if let Ok(arguments) =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(node.tokens.clone())
        {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        }
    }
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("target" | "data" | "testdata")
            ) {
                rust_files(&path, files)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

type Ledger = BTreeMap<(String, String), Counts>;

fn inventory(root: &Path) -> Result<Ledger> {
    // Workspace and standalone crates, including the nested report hosts.
    fn find(directory: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
        if directory.join("Cargo.toml").is_file() {
            found.push(directory.to_path_buf());
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && !matches!(
                    entry.file_name().to_str(),
                    Some("target" | "src" | "tests" | "data" | "node_modules" | "dist")
                )
            {
                find(&entry.path(), found)?;
            }
        }
        Ok(())
    }
    let mut manifests = Vec::new();
    find(&root.join("rust"), &mut manifests)?;
    manifests.push(root.join("runtime/rust"));
    let mut ledger = Ledger::new();
    for directory in manifests {
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(directory.join("Cargo.toml"))?)?;
        let Some(krate) = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
        else {
            continue;
        };
        let mut files = Vec::new();
        rust_files(&directory.join("src"), &mut files)?;
        let mut parsed = Vec::new();
        let mut names = Names::default();
        for path in files {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            let source = fs::read_to_string(&path)?;
            if source.starts_with("// @generated") || source.starts_with("// Generated from") {
                continue;
            }
            let test_only = relative.contains("/tests/")
                || crate::util::test_sources::is_test_module_file(&relative);
            let file = match syn::parse_file(&source) {
                Ok(file) => file,
                Err(error) if !test_only && !source.contains("#[test]") => {
                    eprintln!("test-cleanup: source fragment omitted: {relative}: {error}");
                    continue;
                }
                Err(error) => return Err(error).with_context(|| relative),
            };
            if !test_only {
                names.visit_file(&file);
            }
            parsed.push((relative, test_only, file));
        }
        for (relative, test_only, file) in parsed {
            let mut survey = Survey {
                names: &names,
                test_only,
                function: false,
                modules: vec![relative],
                found: BTreeMap::new(),
                current: Counts::default(),
            };
            survey.visit_file(&file);
            ledger.extend(
                survey
                    .found
                    .into_iter()
                    .map(|(module, counts)| ((krate.to_owned(), module), counts)),
            );
        }
    }
    Ok(ledger)
}

fn render(rows: &Ledger) -> String {
    let mut text = "# Candidate private-state test bodies; review, not automatic deletion.\n# crate\tmodule\tbodies\tprivate-field-sites\tinternal-item-sites\tprivate-method-sites\n".to_owned();
    for ((krate, module), n) in rows {
        writeln!(
            text,
            "{krate}\t{module}\t{}\t{}\t{}\t{}",
            n.bodies, n.fields, n.items, n.methods
        )
        .expect("writing to a string cannot fail");
    }
    text
}

fn read_ledger(text: &str) -> Result<Ledger> {
    let mut rows = Ledger::new();
    for line in text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        ensure!(fields.len() == 6, "invalid test-cleanup row: {line}");
        let key = (fields[0].to_owned(), fields[1].to_owned());
        let counts = Counts {
            bodies: fields[2].parse()?,
            fields: fields[3].parse()?,
            items: fields[4].parse()?,
            methods: fields[5].parse()?,
        };
        ensure!(
            rows.insert(key, counts).is_none(),
            "duplicate test-cleanup row"
        );
    }
    Ok(rows)
}

fn growth(current: &Ledger, previous: &Ledger) -> Vec<String> {
    current
        .iter()
        .filter(|(key, n)| n.grows(previous.get(*key).copied().unwrap_or_default()))
        .map(|((krate, module), _)| format!("{krate}: {module}"))
        .collect()
}

pub(crate) fn run(write: bool, check: bool) -> Result<ExitCode> {
    let root = crate::util::repo_root();
    let current = inventory(&root)?;
    let path = root.join(LEDGER);
    if write || check {
        if path.exists() {
            let previous = read_ledger(&fs::read_to_string(&path)?)?;
            let increases = growth(&current, &previous);
            ensure!(
                increases.is_empty(),
                "private-state candidate inventory grew:\n{}",
                increases.join("\n")
            );
            if check && current != previous {
                bail!("test-cleanup ledger shrank; regenerate with --write");
            }
        } else {
            ensure!(
                write,
                "missing test-cleanup ledger; initialise with --write"
            );
        }
        if write {
            fs::write(&path, render(&current))?;
        }
    } else {
        print!("{}", render(&current));
    }
    let crates: BTreeSet<_> = current.keys().map(|(krate, _)| krate).collect();
    eprintln!(
        "test-cleanup: {} modules, {} candidate bodies in {} crates",
        current.len(),
        current.values().map(|n| n.bodies).sum::<usize>(),
        crates.len()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn survey(source: &str) -> Ledger {
        let file = syn::parse_file(source).unwrap();
        let mut names = Names::default();
        names.visit_file(&file);
        let mut survey = Survey {
            names: &names,
            test_only: false,
            function: false,
            modules: vec!["src/lib.rs".into()],
            found: BTreeMap::new(),
            current: Counts::default(),
        };
        survey.visit_file(&file);
        survey
            .found
            .into_iter()
            .map(|(module, counts)| (("fixture".into(), module), counts))
            .collect()
    }

    #[test]
    fn report_distinguishes_public_contracts_from_private_state() {
        let rows = survey(
            r#"
            pub struct State { secret: usize, pub visible: usize }
            impl State { fn epoch(&self) -> usize { self.secret } pub fn result(&self) -> usize { self.visible } }
            #[cfg(test)] mod tests {
                #[test] fn internals() { assert_eq!(state.secret, state.epoch()); }
                #[test] fn contract() { assert_eq!(state.visible, state.result()); }
                #[test] fn prose() { assert_eq!("state.secret", "state.epoch()"); }
            }
        "#,
        );
        let rendered = render(&rows);
        let data: Vec<_> = rendered
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(data, ["fixture\tsrc/lib.rs::tests\t1\t1\t0\t1"]);
    }

    #[test]
    fn report_follows_feature_test_bodies_and_restricted_items() {
        let rows = survey(
            r#"
            pub(crate) fn hidden() {}
            #[cfg(feature = "test-support")] pub mod bodies {
                pub fn kept_test_name() { crate::hidden(); }
            }
        "#,
        );
        assert!(render(&rows).ends_with("fixture\tsrc/lib.rs::bodies\t1\t0\t1\t0\n"));
    }

    #[test]
    fn ledger_round_trips_and_cannot_hide_growth_by_moving_a_body() {
        let previous = survey(
            "struct S { secret: usize } #[cfg(test)] mod tests { #[test] fn t() { assert_eq!(s.secret, 1); } }",
        );
        assert_eq!(
            render(&read_ledger(&render(&previous)).unwrap()),
            render(&previous)
        );
        assert!(growth(&Ledger::new(), &previous).is_empty());
        assert!(!growth(&previous, &Ledger::new()).is_empty());
    }
    #[test]
    fn integration_receivers_cannot_borrow_production_private_names() {
        let root = std::env::temp_dir().join(format!(
            "tcl-test-cleanup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = root.join("rust/fixture");
        fs::create_dir_all(directory.join("src")).unwrap();
        fs::create_dir_all(directory.join("tests")).unwrap();
        fs::create_dir_all(root.join("runtime/rust/src")).unwrap();
        fs::write(
            directory.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\n",
        )
        .unwrap();
        fs::write(
            root.join("runtime/rust/Cargo.toml"),
            "[package]\nname = \"runtime-fixture\"\n",
        )
        .unwrap();
        fs::write(
            directory.join("src/lib.rs"),
            r"
            struct State { secret: usize }
            #[cfg(test)] mod tests {
                #[test] fn private_state() {
                    let state = super::State { secret: 1 };
                    assert_eq!(state.secret, 1);
                }
            }
        ",
        )
        .unwrap();
        fs::write(
            directory.join("tests/public.rs"),
            r"
            mod external { #[derive(Default)] pub struct State { pub secret: usize } }
            #[test] fn contract() {
                let state = external::State::default();
                assert_eq!(state.secret, 0);
            }
        ",
        )
        .unwrap();
        let rendered = render(&inventory(&root).unwrap());
        fs::remove_dir_all(&root).unwrap();
        assert!(rendered.ends_with("fixture\trust/fixture/src/lib.rs::tests\t1\t1\t0\t0\n"));
        assert!(!rendered.contains("tests/public.rs"));
    }
}
