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

//! The internal-owner manifest: each row's entry is reachable from its consumer module and its receiver returns the owner.

use super::InternalOwnerRow;
use super::entry_declarations::{exported, use_declares};
use super::manifest::parse_internal_manifest;
use anyhow::Result;
use std::path::Path;

pub(super) struct InternalModule {
    pub(super) crate_root: std::path::PathBuf,
    pub(super) crate_name: String,
    pub(super) file: syn::File,
}

pub(super) fn read_internal_module(
    root: &Path,
    source: &str,
    module: &str,
) -> Result<InternalModule, String> {
    let module_name = module
        .strip_prefix("crate::")
        .filter(|name| syn::parse_str::<syn::Ident>(name).is_ok())
        .ok_or_else(|| format!("unsupported internal module route `{module}`"))?;
    let path = Path::new(source);
    let crate_root = path
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == "src"))
        .and_then(Path::parent)
        .ok_or_else(|| format!("internal source is not a direct crate file module: `{source}`"))?;
    if path != crate_root.join("src").join(format!("{module_name}.rs")) {
        return Err(format!(
            "internal source `{source}` does not implement `{module}`"
        ));
    }
    let load = |path: &Path| -> Result<String, String> {
        std::fs::read_to_string(root.join(path))
            .map_err(|error| format!("{}: {error}", path.display()))
    };
    let manifest = load(&crate_root.join("Cargo.toml"))?;
    let crate_name = manifest
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
        .ok_or_else(|| format!("missing literal Cargo package name for `{source}`"))?;
    let lib = syn::parse_file(&load(&crate_root.join("src/lib.rs"))?)
        .map_err(|error| format!("internal crate module route: {error}"))?;
    if !file_attrs_preserve_routes(&lib.attrs) {
        return Err("conditional or unsupported crate-root attributes".to_owned());
    }
    let mut routes = lib.items.iter().filter_map(|item| match item {
        syn::Item::Mod(item) if item.ident == module_name => Some(item),
        _ => None,
    });
    let route = routes
        .next()
        .filter(|item| item.content.is_none() && route_attrs_are_plain(&item.attrs));
    if route.is_none() || routes.next().is_some() {
        return Err(format!(
            "`{module}` requires one unconditional direct lib.rs file-module route"
        ));
    }
    let file = syn::parse_file(&load(path)?).map_err(|error| format!("{source}: {error}"))?;
    Ok(InternalModule {
        crate_root: crate_root.to_owned(),
        crate_name: crate_name.to_owned(),
        file,
    })
}

pub(super) fn validate_internal_manifest(root: &Path, markdown: &str) -> Vec<String> {
    let rows = match parse_internal_manifest(markdown) {
        Ok(rows) => rows,
        Err(problem) => return vec![problem],
    };
    rows.iter()
        .filter_map(|row| {
            let checked = (|| {
                let owner = read_internal_module(root, &row.source, &row.module)?;
                let consumer =
                    read_internal_module(root, &row.consumer_source, &row.consumer_module)?;
                if !internal_entry_is_accessible(row, &owner, &consumer) {
                    return Err(
                        "crate/type/method scope or typed receiver route does not resolve"
                            .to_owned(),
                    );
                }
                Ok(())
            })();
            checked
                .err()
                .map(|problem| format!("internal owner `{}`: {problem}", row.surface))
        })
        .collect()
}

fn crate_accessible(visibility: &syn::Visibility) -> bool {
    exported(visibility)
        || matches!(visibility, syn::Visibility::Restricted(scope)
            if scope.path.is_ident("crate") || scope.path.is_ident("super"))
}

fn passive_file_meta(meta: &syn::Meta) -> bool {
    // The built-in expansion budget changes no declaration or module route.
    // Recognise its actual literal shape, without accepting arbitrary attributes.
    if let syn::Meta::NameValue(setting) = meta
        && setting.path.is_ident("recursion_limit")
        && let syn::Expr::Lit(value) = &setting.value
        && let syn::Lit::Str(limit) = &value.lit
    {
        let limit = limit.value();
        return !limit.is_empty() && limit.bytes().all(|byte| byte.is_ascii_digit());
    }
    ["doc", "allow", "warn", "deny", "forbid", "expect"]
        .iter()
        .any(|name| meta.path().is_ident(name))
}

pub(super) fn file_attrs_preserve_routes(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().all(|attr| {
        if passive_file_meta(&attr.meta) {
            return true;
        }
        let syn::Meta::List(list) = &attr.meta else {
            return false;
        };
        if !list.path.is_ident("cfg_attr") {
            return false;
        }
        let Ok(arguments) = list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            return false;
        };
        arguments.len() >= 2 && arguments.iter().skip(1).all(passive_file_meta)
    })
}

pub(super) fn route_attrs_are_plain(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().all(|attr| attr.path().is_ident("doc"))
}

// Derives add implementations without replacing the declared type.
pub(super) fn type_attrs_are_plain(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .all(|attr| attr.path().is_ident("doc") || attr.path().is_ident("derive"))
}

fn direct_method<'a>(file: &'a syn::File, entry: &str) -> Option<&'a syn::ImplItemFn> {
    let (owner, name) = entry.split_once("::")?;
    let mut methods = file.items.iter().filter_map(|item| match item {
        syn::Item::Impl(item) if item.trait_.is_none()
            && matches!(item.self_ty.as_ref(), syn::Type::Path(path) if path.path.is_ident(owner)) => Some(item),
        _ => None,
    }).flat_map(|implementation| {
        implementation.items.iter().filter_map(move |item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == name => Some((implementation, method)),
            _ => None,
        })
    });
    let (implementation, method) = methods.next()?;
    (methods.next().is_none()
        && route_attrs_are_plain(&implementation.attrs)
        && route_attrs_are_plain(&method.attrs)
        && implementation.generics.params.is_empty()
        && method.sig.generics.params.is_empty()
        && method
            .sig
            .receiver()
            .is_some_and(|receiver| route_attrs_are_plain(&receiver.attrs)))
    .then_some(method)
}

fn direct_type_visibility<'a>(file: &'a syn::File, ident: &str) -> Option<&'a syn::Visibility> {
    if file
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Type(item) if item.ident == ident))
    {
        return None;
    }
    let mut types = file.items.iter().filter_map(|item| match item {
        syn::Item::Struct(item) if item.ident == ident => {
            Some((&item.vis, &item.attrs, &item.generics))
        }
        syn::Item::Enum(item) if item.ident == ident => {
            Some((&item.vis, &item.attrs, &item.generics))
        }
        _ => None,
    });
    let (visibility, attrs, generics) = types.next()?;
    (types.next().is_none() && type_attrs_are_plain(attrs) && generics.params.is_empty())
        .then_some(visibility)
}

pub(super) fn internal_entry_is_accessible(
    row: &InternalOwnerRow,
    owner: &InternalModule,
    consumer: &InternalModule,
) -> bool {
    if !file_attrs_preserve_routes(&owner.file.attrs)
        || !file_attrs_preserve_routes(&consumer.file.attrs)
        || owner.crate_root != consumer.crate_root
        || owner.crate_name != row.crate_name
        || consumer.crate_name != row.crate_name
    {
        return false;
    }
    let Some((owner_type, member)) = row.entry.split_once("::") else {
        return false;
    };
    let Some(visibility) = direct_type_visibility(&owner.file, owner_type) else {
        return false;
    };
    let Some((consumer_type, _)) = row.consumer_entry.split_once("::") else {
        return false;
    };
    if direct_type_visibility(&consumer.file, consumer_type).is_none() {
        return false;
    }
    let Some(method) = direct_method(&owner.file, &row.entry) else {
        return false;
    };
    let Some(getter) = direct_method(&consumer.file, &row.receiver) else {
        return false;
    };
    let Some(caller) = direct_method(&consumer.file, &row.consumer_entry) else {
        return false;
    };
    if !crate_accessible(visibility)
        || !crate_accessible(&method.vis)
        || row.receiver.split_once("::").map(|(name, _)| name)
            != row.consumer_entry.split_once("::").map(|(name, _)| name)
        || getter.sig.inputs.len() != 1
        || method.sig.inputs.len() != 1
    {
        return false;
    }
    receiver_returns_owner(getter, &consumer.file, &row.module, owner_type)
        && caller.block.stmts.iter().any(|statement| {
            let syn::Stmt::Local(local) = statement else {
                return false;
            };
            if !route_attrs_are_plain(&local.attrs) {
                return false;
            }
            let Some(init) = &local.init else {
                return false;
            };
            let syn::Expr::MethodCall(call) = init.expr.as_ref() else {
                return false;
            };
            let syn::Expr::MethodCall(receiver) = call.receiver.as_ref() else {
                return false;
            };
            call.method == member
                && call.args.is_empty()
                && receiver.method == getter.sig.ident
                && receiver.args.is_empty()
                && route_attrs_are_plain(&call.attrs)
                && route_attrs_are_plain(&receiver.attrs)
                && matches!(receiver.receiver.as_ref(), syn::Expr::Path(path)
                    if path.path.is_ident("self") && route_attrs_are_plain(&path.attrs))
        })
}

fn receiver_returns_owner(
    getter: &syn::ImplItemFn,
    consumer: &syn::File,
    module: &str,
    owner: &str,
) -> bool {
    let syn::ReturnType::Type(_, result) = &getter.sig.output else {
        return false;
    };
    let syn::Type::Path(result) = result.as_ref() else {
        return false;
    };
    if result.qself.is_some() || result.path.segments.len() != 3 {
        return false;
    }
    let mut segments = result.path.segments.iter();
    if segments.next().is_none_or(|part| part.ident != "std")
        || segments.next().is_none_or(|part| part.ident != "cell")
    {
        return false;
    }
    let Some(wrapper) = segments.next().filter(|part| part.ident == "Ref") else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(arguments) = &wrapper.arguments else {
        return false;
    };
    let mut arguments = arguments.args.iter();
    let Some(syn::GenericArgument::Lifetime(_)) = arguments.next() else {
        return false;
    };
    let Some(syn::GenericArgument::Type(syn::Type::Path(target))) = arguments.next() else {
        return false;
    };
    if arguments.next().is_some() {
        return false;
    }
    if target.qself.is_some() || !target.path.is_ident(owner) {
        return false;
    }
    let Some(module) = module.strip_prefix("crate::") else {
        return false;
    };
    let declarations = consumer.items.iter().any(|item| match item {
        syn::Item::Struct(item) => item.ident == owner,
        syn::Item::Enum(item) => item.ident == owner,
        syn::Item::Type(item) => item.ident == owner,
        _ => false,
    });
    let mut import_count = 0;
    let mut resolved = false;
    for item in &consumer.items {
        if let syn::Item::Use(item) = item {
            let Some(count) = import_binding_count(&item.tree, owner) else {
                return false;
            };
            if count > 0 && !route_attrs_are_plain(&item.attrs) {
                return false;
            }
            import_count += count;
            resolved |= direct_import(&item.tree, &["crate", module, owner]);
        }
        if matches!(item, syn::Item::Mod(item) if item.ident == "std")
            || matches!(item, syn::Item::Use(item) if use_declares(&item.tree, "std"))
            || matches!(item, syn::Item::ExternCrate(item) if item.rename.as_ref().is_some_and(|(_, name)| name == "std"))
        {
            return false;
        }
    }
    !declarations && resolved && import_count == 1
}

fn import_binding_count(tree: &syn::UseTree, ident: &str) -> Option<usize> {
    match tree {
        syn::UseTree::Path(path) => import_binding_count(&path.tree, ident),
        syn::UseTree::Name(name) => Some(usize::from(name.ident == ident)),
        syn::UseTree::Rename(name) => Some(usize::from(name.rename == ident)),
        syn::UseTree::Group(group) => group.items.iter().try_fold(0, |count, item| {
            Some(count + import_binding_count(item, ident)?)
        }),
        syn::UseTree::Glob(_) => None,
    }
}

fn direct_import(tree: &syn::UseTree, route: &[&str]) -> bool {
    match (tree, route) {
        (syn::UseTree::Path(path), [head, tail @ ..]) if path.ident == *head => {
            direct_import(&path.tree, tail)
        }
        (syn::UseTree::Name(name), [ident]) => name.ident == *ident,
        (syn::UseTree::Group(group), _) => {
            group.items.iter().any(|item| direct_import(item, route))
        }
        _ => false,
    }
}
