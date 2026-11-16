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
use syn::visit::{self, Visit};

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

impl CrateModules {
    #[cfg(test)]
    fn load(root: &Path, crate_root: &Path, required: [&str; 2]) -> Result<Self, String> {
        Self::load_with_scope(root, crate_root, required, false)
    }

    fn direct_self_scope(row: &Row) -> Option<(String, String, String)> {
        let owner = row.entry.split_once("::").map(|(owner, _)| owner);
        let consumer = row.consumer_entry.split_once("::").map(|(owner, _)| owner);
        let direct_self = row.route == "self"
            && row.module == row.consumer_module
            && row.source == row.consumer_source
            && owner.is_some()
            && owner == consumer;
        direct_self.then(|| {
            (
                row.module.clone(),
                row.source.clone(),
                owner.unwrap().to_owned(),
            )
        })
    }

    fn load_for_route(root: &Path, crate_root: &Path, row: &Row) -> Result<Self, String> {
        Self::load_with_scope(
            root,
            crate_root,
            [&row.module, &row.consumer_module],
            Self::direct_self_scope(row).is_some(),
        )
    }

    fn load_with_scope(
        root: &Path,
        crate_root: &Path,
        required: [&str; 2],
        direct_self: bool,
    ) -> Result<Self, String> {
        let cargo = std::fs::read_to_string(root.join(crate_root).join("Cargo.toml"))
            .map_err(|error| error.to_string())?;
        let name = cargo
            .split_once("[package]")
            .and_then(|(_, package)| package.split('[').next())
            .and_then(|package| {
                package.lines().find_map(|line| {
                    let (key, value) = line.split_once('=')?;
                    (key.trim() == "name")
                        .then(|| value.trim().strip_prefix('"')?.strip_suffix('"'))
                        .flatten()
                })
            })
            .ok_or("missing literal Cargo package name")?
            .to_owned();
        let mut modules = BTreeMap::new();
        Self::load_module(
            root,
            Self::crate_source(root, crate_root, &cargo)?,
            "crate",
            required,
            ("crate", true),
            direct_self,
            &mut modules,
        )?;
        Ok(Self {
            name,
            modules,
            externs: BTreeMap::new(),
        })
    }

    fn crate_source(root: &Path, crate_root: &Path, cargo: &str) -> Result<PathBuf, String> {
        let library = crate_root.join("src/lib.rs");
        if root.join(&library).is_file() {
            return Ok(library);
        }
        // This bounded fallback models only an explicit conventional binary.
        // Arbitrary Cargo target paths and competing binary roots remain unavailable.
        let mut section = "";
        let mut binaries = 0;
        let mut binary_name = None;
        let mut binary_path = None;
        for raw in cargo.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                if line == "[lib]" {
                    return Err("custom library root is unavailable".into());
                }
                section = line;
                if section == "[[bin]]" {
                    binaries += 1;
                }
                continue;
            }
            if section != "[[bin]]" {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or("unsupported binary target")?;
            let value = value
                .trim()
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .ok_or("nonliteral binary target")?;
            match key.trim() {
                "name" if binary_name.is_none() => binary_name = Some(value),
                "path" if binary_path.is_none() => binary_path = Some(value),
                _ => return Err("ambiguous or unsupported binary target".into()),
            }
        }
        let main = crate_root.join("src/main.rs");
        if binaries != 1
            || binary_name.is_none_or(str::is_empty)
            || binary_path != Some("src/main.rs")
            || !root.join(&main).is_file()
            || root.join(crate_root).join("src/bin").exists()
        {
            return Err("no unique conventional source crate root".into());
        }
        Ok(main)
    }

    fn load_module(
        root: &Path,
        source: PathBuf,
        module: &str,
        required: [&str; 2],
        (access_scope, public): (&str, bool),
        direct_self: bool,
        modules: &mut BTreeMap<String, SourceModule>,
    ) -> Result<(), String> {
        if modules.len() >= 2048 {
            return Err("source module route exceeds bound".into());
        }
        let text =
            std::fs::read_to_string(root.join(&source)).map_err(|error| error.to_string())?;
        let file = syn::parse_file(&text).map_err(|error| error.to_string())?;
        if !super::file_attrs_preserve_routes(&file.attrs) {
            if required
                .iter()
                .any(|needed| *needed == module || needed.starts_with(&format!("{module}::")))
            {
                return Err(format!("conditional required module `{module}`"));
            }
            return Ok(());
        }
        let directory = if source.file_name().is_some_and(|name| {
            name == "lib.rs" || name == "mod.rs" || (module == "crate" && name == "main.rs")
        }) {
            source.parent().ok_or("missing module parent")?.to_owned()
        } else {
            source.with_extension("")
        };
        let mut declarations = BTreeSet::new();
        for item in &file.items {
            let syn::Item::Mod(child) = item else {
                continue;
            };
            let child_name = format!("{module}::{}", child.ident);
            if !super::route_attrs_are_plain(&child.attrs) || child.content.is_some() {
                if required.iter().any(|needed| {
                    *needed == child_name || needed.starts_with(&format!("{child_name}::"))
                }) {
                    return Err(format!("unsupported required module `{child_name}`"));
                }
                continue;
            }
            if !declarations.insert(child_name.clone()) {
                return Err(format!("duplicate module `{child_name}`"));
            }
            if direct_self
                && !required.iter().any(|needed| {
                    *needed == child_name || needed.starts_with(&format!("{child_name}::"))
                })
            {
                continue;
            }
            let direct = directory.join(format!("{}.rs", child.ident));
            let nested = directory.join(child.ident.to_string()).join("mod.rs");
            let available = [direct, nested]
                .into_iter()
                .filter(|path| root.join(path).is_file())
                .collect::<Vec<_>>();
            let [path] = available.as_slice() else {
                return Err(format!("module `{child_name}` has no unique file"));
            };
            let scope =
                visibility_scope(&child.vis, module).ok_or("unsupported file-module visibility")?;
            Self::load_module(
                root,
                path.clone(),
                &child_name,
                required,
                (&scope, matches!(child.vis, syn::Visibility::Public(_))),
                direct_self,
                modules,
            )?;
        }
        if modules
            .insert(
                module.into(),
                SourceModule {
                    access_scope: access_scope.into(),
                    public,
                    source,
                    file,
                },
            )
            .is_some()
        {
            return Err("duplicate module".into());
        }
        Ok(())
    }

    fn module_accessible(&self, module: &str, caller: &str) -> bool {
        let mut current = module;
        loop {
            let Some(item) = self.modules.get(current) else {
                return false;
            };
            if caller == EXTERN_CALLER {
                if !item.public {
                    return false;
                }
            } else if caller != item.access_scope
                && !caller.starts_with(&format!("{}::", item.access_scope))
            {
                return false;
            }
            let Some((parent, _)) = current.rsplit_once("::") else {
                return true;
            };
            current = parent;
        }
    }

    fn resolve(&self, module: &str, path: &syn::Path, depth: usize) -> Option<String> {
        if depth > 16 || path.leading_colon.is_some() {
            return None;
        }
        let parts = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>();
        self.resolve_parts(module, &parts, depth, module)
    }

    fn resolve_parts(
        &self,
        module: &str,
        parts: &[String],
        depth: usize,
        caller: &str,
    ) -> Option<String> {
        let [first, tail @ ..] = parts else {
            return None;
        };
        if depth > 16 {
            return None;
        }
        if let Some(dependency) = self.externs.get(first) {
            // Another crate's owner: resolved from that crate's root, where
            // only plain `pub` modules, items and re-exports are reachable.
            return dependency
                .resolve_parts("crate", tail, depth + 1, EXTERN_CALLER)
                .map(|identity| format!("{first}::{identity}"));
        }
        if first == "crate" {
            for index in (1..parts.len()).rev() {
                let base = parts[..index].join("::");
                if self.modules.contains_key(&base) && self.module_accessible(&base, caller) {
                    return self.resolve_parts(&base, &parts[index..], depth + 1, caller);
                }
            }
            return None;
        }
        if first == "self" {
            return self.resolve_parts(module, tail, depth + 1, caller);
        }
        if first == "super" {
            return self.resolve_parts(
                module.rsplit_once("::")?.0,
                &std::iter::once("self".into())
                    .chain(tail.iter().cloned())
                    .collect::<Vec<_>>(),
                depth + 1,
                caller,
            );
        }
        let file = &self.modules.get(module)?.file;
        // A local declaration takes precedence over glob imports in Rust.
        // Named imports still participate so duplicate bindings cannot pass.
        let local_declaration = file
            .items
            .iter()
            .any(|item| item_name(item).as_deref() == Some(first.as_str()));
        let mut candidates = Vec::new();
        for item in &file.items {
            if item_name(item).as_deref() == Some(first.as_str()) {
                if matches!(item, syn::Item::Mod(_)) {
                    let child = format!("{module}::{first}");
                    if !self.module_accessible(&child, caller) {
                        return None;
                    }
                    candidates.push(self.resolve_parts(&child, tail, depth + 1, caller)?);
                } else {
                    candidates.push(format!("{module}::{}", parts.join("::")));
                }
            }
            if let syn::Item::Use(import) = item {
                let mut bindings = Vec::new();
                flatten_use_with_local_precedence(
                    &import.tree,
                    &mut Vec::new(),
                    &mut bindings,
                    local_declaration,
                )?;
                for (binding, route) in bindings {
                    if binding == *first {
                        if !super::route_attrs_are_plain(&import.attrs)
                            || !accessible(&import.vis, module, caller)
                        {
                            return None;
                        }
                        let route = route
                            .into_iter()
                            .chain(tail.iter().cloned())
                            .collect::<Vec<_>>();
                        candidates.push(self.resolve_parts(module, &route, depth + 1, module)?);
                    }
                }
            }
        }
        let [target] = candidates.as_slice() else {
            return None;
        };
        Some(target.clone())
    }

    fn field_type_reference(&self, row: &Row, file: &syn::File) -> bool {
        let mut candidates = file.items.iter().filter(|item| match item {
            syn::Item::Enum(item) => item.ident == row.consumer_entry.as_str(),
            syn::Item::Struct(item) => item.ident == row.consumer_entry.as_str(),
            _ => false,
        });
        let Some(item) = candidates.next() else {
            return false;
        };
        if candidates.next().is_some() {
            return false;
        }
        let mut references = FieldTypeReferences {
            modules: self,
            module: &row.consumer_module,
            target: format!("{}::{}", row.module, row.entry),
            matched: false,
            invalid: false,
        };
        match item {
            syn::Item::Enum(item) => {
                if !source_type_attrs_preserve_routes(&item.attrs)
                    || !supported_generics(&item.generics)
                    || item.variants.iter().any(|variant| {
                        !source_attrs_preserve_routes(&variant.attrs)
                            || variant.discriminant.is_some()
                            || variant
                                .fields
                                .iter()
                                .any(|field| !source_attrs_preserve_routes(&field.attrs))
                    })
                {
                    return false;
                }
                for variant in &item.variants {
                    for field in &variant.fields {
                        references.visit_type(&field.ty);
                    }
                }
            }
            syn::Item::Struct(item) => {
                if !source_type_attrs_preserve_routes(&item.attrs)
                    || !supported_generics(&item.generics)
                    || item
                        .fields
                        .iter()
                        .any(|field| !source_attrs_preserve_routes(&field.attrs))
                {
                    return false;
                }
                for field in &item.fields {
                    references.visit_type(&field.ty);
                }
            }
            _ => return false,
        }
        references.matched && !references.invalid
    }

    fn supports_row_route(row: &Row) -> bool {
        !(!matches!(
            row.route.as_str(),
            "static" | "type" | "signature-type" | "self"
        ) && ![
            "parameter:",
            "producer:",
            "field:self.",
            "producer-field:",
            "parameter-field:",
        ]
        .iter()
        .any(|prefix| row.route.starts_with(prefix)))
    }

    fn owner_is_accessible(
        &self,
        row: &Row,
        owner: &Declaration<'_>,
        owner_module: &SourceModule,
    ) -> bool {
        if visibility_name(owner.visibility) != row.visibility
            || !accessible(owner.visibility, &row.module, &row.consumer_module)
            || !super::file_attrs_preserve_routes(&owner_module.file.attrs)
        {
            return false;
        }
        if !matches!(row.route.as_str(), "static" | "type" | "signature-type")
            && owner
                .signature
                .is_none_or(|signature| signature.receiver().is_none())
        {
            return false;
        }
        if let Some(owner_type) = owner.owner_type {
            let Some(identity) = self.resolve(&row.module, owner_type, 0) else {
                return false;
            };
            let Some((module, name)) = identity.rsplit_once("::") else {
                return false;
            };
            let Some(type_module) = self.modules.get(module) else {
                return false;
            };
            let Some(ty) = declaration(&type_module.file, name) else {
                return false;
            };
            if !accessible(ty.visibility, module, &row.consumer_module) {
                return false;
            }
        }
        true
    }

    fn initialise_consumer_parameters(
        calls: &mut ConsumerReferences<'_>,
        signature: &syn::Signature,
    ) -> bool {
        for parameter in &signature.generics.params {
            if let syn::GenericParam::Type(parameter) = parameter {
                calls.shadowed.insert(parameter.ident.to_string());
                calls
                    .type_import_shadows
                    .insert(parameter.ident.to_string());
            }
        }
        for argument in &signature.inputs {
            if let syn::FnArg::Typed(parameter) = argument {
                if !super::route_attrs_are_plain(&parameter.attrs) {
                    return false;
                }
                if let syn::Pat::Ident(name) = parameter.pat.as_ref() {
                    calls.shadowed.insert(name.ident.to_string());
                    if let Some(ty) = calls.type_identity(&parameter.ty) {
                        calls.locals.insert(
                            name.ident.to_string(),
                            TypedReference {
                                identity: ty,
                                origin: format!("parameter:{}", name.ident),
                            },
                        );
                    }
                }
            }
        }
        true
    }

    fn row_owner_declaration(&self, row: &Row) -> Option<Declaration<'_>> {
        if !Self::supports_row_route(row) {
            return None;
        }
        let owner_module = self.modules.get(&row.module)?;
        let owner = declaration(&owner_module.file, &row.entry).or_else(|| {
            (row.route == "self")
                .then(|| declaration_for_self_method(&owner_module.file, &row.entry))
                .flatten()
        });
        let owner = owner?;
        if !self.owner_is_accessible(row, &owner, owner_module) {
            return None;
        }
        Some(owner)
    }

    /// The consumer half of a foreign static route: some call in the consumer
    /// body resolves, through the consumer crate's paths, to `target`.
    fn check_foreign_static_row(&self, row: &Row, target: &str) -> bool {
        let Some(consumer_module) = self.modules.get(&row.consumer_module) else {
            return false;
        };
        if !super::file_attrs_preserve_routes(&consumer_module.file.attrs) {
            return false;
        }
        let Some(consumer) = declaration_in_mode(
            &consumer_module.file,
            &row.consumer_entry,
            true,
            false,
            false,
        ) else {
            return false;
        };
        if consumer.impl_generics.is_some_and(|generics| {
            generics
                .params
                .iter()
                .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
        }) {
            return false;
        }
        let (Some(signature), Some(body)) = (consumer.signature, consumer.body) else {
            return false;
        };
        let mut calls = ConsumerReferences {
            modules: self,
            row,
            target: target.to_owned(),
            consumer,
            locals: BTreeMap::new(),
            shadowed: BTreeSet::new(),
            type_import_shadows: BTreeSet::new(),
            matched: false,
            invalid: false,
        };
        if !Self::initialise_consumer_parameters(&mut calls, signature) {
            return false;
        }
        calls.visit_signature(signature);
        calls.visit_block(body);
        calls.matched && !calls.invalid
    }

    fn check_row(&self, row: &Row) -> bool {
        let Some(owner) = self.row_owner_declaration(row) else {
            return false;
        };
        let Some(consumer_module) = self.modules.get(&row.consumer_module) else {
            return false;
        };
        if !super::file_attrs_preserve_routes(&consumer_module.file.attrs) {
            return false;
        }
        // A trait implementation is a real source consumer body, but cannot
        // become an inherent owner declaration or a result-type producer.
        let consumer = declaration_in_mode(
            &consumer_module.file,
            &row.consumer_entry,
            row.route == "static",
            row.route == "self",
            row.route == "self",
        );
        let Some(consumer) = consumer else {
            return false;
        };
        if consumer.impl_generics.is_some_and(|generics| {
            generics
                .params
                .iter()
                .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
        }) {
            return false;
        }
        if row.route == "type" && consumer.signature.is_none() {
            if owner.signature.is_some() || owner.owner_type.is_some() {
                return false;
            }
            return self.field_type_reference(row, &consumer_module.file);
        }
        let Some(signature) = consumer.signature else {
            return false;
        };
        let Some(body) = consumer.body else {
            return false;
        };
        if signature.generics.params.iter().any(|parameter| matches!(parameter, syn::GenericParam::Type(parameter)
            if owner.owner_type.is_some_and(|ty| ty.segments.last().is_some_and(|segment| segment.ident == parameter.ident)))) {
            return false;
        }

        let target = match owner.owner_type {
            Some(ty) => {
                let Some(identity) = self.resolve(&row.module, ty, 0) else {
                    return false;
                };
                format!(
                    "{identity}::{}",
                    row.entry.rsplit("::").next().unwrap_or_default()
                )
            }
            None => format!("{}::{}", row.module, row.entry),
        };
        if row.route == "signature-type" {
            if owner.signature.is_some()
                || owner.owner_type.is_some()
                || signature.inputs.iter().any(|argument| match argument {
                    syn::FnArg::Typed(argument) => !super::route_attrs_are_plain(&argument.attrs),
                    syn::FnArg::Receiver(argument) => {
                        !super::route_attrs_are_plain(&argument.attrs)
                    }
                })
            {
                return false;
            }
            // This route records only an explicit declaration type occurrence.
            // Function body syntax supplies no value/receiver coverage here.
            let mut references = FieldTypeReferences {
                modules: self,
                module: &row.consumer_module,
                target,
                matched: false,
                invalid: false,
            };
            references.visit_signature(signature);
            return references.matched && !references.invalid;
        }
        let mut calls = ConsumerReferences {
            modules: self,
            row,
            target,
            consumer,
            locals: BTreeMap::new(),
            shadowed: BTreeSet::new(),
            type_import_shadows: BTreeSet::new(),
            matched: false,
            invalid: false,
        };
        if !Self::initialise_consumer_parameters(&mut calls, signature) {
            return false;
        }
        calls.visit_signature(signature);
        calls.visit_block(body);
        calls.matched && !calls.invalid
    }
}

fn item_name(item: &syn::Item) -> Option<String> {
    Some(
        match item {
            syn::Item::Struct(item) => &item.ident,
            syn::Item::Enum(item) => &item.ident,
            syn::Item::Type(item) => &item.ident,
            syn::Item::Fn(item) => &item.sig.ident,
            syn::Item::Mod(item) => &item.ident,
            syn::Item::Const(item) => &item.ident,
            syn::Item::Static(item) => &item.ident,
            _ => return None,
        }
        .to_string(),
    )
}

fn flatten_use(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<(String, Vec<String>)>,
) -> Option<()> {
    flatten_use_with_local_precedence(tree, prefix, out, false)
}

fn flatten_use_with_local_precedence(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<(String, Vec<String>)>,
    local_declaration: bool,
) -> Option<()> {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_with_local_precedence(&path.tree, prefix, out, local_declaration)?;
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            out.push((name.ident.to_string(), path));
        }
        syn::UseTree::Rename(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            out.push((name.rename.to_string(), path));
        }
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                flatten_use_with_local_precedence(tree, prefix, out, local_declaration)?;
            }
        }
        syn::UseTree::Glob(_) if !local_declaration => return None,
        syn::UseTree::Glob(_) => {}
    }
    Some(())
}

struct Declaration<'a> {
    visibility: &'a syn::Visibility,
    signature: Option<&'a syn::Signature>,
    body: Option<&'a syn::Block>,
    owner_type: Option<&'a syn::Path>,
    impl_generics: Option<&'a syn::Generics>,
}

fn source_constant_is_literal(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Lit(value) => super::route_attrs_are_plain(&value.attrs),
        syn::Expr::Reference(value) => {
            super::route_attrs_are_plain(&value.attrs)
                && value.mutability.is_none()
                && source_constant_is_literal(&value.expr)
        }
        syn::Expr::MethodCall(value) => {
            super::route_attrs_are_plain(&value.attrs)
                && value.method == "as_slice"
                && value.args.is_empty()
                && value.turbofish.is_none()
                && matches!(value.receiver.as_ref(), syn::Expr::Lit(literal)
                if matches!(literal.lit, syn::Lit::ByteStr(_))
                    && super::route_attrs_are_plain(&literal.attrs))
        }
        syn::Expr::Array(value) => {
            super::route_attrs_are_plain(&value.attrs)
                && value.elems.iter().all(source_constant_is_literal)
        }
        syn::Expr::Tuple(value) => {
            super::route_attrs_are_plain(&value.attrs)
                && value.elems.iter().all(source_constant_is_literal)
        }
        syn::Expr::Paren(value) => {
            super::route_attrs_are_plain(&value.attrs) && source_constant_is_literal(&value.expr)
        }
        syn::Expr::Group(value) => {
            super::route_attrs_are_plain(&value.attrs) && source_constant_is_literal(&value.expr)
        }
        syn::Expr::Repeat(value) => {
            super::route_attrs_are_plain(&value.attrs)
                && source_constant_is_literal(&value.expr)
                && source_constant_is_literal(&value.len)
        }
        _ => false,
    }
}

fn source_attrs_preserve_routes(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().all(|attribute| {
        [
            "doc", "allow", "warn", "deny", "forbid", "expect", "must_use", "inline", "cold",
        ]
        .iter()
        .any(|name| attribute.path().is_ident(name))
    })
}

fn source_type_attrs_preserve_routes(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().all(|attribute| {
        super::type_attrs_are_plain(std::slice::from_ref(attribute))
            || ["allow", "warn", "deny", "forbid", "expect"]
                .iter()
                .any(|name| attribute.path().is_ident(name))
    })
}

fn supported_path_arguments(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .all(|segment| match &segment.arguments {
            syn::PathArguments::None => true,
            syn::PathArguments::AngleBracketed(arguments) => arguments
                .args
                .iter()
                .all(|argument| matches!(argument, syn::GenericArgument::Lifetime(_))),
            syn::PathArguments::Parenthesized(_) => false,
        })
}

fn supported_generics(generics: &syn::Generics) -> bool {
    generics
        .params
        .iter()
        .all(|parameter| matches!(parameter, syn::GenericParam::Lifetime(_)))
}

// Only concrete static call routes may inspect a function with unrelated
// bounded type parameters. Generic owners, results and receiver routes retain
// the existing lifetime-only declaration contract.
fn supported_static_function_generics(generics: &syn::Generics) -> bool {
    generics.where_clause.is_none()
        && generics.params.iter().all(|parameter| match parameter {
            syn::GenericParam::Lifetime(_) => true,
            syn::GenericParam::Type(parameter) => {
                super::route_attrs_are_plain(&parameter.attrs)
                    && parameter.default.is_none()
                    && !parameter.bounds.is_empty()
                    && parameter.bounds.iter().all(|bound| {
                        matches!(bound,
                    syn::TypeParamBound::Trait(bound)
                        if bound.lifetimes.is_none() && bound.maybe.is_none()
                            && bound.modifiers.require_empty().is_ok()
                            && bound.path.leading_colon.is_none()
                            && supported_path_arguments(&bound.path))
                    })
            }
            syn::GenericParam::Const(_) => false,
        })
}

fn declaration<'a>(file: &'a syn::File, entry: &str) -> Option<Declaration<'a>> {
    declaration_with_static_generics(file, entry, false)
}

// An unbounded result parameter does not change a concrete `&self` owner.
// This is only declaration/source-call evidence: generic result producers keep
// using `declaration`, whose lifetime-only contract is unchanged.
fn supported_self_method_generics(signature: &syn::Signature, owner: &syn::Path) -> bool {
    signature
        .receiver()
        .is_some_and(|receiver| matches!(&receiver.kind, syn::ReceiverKind::Reference(..)))
        && signature.generics.where_clause.is_none()
        && signature
            .generics
            .params
            .iter()
            .all(|parameter| match parameter {
                syn::GenericParam::Lifetime(_) => true,
                syn::GenericParam::Type(parameter) => {
                    super::route_attrs_are_plain(&parameter.attrs)
                        && parameter.bounds.is_empty()
                        && parameter.default.is_none()
                        && owner
                            .segments
                            .last()
                            .is_some_and(|segment| segment.ident != parameter.ident)
                }
                syn::GenericParam::Const(_) => false,
            })
}

fn declaration_for_self_method<'a>(file: &'a syn::File, entry: &str) -> Option<Declaration<'a>> {
    declaration_in_mode(file, entry, false, false, true)
}

fn declaration_with_static_generics<'a>(
    file: &'a syn::File,
    entry: &str,
    static_consumer: bool,
) -> Option<Declaration<'a>> {
    declaration_in_mode(file, entry, static_consumer, false, false)
}

fn declaration_in_mode<'a>(
    file: &'a syn::File,
    entry: &str,
    static_consumer: bool,
    trait_self_consumer: bool,
    generic_self_owner: bool,
) -> Option<Declaration<'a>> {
    let (owner, name) = entry
        .split_once("::")
        .map_or((None, entry), |(owner, name)| (Some(owner), name));
    let mut found = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Fn(item)
                if owner.is_none()
                    && item.sig.ident == name
                    && source_attrs_preserve_routes(&item.attrs)
                    && (supported_generics(&item.sig.generics)
                        || (static_consumer
                            && supported_static_function_generics(&item.sig.generics))) =>
            {
                found.push(Declaration {
                    visibility: &item.vis,
                    signature: Some(&item.sig),
                    body: Some(&item.block),
                    owner_type: None,
                    impl_generics: None,
                });
            }
            syn::Item::Struct(item)
                if owner.is_none()
                    && item.ident == name
                    && source_type_attrs_preserve_routes(&item.attrs)
                    && supported_generics(&item.generics) =>
            {
                found.push(Declaration {
                    visibility: &item.vis,
                    signature: None,
                    body: None,
                    owner_type: None,
                    impl_generics: None,
                });
            }
            syn::Item::Enum(item)
                if owner.is_none()
                    && item.ident == name
                    && source_type_attrs_preserve_routes(&item.attrs)
                    && supported_generics(&item.generics) =>
            {
                found.push(Declaration {
                    visibility: &item.vis,
                    signature: None,
                    body: None,
                    owner_type: None,
                    impl_generics: None,
                });
            }
            syn::Item::Impl(implementation)
                if (implementation.trait_.is_none()
                    || (trait_self_consumer
                        && implementation.trait_.as_ref().is_some_and(|(path, _)| {
                            implementation.modifiers.require_empty().is_ok()
                                && implementation.unsafety.is_none()
                                && supported_path_arguments(path)
                        })))
                    && source_attrs_preserve_routes(&implementation.attrs) =>
            {
                let syn::Type::Path(ty) = implementation.self_ty.as_ref() else {
                    continue;
                };
                if ty.qself.is_some()
                    || !supported_path_arguments(&ty.path)
                    || ty
                        .path
                        .segments
                        .last()
                        .is_none_or(|part| Some(part.ident.to_string().as_str()) != owner)
                {
                    continue;
                }
                for item in &implementation.items {
                    if let syn::ImplItem::Fn(item) = item
                        && item.sig.ident == name
                        && source_attrs_preserve_routes(&item.attrs)
                        && (supported_generics(&item.sig.generics)
                            || (static_consumer
                                && supported_static_function_generics(&item.sig.generics))
                            || (generic_self_owner
                                && supported_self_method_generics(&item.sig, &ty.path)))
                        && supported_generics(&implementation.generics)
                    {
                        found.push(Declaration {
                            visibility: &item.vis,
                            signature: Some(&item.sig),
                            body: Some(&item.block),
                            owner_type: Some(&ty.path),
                            impl_generics: Some(&implementation.generics),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    (found.len() == 1).then(|| found.remove(0))
}

fn visibility_name(visibility: &syn::Visibility) -> String {
    match visibility {
        syn::Visibility::Public(_) => "pub".into(),
        syn::Visibility::Inherited => "private".into(),
        syn::Visibility::Restricted(scope) => format!(
            "pub({}{})",
            if scope.in_token.is_some() { "in " } else { "" },
            scope
                .path
                .segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        ),
    }
}

fn visibility_scope(visibility: &syn::Visibility, owner: &str) -> Option<String> {
    let restriction = match visibility {
        syn::Visibility::Public(_) => return Some("crate".into()),
        syn::Visibility::Inherited => return Some(owner.into()),
        syn::Visibility::Restricted(restriction) => restriction,
    };
    if restriction.path.leading_colon.is_some()
        || restriction
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return None;
    }
    let parts = restriction
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    if restriction.in_token.is_none() && parts.len() != 1 {
        return None;
    }
    let mut base = owner.split("::").map(str::to_owned).collect::<Vec<_>>();
    if base.first().is_none_or(|part| part != "crate") {
        return None;
    }
    let mut rest = parts.as_slice();
    match rest.first()?.as_str() {
        "crate" => {
            base.truncate(1);
            rest = &rest[1..];
        }
        "self" => {
            rest = &rest[1..];
        }
        "super" => {
            while rest.first().is_some_and(|part| part == "super") {
                if base.len() == 1 {
                    return None;
                }
                base.pop();
                rest = &rest[1..];
            }
        }
        _ => return None,
    }
    if rest
        .iter()
        .any(|part| matches!(part.as_str(), "crate" | "self" | "super"))
    {
        return None;
    }
    base.extend(rest.iter().cloned());
    let scope = base.join("::");
    (owner == scope || owner.starts_with(&format!("{scope}::"))).then_some(scope)
}

fn accessible(visibility: &syn::Visibility, owner: &str, consumer: &str) -> bool {
    if consumer == EXTERN_CALLER {
        return matches!(visibility, syn::Visibility::Public(_));
    }
    visibility_scope(visibility, owner)
        .is_some_and(|scope| consumer == scope || consumer.starts_with(&format!("{scope}::")))
}

fn pattern_bindings(pattern: &syn::Pat) -> Option<BTreeSet<String>> {
    #[derive(Default)]
    struct Bindings {
        names: BTreeSet<String>,
        invalid: bool,
    }
    impl<'ast> Visit<'ast> for Bindings {
        fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
            self.names.insert(pattern.ident.to_string());
            visit::visit_pat_ident(self, pattern);
        }
        fn visit_pat(&mut self, pattern: &'ast syn::Pat) {
            if matches!(pattern, syn::Pat::Macro(_) | syn::Pat::Verbatim(_)) {
                self.invalid = true;
                return;
            }
            visit::visit_pat(self, pattern);
        }
    }
    let mut bindings = Bindings::default();
    bindings.visit_pat(pattern);
    (!bindings.invalid).then_some(bindings.names)
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

impl<'ast> Visit<'ast> for FieldTypeReferences<'_> {
    fn visit_type_macro(&mut self, _ty: &'ast syn::TypeMacro) {
        self.invalid = true;
    }

    fn visit_type(&mut self, ty: &'ast syn::Type) {
        if matches!(ty, syn::Type::Macro(_) | syn::Type::Verbatim(_)) {
            self.invalid = true;
            return;
        }
        visit::visit_type(self, ty);
    }

    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        if ty.qself.is_some() {
            self.invalid = true;
            return;
        }
        if self.modules.resolve(self.module, &ty.path, 0).as_ref() == Some(&self.target) {
            if !supported_path_arguments(&ty.path) {
                self.invalid = true;
                return;
            }
            self.matched = true;
        }
        visit::visit_type_path(self, ty);
    }
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

impl ConsumerReferences<'_> {
    /// `Option::map(Owner::method)` applies the owner path to the same single
    /// value the closure form binds.
    fn option_map_applies_target(&self, call: &syn::ExprMethodCall) -> bool {
        if call.method == "map"
            && call.turbofish.is_none()
            && super::route_attrs_are_plain(&call.attrs)
            && call.args.len() == 1
            && let Some(syn::Expr::Path(path)) = call.args.first()
            && path.qself.is_none()
            && super::route_attrs_are_plain(&path.attrs)
            && self
                .modules
                .resolve(&self.row.consumer_module, &path.path, 0)
                .as_ref()
                == Some(&self.target)
            && let Some((target_type, _)) = self.target.rsplit_once("::")
            && let Some(value) = self.expression_identity(&call.receiver)
            && value
                .identity
                .strip_prefix("Option<")
                .and_then(|inner| inner.strip_suffix('>'))
                .is_some_and(|inner| Self::unwrapped_pointer(inner).unwrap_or(inner) == target_type)
            && value.origin == self.row.route
        {
            return true;
        }
        false
    }

    fn parameter_origin_is_shadowed(&self, value: &TypedReference) -> bool {
        struct ShadowedType<'a> {
            names: &'a BTreeSet<String>,
            found: bool,
        }
        impl<'ast> Visit<'ast> for ShadowedType<'_> {
            fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
                if ty.path.leading_colon.is_none()
                    && ty
                        .path
                        .segments
                        .first()
                        .is_some_and(|segment| self.names.contains(&segment.ident.to_string()))
                {
                    self.found = true;
                }
                visit::visit_type_path(self, ty);
            }
        }
        let name = value.origin.strip_prefix("parameter:").or_else(|| {
            value
                .origin
                .strip_prefix("parameter-field:")?
                .split_once(':')
                .map(|(name, _)| name)
        });
        let Some(name) = name else {
            return false;
        };
        let Some(signature) = self.consumer.signature else {
            return true;
        };
        let Some(parameter) = signature.inputs.iter().find_map(|argument| match argument {
            syn::FnArg::Typed(parameter) if matches!(parameter.pat.as_ref(), syn::Pat::Ident(binding) if binding.ident == name) => Some(parameter),
            _ => None,
        }) else { return true; };
        // A body import cannot donate a nominal type spelling to the bounded
        // parameter proof. Qualified declaration paths retain their own origin.
        let mut query = ShadowedType {
            names: &self.type_import_shadows,
            found: false,
        };
        query.visit_type(&parameter.ty);
        query.found
    }

    fn clear_pattern(&mut self, pattern: &syn::Pat) {
        let Some(names) = pattern_bindings(pattern) else {
            self.invalid = true;
            return;
        };
        for name in names {
            self.locals.remove(&name);
            self.shadowed.insert(name);
        }
    }
    fn standard_binding(item: &syn::Item, name: &str) -> bool {
        item_name(item).as_deref() == Some(name)
            || matches!(item, syn::Item::Union(item) if item.ident == name)
            || matches!(item, syn::Item::Trait(item) if item.ident == name)
            || matches!(item, syn::Item::TraitAlias(item) if item.ident == name)
            || matches!(item, syn::Item::ExternCrate(item) if item.rename.as_ref().map_or(&item.ident, |(_, name)| name) == name)
    }

    fn standard_path(&self, module: &str, path: &syn::Path, expected: &str) -> bool {
        if path.leading_colon.is_some()
            || self.modules.modules.contains_key("crate::std")
            || path
                .segments
                .iter()
                .take(path.segments.len().saturating_sub(1))
                .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
        {
            return false;
        }
        let Some(file) = self.modules.modules.get(module).map(|module| &module.file) else {
            return false;
        };
        if [module, "crate"].into_iter().any(|scope| {
            self.modules.modules.get(scope).is_some_and(|module| module.file.items.iter().any(|item| {
                Self::standard_binding(item, "std")
                    || matches!(item, syn::Item::Use(import) if super::use_declares(&import.tree, "std"))
            }))
        }) {
            return false;
        }
        let written = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if written == expected {
            return true;
        }
        if path.segments.len() != 1 {
            return false;
        }
        let local = written.as_str();
        let mut imports = Vec::new();
        for item in &file.items {
            if Self::standard_binding(item, local) {
                return false;
            }
            if let syn::Item::Use(import) = item {
                let mut bindings = Vec::new();
                if flatten_use(&import.tree, &mut Vec::new(), &mut bindings).is_none() {
                    return false;
                }
                for (name, route) in bindings {
                    if name == local {
                        if !super::route_attrs_are_plain(&import.attrs) {
                            return false;
                        }
                        imports.push(route.join("::"));
                    }
                }
            }
        }
        if !imports.is_empty() {
            return imports.as_slice() == [expected];
        }
        matches!(
            expected,
            "std::option::Option"
                | "std::vec::Vec"
                | "std::boxed::Box"
                | "std::iter::Iterator"
                | "std::clone::Clone"
        ) && local == expected.rsplit("::").next().unwrap_or_default()
    }

    fn unwrapped_pointer(identity: &str) -> Option<&str> {
        ["Arc<", "Rc<", "Box<"]
            .into_iter()
            .find_map(|prefix| identity.strip_prefix(prefix)?.strip_suffix('>'))
    }

    fn bind_option_pattern(&mut self, pattern: &syn::Pat, value: TypedReference) {
        let syn::Pat::TupleStruct(pattern) = pattern else {
            return;
        };
        if !super::route_attrs_are_plain(&pattern.attrs)
            || pattern.elems.len() != 1
            || !pattern.path.is_ident("Some")
            || self.shadowed.contains("Some")
            || self.modules.modules.get(&self.row.consumer_module).is_none_or(|module| {
                module.file.items.iter().any(|item| {
                    Self::standard_binding(item, "Some")
                        || matches!(item, syn::Item::Use(import) if super::use_declares(&import.tree, "Some"))
                })
            })
        {
            return;
        }
        let syn::Pat::Ident(binding) = &pattern.elems[0] else {
            return;
        };
        if binding.subpat.is_some() || !super::route_attrs_are_plain(&binding.attrs) {
            return;
        }
        let Some(inner) = value
            .identity
            .strip_prefix("Option<")
            .and_then(|value| value.strip_suffix('>'))
        else {
            return;
        };
        self.locals.insert(
            binding.ident.to_string(),
            TypedReference {
                identity: inner.into(),
                origin: value.origin,
            },
        );
    }

    fn bind_enum_pattern(&mut self, pattern: &syn::Pat, value: &TypedReference) {
        let syn::Pat::TupleStruct(pattern) = pattern else {
            return;
        };
        if !super::route_attrs_are_plain(&pattern.attrs) || pattern.elems.len() != 1 {
            return;
        }
        let syn::Pat::Ident(binding) = &pattern.elems[0] else {
            return;
        };
        if binding.subpat.is_some() || !super::route_attrs_are_plain(&binding.attrs) {
            return;
        }
        let identity = Self::unwrapped_pointer(&value.identity).unwrap_or(&value.identity);
        let Some((module, name)) = identity.rsplit_once("::") else {
            return;
        };
        let Some(source) = self.modules.modules.get(module) else {
            return;
        };
        let mut enums = source.file.items.iter().filter_map(|item| match item {
            syn::Item::Enum(item) if item.ident == name => Some(item),
            _ => None,
        });
        let Some(item) = enums.next() else {
            return;
        };
        if enums.next().is_some()
            || !source_type_attrs_preserve_routes(&item.attrs)
            || !supported_generics(&item.generics)
            || !accessible(&item.vis, module, &self.row.consumer_module)
            || item.variants.iter().any(|variant| {
                !source_attrs_preserve_routes(&variant.attrs)
                    || variant.discriminant.is_some()
                    || variant
                        .fields
                        .iter()
                        .any(|field| !source_attrs_preserve_routes(&field.attrs))
            })
        {
            return;
        }
        let Some(variant_name) = pattern
            .path
            .segments
            .last()
            .map(|part| part.ident.to_string())
        else {
            return;
        };
        if !supported_path_arguments(&pattern.path)
            || self
                .modules
                .resolve(&self.row.consumer_module, &pattern.path, 0)
                .as_deref()
                != Some(format!("{identity}::{variant_name}").as_str())
        {
            return;
        }
        let mut variants = item
            .variants
            .iter()
            .filter(|variant| variant.ident == variant_name);
        let Some(variant) = variants.next() else {
            return;
        };
        if variants.next().is_some() {
            return;
        }
        let syn::Fields::Unnamed(fields) = &variant.fields else {
            return;
        };
        if fields.unnamed.len() != 1 {
            return;
        }
        let Some(field_identity) = self.type_identity_in(module, &fields.unnamed[0].ty, None)
        else {
            return;
        };
        self.locals.insert(
            binding.ident.to_string(),
            TypedReference {
                identity: field_identity,
                origin: format!("{}:variant:{variant_name}:0", value.origin),
            },
        );
    }

    fn type_identity_in(
        &self,
        module: &str,
        ty: &syn::Type,
        self_type: Option<&syn::Path>,
    ) -> Option<String> {
        match ty {
            syn::Type::Reference(ty) => self.type_identity_in(module, &ty.elem, self_type),
            syn::Type::ImplTrait(ty) => {
                let mut item = None;
                for bound in &ty.bounds {
                    let syn::TypeParamBound::Trait(bound) = bound else {
                        return None;
                    };
                    if bound.lifetimes.is_some()
                        || bound.maybe.is_some()
                        || bound.modifiers.require_empty().is_err()
                    {
                        return None;
                    }
                    if self.standard_path(module, &bound.path, "std::clone::Clone")
                        && matches!(
                            bound.path.segments.last()?.arguments,
                            syn::PathArguments::None
                        )
                    {
                        continue;
                    }
                    if !self.standard_path(module, &bound.path, "std::iter::Iterator")
                        || item.is_some()
                    {
                        return None;
                    }
                    let syn::PathArguments::AngleBracketed(arguments) =
                        &bound.path.segments.last()?.arguments
                    else {
                        return None;
                    };
                    let arguments = arguments.args.iter().collect::<Vec<_>>();
                    let [syn::GenericArgument::AssocType(binding)] = arguments.as_slice() else {
                        return None;
                    };
                    if binding.ident != "Item" || binding.generics.is_some() {
                        return None;
                    }
                    item = Some(self.type_identity_in(module, &binding.ty, self_type)?);
                }
                Some(format!("Iter<{}>", item?))
            }
            syn::Type::Path(ty) if ty.qself.is_none() => {
                if ty.path.is_ident("Self") {
                    return self.modules.resolve(module, self_type?, 0);
                }
                let last = ty.path.segments.last()?;
                if let Some((wrapper, _expected)) = [
                    ("Option", "std::option::Option"),
                    ("Vec", "std::vec::Vec"),
                    ("Arc", "std::sync::Arc"),
                    ("Rc", "std::rc::Rc"),
                    ("Box", "std::boxed::Box"),
                ]
                .into_iter()
                .find(|(_, expected)| self.standard_path(module, &ty.path, expected))
                {
                    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
                        return None;
                    };
                    let arguments = arguments.args.iter().collect::<Vec<_>>();
                    let [syn::GenericArgument::Type(inner)] = arguments.as_slice() else {
                        return None;
                    };
                    return Some(format!(
                        "{wrapper}<{}>",
                        self.type_identity_in(module, inner, self_type)?
                    ));
                }
                supported_path_arguments(&ty.path).then_some(())?;
                self.modules.resolve(module, &ty.path, 0)
            }
            _ => None,
        }
    }
    fn type_identity(&self, ty: &syn::Type) -> Option<String> {
        self.type_identity_in(&self.row.consumer_module, ty, self.consumer.owner_type)
    }
    fn function_result(&self, module: &str, entry: &str) -> Option<String> {
        let declaration = declaration(&self.modules.modules.get(module)?.file, entry)?;
        let syn::ReturnType::Type(_, result) = &declaration.signature?.output else {
            return None;
        };
        self.type_identity_in(module, result, declaration.owner_type)
    }
    fn declared_field_identity(
        &self,
        identity: &str,
        field: &syn::Ident,
    ) -> Option<DeclaredFieldIdentity> {
        let identity = Self::unwrapped_pointer(identity).unwrap_or(identity);
        let (module, owner) = identity.rsplit_once("::")?;
        let file = &self.modules.modules.get(module)?.file;
        let mut structures = file.items.iter().filter_map(|item| match item {
            syn::Item::Struct(item) if item.ident == owner => Some(item),
            _ => None,
        });
        let structure = structures.next()?;
        if structures.next().is_some()
            || !source_type_attrs_preserve_routes(&structure.attrs)
            || !supported_generics(&structure.generics)
        {
            return None;
        }
        let mut fields = structure
            .fields
            .iter()
            .filter(|item| item.ident.as_ref() == Some(field));
        let Some(selected) = fields.next() else {
            return Some(DeclaredFieldIdentity::Missing);
        };
        if fields.next().is_some()
            || !super::route_attrs_are_plain(&selected.attrs)
            || !accessible(&selected.vis, module, &self.row.consumer_module)
        {
            return None;
        }
        Some(DeclaredFieldIdentity::Present(self.type_identity_in(
            module,
            &selected.ty,
            None,
        )?))
    }

    fn transparent_method_field<'a>(
        &self,
        module: &str,
        method: &'a syn::ImplItemFn,
        mutable: bool,
        target: &str,
    ) -> Option<&'a syn::Ident> {
        let signature = &method.sig;
        if signature.ident != if mutable { "deref_mut" } else { "deref" }
            || !source_attrs_preserve_routes(&method.attrs)
            || method.modifiers.require_empty().is_err()
            || !matches!(method.vis, syn::Visibility::Inherited)
            || signature.constness.is_some()
            || signature.asyncness.is_some()
            || !matches!(signature.safety, syn::Safety::Default)
            || signature.abi.is_some()
            || signature.variadic.is_some()
            || !signature.generics.params.is_empty()
            || signature.generics.where_clause.is_some()
            || signature.inputs.len() != 1
        {
            return None;
        }
        let receiver = signature.receiver()?;
        let syn::ReceiverKind::Reference(_, None, receiver_mutability) = &receiver.kind else {
            return None;
        };
        if !super::route_attrs_are_plain(&receiver.attrs)
            || receiver.mutability.is_some()
            || receiver_mutability.is_some() != mutable
        {
            return None;
        }
        let syn::ReturnType::Type(_, result) = &signature.output else {
            return None;
        };
        let syn::Type::Reference(result) = result.as_ref() else {
            return None;
        };
        if result.mutability.is_some() != mutable || result.lifetime.is_some() {
            return None;
        }
        let target_projection = matches!(result.elem.as_ref(), syn::Type::Path(path)
            if path.qself.is_none() && path.path.leading_colon.is_none()
            && path.path.segments.len() == 2
            && path.path.segments[0].ident == "Self" && path.path.segments[1].ident == "Target"
            && path.path.segments.iter().all(|segment| matches!(segment.arguments, syn::PathArguments::None)));
        if !target_projection
            && self.type_identity_in(module, &result.elem, None).as_deref() != Some(target)
        {
            return None;
        }
        let [syn::Stmt::Expr(syn::Expr::Reference(reference), None)] =
            method.block.stmts.as_slice()
        else {
            return None;
        };
        if !super::route_attrs_are_plain(&reference.attrs)
            || reference.mutability.is_some() != mutable
        {
            return None;
        }
        let syn::Expr::Field(field) = reference.expr.as_ref() else {
            return None;
        };
        if !super::route_attrs_are_plain(&field.attrs)
            || !matches!(field.base.as_ref(), syn::Expr::Path(path) if path.qself.is_none() && path.attrs.is_empty() && path.path.is_ident("self"))
        {
            return None;
        }
        let syn::Member::Named(field) = &field.member else {
            return None;
        };
        Some(field)
    }

    fn transparent_state_target(&self, identity: &str) -> Option<String> {
        let (module, _) = identity.rsplit_once("::")?;
        let file = &self.modules.modules.get(module)?.file;
        let mut dereferences = Vec::new();
        let mut mutable_dereferences = Vec::new();
        for item in &file.items {
            let syn::Item::Impl(implementation) = item else {
                continue;
            };
            let Some((trait_path, _)) = &implementation.trait_ else {
                continue;
            };
            if self
                .type_identity_in(module, &implementation.self_ty, None)
                .as_deref()
                != Some(identity)
            {
                continue;
            }
            let destination = if self.standard_path(module, trait_path, "std::ops::Deref") {
                &mut dereferences
            } else if self.standard_path(module, trait_path, "std::ops::DerefMut") {
                &mut mutable_dereferences
            } else {
                continue;
            };
            if !source_attrs_preserve_routes(&implementation.attrs)
                || implementation.modifiers.require_empty().is_err()
                || implementation.unsafety.is_some()
                || !implementation.generics.params.is_empty()
                || implementation.generics.where_clause.is_some()
            {
                return None;
            }
            destination.push(implementation);
        }
        let [immutable] = dereferences.as_slice() else {
            return None;
        };
        let [mutable] = mutable_dereferences.as_slice() else {
            return None;
        };
        if immutable.items.len() != 2 || mutable.items.len() != 1 {
            return None;
        }
        let mut target = None;
        let mut immutable_method = None;
        for item in &immutable.items {
            match item {
                syn::ImplItem::Type(item)
                    if item.ident == "Target"
                        && super::route_attrs_are_plain(&item.attrs)
                        && item.modifiers.require_empty().is_ok()
                        && matches!(item.vis, syn::Visibility::Inherited)
                        && item.generics.params.is_empty()
                        && item.generics.where_clause.is_none() =>
                {
                    if target.is_some() {
                        return None;
                    }
                    target = Some(self.type_identity_in(module, &item.ty, None)?);
                }
                syn::ImplItem::Fn(method) if immutable_method.is_none() => {
                    immutable_method = Some(method);
                }
                _ => return None,
            }
        }
        let target = target?;
        let [syn::ImplItem::Fn(mutable_method)] = mutable.items.as_slice() else {
            return None;
        };
        let mut selected_field = None;
        for (method, mutable) in [(immutable_method?, false), (mutable_method, true)] {
            let field = self.transparent_method_field(module, method, mutable, &target)?;
            if selected_field
                .as_ref()
                .is_some_and(|selected| selected != field)
            {
                return None;
            }
            selected_field = Some(field.clone());
        }
        let DeclaredFieldIdentity::Present(field_type) =
            self.declared_field_identity(identity, &selected_field?)?
        else {
            return None;
        };
        (field_type == format!("Box<{target}>")).then_some(target)
    }

    fn field(&self, value: &TypedReference, field: &syn::Member) -> Option<TypedReference> {
        let syn::Member::Named(field) = field else {
            return None;
        };
        let identity = match self.declared_field_identity(&value.identity, field)? {
            DeclaredFieldIdentity::Present(identity) => identity,
            DeclaredFieldIdentity::Missing if value.origin == "self" => {
                let target = self.transparent_state_target(&value.identity)?;
                let DeclaredFieldIdentity::Present(identity) =
                    self.declared_field_identity(&target, field)?
                else {
                    return None;
                };
                identity
            }
            DeclaredFieldIdentity::Missing => return None,
        };
        let origin = if value.origin == "self" {
            format!("field:self.{field}")
        } else if let Some(fields) = value.origin.strip_prefix("field:self.") {
            if fields.split('.').count() >= 2 {
                return None;
            }
            format!("field:self.{fields}.{field}")
        } else if let Some(parameter) = value.origin.strip_prefix("parameter:") {
            format!("parameter-field:{parameter}:{field}")
        } else if let Some(previous) = value.origin.strip_prefix("parameter-field:") {
            let (parameter, fields) = previous.split_once(':')?;
            if fields.split('.').count() >= 2 {
                return None;
            }
            format!("parameter-field:{parameter}:{fields}.{field}")
        } else if let Some(producer) = value.origin.strip_prefix("self-producer:") {
            format!("producer-field:{producer}:{field}")
        } else {
            let producer = value.origin.strip_prefix("producer:")?;
            format!("producer-field:{producer}:{field}")
        };
        Some(TypedReference { identity, origin })
    }
    fn method_expression_identity(&self, call: &syn::ExprMethodCall) -> Option<TypedReference> {
        let mut value = self.expression_identity(&call.receiver)?;
        let name = call.method.to_string();
        if matches!(name.as_str(), "as_ref" | "as_mut")
            && call.args.is_empty()
            && call.turbofish.is_none()
            && value.identity.starts_with("Option<")
        {
            return Some(value);
        }
        if name == "as_deref" && call.args.is_empty() {
            let inner = value.identity.strip_prefix("Option<")?.strip_suffix('>')?;
            value.identity = format!("Option<{}>", Self::unwrapped_pointer(inner)?);
            return Some(value);
        }
        if name == "as_ref" && call.args.is_empty() {
            value.identity = Self::unwrapped_pointer(&value.identity)?.into();
            return Some(value);
        }
        if name == "iter" && call.args.is_empty() {
            value.identity = format!(
                "Iter<{}>",
                value.identity.strip_prefix("Vec<")?.strip_suffix('>')?
            );
            return Some(value);
        }
        if name == "skip" && call.args.len() == 1 && value.identity.starts_with("Iter<") {
            return Some(value);
        }
        if name == "zip" && call.args.len() == 1 && value.identity.starts_with("Iter<") {
            value.identity = format!(
                "Zip<{}>",
                value.identity.strip_prefix("Iter<")?.strip_suffix('>')?
            );
            return Some(value);
        }
        if value.origin == "self" {
            let (module, owner) = value.identity.rsplit_once("::")?;
            let entry = format!("{owner}::{name}");
            return Some(TypedReference {
                identity: self.function_result(module, &entry)?,
                origin: format!("self-producer:{entry}"),
            });
        }
        None
    }

    fn expression_identity(&self, expression: &syn::Expr) -> Option<TypedReference> {
        match expression {
            syn::Expr::Path(path) if path.path.is_ident("self") => Some(TypedReference {
                identity: self
                    .consumer
                    .owner_type
                    .and_then(|ty| self.modules.resolve(&self.row.consumer_module, ty, 0))?,
                origin: "self".into(),
            }),
            syn::Expr::Path(path) => {
                let value = self.locals.get(&path.path.get_ident()?.to_string())?;
                (!self.parameter_origin_is_shadowed(value)).then(|| value.clone())
            }
            syn::Expr::Reference(expression) => self.expression_identity(&expression.expr),
            syn::Expr::Paren(expression) => self.expression_identity(&expression.expr),
            syn::Expr::Try(expression) => {
                let mut value = self.expression_identity(&expression.expr)?;
                value.identity = value
                    .identity
                    .strip_prefix("Option<")?
                    .strip_suffix('>')?
                    .into();
                Some(value)
            }
            syn::Expr::Field(field) => {
                self.field(&self.expression_identity(&field.base)?, &field.member)
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                if path.qself.is_some()
                    || path
                        .path
                        .segments
                        .first()
                        .is_some_and(|part| self.shadowed.contains(&part.ident.to_string()))
                {
                    return None;
                }
                let target = self
                    .modules
                    .resolve(&self.row.consumer_module, &path.path, 0)?;
                for index in target.rmatch_indices("::").map(|(index, _)| index) {
                    let module = &target[..index];
                    if self.modules.modules.contains_key(module) {
                        return Some(TypedReference {
                            identity: self.function_result(module, &target[index + 2..])?,
                            origin: format!("producer:{target}"),
                        });
                    }
                }
                None
            }
            syn::Expr::MethodCall(call) => self.method_expression_identity(call),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for ConsumerReferences<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.row.route == "static"
            && super::route_attrs_are_plain(&call.attrs)
            && let syn::Expr::Path(path) = call.func.as_ref()
            && path.qself.is_none()
            && self
                .modules
                .resolve(&self.row.consumer_module, &path.path, 0)
                .as_ref()
                == Some(&self.target)
            && path
                .path
                .segments
                .first()
                .is_none_or(|part| !self.shadowed.contains(&part.ident.to_string()))
        {
            self.matched = true;
        }
        visit::visit_expr_call(self, call);
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if let Some((target_type, member)) = self.target.rsplit_once("::")
            && call.method == member
            && super::route_attrs_are_plain(&call.attrs)
            && let Some(value) = self.expression_identity(&call.receiver)
            && Self::unwrapped_pointer(&value.identity).unwrap_or(&value.identity) == target_type
            && value.origin == self.row.route
        {
            self.matched = true;
        }
        if self.option_map_applies_target(call) {
            self.matched = true;
        }
        // Only the standard Option identity can supply its single map argument.
        // Keep the original receiver route through the closure's local binding.
        if call.method == "map"
            && call.turbofish.is_none()
            && super::route_attrs_are_plain(&call.attrs)
            && call.args.len() == 1
            && let Some(mut value) = self.expression_identity(&call.receiver)
            && let Some(inner) = value
                .identity
                .strip_prefix("Option<")
                .and_then(|value| value.strip_suffix('>'))
            && let Some(syn::Expr::Closure(closure)) = call.args.first()
            && closure.inputs.len() == 1
            && super::route_attrs_are_plain(&closure.attrs)
            && let Some(syn::Pat::Ident(parameter)) = closure.inputs.first()
            && super::route_attrs_are_plain(&parameter.attrs)
            && parameter.by_ref.is_none()
            && parameter.subpat.is_none()
        {
            value.identity = inner.into();
            let previous = self.locals.clone();
            let shadowed = self.shadowed.clone();
            self.clear_pattern(closure.inputs.first().unwrap());
            self.locals.insert(parameter.ident.to_string(), value);
            self.visit_expr(&closure.body);
            self.locals = previous;
            self.shadowed = shadowed;
            self.visit_expr(&call.receiver);
            return;
        }
        if ["any", "map"].iter().any(|name| call.method == *name)
            && call.args.len() == 1
            && let Some(mut value) = self.expression_identity(&call.receiver)
            && let Some(syn::Expr::Closure(closure)) = call.args.first()
        {
            let tuple = value.identity.starts_with("Zip<");
            let inner = value
                .identity
                .strip_prefix(if tuple { "Zip<" } else { "Iter<" })
                .and_then(|value| value.strip_suffix('>'));
            if let Some(inner) = inner {
                value.identity = inner.into();
                let parameter = closure
                    .inputs
                    .first()
                    .and_then(|parameter| match parameter {
                        syn::Pat::Ident(name) if !tuple => Some(name.ident.to_string()),
                        syn::Pat::Tuple(tuple) => {
                            tuple.elems.first().and_then(|parameter| match parameter {
                                syn::Pat::Ident(name) => Some(name.ident.to_string()),
                                _ => None,
                            })
                        }
                        _ => None,
                    });
                if let Some(parameter) = parameter {
                    if !super::route_attrs_are_plain(&closure.attrs) {
                        self.invalid = true;
                        return;
                    }
                    let previous = self.locals.clone();
                    let shadowed = self.shadowed.clone();
                    for pattern in &closure.inputs {
                        self.clear_pattern(pattern);
                    }
                    self.locals.insert(parameter, value);
                    self.visit_expr(&closure.body);
                    self.locals = previous;
                    self.shadowed = shadowed;
                    self.visit_expr(&call.receiver);
                    return;
                }
            }
        }
        visit::visit_expr_method_call(self, call);
    }
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        if self.row.route == "type"
            && ty.qself.is_none()
            && ty
                .path
                .segments
                .first()
                .is_none_or(|part| !self.shadowed.contains(&part.ident.to_string()))
            && self
                .modules
                .resolve(&self.row.consumer_module, &ty.path, 0)
                .as_ref()
                == Some(&self.target)
        {
            self.matched = true;
        }
        visit::visit_type_path(self, ty);
    }
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        if self.row.route == "type"
            && expression.qself.is_none()
            && super::route_attrs_are_plain(&expression.attrs)
            && supported_path_arguments(&expression.path)
            && expression
                .path
                .segments
                .first()
                .is_none_or(|part| !self.shadowed.contains(&part.ident.to_string()))
            && self
                .modules
                .resolve(&self.row.consumer_module, &expression.path, 0)
                .as_ref()
                == Some(&self.target)
        {
            self.matched = true;
        }
        visit::visit_expr_struct(self, expression);
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if !super::route_attrs_are_plain(&local.attrs) {
            self.invalid = true;
            return;
        }
        if let syn::Pat::Type(typed) = &local.pat {
            self.visit_type(&typed.ty);
        }
        let name = match &local.pat {
            syn::Pat::Ident(name) => Some(name.ident.to_string()),
            syn::Pat::Type(ty) => match ty.pat.as_ref() {
                syn::Pat::Ident(name) => Some(name.ident.to_string()),
                _ => None,
            },
            _ => None,
        };
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, alternative)) = &init.diverge {
                self.visit_expr(alternative);
            }
        }
        let initialized = local
            .init
            .as_ref()
            .and_then(|init| self.expression_identity(&init.expr));
        self.clear_pattern(&local.pat);
        if let Some(value) = initialized.clone() {
            self.bind_option_pattern(&local.pat, value);
        }
        if let Some(name) = name
            && let Some(value) = local
                .init
                .as_ref()
                .and_then(|init| self.expression_identity(&init.expr))
        {
            self.locals.insert(name, value);
        }
    }
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let previous = self.locals.clone();
        let shadowed = self.shadowed.clone();
        if !super::route_attrs_are_plain(&closure.attrs) {
            self.invalid = true;
            return;
        }
        for pattern in &closure.inputs {
            self.clear_pattern(pattern);
        }
        self.visit_expr(&closure.body);
        self.locals = previous;
        self.shadowed = shadowed;
    }
    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        if !super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        self.visit_expr(&expression.expr);
        let item = self
            .expression_identity(&expression.expr)
            .and_then(|value| {
                let identity = value
                    .identity
                    .strip_prefix("Iter<")?
                    .strip_suffix('>')?
                    .into();
                Some(TypedReference {
                    identity,
                    origin: value.origin,
                })
            });
        let previous = self.locals.clone();
        let shadowed = self.shadowed.clone();
        self.clear_pattern(&expression.pat);
        if let (syn::Pat::Ident(binding), Some(value)) = (expression.pat.as_ref(), item)
            && binding.subpat.is_none()
            && super::route_attrs_are_plain(&binding.attrs)
        {
            self.locals.insert(binding.ident.to_string(), value);
        }
        self.visit_block(&expression.body);
        self.locals = previous;
        self.shadowed = shadowed;
    }
    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if !super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        self.visit_expr(&expression.expr);
        let selected = self.expression_identity(&expression.expr);
        for arm in &expression.arms {
            let previous = self.locals.clone();
            let shadowed = self.shadowed.clone();
            if !super::route_attrs_are_plain(&arm.attrs) {
                self.invalid = true;
            }
            self.clear_pattern(&arm.pat);
            if let Some(value) = &selected {
                self.bind_enum_pattern(&arm.pat, value);
            }
            self.visit_pat(&arm.pat);
            self.visit_expr(&arm.body);
            self.locals = previous;
            self.shadowed = shadowed;
        }
    }
    fn visit_expr_let(&mut self, expression: &'ast syn::ExprLet) {
        if !super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        self.visit_expr(&expression.expr);
        let value = self.expression_identity(&expression.expr);
        self.clear_pattern(&expression.pat);
        if let Some(value) = value {
            self.bind_option_pattern(&expression.pat, value);
        }
    }
    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        if !super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        let previous = self.locals.clone();
        let shadowed = self.shadowed.clone();
        self.visit_expr(&expression.cond);
        self.visit_block(&expression.then_branch);
        self.locals = previous.clone();
        self.shadowed = shadowed.clone();
        if let Some((_, alternative)) = &expression.else_branch {
            self.visit_expr(alternative);
        }
        self.locals = previous;
        self.shadowed = shadowed;
    }
    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        if !super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        let previous = self.locals.clone();
        let shadowed = self.shadowed.clone();
        self.visit_expr(&expression.cond);
        self.visit_block(&expression.body);
        self.locals = previous;
        self.shadowed = shadowed;
    }
    fn visit_stmt_macro(&mut self, _statement: &'ast syn::StmtMacro) {
        self.invalid = true;
    }
    fn visit_item_macro(&mut self, _item: &'ast syn::ItemMacro) {
        self.invalid = true;
    }
    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if matches!(
            statement,
            syn::Stmt::Item(syn::Item::Use(_) | syn::Item::Const(_))
        ) {
            // Imports and literal constants are checked and shadowed for the whole block.
            return;
        }
        if matches!(statement, syn::Stmt::Item(_) | syn::Stmt::Macro(_)) {
            self.invalid = true;
            return;
        }
        visit::visit_stmt(self, statement);
    }
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if !source_attrs_preserve_routes(std::slice::from_ref(attribute)) {
            self.invalid = true;
        }
    }
    fn visit_item_type(&mut self, _item: &'ast syn::ItemType) {
        self.invalid = true;
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let previous = self.locals.clone();
        let shadowed = self.shadowed.clone();
        let type_import_shadows = self.type_import_shadows.clone();
        // Rust imports apply to the whole block, including calls before the use.
        for statement in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Const(constant)) = statement {
                if !super::route_attrs_are_plain(&constant.attrs)
                    || !source_constant_is_literal(&constant.expr)
                {
                    self.invalid = true;
                    continue;
                }
                let name = constant.ident.to_string();
                self.locals.remove(&name);
                self.type_import_shadows.insert(name.clone());
                self.shadowed.insert(name);
                continue;
            }
            if let syn::Stmt::Item(syn::Item::Use(import)) = statement {
                let mut bindings = Vec::new();
                if !super::route_attrs_are_plain(&import.attrs)
                    || flatten_use(&import.tree, &mut Vec::new(), &mut bindings).is_none()
                {
                    self.invalid = true;
                    continue;
                }
                for (name, _) in bindings {
                    self.locals.remove(&name);
                    self.type_import_shadows.insert(name.clone());
                    self.shadowed.insert(name);
                }
            }
        }
        visit::visit_block(self, block);
        self.locals = previous;
        self.shadowed = shadowed;
        self.type_import_shadows = type_import_shadows;
    }
    fn visit_expr_macro(&mut self, _expression: &'ast syn::ExprMacro) {}
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod file_route_tests;
