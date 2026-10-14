// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
    source: PathBuf,
    file: syn::File,
}

struct CrateModules {
    name: String,
    modules: BTreeMap<String, SourceModule>,
}

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
            if crate_root(&row.consumer_source)? != package_root {
                return Err("foreign consumer crate".into());
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
            "crate",
            direct_self,
            &mut modules,
        )?;
        Ok(Self { name, modules })
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
        access_scope: &str,
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
                &scope,
                direct_self,
                modules,
            )?;
        }
        if modules
            .insert(
                module.into(),
                SourceModule {
                    access_scope: access_scope.into(),
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
            if caller != item.access_scope
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
                flatten_use(&import.tree, &mut Vec::new(), &mut bindings)?;
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

    fn enum_field_type_reference(&self, row: &Row, file: &syn::File) -> bool {
        let mut candidates = file.items.iter().filter_map(|item| match item {
            syn::Item::Enum(item) if item.ident == row.consumer_entry.as_str() => Some(item),
            _ => None,
        });
        let Some(item) = candidates.next() else {
            return false;
        };
        if candidates.next().is_some()
            || !source_type_attrs_preserve_routes(&item.attrs)
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
        let mut references = EnumFieldTypeReferences {
            modules: self,
            module: &row.consumer_module,
            target: format!("{}::{}", row.module, row.entry),
            matched: false,
            invalid: false,
        };
        for variant in &item.variants {
            for field in &variant.fields {
                references.visit_type(&field.ty);
            }
        }
        references.matched && !references.invalid
    }

    fn check_row(&self, row: &Row) -> bool {
        if !matches!(
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
        .any(|prefix| row.route.starts_with(prefix))
        {
            return false;
        }
        let Some(owner_module) = self.modules.get(&row.module) else {
            return false;
        };
        let Some(owner) = declaration(&owner_module.file, &row.entry) else {
            return false;
        };
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
        let Some(consumer_module) = self.modules.get(&row.consumer_module) else {
            return false;
        };
        if !super::file_attrs_preserve_routes(&consumer_module.file.attrs) {
            return false;
        }
        let Some(consumer) = declaration(&consumer_module.file, &row.consumer_entry) else {
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
            return self.enum_field_type_reference(row, &consumer_module.file);
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
            let mut references = EnumFieldTypeReferences {
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
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use(&path.tree, prefix, out)?;
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
                flatten_use(tree, prefix, out)?;
            }
        }
        syn::UseTree::Glob(_) => return None,
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

fn declaration<'a>(file: &'a syn::File, entry: &str) -> Option<Declaration<'a>> {
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
                    && supported_generics(&item.sig.generics) =>
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
                if implementation.trait_.is_none()
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
                        && supported_generics(&item.sig.generics)
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

struct EnumFieldTypeReferences<'a> {
    modules: &'a CrateModules,
    module: &'a str,
    target: String,
    matched: bool,
    invalid: bool,
}

impl<'ast> Visit<'ast> for EnumFieldTypeReferences<'_> {
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
    fn parameter_origin_is_shadowed(&self, value: &TypedReference) -> bool {
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

    fn bind_enum_pattern(&mut self, pattern: &syn::Pat, value: TypedReference) {
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
    fn field(&self, value: TypedReference, field: &syn::Member) -> Option<TypedReference> {
        let syn::Member::Named(field) = field else {
            return None;
        };
        let identity = Self::unwrapped_pointer(&value.identity).unwrap_or(&value.identity);
        let (module, owner) = identity.rsplit_once("::")?;
        let file = &self.modules.modules.get(module)?.file;
        let mut fields = file
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Struct(item)
                    if item.ident == owner
                        && source_type_attrs_preserve_routes(&item.attrs)
                        && supported_generics(&item.generics) =>
                {
                    Some(item)
                }
                _ => None,
            })
            .flat_map(|item| {
                item.fields
                    .iter()
                    .filter(|item| item.ident.as_ref() == Some(field))
            });
        let selected = fields.next()?;
        if fields.next().is_some()
            || !super::route_attrs_are_plain(&selected.attrs)
            || !accessible(&selected.vis, module, &self.row.consumer_module)
        {
            return None;
        }
        let identity = self.type_identity_in(module, &selected.ty, None)?;
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
        } else if let Some(producer) = value.origin.strip_prefix("producer:") {
            format!("producer-field:{producer}:{field}")
        } else {
            return None;
        };
        Some(TypedReference { identity, origin })
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
                self.field(self.expression_identity(&field.base)?, &field.member)
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
            syn::Expr::MethodCall(call) => {
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
        if let Some(name) = name {
            if let Some(value) = local
                .init
                .as_ref()
                .and_then(|init| self.expression_identity(&init.expr))
            {
                self.locals.insert(name, value);
            }
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
            if let Some(value) = selected.clone() {
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
        if matches!(statement, syn::Stmt::Item(syn::Item::Use(_))) {
            // Plain imports are scoped and shadowed before this block is visited.
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
mod tests {
    use super::*;

    fn fixture(owner: &str, consumer: &str, entry: &str, route: &str) -> (CrateModules, Row) {
        let modules = CrateModules {
            name: "fixture".into(),
            modules: [
                ("crate", ""),
                ("crate::shared", "mod owner; mod caller;"),
                ("crate::shared::owner", owner),
                ("crate::shared::caller", consumer),
            ]
            .into_iter()
            .map(|(module, source)| {
                (
                    module.into(),
                    SourceModule {
                        access_scope: module
                            .rsplit_once("::")
                            .map_or("crate", |(parent, _)| parent)
                            .into(),
                        source: PathBuf::from(format!(
                            "rust/fixture/src/{}.rs",
                            module.replace("::", "/")
                        )),
                        file: syn::parse_file(source).unwrap(),
                    },
                )
            })
            .collect(),
        };
        let row = Row {
            surface: "source".into(),
            source: "rust/fixture/src/shared/owner.rs".into(),
            crate_name: "fixture".into(),
            module: "crate::shared::owner".into(),
            entry: entry.into(),
            visibility: "pub(super)".into(),
            consumer_source: "rust/fixture/src/shared/caller.rs".into(),
            consumer_module: "crate::shared::caller".into(),
            consumer_entry: "consume".into(),
            route: route.into(),
        };
        (modules, row)
    }

    #[test]
    fn source_routes_follow_current_nested_fields_and_standard_pointer_types() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }";
        for pointer in ["std::sync::Arc", "std::rc::Rc", "Box"] {
            let caller = format!(
                "use super::owner::Ledger; struct Module {{ ledger: Option<{pointer}<Ledger>> }} struct Unit {{ module: Module }} fn consume(unit: &Unit) -> Option<()> {{ let module = &unit.module; let ledger = module.ledger.as_deref()?; ledger.inspect(); Some(()) }}"
            );
            let (modules, row) = fixture(
                owner,
                &caller,
                "Ledger::inspect",
                "parameter-field:unit:module.ledger",
            );
            assert!(modules.check_row(&row), "{pointer}");
        }
        let caller = "use super::owner::Ledger; use std::sync::Arc as Shared; struct Module { ledger: Option<Shared<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let module = &unit.module; let ledger = module.ledger.as_deref()?; ledger.inspect(); Some(()) }";
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "parameter-field:unit:module.ledger",
        );
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_routes_refuse_shadowed_pointers_and_foreign_nested_field_origins() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }";
        for caller in [
            "use super::owner::Ledger; struct Arc<T>(T); struct Module { ledger: Option<Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
            "use super::owner::Ledger; type Shared<T> = std::sync::Arc<T>; struct Module { ledger: Option<Shared<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
            "use super::owner::Ledger; struct Module { #[cfg(any())] ledger: Option<std::sync::Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
            "use super::owner::Ledger; struct Module { ledger: Option<std::sync::Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit, other: &Unit) -> Option<()> { let unit = other; let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
        ] {
            let (modules, row) = fixture(
                owner,
                caller,
                "Ledger::inspect",
                "parameter-field:unit:module.ledger",
            );
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_routes_bind_only_actual_optional_constructor_results() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) fn issue() -> Option<Ledger> { None }";
        for caller in [
            "fn consume() { if let Some(current) = super::owner::issue() { current.inspect(); } }",
            "fn consume() { let Some(current) = super::owner::issue() else { return; }; current.inspect(); }",
        ] {
            let (modules, row) = fixture(
                owner,
                caller,
                "Ledger::inspect",
                "producer:crate::shared::owner::issue",
            );
            assert!(modules.check_row(&row), "{caller}");
        }
        for caller in [
            "struct Some<T>(T); fn consume() { if let Some(current) = super::owner::issue() { current.inspect(); } }",
            "fn consume() { if let Some(current) = super::owner::issue() { let current = (); current.inspect(); } }",
            "fn consume() { let Some(current) = super::owner::issue() else { donate!(); return; }; current.inspect(); }",
            "fn consume() { if let Some(current) = super::owner::issue() {} current.inspect(); }",
        ] {
            let (modules, row) = fixture(
                owner,
                caller,
                "Ledger::inspect",
                "producer:crate::shared::owner::issue",
            );
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_enum_receivers_follow_actual_iterator_item_and_tuple_variant_fields() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) enum Entry { Declared(std::sync::Arc<Ledger>), Empty } #[derive(Clone)] pub(super) struct Row { pub(super) entry: std::sync::Arc<Entry> } pub(super) fn rows() -> Option<impl Iterator<Item = Row> + Clone> { None::<std::vec::IntoIter<Row>> }";
        let caller = "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => current.inspect(), _ => {} } } }";
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
        );
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_enum_receivers_refuse_other_variants_shadows_and_unavailable_shapes() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) enum Entry { Declared(std::sync::Arc<Ledger>), Other(std::sync::Arc<Ledger>) } #[derive(Clone)] pub(super) struct Row { pub(super) entry: std::sync::Arc<Entry> } pub(super) fn rows() -> Option<impl Iterator<Item = Row> + Clone> { None::<std::vec::IntoIter<Row>> }";
        for caller in [
            "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Other(current) => current.inspect(), _ => {} } } }",
            "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => { let current = (); current.inspect(); }, _ => {} } } }",
            "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => {}, _ => {} } current.inspect(); } }",
        ] {
            let (modules, row) = fixture(
                owner,
                caller,
                "Ledger::inspect",
                "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
            );
            assert!(!modules.check_row(&row), "{caller}");
        }
        for changed in [
            owner.replace(
                "Declared(std::sync::Arc<Ledger>)",
                "#[cfg(any())] Declared(std::sync::Arc<Ledger>)",
            ),
            owner.replace("Iterator<Item = Row>", "Iterator<Item = ()>"),
            owner.replace("Iterator<Item = Row>", "?Iterator<Item = Row>"),
            owner.replace("pub(super) entry:", "entry:"),
        ] {
            let caller = "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => current.inspect(), _ => {} } } }";
            let (modules, row) = fixture(
                &changed,
                caller,
                "Ledger::inspect",
                "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
            );
            assert!(!modules.check_row(&row));
        }
    }

    #[test]
    fn source_enum_type_rows_require_actual_accessible_field_occurrences() {
        for caller in [
            "enum Selected { Owned(Box<super::owner::Ledger>) }",
            "use super::owner::Ledger as Current; enum Selected { Owned { current: std::sync::Arc<Current> } }",
            "enum Selected<'a> { Owned(&'a super::owner::Ledger) }",
        ] {
            let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
            row.consumer_entry = "Selected".into();
            assert!(modules.check_row(&row), "{caller}");
        }
        for caller in [
            "enum Selected { Empty }",
            "struct Selected { value: super::owner::Ledger }",
            "enum Selected<Ledger> { Owned(Ledger) }",
            "enum Selected<const N: usize> { Owned(super::owner::Ledger) }",
            "#[cfg(any())] enum Selected { Owned(super::owner::Ledger) }",
            "#[custom] enum Selected { Owned(super::owner::Ledger) }",
            "enum Selected { #[cfg(any())] Owned(super::owner::Ledger) }",
            "enum Selected { Owned(#[cfg(any())] super::owner::Ledger) }",
            "enum Selected { Owned(#[custom] super::owner::Ledger) }",
            "type Alias = super::owner::Ledger; enum Selected { Owned(Alias) }",
            "enum Selected { Owned(super::owner::Ledger<u8>) }",
            "enum Selected { Owned(<super::owner::Ledger as Unknown>::Output) }",
            "enum Selected { Owned(super::owner::Ledger), Unknown(select_type!()) }",
            "enum Selected { Empty = 1, Owned(super::owner::Ledger) }",
            "enum Selected { Owned(super::owner::Ledger) } enum Selected { Owned(super::owner::Ledger) }",
        ] {
            let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
            row.consumer_entry = "Selected".into();
            assert!(!modules.check_row(&row), "{caller}");
        }
        let (modules, mut row) = fixture(
            "struct Ledger;",
            "enum Selected { Owned(super::owner::Ledger) }",
            "Ledger",
            "type",
        );
        row.consumer_entry = "Selected".into();
        row.visibility = "private".into();
        assert!(!modules.check_row(&row));
        let (modules, mut row) = fixture(
            "pub(super) struct Ledger;",
            "enum Selected { Owned(super::owner::Ledger) }",
            "Ledger",
            "static",
        );
        row.consumer_entry = "Selected".into();
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_standard_wrappers_refuse_local_union_shadows() {
        for caller in [
            "union Box<T> { value: std::mem::ManuallyDrop<T> } trait Audit { fn inspect(&self); } impl<T> Audit for Box<T> { fn inspect(&self) {} } fn consume(current: &Box<super::owner::Ledger>) { current.inspect(); }",
            "union Option<T> { value: std::mem::ManuallyDrop<T> } impl<T> Option<T> { fn as_ref(&self) -> std::option::Option<&Self> { None } } trait Audit { fn inspect(&self); } impl<T> Audit for Option<T> { fn inspect(&self) {} } fn consume(current: &Option<super::owner::Ledger>) -> std::option::Option<()> { current.as_ref()?.inspect(); Some(()) }",
        ] {
            let (modules, row) = fixture(
                "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }",
                caller,
                "Ledger::inspect",
                "parameter:current",
            );
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_dot_routes_require_the_actual_owner_receiver() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect() {} } pub(super) trait Audit { fn inspect(&self); } impl Audit for Ledger { fn inspect(&self) {} }";
        let (modules, row) = fixture(
            owner,
            "use super::owner::Audit; fn consume(ledger: &super::owner::Ledger) { ledger.inspect(); }",
            "Ledger::inspect",
            "parameter:ledger",
        );
        assert!(!modules.check_row(&row));
        let (modules, row) = fixture(
            owner,
            "fn consume() { super::owner::Ledger::inspect(); }",
            "Ledger::inspect",
            "static",
        );
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_routes_accept_actual_nested_static_and_typed_parameter_calls() {
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} pub(super) fn revision(&self, depth: usize) {} }",
            "fn consume() { super::owner::Ledger::capture(); }",
            "Ledger::capture",
            "static",
        );
        assert!(modules.check_row(&row));
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self, depth: usize) {} }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(4); }",
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_routes_accept_only_actual_fields_and_constructor_results() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }";
        let (modules, row) = fixture(
            owner,
            "use super::owner::Ledger; fn consume() -> Option<()> { let values = Ledger::capture()?; values.iter().map(|value| value.revision()).count(); Some(()) }",
            "Ledger::revision",
            "producer:crate::shared::owner::Ledger::capture",
        );
        assert!(modules.check_row(&row));
        let (modules, mut row) = fixture(
            owner,
            "struct Reader { values: Vec<super::owner::Ledger> } impl Reader { fn consume(&self) { self.values.iter().zip(&[()]).any(|(value, other)| { value.revision(); false }); } }",
            "Ledger::revision",
            "field:self.values",
        );
        row.consumer_entry = "Reader::consume".into();
        assert!(modules.check_row(&row));
        let (modules, row) = fixture(
            owner,
            "struct Fake; fn consume(ledger: &Fake) { ledger.revision(); }",
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_routes_verify_actual_parameter_and_producer_fields() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
        let (modules, row) = fixture(
            owner,
            "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &Tokens) -> Option<()> { let binding = parent.binding.as_ref()?; binding.revision(); Some(()) }",
            "Ledger::revision",
            "parameter-field:parent:binding",
        );
        assert!(modules.check_row(&row));
        let (modules, mut row) = fixture(
            owner,
            "struct Tokens { binding: Option<super::owner::Ledger> } struct Reader; impl Reader { fn tokens(&self) -> Tokens { todo!() } fn consume(&self) -> Option<()> { let tokens = self.tokens(); tokens.binding.as_ref()?.revision(); Some(()) } }",
            "Ledger::revision",
            "producer-field:Reader::tokens:binding",
        );
        row.consumer_entry = "Reader::consume".into();
        assert!(modules.check_row(&row));
        let (modules, row) = fixture(
            owner,
            "struct Fake; impl Fake { fn revision(&self) {} } struct Tokens { binding: Option<Fake> } fn consume(parent: &Tokens) -> Option<()> { let binding = parent.binding.as_ref()?; binding.revision(); Some(()) }",
            "Ledger::revision",
            "parameter-field:parent:binding",
        );
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_routes_keep_plain_local_imports_scoped_and_preserve_target_shadows() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }";
        for caller in [
            "fn consume() { use std::collections::HashSet; super::owner::Ledger::capture(); }",
            "use super::owner::Ledger; fn consume() { { use other::Ledger; } Ledger::capture(); }",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger::capture", "static");
            assert!(modules.check_row(&row), "{caller}");
        }
        for caller in [
            "use super::owner::Ledger; fn consume() { Ledger::capture(); use other::Ledger; }",
            "use super::owner::Ledger; fn consume() { use other::Other as Ledger; Ledger::capture(); }",
            "fn consume() { use other::*; super::owner::Ledger::capture(); }",
            "fn consume() { #[cfg(test)] use other::Other; super::owner::Ledger::capture(); }",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger::capture", "static");
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn parameter_origins_refuse_body_type_shadows_and_restore_enclosing_scope() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
        for caller in [
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(); use fake::Ledger; }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Other as Ledger; let alias = ledger; alias.revision(); }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { let alias = ledger; { use fake::Ledger; alias.revision(); } }",
            "use super::owner::Ledger; fn consume(ledger: &Option<Ledger>) { use fake::Ledger; let Some(current) = ledger.as_ref() else { return; }; current.revision(); }",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:ledger");
            assert!(!modules.check_row(&row), "{caller}");
        }
        for caller in [
            "fn consume(ledger: &super::owner::Ledger) { use fake::Ledger; ledger.revision(); }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Other; ledger.revision(); }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { { use fake::Ledger; } ledger.revision(); }",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:ledger");
            assert!(modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_type_declarations_keep_passive_lints_and_refuse_availability_attributes() {
        let caller = "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(); }";
        for attribute in [
            "#[allow(dead_code)]",
            "#[warn(clippy::all)]",
            "#[deny(unused)]",
            "#[forbid(unsafe_code)]",
            "#[expect(dead_code)]",
        ] {
            let owner = format!(
                "{attribute} pub(super) struct Ledger; impl Ledger {{ pub(super) fn revision(&self) {{}} }}"
            );
            let (modules, row) = fixture(&owner, caller, "Ledger::revision", "parameter:ledger");
            assert!(modules.check_row(&row), "{attribute}");
        }
        for attribute in [
            "#[cfg(any())]",
            "#[cfg_attr(any(), derive(Clone))]",
            "#[unknown]",
        ] {
            let owner = format!(
                "{attribute} pub(super) struct Ledger; impl Ledger {{ pub(super) fn revision(&self) {{}} }}"
            );
            let (modules, row) = fixture(&owner, caller, "Ledger::revision", "parameter:ledger");
            assert!(!modules.check_row(&row), "{attribute}");
        }
    }

    #[test]
    fn signature_type_routes_count_only_explicit_supported_declaration_types() {
        let owner = "pub(super) enum Ledger { Fixed }";
        let caller =
            "use super::owner::Ledger; fn consume(selected: Ledger) { diagnostic!(selected); }";
        let (modules, mut row) = fixture(owner, caller, "Ledger", "signature-type");
        assert!(modules.check_row(&row));
        row.route = "type".into();
        assert!(!modules.check_row(&row));
        for caller in [
            "fn consume() { let selected = super::owner::Ledger::Fixed; }",
            "use super::owner::Ledger; fn consume<T>(selected: Ledger) {}",
            "use super::owner::Ledger; fn consume(selected: Ledger<Other>) {}",
            "use super::owner::Ledger; #[cfg(any())] fn consume(selected: Ledger) {}",
            "use super::owner::Ledger; fn consume(#[cfg(any())] selected: Ledger) {}",
            "use super::owner::Ledger; fn consume(selected: Ledger, other: hidden!()) {}",
            "type Alias = super::owner::Ledger; fn consume(selected: Alias) {}",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger", "signature-type");
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_type_routes_require_explicit_current_struct_constructors() {
        let owner = "pub(super) struct Ledger { pub(super) current: bool }";
        let (modules, row) = fixture(
            owner,
            "fn consume() { let current = super::owner::Ledger { current: true }; }",
            "Ledger",
            "type",
        );
        assert!(modules.check_row(&row));
        for caller in [
            "type Donated = super::owner::Ledger; fn consume() { let current = Donated { current: true }; }",
            "use super::owner::Ledger; fn consume() { let current = Ledger { current: true }; use other::Ledger; }",
            "fn consume() { #[cfg(test)] let current = super::owner::Ledger { current: true }; }",
            "fn consume() { make!(super::owner::Ledger { current: true }); }",
            "fn consume() { let current = super::owner::Ledger::<Other> { current: true }; }",
        ] {
            let (modules, row) = fixture(owner, caller, "Ledger", "type");
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_routes_follow_only_actual_standard_mutable_option_fields() {
        let owner =
            "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&mut self) {} }";
        let (modules, row) = fixture(
            owner,
            "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
            "Ledger::revision",
            "parameter-field:parent:binding",
        );
        assert!(modules.check_row(&row));
        for caller in [
            "struct Fake; impl Fake { fn revision(&mut self) {} } struct Tokens { binding: Option<Fake> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
            "struct Other<T>(T); impl<T> Other<T> { fn as_mut(&mut self) -> Option<&mut T> { Some(&mut self.0) } } struct Tokens { binding: Other<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
            "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut::<Other>() { binding.revision(); } }",
        ] {
            let (modules, row) = fixture(
                owner,
                caller,
                "Ledger::revision",
                "parameter-field:parent:binding",
            );
            assert!(!modules.check_row(&row), "{caller}");
        }
    }

    #[test]
    fn source_routes_reject_shadow_macro_alias_and_conditional_calls() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }";
        for consumer in [
            "use super::owner::Ledger; fn consume() { let Ledger = fake; Ledger::capture(); }",
            "type Ledger = super::owner::Ledger; fn consume() { Ledger::capture(); }",
            "fn consume() { invoke!(super::owner::Ledger::capture()); }",
            "fn consume() { #[cfg(test)] super::owner::Ledger::capture(); }",
            "#[cfg(test)] fn consume() { super::owner::Ledger::capture(); }",
            "fn consume() { super::owner::Ledger::other(); }",
        ] {
            let (modules, row) = fixture(owner, consumer, "Ledger::capture", "static");
            assert!(!modules.check_row(&row), "{consumer}");
        }
    }

    #[test]
    fn source_routes_clear_every_shadowing_pattern_and_refuse_binding_macros() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
        for body in [
            "items.map(|ledger| ledger.revision());",
            "for ledger in items { ledger.revision(); }",
            "match fake { Some(ledger) => ledger.revision(), _ => () }",
            "match fake { Some(ledger) if { ledger.revision(); true } => (), _ => () }",
            "match fake { (Some(ledger), _) if { ledger.revision(); true } => (), _ => () }",
            "match fake { Some(other) if { bind!(ledger); ledger.revision(); true } => (), _ => () }",
            "if let Some(ledger) = fake { ledger.revision(); }",
            "while let Some(ledger) = fake { ledger.revision(); }",
            "let (ledger, other) = fake; ledger.revision();",
            "let Some(ledger) = fake else { return; }; ledger.revision();",
            "bind!(ledger); ledger.revision();",
            "macro_rules! bind { () => {} } ledger.revision();",
            "#[cfg(test)] for other in items { ledger.revision(); }",
        ] {
            let consumer =
                format!("use super::owner::Ledger; fn consume(ledger: &Ledger) {{ {body} }}");
            let (modules, row) = fixture(owner, &consumer, "Ledger::revision", "parameter:ledger");
            assert!(!modules.check_row(&row), "{body}");
        }
        let (modules, row) = fixture(
            owner,
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { for unrelated in items {} ledger.revision(); }",
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_routes_refuse_enclosing_generic_receiver_name_shadows() {
        let (modules, mut row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }",
            "use super::owner::Ledger; struct Reader<T>(T); impl<Ledger> Reader<Ledger> { fn consume(&self, ledger: &Ledger) { ledger.revision(); } }",
            "Ledger::revision",
            "parameter:ledger",
        );
        row.consumer_entry = "Reader::consume".into();
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_routes_require_direct_module_visibility_and_preserve_real_reexports() {
        let (mut modules, mut row) = fixture(
            "pub(crate) struct Ledger; impl Ledger { pub(crate) fn capture() {} }",
            "fn consume() { crate::shared::owner::Ledger::capture(); }",
            "Ledger::capture",
            "static",
        );
        row.visibility = "pub(crate)".into();
        let caller = modules.modules.remove("crate::shared::caller").unwrap();
        modules.modules.insert(
            "crate::outside".into(),
            SourceModule {
                access_scope: "crate".into(),
                ..caller
            },
        );
        row.consumer_module = "crate::outside".into();
        assert!(!modules.check_row(&row));
        modules.modules.get_mut("crate::shared").unwrap().file =
            syn::parse_file("mod owner; pub(crate) use owner::Ledger;").unwrap();
        modules.modules.get_mut("crate::outside").unwrap().file =
            syn::parse_file("fn consume() { crate::shared::Ledger::capture(); }").unwrap();
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_visibility_requires_a_real_ancestor_scope() {
        for (written, expected) in [
            ("pub(in crate::shared)", "crate::shared"),
            ("pub(in super)", "crate::shared"),
            ("pub(in super::super)", "crate"),
            ("pub(in self)", "crate::shared::owner"),
            ("pub(crate)", "crate"),
        ] {
            let visibility = syn::parse_str::<syn::Visibility>(written).unwrap();
            assert_eq!(
                visibility_scope(&visibility, "crate::shared::owner"),
                Some(expected.into())
            );
            assert_eq!(visibility_name(&visibility), written);
        }
        for written in [
            "pub(in crate::outside)",
            "pub(in self::child)",
            "pub(in super::super::super)",
            "pub(in alias)",
        ] {
            let visibility = syn::parse_str::<syn::Visibility>(written).unwrap();
            assert!(
                visibility_scope(&visibility, "crate::shared::owner").is_none(),
                "{written}"
            );
        }
        let visibility = syn::parse_str::<syn::Visibility>("pub(in crate::shared)").unwrap();
        assert!(accessible(
            &visibility,
            "crate::shared::owner",
            "crate::shared::caller"
        ));
        assert!(!accessible(
            &visibility,
            "crate::shared::owner",
            "crate::shared_extra::caller"
        ));
        assert!(!accessible(
            &visibility,
            "crate::shared::owner",
            "crate::outside"
        ));
        let mut generic = visibility.clone();
        let syn::Visibility::Restricted(scope) = &mut generic else {
            panic!();
        };
        *scope.path = syn::parse_str::<syn::Path>("crate::shared::<Other>").unwrap();
        assert!(visibility_scope(&generic, "crate::shared::owner").is_none());
    }

    #[test]
    fn source_file_module_loader_keeps_ancestor_and_parent_visibility() {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "tcl-owner-ancestor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let crate_root = PathBuf::from("rust/fixture");
        std::fs::create_dir_all(root.join(&crate_root).join("src/shared")).unwrap();
        for (file, contents) in [
            ("Cargo.toml", "[package]\nname = \"fixture\"\n"),
            ("src/lib.rs", "pub(crate) mod shared; mod outside;"),
            (
                "src/shared/mod.rs",
                "pub(in crate::shared) mod owner; pub(crate) mod caller;",
            ),
            (
                "src/shared/owner.rs",
                "pub(crate) struct Ledger; impl Ledger { pub(crate) fn capture() {} }",
            ),
            (
                "src/shared/caller.rs",
                "fn consume() { super::owner::Ledger::capture(); }",
            ),
            (
                "src/outside.rs",
                "fn consume() { crate::shared::owner::Ledger::capture(); }",
            ),
        ] {
            std::fs::write(root.join(&crate_root).join(file), contents).unwrap();
        }
        let (_, mut row) = fixture("", "", "Ledger::capture", "static");
        row.visibility = "pub(crate)".into();
        let modules =
            CrateModules::load(&root, &crate_root, [&row.module, &row.consumer_module]).unwrap();
        assert!(modules.check_row(&row));
        row.consumer_module = "crate::outside".into();
        row.consumer_source = "rust/fixture/src/outside.rs".into();
        assert!(!modules.check_row(&row));
        assert!(!modules.module_accessible("crate::shared::owner", "crate::outside"));
        std::fs::write(
            root.join(&crate_root).join("src/shared/mod.rs"),
            "pub(in crate::outside) mod owner; mod caller;",
        )
        .unwrap();
        assert!(
            CrateModules::load(&root, &crate_root, [&row.module, &row.consumer_module]).is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_routes_refuse_function_generic_wrappers_and_nested_item_scopes() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }";
        for consumer in [
            "use super::owner::Ledger; fn consume<Vec>(ledger: &Ledger) { ledger.revision(); }",
            "use super::owner::Ledger; fn consume<const N: usize>(ledger: &Ledger) { ledger.revision(); }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { fn nested() { ledger.revision(); } }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Ledger; ledger.revision(); }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger) { struct Hidden; ledger.revision(); }",
        ] {
            let (modules, row) = fixture(owner, consumer, "Ledger::revision", "parameter:ledger");
            assert!(!modules.check_row(&row), "{consumer}");
        }
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn capture<Vec>() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }",
            "use super::owner::Ledger; fn consume() { let values = Ledger::capture()?; values.iter().map(|value| value.revision()); }",
            "Ledger::revision",
            "producer:crate::shared::owner::Ledger::capture",
        );
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_declaration_metadata_keeps_passive_lints_and_refuses_availability_changes() {
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { #[must_use] #[inline] pub(super) fn revision(&self) -> bool { true } }",
            "use super::owner::Ledger; #[allow(clippy::too_many_lines)] fn consume(ledger: &Ledger) { ledger.revision(); }",
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(modules.check_row(&row));
        for attribute in [
            "#[cfg(test)]",
            "#[cfg_attr(test, allow(dead_code))]",
            "#[rewrite]",
            "#[path = \"other.rs\"]",
        ] {
            let consumer = format!(
                "use super::owner::Ledger; {attribute} fn consume(ledger: &Ledger) {{ ledger.revision(); }}"
            );
            let (modules, row) = fixture(
                "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }",
                &consumer,
                "Ledger::revision",
                "parameter:ledger",
            );
            assert!(!modules.check_row(&row), "{attribute}");
        }
    }

    #[test]
    fn source_routes_refuse_generic_field_and_nominal_type_argument_erasure() {
        let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
        let (modules, row) = fixture(
            owner,
            "use super::owner::Ledger; struct Tokens<Ledger> { binding: Ledger } struct Fake; fn consume(parent: &Tokens<Fake>) { parent.binding.revision(); }",
            "Ledger::revision",
            "parameter-field:parent:binding",
        );
        assert!(!modules.check_row(&row));
        let (modules, row) = fixture(
            "pub(super) struct Ledger<T>(T); impl Ledger<Fake> { pub(super) fn revision(&self) {} }",
            "use super::owner::Ledger; fn consume(ledger: &Ledger<Other>) { ledger.revision(); }",
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(!modules.check_row(&row));
        let (modules, mut row) = fixture(
            owner,
            "struct Reader<'a> { ledger: &'a super::owner::Ledger } impl<'a> Reader<'a> { fn consume(&self) { self.ledger.revision(); } }",
            "Ledger::revision",
            "field:self.ledger",
        );
        row.consumer_entry = "Reader::consume".into();
        assert!(modules.check_row(&row));
    }

    #[test]
    fn source_routes_reject_duplicate_declarations_and_wrong_visibility() {
        for owner in [
            "pub(super) struct Ledger; impl Ledger { fn capture() {} }",
            "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} pub(super) fn capture() {} }",
            "#[cfg(test)] pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
            "pub(super) struct Ledger; #[cfg(test)] impl Ledger { pub(super) fn capture() {} }",
        ] {
            let (modules, row) = fixture(
                owner,
                "fn consume() { super::owner::Ledger::capture(); }",
                "Ledger::capture",
                "static",
            );
            assert!(!modules.check_row(&row), "{owner}");
        }
        let (modules, mut row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
            "fn consume() { super::owner::Ledger::capture(); }",
            "Ledger::capture",
            "static",
        );
        row.consumer_module = "crate::outside".into();
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_routes_resolve_actual_reexports_and_reject_private_alias_routes() {
        let (mut modules, row) = fixture(
            "pub(crate) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
            "fn consume() { crate::shared::Ledger::capture(); }",
            "Ledger::capture",
            "static",
        );
        modules.modules.get_mut("crate::shared").unwrap().file =
            syn::parse_file("mod owner; pub(crate) use owner::Ledger;").unwrap();
        assert!(modules.check_row(&row));
        modules.modules.get_mut("crate::shared").unwrap().file =
            syn::parse_file("mod owner; #[cfg(test)] pub(crate) use owner::Ledger;").unwrap();
        assert!(!modules.check_row(&row));
    }

    #[test]
    fn source_manifest_requires_exact_markers_scope_and_repository_paths() {
        assert!(parse(START).is_err());
        assert!(parse(END).is_err());
        assert!(parse(&format!("{START}\n{END}")).is_err());
        for source in [
            "/tmp/lib.rs",
            "rust/a/../a/src/lib.rs",
            "src/lib.rs",
            "rust/a/lib.rs",
        ] {
            assert!(crate_root(source).is_err(), "{source}");
        }
        assert_eq!(
            crate_root("rust/a/src/nested/owner.rs").unwrap(),
            Path::new("rust/a")
        );
    }
}

#[cfg(test)]
mod file_route_tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn root() -> Fixture {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "source-owner-fixture-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::create_dir_all(directory.join("rust/a/src/shared")).unwrap();
        for (file, contents) in [
            ("Cargo.toml", "[package]\nname = \"a\"\n"),
            ("src/lib.rs", "mod shared;"),
            ("src/shared.rs", "mod owner; mod caller;"),
            ("src/shared/owner.rs", "pub(super) fn selected() {}"),
            (
                "src/shared/caller.rs",
                "fn consume() { super::owner::selected(); }",
            ),
        ] {
            std::fs::write(directory.join("rust/a").join(file), contents).unwrap();
        }
        Fixture(directory)
    }
    fn manifest() -> String {
        format!(
            "{START}\n| Surface | Owner source | Crate | Owner module | Entry | Visibility | Consumer source | Consumer module | Consumer entry | Route |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n| source | `rust/a/src/shared/owner.rs` | `a` | `crate::shared::owner` | `selected` | pub(super) | `rust/a/src/shared/caller.rs` | `crate::shared::caller` | `consume` | `static` |\n{END}"
        )
    }

    #[test]
    fn actual_file_routes_keep_exact_source_crate_module_and_consumer_identity() {
        let fixture = root();
        let (coverage, problems) = validate(&fixture.0, &manifest());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(coverage.len(), 1);
        for changed in [
            manifest().replace("| `a` |", "| `other` |"),
            manifest().replace("crate::shared::owner", "crate::shared::other"),
            manifest().replace("rust/a/src/shared/owner.rs", "rust/a/src/shared/caller.rs"),
            manifest().replace("| `consume` |", "| `changed` |"),
            manifest().replace(
                "rust/a/src/shared/caller.rs",
                "rust/other/src/shared/caller.rs",
            ),
        ] {
            let (coverage, problems) = validate(&fixture.0, &changed);
            assert!(coverage.is_empty());
            assert!(!problems.is_empty());
        }
    }

    #[test]
    fn direct_self_module_routes_keep_the_existing_tree_bound_without_unrelated_loading() {
        let fixture = root();
        let source = "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } }";
        std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"), source).unwrap();
        let mut library = String::from("mod shared;\n");
        for index in 0..2050 {
            let module = format!("unrelated{index}");
            library.push_str(&format!("mod {module};\n"));
            std::fs::write(fixture.0.join(format!("rust/a/src/{module}.rs")), "").unwrap();
        }
        std::fs::write(fixture.0.join("rust/a/src/lib.rs"), library).unwrap();
        let mut rows = parse(&manifest()).unwrap();
        let row = rows.first_mut().unwrap();
        row.entry = "Ledger::selected".into();
        row.consumer_entry = "Ledger::consume".into();
        row.consumer_module = row.module.clone();
        row.consumer_source = row.source.clone();
        row.route = "self".into();
        let modules = CrateModules::load_for_route(&fixture.0, Path::new("rust/a"), row).unwrap();
        assert!(modules.check_row(row));
        assert!(modules.modules.len() < 2048);
        assert!(
            CrateModules::load(
                &fixture.0,
                Path::new("rust/a"),
                [&row.module, &row.consumer_module]
            )
            .is_err()
        );
        row.consumer_entry = "Other::consume".into();
        assert!(CrateModules::load_for_route(&fixture.0, Path::new("rust/a"), row).is_err());
    }

    #[test]
    fn targeted_self_cache_cannot_lend_its_tree_to_full_receiver_routes() {
        let fixture = root();
        let mut library = String::from("mod shared;\n");
        for index in 0..2050 {
            let module = format!("unrelated{index}");
            library.push_str(&format!("mod {module};\n"));
            std::fs::write(fixture.0.join(format!("rust/a/src/{module}.rs")), "").unwrap();
        }
        std::fs::write(fixture.0.join("rust/a/src/lib.rs"), library).unwrap();
        std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"),
            "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } } fn borrow(ledger: &Ledger) { ledger.selected(); }").unwrap();
        let self_row = "| source | `rust/a/src/shared/owner.rs` | `a` | `crate::shared::owner` | `Ledger::selected` | pub(super) | `rust/a/src/shared/owner.rs` | `crate::shared::owner` | `Ledger::consume` | `self` |";
        let borrowed_row = self_row
            .replace("`Ledger::consume`", "`borrow`")
            .replace("`self`", "`parameter:ledger`");
        let text = format!("{START}\n{self_row}\n{borrowed_row}\n{END}");
        let (coverage, failures) = validate(&fixture.0, &text);
        assert_eq!(coverage.len(), 1);
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("source module route exceeds bound"));

        // A separately loaded full mode keeps actual std shadow declarations.
        std::fs::write(fixture.0.join("rust/a/src/lib.rs"), "mod shared; mod std;").unwrap();
        std::fs::write(
            fixture.0.join("rust/a/src/std.rs"),
            "pub mod boxed { pub struct Box<T>(T); }",
        )
        .unwrap();
        std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"),
            "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } } fn borrow(ledger: &std::boxed::Box<Ledger>) { ledger.selected(); }").unwrap();
        let (coverage, failures) = validate(&fixture.0, &text);
        assert_eq!(coverage.len(), 1);
        assert_eq!(failures.len(), 1);
    }

    #[test]
    fn explicit_binary_root_keeps_actual_child_modules_and_refuses_ambiguous_targets() {
        let fixture = root();
        std::fs::rename(
            fixture.0.join("rust/a/src/lib.rs"),
            fixture.0.join("rust/a/src/main.rs"),
        )
        .unwrap();
        let cargo = "[package]\nname = \"a\"\n[[bin]]\nname = \"a\"\npath = \"src/main.rs\"\n";
        std::fs::write(fixture.0.join("rust/a/Cargo.toml"), cargo).unwrap();
        let (coverage, failures) = validate(&fixture.0, &manifest());
        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(coverage.len(), 1);
        for changed in [
            cargo.replace("src/main.rs", "src/other.rs"),
            format!("{cargo}[[bin]]\nname = \"other\"\npath = \"src/main.rs\"\n"),
            format!("{cargo}[lib]\npath = \"src/custom.rs\"\n"),
        ] {
            std::fs::write(fixture.0.join("rust/a/Cargo.toml"), changed).unwrap();
            let (coverage, failures) = validate(&fixture.0, &manifest());
            assert!(coverage.is_empty());
            assert!(!failures.is_empty());
        }
        std::fs::write(fixture.0.join("rust/a/Cargo.toml"), cargo).unwrap();
        std::fs::create_dir(fixture.0.join("rust/a/src/bin")).unwrap();
        assert!(!validate(&fixture.0, &manifest()).1.is_empty());
    }

    #[test]
    fn file_routes_refuse_path_overrides_conditionals_and_duplicate_modules() {
        for declaration in [
            "#[path = \"alternate.rs\"] mod owner; mod caller;",
            "#[cfg(test)] mod owner; mod caller;",
            "mod owner; mod owner; mod caller;",
            "mod owner { pub(super) fn selected() {} } mod caller;",
        ] {
            let fixture = root();
            std::fs::write(fixture.0.join("rust/a/src/shared.rs"), declaration).unwrap();
            let (coverage, problems) = validate(&fixture.0, &manifest());
            assert!(coverage.is_empty(), "{declaration}");
            assert!(!problems.is_empty(), "{declaration}");
        }
    }
}
