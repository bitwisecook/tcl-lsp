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

//! The `module-layers` gate: the module-to-crate map for splitting
//! `tcl-compiler`, the back-edges that block each target crate, upward
//! dev-dependencies, and test bodies behind `test-support`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use super::DIRECTORY;
use super::source::{Token, declares, tokens};
use crate::util::{test_sources, tsv};

const LAYERS: &str = "module-layers.tsv";
const BACK_EDGES: &str = "module-back-edges.tsv";
const DEV_EDGES: &str = "dev-edges.tsv";
const TEST_BODIES: &str = "test-support-bodies.tsv";
const COMPILER: &str = "rust/tcl-compiler/src";
const UNASSIGNED: &str = "unassigned";
const FEATURE: &str = "test-support";
/// Other dev-only features whose test builds duplicate a cone.
const DEV_FEATURES: &[(&str, &str)] = &[("tcl-registry", "testing")];
const TOOLING: &[&str] = &["xtask", "xtask-backing"];
const DUPLICATE_ROOT: &str = "tcl-lsp-server";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Gate {
    Test,
    TestSupport,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Reporting,
    Enforcing,
}

#[derive(Debug)]
struct Layer {
    krate: String,
    rank: u32,
}

struct Ledger {
    mode: Mode,
    rows: BTreeMap<String, Layer>,
}

fn at<'a>(tokens: &[Token<'a>], index: usize) -> &'a str {
    tokens.get(index).map_or("", |token| token.text)
}

fn identifier(text: &str) -> bool {
    text.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
}

fn join(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_owned()
    } else {
        format!("{module}::{name}")
    }
}

fn split_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let (mut depth, mut start) = (0_i32, 0);
    for (offset, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(&text[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    args.push(&text[start..]);
    args.into_iter().filter(|arg| !arg.is_empty()).collect()
}

/// Whether a `cfg` predicate (its tokens joined without spaces) compiles
/// only for tests, and whether `test-support` builds also see it.
fn cfg_gate(predicate: &str) -> Option<Gate> {
    match predicate {
        "test" => return Some(Gate::Test),
        "feature=\"test-support\"" => return Some(Gate::TestSupport),
        _ => {}
    }
    let (every, args) = if let Some(rest) = predicate.strip_prefix("any(") {
        (true, rest)
    } else {
        (false, predicate.strip_prefix("all(")?)
    };
    let gates: Vec<_> = split_args(args.strip_suffix(')')?)
        .into_iter()
        .map(cfg_gate)
        .collect();
    if every && gates.iter().any(Option::is_none) {
        return None;
    }
    gates.into_iter().flatten().max()
}

/// The end (exclusive) of the attribute starting at `start`, if one does.
fn attribute_end(tokens: &[Token<'_>], start: usize) -> Option<usize> {
    if at(tokens, start) != "#" {
        return None;
    }
    let open = start + 1 + usize::from(at(tokens, start + 1) == "!");
    if at(tokens, open) != "[" {
        return None;
    }
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.text {
            "[" => depth += 1,
            "]" => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn attribute_gate(tokens: &[Token<'_>], start: usize, end: usize) -> Option<Gate> {
    let head = start + 1 + usize::from(at(tokens, start + 1) == "!");
    if at(tokens, head + 1) != "cfg" || at(tokens, head + 2) != "(" || end < head + 5 {
        return None;
    }
    let predicate: String = tokens[head + 3..end - 2]
        .iter()
        .map(|token| token.text)
        .collect();
    cfg_gate(&predicate)
}

const ITEM_KEYWORDS: &[&str] = &[
    "pub",
    "fn",
    "impl",
    "mod",
    "struct",
    "enum",
    "use",
    "const",
    "static",
    "type",
    "trait",
    "macro_rules",
    "unsafe",
    "extern",
    "async",
    "let",
];

/// The end (exclusive) of the item, statement or field starting at `start`.
fn item_end(tokens: &[Token<'_>], start: usize) -> usize {
    let item = ITEM_KEYWORDS.contains(&at(tokens, start));
    let mut depth = 0_i32;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.text {
            "(" | "[" => depth += 1,
            "{" => {
                if depth == 0 {
                    return block_end(tokens, index);
                }
                depth += 1;
            }
            ")" | "]" | "}" => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            ";" if depth == 0 => return index + 1,
            "," if depth == 0 && !item => return index + 1,
            _ => {}
        }
    }
    tokens.len()
}

fn block_end(tokens: &[Token<'_>], open: usize) -> usize {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.text {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return index + 1;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// The outermost items compiled only for tests, with their attributes.
fn gated_items(tokens: &[Token<'_>]) -> Vec<(Range<usize>, Gate)> {
    let mut items = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let Some(end) = attribute_end(tokens, index) else {
            index += 1;
            continue;
        };
        let Some(gate) = attribute_gate(tokens, index, end) else {
            index = end;
            continue;
        };
        if at(tokens, index + 1) == "!" {
            items.push((0..tokens.len(), gate));
            break;
        }
        let mut body = end;
        while let Some(next) = attribute_end(tokens, body) {
            body = next;
        }
        let stop = item_end(tokens, body);
        items.push((index..stop, gate));
        index = stop;
    }
    items
}

fn public_before(tokens: &[Token<'_>], index: usize) -> bool {
    match at(tokens, index.wrapping_sub(1)) {
        "pub" => true,
        ")" => (0..index)
            .rev()
            .find(|at_open| tokens[*at_open].text == "(")
            .is_some_and(|open| at(tokens, open.wrapping_sub(1)) == "pub"),
        _ => false,
    }
}

fn path_attribute(tokens: &[Token<'_>], declaration: usize) -> Option<String> {
    let mut index = declaration;
    if public_before(tokens, index) {
        index -= 1;
        if tokens[index].text == ")" {
            index = (0..index).rev().find(|i| tokens[*i].text == "(")? - 1;
        }
    }
    while at(tokens, index.wrapping_sub(1)) == "]" {
        let open = (0..index).rev().find(|i| tokens[*i].text == "[")?;
        let start = open.checked_sub(1)?;
        if tokens[start..index]
            .iter()
            .map(|token| token.text)
            .take(4)
            .eq(["#", "[", "path", "="])
        {
            return Some(at(tokens, start + 4).trim_matches('"').to_owned());
        }
        index = start;
    }
    None
}

#[derive(Debug, PartialEq, Eq)]
struct Leaf {
    path: Vec<String>,
    name: Option<String>,
}

/// The leaves of the use tree starting at `index`, and the index after it.
fn use_tree(tokens: &[Token<'_>], mut index: usize, mut prefix: Vec<String>) -> (usize, Vec<Leaf>) {
    let mut leaves = Vec::new();
    loop {
        match at(tokens, index) {
            ":" => index += 1,
            "{" => {
                index += 1;
                while !matches!(at(tokens, index), "}" | "") {
                    let (next, mut inner) = use_tree(tokens, index, prefix.clone());
                    leaves.append(&mut inner);
                    index = next + usize::from(at(tokens, next) == ",");
                }
                return (index + 1, leaves);
            }
            "*" => {
                leaves.push(Leaf {
                    path: prefix,
                    name: None,
                });
                return (index + 1, leaves);
            }
            word if identifier(word) => {
                index += 1;
                if at(tokens, index) == ":" && at(tokens, index + 1) == ":" {
                    prefix.push(word.to_owned());
                    index += 2;
                    continue;
                }
                let mut name = word.to_owned();
                if word != "self" || prefix.is_empty() {
                    prefix.push(word.to_owned());
                } else if let Some(last) = prefix.last() {
                    name.clone_from(last);
                }
                if at(tokens, index) == "as" {
                    at(tokens, index + 1).clone_into(&mut name);
                    index += 2;
                }
                leaves.push(Leaf {
                    path: prefix,
                    name: (name != "_").then_some(name),
                });
                return (index, leaves);
            }
            _ => return (index, leaves),
        }
    }
}

#[derive(Default, Debug)]
struct FileScan {
    /// `mod name;` declarations: the child's module path and `#[path]`.
    children: Vec<(String, Option<String>)>,
    inline: Vec<String>,
    /// Production `crate::` paths: the module that names it and the path.
    refs: Vec<(String, Vec<String>)>,
    /// Public `use` items: module, the path used and the name exported.
    exports: Vec<(String, Vec<String>, String)>,
}

fn scan_file(module: &str, text: &str) -> FileScan {
    let tokens = tokens(text);
    let gated = gated_items(&tokens);
    let mut gated = gated.iter().map(|(range, _)| range).peekable();
    let mut scan = FileScan::default();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let (mut index, mut depth) = (0, 0);
    while index < tokens.len() {
        if let Some(range) = gated.next_if(|range| range.start <= index) {
            index = index.max(range.end);
            continue;
        }
        let current = stack.last().map_or(module, |(_, inner)| inner.as_str());
        match tokens[index].text {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if stack.last().is_some_and(|(open, _)| *open == depth) {
                    stack.pop();
                }
            }
            "mod" if identifier(at(&tokens, index + 1)) => {
                let child = join(current, at(&tokens, index + 1));
                match at(&tokens, index + 2) {
                    ";" => scan.children.push((child, path_attribute(&tokens, index))),
                    "{" => {
                        scan.inline.push(child.clone());
                        stack.push((depth, child));
                        depth += 1;
                        index += 3;
                        continue;
                    }
                    _ => {}
                }
            }
            "use"
                if index == 0
                    || matches!(at(&tokens, index - 1), ";" | "{" | "}" | "]" | "pub" | ")") =>
            {
                let (end, leaves) = use_tree(&tokens, index + 1, Vec::new());
                let exported = public_before(&tokens, index);
                for leaf in leaves {
                    if leaf.path.first().is_some_and(|first| first == "crate") {
                        scan.refs
                            .push((current.to_owned(), leaf.path[1..].to_vec()));
                    }
                    if let (true, Some(name)) = (exported, leaf.name) {
                        scan.exports.push((current.to_owned(), leaf.path, name));
                    }
                }
                index = end;
                continue;
            }
            "crate"
                if at(&tokens, index + 1) == ":"
                    && at(&tokens, index + 2) == ":"
                    && at(&tokens, index.wrapping_sub(1)) != "in" =>
            {
                let mut path = Vec::new();
                let mut next = index + 3;
                while identifier(at(&tokens, next)) {
                    path.push(at(&tokens, next).to_owned());
                    if at(&tokens, next + 1) == ":"
                        && at(&tokens, next + 2) == ":"
                        && identifier(at(&tokens, next + 3))
                    {
                        next += 3;
                    } else {
                        break;
                    }
                }
                scan.refs.push((current.to_owned(), path));
                index = next + 1;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    scan
}

#[derive(Default)]
struct Tree {
    /// Production module paths, each with the file that declares its items.
    modules: BTreeMap<String, String>,
    refs: Vec<(String, Vec<String>)>,
    aliases: BTreeMap<String, Vec<String>>,
}

impl Tree {
    fn add(&mut self, rel: &str, module: &str, scan: FileScan) {
        self.modules.insert(module.to_owned(), rel.to_owned());
        for inline in scan.inline {
            self.modules.insert(inline, rel.to_owned());
        }
        self.refs.extend(scan.refs);
        for (owner, path, name) in scan.exports {
            let key = join(&owner, &name);
            self.aliases.insert(key, [vec![owner], path].concat());
        }
    }

    fn finish(&mut self) {
        let raw = std::mem::take(&mut self.aliases);
        for (key, mut path) in raw {
            let owner = path.remove(0);
            if let Some(target) = self.absolute(&owner, &path)
                && target.join("::") != key
            {
                self.aliases.insert(key, target);
            }
        }
    }

    fn absolute(&self, owner: &str, path: &[String]) -> Option<Vec<String>> {
        let mut base: Vec<String> = owner
            .split("::")
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect();
        let mut rest = path;
        match rest.first()?.as_str() {
            "crate" => {
                base.clear();
                rest = &rest[1..];
            }
            "self" => rest = &rest[1..],
            "super" => {
                while rest.first().is_some_and(|part| part == "super") {
                    base.pop()?;
                    rest = &rest[1..];
                }
            }
            first if self.modules.contains_key(&join(owner, first)) => {}
            _ => return None,
        }
        base.extend(rest.iter().cloned());
        Some(base)
    }

    fn resolve(&self, path: &[String]) -> Vec<String> {
        let mut path = path.to_vec();
        for _ in 0..16 {
            let found = (1..=path.len()).rev().find_map(|len| {
                self.aliases
                    .get(&path[..len].join("::"))
                    .map(|to| (len, to))
            });
            let Some((len, target)) = found else { break };
            path = [target.clone(), path[len..].to_vec()].concat();
        }
        path
    }
}

fn compiler_tree(root: &Path) -> Result<Tree> {
    let mut tree = Tree::default();
    let mut pending = vec![(format!("{COMPILER}/lib.rs"), String::new())];
    while let Some((rel, module)) = pending.pop() {
        if test_sources::is_test_module_file(&rel) {
            continue;
        }
        let text = fs::read_to_string(root.join(&rel)).with_context(|| rel.clone())?;
        let scan = scan_file(&module, &text);
        for (child, path) in &scan.children {
            let name = child.rsplit("::").next().unwrap_or(child);
            let file = match path {
                Some(path) => rel
                    .rsplit_once('/')
                    .map(|(directory, _)| format!("{directory}/{path}")),
                None => test_sources::module_file(root, &rel, name),
            }
            .with_context(|| format!("{rel}: no file for module {child}"))?;
            pending.push((file, child.clone()));
        }
        tree.add(&rel, &module, scan);
    }
    tree.finish();
    Ok(tree)
}

fn parse_layers(text: &str) -> Result<Ledger> {
    let mode = match text.lines().find_map(|line| line.strip_prefix("# mode: ")) {
        None | Some("reporting") => Mode::Reporting,
        Some("enforcing") => Mode::Enforcing,
        Some(other) => bail!("{LAYERS}: unknown mode {other}"),
    };
    let mut rows = BTreeMap::new();
    for row in tsv::rows(text) {
        let fields: Vec<_> = tsv::fields(row).collect();
        let [module, krate, rank] = fields.as_slice() else {
            bail!("{LAYERS}: invalid row: {row}");
        };
        let layer = Layer {
            krate: (*krate).to_owned(),
            rank: rank
                .parse()
                .with_context(|| format!("{LAYERS}: rank in {row}"))?,
        };
        ensure!(
            rows.insert((*module).to_owned(), layer).is_none(),
            "{LAYERS}: duplicate module {module}"
        );
    }
    Ok(Ledger { mode, rows })
}

fn row_for<'a>(rows: &'a BTreeMap<String, Layer>, path: &[String]) -> Option<(&'a str, &'a Layer)> {
    (1..=path.len()).rev().find_map(|len| {
        rows.get_key_value(&path[..len].join("::"))
            .map(|(key, layer)| (key.as_str(), layer))
    })
}

/// Map problems that fail in every mode.
fn check_map(root: &Path, tree: &Tree, rows: &BTreeMap<String, Layer>) -> Vec<String> {
    let mut problems = Vec::new();
    for module in tree.modules.keys() {
        if !module.is_empty() && !module.contains("::") && !rows.contains_key(module) {
            problems.push(format!("module {module} has no row in {LAYERS}"));
        }
    }
    let mut ranks: BTreeMap<&str, u32> = BTreeMap::new();
    for (module, layer) in rows {
        if layer.krate == UNASSIGNED || layer.rank == 0 {
            problems.push(format!("module {module} has no target crate"));
        } else if *ranks.entry(&layer.krate).or_insert(layer.rank) != layer.rank {
            problems.push(format!("{} has two ranks ({module})", layer.krate));
        }
        if tree.modules.contains_key(module) {
            continue;
        }
        let declared = module.rsplit_once("::").is_some_and(|(parent, item)| {
            tree.modules.get(parent).is_some_and(|rel| {
                fs::read_to_string(root.join(rel)).is_ok_and(|text| declares(&text, item))
            })
        });
        if !declared {
            problems.push(format!("stale row {module}: no such module or item"));
        }
    }
    problems
}

type Pairs = BTreeMap<(String, String), usize>;

fn back_edges(tree: &Tree, rows: &BTreeMap<String, Layer>) -> Pairs {
    let mut pairs = Pairs::new();
    for (from, path) in &tree.refs {
        let from: Vec<String> = from.split("::").map(str::to_owned).collect();
        let (Some((from, lower)), Some((to, upper))) =
            (row_for(rows, &from), row_for(rows, &tree.resolve(path)))
        else {
            continue;
        };
        if lower.krate != upper.krate && lower.rank <= upper.rank {
            *pairs.entry((from.to_owned(), to.to_owned())).or_default() += 1;
        }
    }
    pairs
}

fn parse_pairs(text: &str) -> Result<Pairs> {
    let mut pairs = Pairs::new();
    for row in tsv::rows(text) {
        let fields: Vec<_> = tsv::fields(row).collect();
        let [from, to, count] = fields.as_slice() else {
            bail!("{BACK_EDGES}: invalid row: {row}");
        };
        let key = ((*from).to_owned(), (*to).to_owned());
        ensure!(
            pairs.insert(key, count.parse()?).is_none(),
            "{BACK_EDGES}: duplicate pair {from} -> {to}"
        );
    }
    Ok(pairs)
}

/// The ledger's diff against the tree: (message, whether it fails).
fn diff_pairs(actual: &Pairs, ledger: &Pairs, mode: Mode) -> Vec<(String, bool)> {
    let enforcing = mode == Mode::Enforcing;
    let mut lines = Vec::new();
    for ((from, to), count) in actual {
        match ledger.get(&(from.clone(), to.clone())) {
            None => lines.push((format!("new back-edge {from} -> {to} ({count})"), enforcing)),
            Some(old) if count > old => lines.push((
                format!("back-edge {from} -> {to} rose from {old} to {count}"),
                enforcing,
            )),
            Some(old) if count < old => lines.push((
                format!("back-edge {from} -> {to} fell from {old} to {count}; lower its row"),
                enforcing,
            )),
            Some(_) => {}
        }
    }
    for ((from, to), old) in ledger {
        if !actual.contains_key(&(from.clone(), to.clone())) {
            lines.push((
                format!("back-edge {from} -> {to} is gone (was {old}); remove its row"),
                enforcing,
            ));
        }
    }
    lines
}

fn crate_report(rows: &BTreeMap<String, Layer>, pairs: &Pairs) -> Vec<String> {
    let mut crates: BTreeMap<(u32, &str), (usize, usize)> = BTreeMap::new();
    for layer in rows.values() {
        crates.entry((layer.rank, &layer.krate)).or_default();
    }
    for ((from, _), count) in pairs {
        let layer = &rows[from];
        let entry = crates.entry((layer.rank, &layer.krate)).or_default();
        entry.0 += count;
        entry.1 += 1;
    }
    crates
        .into_iter()
        .map(|((rank, krate), (refs, pairs))| {
            if refs == 0 {
                format!("{rank:>2} {krate}: no back-edges, can be extracted")
            } else {
                format!("{rank:>2} {krate}: {refs} refs in {pairs} pairs")
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    manifest_path: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
    #[serde(default)]
    features: Vec<String>,
}

struct Graph<'a> {
    packages: BTreeMap<&'a str, &'a Package>,
    /// Workspace crates each crate reaches through normal and build edges.
    cones: BTreeMap<&'a str, BTreeSet<&'a str>>,
    depended_on: BTreeSet<&'a str>,
}

impl<'a> Graph<'a> {
    fn new(packages: &'a [Package]) -> Self {
        let packages: BTreeMap<_, _> = packages.iter().map(|p| (p.name.as_str(), p)).collect();
        let normal = |package: &'a Package| {
            package
                .dependencies
                .iter()
                .filter(|dep| dep.kind.as_deref() != Some("dev") && dep.name != package.name)
                .filter_map(|dep| packages.get_key_value(dep.name.as_str()).map(|(k, _)| *k))
        };
        let depended_on = packages.values().flat_map(|p| normal(p)).collect();
        let mut cones = BTreeMap::new();
        for name in packages.keys() {
            let mut cone = BTreeSet::new();
            let mut pending: Vec<&str> = normal(packages[name]).collect();
            while let Some(next) = pending.pop() {
                if cone.insert(next) {
                    pending.extend(normal(packages[next]));
                }
            }
            cones.insert(*name, cone);
        }
        Self {
            packages,
            cones,
            depended_on,
        }
    }

    fn reaches(&self, from: &str, to: &str) -> bool {
        from == to || self.cones[from].contains(to)
    }

    /// Dev-dependencies that name a crate above the crate they test: one
    /// that depends on it, or one above a layer of `deny.toml` it is below.
    fn upward_dev_edges(&self, layers: &BTreeSet<String>) -> BTreeSet<(String, String)> {
        let mut edges = BTreeSet::new();
        for (name, package) in &self.packages {
            if TOOLING.contains(name) {
                continue;
            }
            for dep in &package.dependencies {
                let to = dep.name.as_str();
                if dep.kind.as_deref() != Some("dev") || to == *name || !self.cones.contains_key(to)
                {
                    continue;
                }
                let above_layer = self.depended_on.contains(name)
                    && layers
                        .iter()
                        .any(|layer| self.reaches(to, layer) && !self.reaches(name, layer));
                if self.cones[to].contains(name) || above_layer {
                    edges.insert(((*name).to_owned(), to.to_owned()));
                }
            }
        }
        edges
    }

    /// Crates enabling `owner`'s `feature`, each with the workspace crates
    /// its test build compiles a second time beside a build of
    /// [`DUPLICATE_ROOT`] (item 296's measure).
    fn feature_variants(&self, owner: &str, feature: &str) -> Vec<(&'a str, usize)> {
        let root_cone = self.cones.get(DUPLICATE_ROOT);
        let mut variants = Vec::new();
        for (name, package) in &self.packages {
            if *name == owner
                || !package
                    .dependencies
                    .iter()
                    .any(|dep| dep.name == owner && dep.features.iter().any(|f| f == feature))
            {
                continue;
            }
            let mut graph: BTreeSet<&str> = self.cones[name].clone();
            graph.insert(name);
            for dep in &package.dependencies {
                if let Some(cone) = self.cones.get(dep.name.as_str()) {
                    graph.insert(dep.name.as_str());
                    graph.extend(cone);
                }
            }
            let duplicated = graph
                .iter()
                .filter(|krate| self.reaches(krate, owner))
                .filter(|krate| {
                    root_cone.is_some_and(|cone| **krate == DUPLICATE_ROOT || cone.contains(*krate))
                })
                .count();
            variants.push((*name, duplicated));
        }
        variants
    }
}

fn metadata(root: &Path) -> Result<Metadata> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1", "--locked"])
        .output()
        .context("read the Cargo dependency graph")?;
    ensure!(
        output.status.success(),
        "cargo metadata: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn deny_layers(root: &Path) -> Result<BTreeSet<String>> {
    let config: toml::Value = toml::from_str(&fs::read_to_string(root.join("deny.toml"))?)?;
    let bans = config
        .get("bans")
        .and_then(|bans| bans.get("deny"))
        .and_then(toml::Value::as_array)
        .context("deny.toml has no [bans] deny list")?;
    Ok(bans
        .iter()
        .filter(|ban| {
            ban.get("wrappers")
                .and_then(toml::Value::as_array)
                .is_some_and(|w| !w.is_empty())
        })
        .filter_map(|ban| {
            ban.get("name")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        })
        .collect())
}

/// Test bodies in a module behind `test-support`: `#[test]` functions, and
/// `pub fn`s that a test of the same name in an enabling crate calls.
fn body_count(tokens: &[Token<'_>], called: &BTreeSet<String>) -> usize {
    (0..tokens.len())
        .filter(|&index| {
            let test = is_test_attribute(tokens, index);
            let body = at(tokens, index) == "fn"
                && public_before(tokens, index)
                && called.contains(at(tokens, index + 1));
            test || body
        })
        .count()
}

fn public_fns<'a, 't>(tokens: &'t [Token<'a>]) -> impl Iterator<Item = &'a str> + 't {
    (0..tokens.len())
        .filter(|&index| at(tokens, index) == "fn" && public_before(tokens, index))
        .map(|index| at(tokens, index + 1))
}

fn is_test_attribute(tokens: &[Token<'_>], index: usize) -> bool {
    (0..4)
        .map(|offset| at(tokens, index + offset))
        .eq(["#", "[", "test", "]"])
}

/// Macros whose expansion declares a `#[test]` function.
fn test_macros(tokens: &[Token<'_>]) -> Vec<String> {
    (0..tokens.len())
        .filter(|&index| at(tokens, index) == "macro_rules" && at(tokens, index + 1) == "!")
        .filter(|&index| {
            let end = item_end(tokens, index + 3);
            (index..end).any(|at_test| is_test_attribute(tokens, at_test))
        })
        .map(|index| at(tokens, index + 2).to_owned())
        .collect()
}

/// Names of tests that run a function of the same name: a `#[test] fn`
/// whose body calls it, or a name passed to a macro that declares tests.
fn self_named_tests(
    tokens: &[Token<'_>],
    candidates: &BTreeSet<String>,
    macros: &BTreeSet<String>,
) -> Vec<String> {
    let mut found = Vec::new();
    for index in 0..tokens.len() {
        if macros.contains(at(tokens, index)) && at(tokens, index + 1) == "!" {
            let end = item_end(tokens, index + 2);
            found.extend(
                (index + 2..end)
                    .map(|inner| at(tokens, inner))
                    .filter(|word| candidates.contains(*word))
                    .map(str::to_owned),
            );
            continue;
        }
        if !is_test_attribute(tokens, index) {
            continue;
        }
        let mut next = index + 4;
        while let Some(end) = attribute_end(tokens, next) {
            next = end;
        }
        let name = at(tokens, next + 1);
        if at(tokens, next) != "fn" || !candidates.contains(name) {
            continue;
        }
        let end = item_end(tokens, next);
        if (next + 2..end).any(|i| at(tokens, i) == name && at(tokens, i + 1) == "(") {
            found.push(name.to_owned());
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if entry.file_name() != "target" {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn file_module(src: &str, rel: &str) -> String {
    let stem = rel
        .strip_prefix(src)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(rel)
        .trim_end_matches(".rs");
    let stem = stem.strip_suffix("/mod").unwrap_or(stem);
    if matches!(stem, "lib" | "main") {
        String::new()
    } else {
        stem.replace('/', "::")
    }
}

struct GatedModule {
    key: String,
    texts: Vec<String>,
    inline: Option<Range<usize>>,
}

/// Modules behind `test-support` in one crate's `src`.
fn gated_modules(root: &Path, krate: &str, src: &str) -> Result<Vec<GatedModule>> {
    let mut files = Vec::new();
    rust_files(&root.join(src), &mut files);
    files.sort();
    let mut modules = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path)?;
        if !text.contains(FEATURE) {
            continue;
        }
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let tokens = tokens(&text);
        for (range, gate) in gated_items(&tokens) {
            if gate != Gate::TestSupport {
                continue;
            }
            let mut declaration = range.start;
            while let Some(end) = attribute_end(&tokens, declaration) {
                declaration = end;
            }
            if at(&tokens, declaration) == "pub" {
                declaration += 1;
                if at(&tokens, declaration) == "(" {
                    declaration = (declaration..tokens.len())
                        .find(|&i| at(&tokens, i) == ")")
                        .map_or(declaration, |close| close + 1);
                }
            }
            if at(&tokens, declaration) != "mod" {
                continue;
            }
            let name = at(&tokens, declaration + 1);
            let key = format!("{krate}\t{}", join(&file_module(src, &rel), name));
            if at(&tokens, declaration + 2) == "{" {
                modules.push(GatedModule {
                    key,
                    texts: vec![text.clone()],
                    inline: Some(range),
                });
                continue;
            }
            let file = match path_attribute(&tokens, declaration) {
                Some(attribute) => rel
                    .rsplit_once('/')
                    .map(|(directory, _)| format!("{directory}/{attribute}")),
                None => test_sources::module_file(root, &rel, name),
            }
            .with_context(|| format!("{rel}: no file for module {name}"))?;
            let mut texts = Vec::new();
            let mut pending = vec![file];
            while let Some(file) = pending.pop() {
                let child_text = fs::read_to_string(root.join(&file))?;
                for (child, _) in scan_file("", &child_text).children {
                    pending.extend(test_sources::module_file(root, &file, &child));
                }
                texts.push(child_text);
            }
            modules.push(GatedModule {
                key,
                texts,
                inline: None,
            });
        }
    }
    Ok(modules)
}

fn test_support_bodies(
    root: &Path,
    graph: &Graph<'_>,
) -> Result<(BTreeMap<String, usize>, Vec<String>)> {
    let mut bodies = BTreeMap::new();
    let mut notes = Vec::new();
    for (krate, package) in &graph.packages {
        let Some(directory) = Path::new(&package.manifest_path).parent() else {
            continue;
        };
        let Ok(relative) = directory.strip_prefix(root) else {
            continue;
        };
        if TOOLING.contains(krate) {
            continue;
        }
        let src = format!("{}/src", relative.to_string_lossy().replace('\\', "/"));
        let modules = gated_modules(root, krate, &src)?;
        let variants = graph.feature_variants(krate, FEATURE);
        for (enabler, duplicated) in &variants {
            notes.push(format!(
                "{krate}/{FEATURE} from {enabler}: {duplicated} workspace crates built twice beside {DUPLICATE_ROOT}"
            ));
        }
        for (owner, feature) in DEV_FEATURES {
            if owner == krate {
                for (enabler, duplicated) in graph.feature_variants(krate, feature) {
                    notes.push(format!(
                        "{krate}/{feature} from {enabler}: {duplicated} workspace crates built twice beside {DUPLICATE_ROOT}"
                    ));
                }
            }
        }
        if modules.is_empty() {
            continue;
        }
        let mut candidates = BTreeSet::new();
        for module in &modules {
            for text in &module.texts {
                let tokens = tokens(text);
                let tokens = module
                    .inline
                    .clone()
                    .map_or(&tokens[..], |range| &tokens[range]);
                candidates.extend(public_fns(tokens).map(str::to_owned));
            }
        }
        let mut called = BTreeSet::new();
        for (enabler, _) in &variants {
            let Some(directory) = Path::new(&graph.packages[enabler].manifest_path).parent() else {
                continue;
            };
            let mut files = Vec::new();
            rust_files(directory, &mut files);
            let mut texts = Vec::new();
            for file in files {
                let text = fs::read_to_string(&file)?;
                if candidates.iter().any(|name| text.contains(name.as_str()))
                    || text.contains("macro_rules")
                {
                    texts.push(text);
                }
            }
            let macros: BTreeSet<String> = texts
                .iter()
                .filter(|text| text.contains("macro_rules") && text.contains("#[test]"))
                .flat_map(|text| test_macros(&tokens(text)))
                .collect();
            for text in &texts {
                if candidates.iter().any(|name| text.contains(name.as_str())) {
                    called.extend(self_named_tests(&tokens(text), &candidates, &macros));
                }
            }
        }
        for module in modules {
            let count: usize = module
                .texts
                .iter()
                .map(|text| {
                    let tokens = tokens(text);
                    let tokens = module
                        .inline
                        .clone()
                        .map_or(&tokens[..], |range| &tokens[range]);
                    body_count(tokens, &called)
                })
                .sum();
            if count > 0 {
                *bodies.entry(module.key).or_default() += count;
            }
        }
    }
    Ok((bodies, notes))
}

/// A known list that may only shrink: new or higher entries fail, and so do
/// entries the tree no longer has, so the list is lowered with the tree.
fn ratchet(name: &str, actual: &BTreeMap<String, usize>, ledger: &str) -> Result<Vec<String>> {
    let mut known = BTreeMap::new();
    for row in tsv::rows(ledger) {
        let (key, count) = tcl_test_support::fixture::tsv_split_last(row)
            .context("a known-list row needs a count")?;
        ensure!(
            known.insert(key, count.parse::<usize>()?).is_none(),
            "{name}: duplicate {key}"
        );
    }
    let mut problems = Vec::new();
    for (key, count) in actual {
        match known.remove(key.as_str()) {
            None => problems.push(format!("{name}: new entry {key} ({count})")),
            Some(old) if *count > old => {
                problems.push(format!("{name}: {key} rose from {old} to {count}"));
            }
            Some(old) if *count < old => {
                problems.push(format!(
                    "{name}: {key} fell from {old} to {count}; lower its row"
                ));
            }
            Some(_) => {}
        }
    }
    problems.extend(
        known
            .keys()
            .map(|key| format!("{name}: {key} is gone; remove its row")),
    );
    Ok(problems)
}

fn shrunk(actual: &BTreeMap<String, usize>, ledger: Option<&str>) -> BTreeMap<String, usize> {
    let Some(ledger) = ledger else {
        return actual.clone();
    };
    tsv::rows(ledger)
        .filter_map(tcl_test_support::fixture::tsv_split_last)
        .filter_map(|(key, count)| {
            let now = *actual.get(key)?;
            Some((key.to_owned(), now.min(count.parse().ok()?)))
        })
        .collect()
}

fn write_ledger(path: &Path, header: &str, rows: impl IntoIterator<Item = String>) -> Result<()> {
    let mut text = header.to_owned();
    for row in rows {
        text.push_str(&row);
        text.push('\n');
    }
    fs::write(path, text).with_context(|| path.display().to_string())
}

pub(super) fn run(root: &Path, write: bool) -> Result<ExitCode> {
    let directory = root.join(DIRECTORY);
    let layers_path = directory.join(LAYERS);
    let tree = compiler_tree(root)?;
    let layers_text = fs::read_to_string(&layers_path).unwrap_or_default();
    let mut ledger = parse_layers(&layers_text)?;
    let metadata = metadata(root)?;
    let graph = Graph::new(&metadata.packages);
    let dev_edges: BTreeMap<String, usize> = graph
        .upward_dev_edges(&deny_layers(root)?)
        .into_iter()
        .map(|(from, to)| (format!("{from}\t{to}"), 1))
        .collect();
    let (bodies, notes) = test_support_bodies(root, &graph)?;

    if write {
        let mut text = layers_text;
        for module in tree.modules.keys() {
            if !module.is_empty() && !module.contains("::") && !ledger.rows.contains_key(module) {
                writeln!(text, "{module}\t{UNASSIGNED}\t0")?;
            }
        }
        fs::write(&layers_path, &text)?;
        ledger = parse_layers(&text)?;
        let pairs = back_edges(&tree, &ledger.rows);
        write_ledger(
            &directory.join(BACK_EDGES),
            "# from\tto\tproduction references\n",
            pairs
                .iter()
                .map(|((from, to), count)| format!("{from}\t{to}\t{count}")),
        )?;
        for (name, header, actual) in [
            (
                DEV_EDGES,
                "# crate\tdev-dependency above it\tcount\n",
                &dev_edges,
            ),
            (
                TEST_BODIES,
                "# crate\tmodule behind test-support\ttest bodies\n",
                &bodies,
            ),
        ] {
            let old = fs::read_to_string(directory.join(name)).ok();
            let rows = shrunk(actual, old.as_deref());
            write_ledger(
                &directory.join(name),
                header,
                rows.iter().map(|(key, count)| format!("{key}\t{count}")),
            )?;
        }
    }

    let mut problems = check_map(root, &tree, &ledger.rows);
    let pairs = back_edges(&tree, &ledger.rows);
    let recorded =
        parse_pairs(&fs::read_to_string(directory.join(BACK_EDGES)).unwrap_or_default())?;
    for (line, fails) in diff_pairs(&pairs, &recorded, ledger.mode) {
        if fails {
            problems.push(line);
        } else {
            println!("module-layers: {line}");
        }
    }
    for (name, actual) in [(DEV_EDGES, &dev_edges), (TEST_BODIES, &bodies)] {
        let known = fs::read_to_string(directory.join(name)).unwrap_or_default();
        problems.extend(ratchet(name, actual, &known)?);
    }
    for line in crate_report(&ledger.rows, &pairs).into_iter().chain(notes) {
        println!("module-layers: {line}");
    }
    for problem in &problems {
        eprintln!("module-layers: {problem}");
    }
    let mode = match ledger.mode {
        Mode::Reporting => "reporting",
        Mode::Enforcing => "enforcing",
    };
    println!(
        "module-layers: {} ({mode}; {} modules; {} back-edge refs in {} pairs; {} known dev-edges; {} test-support modules with bodies)",
        if problems.is_empty() { "OK" } else { "FAILED" },
        ledger.rows.len(),
        pairs.values().sum::<usize>(),
        pairs.len(),
        dev_edges.len(),
        bodies.len(),
    );
    Ok(if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layers(rows: &[(&str, &str, u32)]) -> BTreeMap<String, Layer> {
        rows.iter()
            .map(|(module, krate, rank)| {
                (
                    (*module).to_owned(),
                    Layer {
                        krate: (*krate).to_owned(),
                        rank: *rank,
                    },
                )
            })
            .collect()
    }

    fn tree(files: &[(&str, &str)]) -> Tree {
        let mut tree = Tree::default();
        for (module, text) in files {
            tree.add(&format!("{module}.rs"), module, scan_file(module, text));
        }
        tree.finish();
        tree
    }

    fn package(name: &str, deps: &[(&str, Option<&str>)]) -> Package {
        Package {
            name: name.to_owned(),
            manifest_path: format!("/w/rust/{name}/Cargo.toml"),
            dependencies: deps
                .iter()
                .map(|(dep, kind)| Dependency {
                    name: (*dep).to_owned(),
                    kind: kind.map(str::to_owned),
                    features: Vec::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn a_lower_module_naming_a_higher_one_is_a_back_edge_through_a_re_export() {
        let rows = layers(&[
            ("binding", "tcl-binding", 6),
            ("analyser", "tcl-diagnostics", 11),
            ("analyser::input", "tcl-model", 4),
        ]);
        let tree = tree(&[
            (
                "binding",
                "use crate::analyser::AnalysisResult;\nuse crate::analyser::ResolvedAnalysisInput;\nfn f() { crate::analyser::run(); }",
            ),
            (
                "analyser",
                "mod input;\npub use input::ResolvedAnalysisInput;\npub struct AnalysisResult;",
            ),
            ("analyser::input", "pub struct ResolvedAnalysisInput;"),
        ]);
        let pairs = back_edges(&tree, &rows);
        assert_eq!(
            pairs,
            Pairs::from([(("binding".into(), "analyser".into()), 2)])
        );
        let report = diff_pairs(&pairs, &Pairs::new(), Mode::Reporting);
        assert_eq!(
            report,
            [("new back-edge binding -> analyser (2)".to_owned(), false)]
        );
    }

    #[test]
    fn test_code_is_not_counted() {
        let rows = layers(&[("ir", "tcl-ir", 5), ("binding", "tcl-binding", 6)]);
        let tree = tree(&[(
            "ir",
            "#[cfg(test)]\nmod tests { use crate::binding::X; }\n#[cfg(any(test, feature = \"test-support\"))]\npub fn fixture() { crate::binding::y(); }\n#[cfg(test)]\nuse crate::binding::Z;\nfn production() {}",
        )]);
        assert_eq!(back_edges(&tree, &rows), Pairs::new());
    }

    #[test]
    fn equal_rank_crates_naming_each_other_are_reported() {
        let rows = layers(&[
            ("optimiser", "tcl-optimiser", 10),
            ("native_lowering", "tcl-native-plan", 10),
        ]);
        let tree = tree(&[("optimiser", "fn f() { crate::native_lowering::plan(); }")]);
        assert_eq!(back_edges(&tree, &rows).len(), 1);
    }

    #[test]
    fn a_new_back_edge_fails_only_in_enforcing_mode() {
        let actual = Pairs::from([(("ir".into(), "binding".into()), 3)]);
        let ledger = Pairs::from([(("ir".into(), "binding".into()), 2)]);
        assert!(
            diff_pairs(&actual, &ledger, Mode::Reporting)
                .iter()
                .all(|(_, fails)| !fails)
        );
        assert!(
            diff_pairs(&actual, &ledger, Mode::Enforcing)
                .iter()
                .all(|(_, fails)| *fails)
        );
        assert_eq!(diff_pairs(&actual, &actual, Mode::Enforcing), []);
        let text = "# mode: enforcing\nir\ttcl-ir\t5\n";
        assert_eq!(parse_layers(text).unwrap().mode, Mode::Enforcing);
    }

    #[test]
    fn a_new_upward_dev_dependency_fails() {
        let packages = [
            package("tcl-registry", &[("tcl-spectcl", Some("dev"))]),
            package("tcl-compiler", &[("tcl-registry", None)]),
            package(
                "tcl-spectcl",
                &[("tcl-compiler", None), ("tcl-spectcl", Some("dev"))],
            ),
            package("tcl-vm", &[("tcl-compiler", Some("dev"))]),
            package(
                "tcl-vm-it",
                &[("tcl-vm", Some("dev")), ("tcl-compiler", Some("dev"))],
            ),
            package("tcl-cli", &[("tcl-vm", None)]),
        ];
        let graph = Graph::new(&packages);
        let edges = graph.upward_dev_edges(&BTreeSet::from(["tcl-compiler".to_owned()]));
        assert_eq!(
            edges,
            BTreeSet::from([
                ("tcl-registry".to_owned(), "tcl-spectcl".to_owned()),
                ("tcl-vm".to_owned(), "tcl-compiler".to_owned()),
            ])
        );
        let actual: BTreeMap<String, usize> = edges
            .into_iter()
            .map(|(a, b)| (format!("{a}\t{b}"), 1))
            .collect();
        assert_eq!(
            ratchet(DEV_EDGES, &actual, "tcl-registry\ttcl-spectcl\t1\n")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_new_test_body_behind_test_support_fails() {
        let source = "#[cfg(feature = \"test-support\")]\npub mod bodies {\n    pub fn fixture() {}\n    pub fn keeps_the_world() { fixture(); }\n    pub fn seeded() {}\n    #[test]\n    fn inline() {}\n}\n";
        let tokens = tokens(source);
        let gated = gated_items(&tokens);
        let [(range, Gate::TestSupport)] = gated.as_slice() else {
            panic!("one test-support module");
        };
        let source_of_tests = "#[test]\nfn keeps_the_world() { tcl_compiler::bodies::keeps_the_world(); }\n#[test]\nfn fixture_user() { tcl_compiler::bodies::fixture(); }\nmacro_rules! run { ($($n:ident),*) => { $( #[test] fn $n() { bodies::$n(); } )* } }\nrun!(seeded);";
        let candidates = public_fns(&tokens[range.clone()])
            .map(str::to_owned)
            .collect();
        let test_tokens = super::tokens(source_of_tests);
        let macros = test_macros(&test_tokens).into_iter().collect();
        let called: BTreeSet<_> = self_named_tests(&test_tokens, &candidates, &macros)
            .into_iter()
            .collect();
        assert_eq!(
            called,
            BTreeSet::from(["keeps_the_world".to_owned(), "seeded".to_owned()])
        );
        assert_eq!(body_count(&tokens[range.clone()], &called), 3);
        let actual = BTreeMap::from([("tcl-compiler\tbodies".to_owned(), 2)]);
        assert_eq!(
            ratchet(TEST_BODIES, &actual, "tcl-compiler\tbodies\t1\n")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            ratchet(TEST_BODIES, &actual, "tcl-compiler\tbodies\t2\n").unwrap(),
            [] as [String; 0]
        );
    }
}
