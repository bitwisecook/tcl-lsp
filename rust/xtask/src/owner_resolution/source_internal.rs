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

//! Exact source declaration and consumer registration, without runtime authority.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

mod consumer_identity;
mod consumer_walk;
mod declarations;
mod module_resolution;
mod row_checks;

#[cfg(test)]
use declarations::{accessible, visibility_scope};
use declarations::{declaration, visibility_name};

const START: &str = "<!-- owner-resolution-source-manifest -->";
const END: &str = "<!-- end-owner-resolution-source-manifest -->";
type Coverage = BTreeSet<(String, String, String)>;

#[derive(Debug, Clone)]
struct Row {
    surface: String,
    source: String,
    crate_name: String,
    module: String,
    entry: String,
    visibility: String,
    consumer_source: String,
    consumer_module: String,
    consumer_entry: String,
    route: String,
}

struct SourceModule {
    access_scope: String,
    /// Declared plain `pub`, so another crate may name it.
    public: bool,
    source: PathBuf,
    file: syn::File,
}

struct CrateModules {
    name: String,
    modules: BTreeMap<String, SourceModule>,
    /// Dependency crates by extern name, for routes another crate's owner
    /// serves. Paths into them resolve with [`EXTERN_CALLER`] access only.
    externs: BTreeMap<String, CrateModules>,
}

/// The caller of a path resolved from another crate: only plain `pub`
/// modules, items and re-exports are reachable.
const EXTERN_CALLER: &str = "<extern>";

fn parse(markdown: &str) -> Result<Vec<Row>, String> {
    let Some((_, tail)) = markdown.split_once(START) else {
        return if markdown.contains(END) {
            Err(format!("missing `{START}`"))
        } else {
            Ok(Vec::new())
        };
    };
    if markdown.matches(START).count() != 1 || markdown.matches(END).count() != 1 {
        return Err("source owner manifest requires one matched marker pair".into());
    }
    let (body, _) = tail
        .split_once(END)
        .ok_or_else(|| format!("missing `{END}`"))?;
    let mut rows = Vec::new();
    for line in body.lines() {
        let cells = super::split_row(line);
        if cells.is_empty() || cells[0] == "Surface" || cells.iter().all(|cell| *cell == "---") {
            continue;
        }
        if cells.len() != 10 || cells[0].is_empty() {
            return Err("source owner row requires ten columns".into());
        }
        rows.push(Row {
            surface: cells[0].into(),
            source: super::single_code_token(cells[1])?,
            crate_name: super::single_code_token(cells[2])?,
            module: super::single_code_token(cells[3])?,
            entry: super::single_code_token(cells[4])?,
            visibility: cells[5].into(),
            consumer_source: super::single_code_token(cells[6])?,
            consumer_module: super::single_code_token(cells[7])?,
            consumer_entry: super::single_code_token(cells[8])?,
            route: super::single_code_token(cells[9])?,
        });
    }
    if rows.is_empty() {
        return Err("source owner manifest has no rows".into());
    }
    Ok(rows)
}

pub(super) fn validate(root: &Path, markdown: &str) -> (Coverage, Vec<String>) {
    let mut coverage = Coverage::new();
    let rows = match parse(markdown) {
        Ok(rows) => rows,
        Err(error) => return (coverage, vec![error]),
    };
    let mut errors = Vec::new();
    let mut cached = BTreeMap::<(PathBuf, Option<(String, String, String)>), CrateModules>::new();
    for row in rows {
        let checked = (|| {
            let package_root = crate_root(&row.source)?;
            let consumer_root = crate_root(&row.consumer_source)?;
            if consumer_root != package_root {
                return check_foreign_route(root, &row, &package_root, &consumer_root);
            }
            let cache_key = (package_root.clone(), CrateModules::direct_self_scope(&row));
            if !cached.contains_key(&cache_key) {
                let modules = CrateModules::load_for_route(root, &package_root, &row)?;
                cached.insert(cache_key.clone(), modules);
            }
            let modules = cached.get(&cache_key).ok_or("missing parsed crate")?;
            if modules.name != row.crate_name {
                return Err("Cargo package identity differs".into());
            }
            let owner = modules
                .modules
                .get(&row.module)
                .ok_or("missing owner module")?;
            let consumer = modules
                .modules
                .get(&row.consumer_module)
                .ok_or("missing consumer module")?;
            if owner.source != Path::new(&row.source)
                || consumer.source != Path::new(&row.consumer_source)
            {
                return Err("registered file does not implement its actual module".into());
            }
            if !modules.check_row(&row) {
                return Err(
                    "declaration, visibility or actual consumer route does not resolve".into(),
                );
            }
            Ok::<_, String>(())
        })();
        match checked {
            Ok(()) => {
                coverage.insert((row.surface, row.source, row.entry));
            }
            Err(error) => errors.push(format!(
                "source owner `{}` `{}`: {error}",
                row.surface, row.entry
            )),
        }
    }
    (coverage, errors)
}

/// A route whose consumer is another crate: only a `static` call of a plain
/// `pub fn`, reached through the consumer's declared dependency on the owner
/// crate and plain `pub` modules or re-exports there.
fn check_foreign_route(
    root: &Path,
    row: &Row,
    owner_root: &Path,
    consumer_root: &Path,
) -> Result<(), String> {
    if row.route != "static" {
        return Err("a foreign consumer crate supports only a static route".into());
    }
    let owner = CrateModules::load_with_scope(root, owner_root, [&row.module, "crate"], false)?;
    if owner.name != row.crate_name {
        return Err("Cargo package identity differs".into());
    }
    {
        let module = owner
            .modules
            .get(&row.module)
            .ok_or("missing owner module")?;
        if module.source != Path::new(&row.source) {
            return Err("registered file does not implement its actual module".into());
        }
        let declaration =
            declaration(&module.file, &row.entry).ok_or("missing owner declaration")?;
        if declaration.owner_type.is_some()
            || declaration.signature.is_none()
            || !matches!(declaration.visibility, syn::Visibility::Public(_))
            || visibility_name(declaration.visibility) != row.visibility
            || !super::file_attrs_preserve_routes(&module.file.attrs)
        {
            return Err("a foreign route owner must be a plain `pub fn`".into());
        }
    }
    let mut consumer =
        CrateModules::load_with_scope(root, consumer_root, [&row.consumer_module, "crate"], false)?;
    let module = consumer
        .modules
        .get(&row.consumer_module)
        .ok_or("missing consumer module")?;
    if module.source != Path::new(&row.consumer_source) {
        return Err("registered file does not implement its actual module".into());
    }
    if !declares_dependency(root, consumer_root, &owner.name)? {
        return Err("consumer crate does not depend on the owner crate".into());
    }
    let extern_name = owner.name.replace('-', "_");
    let target = format!("{extern_name}::{}::{}", row.module, row.entry);
    consumer.externs.insert(extern_name, owner);
    if !consumer.check_foreign_static_row(row, &target) {
        return Err("declaration, visibility or actual consumer route does not resolve".into());
    }
    Ok(())
}

/// Whether `crate_root`'s manifest names `dependency` as an ordinary,
/// unrenamed `[dependencies]` entry.
fn declares_dependency(root: &Path, crate_root: &Path, dependency: &str) -> Result<bool, String> {
    let cargo = std::fs::read_to_string(root.join(crate_root).join("Cargo.toml"))
        .map_err(|error| error.to_string())?;
    let mut section = "";
    for raw in cargo.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = line;
            continue;
        }
        if section != "[dependencies]" {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == dependency
        {
            return Ok(!value.contains("package"));
        }
    }
    Ok(false)
}

fn crate_root(source: &str) -> Result<PathBuf, String> {
    let path = Path::new(source);
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("source path must be repository relative without traversal".into());
    }
    let parts: Vec<_> = path.components().collect();
    let position = parts
        .iter()
        .position(|part| part.as_os_str() == "src")
        .ok_or("source is outside src")?;
    if position == 0 {
        return Err("source has no Cargo crate root".into());
    }
    Ok(parts[..position].iter().collect())
}

struct Declaration<'a> {
    visibility: &'a syn::Visibility,
    signature: Option<&'a syn::Signature>,
    body: Option<&'a syn::Block>,
    owner_type: Option<&'a syn::Path>,
    impl_generics: Option<&'a syn::Generics>,
}

#[derive(Clone)]
struct TypedReference {
    identity: String,
    origin: String,
}

enum DeclaredFieldIdentity {
    Missing,
    Present(String),
}

struct FieldTypeReferences<'a> {
    modules: &'a CrateModules,
    module: &'a str,
    target: String,
    matched: bool,
    invalid: bool,
}

struct ConsumerReferences<'a> {
    modules: &'a CrateModules,
    row: &'a Row,
    target: String,
    consumer: Declaration<'a>,
    locals: BTreeMap<String, TypedReference>,
    shadowed: BTreeSet<String>,
    type_import_shadows: BTreeSet<String>,
    matched: bool,
    invalid: bool,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod file_route_tests;
