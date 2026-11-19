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

//! Enforce closed parser routes and surface unreviewed sources across Cargo
//! targets and host grammars. A lexical lead is a review item.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, ensure};
use syn::visit::{self, Visit};

const CENSUS: &str = "docs/work/name-handling/validation-tools/parser_census.py";

/// Production function, exact owner path it calls, and any required argument
/// carrying the selected configuration. Scope defaults are deliberate only
/// for the two fixed SpecTcl descriptor list fields.
struct Route {
    file: &'static str,
    function: &'static str,
    owner: &'static str,
    selected: &'static [(usize, &'static str)],
}

const PACK: &str = "rust/tcl-spectcl/src/loader/command/world_state.rs";
const APL: &str = "rust/tcl-bigip/src/apl/tokens.rs";
const STUDIO: &str = "rust/tcl-spec-studio/src/corpus.rs";
const VENV: &str = "rust/tcl-pkg/src/venv.rs";

const ROUTES: &[Route] = &[
    Route {
        file: PACK,
        function: "coverage_row",
        owner: "crate::loader::value_parsers::strict_pack_list",
        selected: &[],
    },
    Route {
        file: PACK,
        function: "world_state_domain",
        owner: "crate::loader::value_parsers::strict_pack_list",
        selected: &[],
    },
    Route {
        file: "rust/tcl-spectcl/src/loader/value_parsers.rs",
        function: "strict_pack_list",
        owner: "tcl_syntax::list::split_list_with_syntax",
        selected: &[(1, "config.list_parse"), (2, "config.escapes")],
    },
    Route {
        file: APL,
        function: "embedded_tcl_regions",
        owner: "tcl_lexer::command_substitution_end_bytes_in",
        selected: &[(2, "config")],
    },
    Route {
        file: APL,
        function: "embedded_tcl_regions",
        owner: "tcl_registry::model::ingress::resolve_environment",
        selected: &[(0, r#""f5-iapps""#)],
    },
    Route {
        file: APL,
        function: "embedded_tcl_regions",
        owner: "tcl_lexer::LexerConfig::from_grammar",
        selected: &[(0, "environment.grammar()")],
    },
    Route {
        file: STUDIO,
        function: "options_from_dispatch",
        owner: "tcl_syntax::expr::parser::parse_expr_for_profile_with_lexer_config",
        selected: &[(1, "Some(profile)"), (2, "config")],
    },
    Route {
        file: "rust/tcl-cli/src/commands/pkg.rs",
        function: "build_profile",
        owner: "tcl_pkg::venv::tcllibpath_directories",
        selected: &[],
    },
    Route {
        file: VENV,
        function: "tcllibpath_directories",
        owner: "tcl_syntax::list::split_list_bytes_in",
        selected: &[(1, "syntax"), (2, "escapes")],
    },
];

/// Real tests must call the public entry. A comment containing a test or a
/// helper with its old name cannot replace it.
const CONTROLS: &[(&str, &str, &str)] = &[
    (
        "rust/tcl-spectcl/src/loader/tests/block_descriptors.rs",
        "state_transition_blocks_preserve_complete_authored_rows",
        "evaluate_pack",
    ),
    (
        APL,
        "apl_hands_complete_source_ranges_to_the_tcl_walker",
        "embedded_tcl_regions",
    ),
    (
        "rust/tcl-spec-studio/src/infer.rs",
        "imported_if_options_require_a_fixed_comparison_with_a_parameter",
        "import_package",
    ),
    (
        STUDIO,
        "switch_patterns_suggest_values_without_closing_the_argument",
        "scan",
    ),
    (
        VENV,
        "created_and_relocated_wrappers_prepend_one_tcl_list_element",
        "create_venv",
    ),
];

fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Argument spelling is taken from the call AST. Strings, selected variables,
/// and the two configuration projections have distinct expression forms.
fn argument_identity(arg: &syn::Expr) -> Option<String> {
    match arg {
        syn::Expr::Path(p) => Some(path_text(&p.path)),
        syn::Expr::Field(field) => {
            let syn::Expr::Path(root) = &*field.base else {
                return None;
            };
            let syn::Member::Named(member) = &field.member else {
                return None;
            };
            Some(format!("{}.{member}", path_text(&root.path)))
        }
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(format!("{:?}", s.value())),
            _ => None,
        },
        syn::Expr::Call(call) if call.args.len() == 1 => {
            let syn::Expr::Path(p) = &*call.func else {
                return None;
            };
            let syn::Expr::Path(v) = call.args.first()? else {
                return None;
            };
            Some(format!("{}({})", path_text(&p.path), path_text(&v.path)))
        }
        syn::Expr::MethodCall(call) if call.args.is_empty() => {
            let syn::Expr::Path(p) = &*call.receiver else {
                return None;
            };
            Some(format!("{}.{}()", path_text(&p.path), call.method))
        }
        _ => None,
    }
}

#[derive(Default)]
struct Facts {
    calls: BTreeMap<String, Vec<Vec<Option<String>>>>,
    scope: String,
    leads: Vec<String>,
    library_list: bool,
    whitespace: bool,
    bracket_open: bool,
    bracket_close: bool,
    increments: bool,
    decrements: bool,
    delimiter: bool,
    branching: bool,
    iteration: bool,
    host_readers: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for Facts {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func {
            let name = path_text(&path.path);
            self.calls
                .entry(name.clone())
                .or_default()
                .push(call.args.iter().map(argument_identity).collect());
            let leaf = name.rsplit("::").next().unwrap_or(&name);
            if matches!(leaf, "parse_expr" | "split_list" | "join_list")
                || name.ends_with("LexerConfig::default")
                || name.ends_with("DialectProfile::plain_tcl")
            {
                self.leads.push(name);
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method = call.method.to_string();
        if matches!(
            method.as_str(),
            "split_whitespace" | "split_ascii_whitespace"
        ) {
            self.whitespace = true;
        }
        if matches!(
            method.as_str(),
            "split"
                | "split_once"
                | "rsplit"
                | "rsplit_once"
                | "splitn"
                | "rsplitn"
                | "split_whitespace"
                | "split_ascii_whitespace"
                | "trim_matches"
                | "parse"
        ) {
            self.host_readers.insert(method.clone());
        }
        self.iteration |= matches!(
            method.as_str(),
            "iter"
                | "bytes"
                | "chars"
                | "char_indices"
                | "enumerate"
                | "position"
                | "take_while"
                | "skip_while"
                | "split"
                | "find"
        );
        visit::visit_expr_method_call(self, call);
    }

    fn visit_lit_str(&mut self, lit: &'ast syn::LitStr) {
        self.library_list |= lit.value() == "TCLLIBPATH";
    }

    fn visit_lit_byte(&mut self, lit: &'ast syn::LitByte) {
        self.delimiter |= lit.value().is_ascii_punctuation() || lit.value().is_ascii_whitespace();
        self.bracket_open |= lit.value() == b'[';
        self.bracket_close |= lit.value() == b']';
    }

    fn visit_lit_char(&mut self, lit: &'ast syn::LitChar) {
        self.delimiter |= lit.value().is_ascii_punctuation() || lit.value().is_ascii_whitespace();
    }
    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        self.iteration |= matches!(
            expr,
            syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_)
        );
        self.branching |= matches!(
            expr,
            syn::Expr::Binary(_) | syn::Expr::Match(_) | syn::Expr::If(_)
        );
        visit::visit_expr(self, expr);
    }
    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        if item.path.is_ident("matches") {
            self.branching = true;
            // Rust macro arguments are opaque to syn's expression visitor.
            // Preserve the potential boundary for review even if it is not
            // written as one of the known Tcl parser spellings.
            self.delimiter = true;
        }
        visit::visit_macro(self, item);
    }
    fn visit_bin_op(&mut self, op: &'ast syn::BinOp) {
        self.increments |= matches!(op, syn::BinOp::AddAssign(_));
        self.decrements |= matches!(op, syn::BinOp::SubAssign(_));
    }
}

#[derive(Default)]
struct Source {
    functions: BTreeMap<String, Vec<Facts>>,
    tests: BTreeMap<String, Facts>,
    test_module: bool,
    scopes: Vec<String>,
    declarations: BTreeMap<String, usize>,
}

impl Source {
    fn record(&mut self, name: &syn::Ident, attrs: &[syn::Attribute], body: &syn::Block) {
        let scope = self
            .scopes
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(name.to_string().as_str()))
            .collect::<Vec<_>>()
            .join("::");
        let count = self.declarations.entry(scope.clone()).or_default();
        *count += 1;
        let mut facts = Facts {
            scope: if *count == 1 {
                scope
            } else {
                format!("{scope}[declaration {count}]")
            },
            ..Facts::default()
        };
        facts.visit_block(body);
        if attrs.iter().any(|attr| attr.path().is_ident("test")) {
            if !attrs.iter().any(|attr| attr.path().is_ident("ignore")) {
                self.tests.insert(name.to_string(), facts);
            }
        } else if !self.test_module {
            self.functions
                .entry(name.to_string())
                .or_default()
                .push(facts);
        }
    }
}

impl<'ast> Visit<'ast> for Source {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let old = self.test_module;
        self.test_module |= item.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Ident>()
                    .is_ok_and(|name| name == "test")
        });
        self.scopes.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scopes.pop();
        self.test_module = old;
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.record(&item.sig.ident, &item.attrs, &item.block);
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let target = match &*item.self_ty {
            syn::Type::Path(p) => path_text(&p.path),
            _ => "<type expression>".to_owned(),
        };
        let owner = match &item.trait_ {
            Some((_, p, _)) => format!("<{target} as {}>", path_text(p)),
            None => target,
        };
        self.scopes.push(owner);
        visit::visit_item_impl(self, item);
        self.scopes.pop();
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.record(&item.sig.ident, &item.attrs, &item.block);
    }
}

impl Source {
    fn has_route(&self, route: &Route) -> bool {
        self.functions.get(route.function).is_some_and(|functions| {
            functions.len() == 1
                && functions[0].calls.get(route.owner).is_some_and(|sites| {
                    sites.iter().any(|args| {
                        route.selected.iter().all(|&(index, name)| {
                            args.get(index).and_then(Option::as_deref) == Some(name)
                        })
                    })
                })
        })
    }

    /// Follow locally defined helpers by call structure. A private helper may
    /// change names without changing the retired implementation it carries.
    fn reachable(&self, start: &str, predicate: impl Fn(&Facts) -> bool) -> bool {
        let mut pending = vec![start];
        let mut seen = BTreeSet::new();
        while let Some(name) = pending.pop() {
            if seen.insert(name)
                && let Some(items) = self.functions.get(name)
            {
                for facts in items {
                    if predicate(facts) {
                        return true;
                    }
                    pending.extend(facts.calls.keys().filter_map(|call| {
                        let local = call.strip_prefix("self::").unwrap_or(call);
                        (!local.contains("::")).then_some(local)
                    }));
                }
            }
        }
        false
    }

    fn drift(&self, file: &str) -> Vec<String> {
        let mut errors = Vec::new();
        for route in ROUTES.iter().filter(|route| route.file == file) {
            if !self.has_route(route) {
                errors.push(format!(
                    "{file}::{} must call {} with its selected grammar",
                    route.function, route.owner
                ));
            }
            if !route.selected.is_empty()
                && self.reachable(route.function, |facts| {
                    facts.leads.iter().any(|entry| {
                        entry.ends_with("LexerConfig::default")
                            || entry.ends_with("DialectProfile::plain_tcl")
                    })
                })
            {
                errors.push(format!(
                    "{file}::{} reaches an ambient parser configuration",
                    route.function
                ));
            }
            if matches!(route.file, PACK | STUDIO)
                && self.reachable(route.function, |f| f.whitespace)
            {
                errors.push(format!(
                    "{file}::{} reaches a host whitespace splitter for a Tcl operand",
                    route.function
                ));
            }
            if route.file == APL
                && self.reachable(route.function, |f| {
                    f.bracket_open && f.bracket_close && f.increments && f.decrements
                })
            {
                errors.push(format!(
                    "{file}::{} reaches a private bracket depth counter",
                    route.function
                ));
            }
        }
        for &(path, test, public) in CONTROLS.iter().filter(|(path, ..)| *path == file) {
            if !self
                .tests
                .get(test)
                .is_some_and(|f| f.calls.contains_key(public))
            {
                errors.push(format!(
                    "{path}::{test} must exercise {public} through a #[test]"
                ));
            }
        }
        for (name, items) in &self.functions {
            if items.iter().any(|f| f.library_list) && self.reachable(name, |f| f.whitespace) {
                errors.push(format!(
                    "{file}::{name} reaches a host whitespace splitter while reading TCLLIBPATH"
                ));
            }
        }
        errors
    }
}

fn census(root: &Path, mode: &str) -> Result<String> {
    let out = Command::new("python3")
        .current_dir(root)
        .args([CENSUS, mode])
        .output()?;
    ensure!(
        out.status.success(),
        "parser census {mode}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?)
}

/// A tracked foreign source can implement a parser under any function or file
/// name. Keep its boundary inspectable until an owner records its entry.
fn foreign_review(surface: &str) -> &'static str {
    match surface {
        "generated" => "verify generator and source differential",
        "vendored" => "verify external owner and integration",
        "grammar-or-editor-rules" => "review host grammar and embedded boundaries",
        "source" => "review foreign source entry and consumer",
        "other-review" => "unclassified tracked input",
        _ => "inspect declared surface and owning consumer",
    }
}

pub fn run(report: bool) -> Result<ExitCode> {
    let root = crate::util::repo_root();
    let inventory = census(&root, "--files")?;
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut unreviewed = 0;
    let mut structural = 0;
    let mut ast_sources = 0;
    let mut foreign = BTreeMap::<&str, usize>::new();
    for line in inventory.lines().skip(1) {
        let mut fields = line.splitn(3, '\t');
        let (_component, surface, file) = (fields.next(), fields.next(), fields.next());
        let (Some(surface), Some(file)) = (surface, file) else {
            anyhow::bail!("invalid tracked-file inventory row: {line}")
        };
        seen.insert(file);
        if !file.ends_with(".rs") {
            *foreign.entry(surface).or_default() += 1;
            if report {
                println!("{file}\t{surface}\t{}", foreign_review(surface));
            }
            continue;
        }
        let scoped = ROUTES.iter().any(|r| r.file == file)
            || CONTROLS.iter().any(|(path, ..)| *path == file);
        if !scoped
            && (file.contains("/tests/") || crate::util::test_sources::is_test_module_file(file))
        {
            continue;
        }
        let text = std::fs::read_to_string(root.join(file)).with_context(|| file.to_owned())?;
        let mut source = Source::default();
        match syn::parse_file(&text) {
            Ok(tree) => {
                source.visit_file(&tree);
                ast_sources += 1;
            }
            Err(err)
                if !scoped
                    && matches!(
                        surface,
                        "test-or-example"
                            | "corpus-or-oracle"
                            | "vendored"
                            | "generated"
                            | "other-review"
                    ) =>
            {
                if report {
                    println!("{file}\t{surface}\tRust fragment/Syn parse unavailable: {err}");
                }
                continue;
            }
            Err(err) => return Err(anyhow::anyhow!("parse tracked Rust source {file}: {err}")),
        }
        errors.extend(source.drift(file));
        for (name, functions) in &source.functions {
            for facts in functions {
                if facts.delimiter && (facts.iteration || facts.branching) {
                    structural += 1;
                    if report {
                        println!(
                            "{file}::{}\tdelimiter and control flow\treview structural reader",
                            facts.scope
                        );
                    }
                }
                for method in &facts.host_readers {
                    structural += 1;
                    if report {
                        println!("{file}::{}\t{method}\treview host operation", facts.scope);
                    }
                }
                let mut occurrences = BTreeMap::new();
                for lead in &facts.leads {
                    let occurrence = occurrences.entry(lead).or_insert(0usize);
                    *occurrence += 1;
                    if ROUTES.iter().any(|rule| {
                        rule.file == file && rule.function == name && rule.owner == lead
                    }) {
                        continue;
                    }
                    unreviewed += 1;
                    if report {
                        println!(
                            "{file}::{} [call {occurrence}]\t{lead}\treview grammar selection",
                            facts.scope
                        );
                    }
                }
            }
        }
    }
    for path in ROUTES
        .iter()
        .map(|route| route.file)
        .chain(CONTROLS.iter().map(|row| row.0))
        .chain([
            "runtime/rust/Cargo.toml",
            "rust/tcl-vm-wasm/Cargo.toml",
            "grammars/tree-sitter-apl/grammar.js",
            "editors/zed/languages/apl/injections.scm",
        ])
    {
        if !seen.contains(path) {
            errors.push(format!("missing tracked parser surface {path}"));
        }
    }
    let targets = census(&root, "--cargo-targets")?;
    let mut manifests = BTreeSet::new();
    let mut target_count = 0;
    for line in targets.lines().skip(1) {
        let fields: Vec<_> = line.split('\t').collect();
        ensure!(fields.len() == 7, "invalid Cargo target inventory: {line}");
        manifests.insert(fields[0]);
        target_count += 1;
        for file in [fields[0], fields[4]] {
            if !seen.contains(file) {
                errors.push(format!(
                    "{} target {} has untracked manifest or entry {file}",
                    fields[1], fields[2]
                ));
            }
        }
    }
    // Cargo accepts virtual workspace manifests with no package of their own.
    // Every tracked package manifest must appear, including an excluded crate.
    for &file in &seen {
        if Path::new(file)
            .file_name()
            .is_some_and(|n| n == "Cargo.toml")
            && !manifests.contains(file)
            && std::fs::read_to_string(root.join(file))?
                .lines()
                .any(|line| line.trim() == "[package]")
        {
            errors.push(format!("unmeasured Cargo package {file}"));
        }
    }
    if report {
        print!("{targets}");
        print!("{}", census(&root, "--sites")?);
    }
    for (surface, count) in &foreign {
        println!("parser-ownership: {count} tracked {surface} boundaries");
    }
    println!(
        "parser-ownership: {} tracked paths, {target_count} Cargo targets, {ast_sources} AST-scanned Rust files, {structural} structural/host leads, {unreviewed} Rust entry/default review leads. Run cargo xtask parser-ownership --report for sites, including host grammars.",
        seen.len()
    );
    for error in &errors {
        eprintln!("{error}");
    }
    Ok(if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(text: &str) -> Source {
        let mut source = Source::default();
        source.visit_file(&syn::parse_file(text).unwrap());
        source
    }

    #[test]
    fn library_list_retirement_applies_to_runtime_and_feature_gated_standalones() {
        let renamed = r#"
            #[cfg(feature="wasm")]
            fn lookup() { renamed(std::env::var("TCLLIBPATH").unwrap()); }
            fn renamed(value: String) { value.split_ascii_whitespace(); }
        "#;
        for path in [
            "runtime/rust/src/lib.rs",
            "rust/tcl-vm-wasm/src/lib.rs",
            "new/consumer/src/lib.rs",
        ] {
            assert!(
                source(renamed)
                    .drift(path)
                    .iter()
                    .any(|p| p.contains("lookup") && p.contains("whitespace"))
            );
            assert!(
                source(
                    r#"fn lookup() { owned(std::env::var("TCLLIBPATH")); }
                              fn an_os_format(s: &str) { s.split_whitespace(); }"#
                )
                .drift(path)
                .is_empty()
            );
        }
    }

    #[test]
    fn borrowed_apl_extent_requires_selected_bracket_owner_and_public_control() {
        let valid = r#"
            fn embedded_tcl_regions(src: &[u8]) {
                let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
                let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
                tcl_lexer::command_substitution_end_bytes_in(src, 0, config);
            }
            #[test] fn apl_hands_complete_source_ranges_to_the_tcl_walker() { embedded_tcl_regions(b""); }
        "#;
        assert!(source(valid).drift(APL).is_empty());
        for changed in [
            valid.replace("environment.grammar()", "alternate.grammar()"),
            valid.replace("src, 0, config", "src, 0, LexerConfig::default()"),
            valid.replace("f5-iapps", "tcl9.1"),
            valid.replace("#[test]", "#[test] #[ignore]"),
            valid.replace("embedded_tcl_regions(b", "copied_parser(b"),
            valid.replace("src, 0, config);", "src, 0, config); new_name(src);")
                + "fn new_name(s: &[u8]) {for c in s { if *c == b'[' {n += 1} else if *c == b']' {n -= 1} }}",
        ] {
            assert!(!source(&changed).drift(APL).is_empty(), "{changed}");
        }
    }

    #[test]
    fn newly_named_byte_scanner_and_foreign_adapter_remain_reviewable() {
        let parsed = source("fn field(image: &[u8]) { for &v in image { if v == b']' {break;} } }");
        let facts = &parsed.functions["field"][0];
        assert!(facts.delimiter && facts.iteration && facts.branching);
        assert_ne!(foreign_review("source"), foreign_review("generated"));
        assert_eq!(
            foreign_review("source"),
            "review foreign source entry and consumer"
        );
    }

    #[test]
    fn occurrence_inventory_distinguishes_impls_and_calls_within_one_body() {
        let parsed = source(
            "impl A { fn build() { parse_expr(a); parse_expr(b); } }
                             mod nested { impl B { fn build() { parse_expr(c); } } }",
        );
        let entries = &parsed.functions["build"];
        assert_eq!(entries[0].scope, "A::build");
        assert_eq!(entries[0].leads, ["parse_expr", "parse_expr"]);
        assert_eq!(entries[1].scope, "nested::B::build");
        assert_eq!(entries[1].leads, ["parse_expr"]);
    }

    #[test]
    fn fixed_pack_and_selected_expression_routes_reject_host_fallbacks() {
        let pack = "fn coverage_row(s: &str) {crate::loader::value_parsers::strict_pack_list(s);}
                    fn world_state_domain(s: &str) {crate::loader::value_parsers::strict_pack_list(s);}";
        assert!(source(pack).drift(PACK).is_empty());
        let renamed = pack.replace("strict_pack_list(s);", "strict_pack_list(s); fragment(s);")
            + "fn fragment(s: &str) {s.split_whitespace();}";
        assert!(
            source(&renamed)
                .drift(PACK)
                .iter()
                .any(|message| message.contains("whitespace"))
        );

        let studio = r"fn options_from_dispatch() {
             tcl_syntax::expr::parser::parse_expr_for_profile_with_lexer_config(source, Some(profile), config);
           }
           #[test] fn switch_patterns_suggest_values_without_closing_the_argument() {scan();}";
        assert!(source(studio).drift(STUDIO).is_empty());
        for changed in [
            studio.replace("Some(profile)", "None"),
            studio.replace("config);", "default());"),
        ] {
            assert!(!source(&changed).drift(STUDIO).is_empty());
        }
    }
}
