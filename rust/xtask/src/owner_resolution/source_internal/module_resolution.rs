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

//! Loading a crate's modules from source and resolving a path, or a field's type, through them.

use super::declarations::{
    accessible, flatten_use_with_local_precedence, item_name, source_attrs_preserve_routes,
    source_type_attrs_preserve_routes, supported_generics, supported_path_arguments,
    visibility_scope,
};
use super::{CrateModules, EXTERN_CALLER, FieldTypeReferences, Row, SourceModule};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::visit;
use syn::visit::Visit;

impl CrateModules {
    #[cfg(test)]
    pub(super) fn load(
        root: &Path,
        crate_root: &Path,
        required: [&str; 2],
    ) -> Result<Self, String> {
        Self::load_with_scope(root, crate_root, required, false)
    }

    pub(super) fn direct_self_scope(row: &Row) -> Option<(String, String, String)> {
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

    pub(super) fn load_for_route(
        root: &Path,
        crate_root: &Path,
        row: &Row,
    ) -> Result<Self, String> {
        Self::load_with_scope(
            root,
            crate_root,
            [&row.module, &row.consumer_module],
            Self::direct_self_scope(row).is_some(),
        )
    }

    pub(super) fn load_with_scope(
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
        if !super::super::file_attrs_preserve_routes(&file.attrs) {
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
            if !super::super::route_attrs_are_plain(&child.attrs) || child.content.is_some() {
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

    pub(super) fn module_accessible(&self, module: &str, caller: &str) -> bool {
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

    pub(super) fn resolve(&self, module: &str, path: &syn::Path, depth: usize) -> Option<String> {
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
                        if !super::super::route_attrs_are_plain(&import.attrs)
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

    pub(super) fn field_type_reference(&self, row: &Row, file: &syn::File) -> bool {
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
