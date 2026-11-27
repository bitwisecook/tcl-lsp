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

//! Whether a source file declares a manifest row's entry point: items, fields, exports and re-exports.

#[cfg(test)]
pub(super) fn entry_is_declared(source: &str, entry: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    file_declares(&file, entry)
}

pub(super) fn file_declares(file: &syn::File, entry: &str) -> bool {
    let (owner, ident) = entry
        .rsplit_once("::")
        .map_or((None, entry), |(owner, ident)| {
            (Some(owner.rsplit("::").next().unwrap_or(owner)), ident)
        });
    items_declare(&file.items, owner, ident)
}

pub(super) fn exported(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn fields_declare(fields: &syn::Fields, ident: &str) -> bool {
    fields
        .iter()
        .any(|field| exported(&field.vis) && field.ident.as_ref().is_some_and(|name| name == ident))
}

fn items_declare(items: &[syn::Item], owner: Option<&str>, ident: &str) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Fn(item) => owner.is_none() && exported(&item.vis) && item.sig.ident == ident,
        syn::Item::Struct(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner && fields_declare(&item.fields, ident)
                } else {
                    item.ident == ident || fields_declare(&item.fields, ident)
                }
        }
        syn::Item::Enum(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner
                        && item.variants.iter().any(|variant| variant.ident == ident)
                } else {
                    item.ident == ident
                }
        }
        syn::Item::Trait(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner && item.items.iter().any(|member| {
                        matches!(member, syn::TraitItem::Fn(method) if method.sig.ident == ident)
                    })
                } else {
                    item.ident == ident
                        || item.items.iter().any(|member| {
                            matches!(member, syn::TraitItem::Fn(method) if method.sig.ident == ident)
                        })
                }
        }
        syn::Item::Impl(item) => {
            let matches_owner = owner.is_none_or(|owner| {
                matches!(item.self_ty.as_ref(), syn::Type::Path(path)
                    if path.path.segments.last().is_some_and(|segment| segment.ident == owner))
            });
            let locally_private = match item.self_ty.as_ref() {
                syn::Type::Path(path) => path.path.segments.last().is_some_and(|segment| {
                    items.iter().any(|declaration| match declaration {
                        syn::Item::Struct(value) => {
                            value.ident == segment.ident && !exported(&value.vis)
                        }
                        syn::Item::Enum(value) => {
                            value.ident == segment.ident && !exported(&value.vis)
                        }
                        _ => false,
                    })
                }),
                _ => false,
            };
            matches_owner
                && !locally_private
                && item.items.iter().any(|member| match member {
                    syn::ImplItem::Fn(method) => {
                        (item.trait_.is_some() || exported(&method.vis)) && method.sig.ident == ident
                    }
                    syn::ImplItem::Const(value) => exported(&value.vis) && value.ident == ident,
                    syn::ImplItem::Type(value) => exported(&value.vis) && value.ident == ident,
                    _ => false,
                })
        }
        syn::Item::Const(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Static(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Type(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Use(item) => exported(&item.vis) && use_declares(&item.tree, ident),
        syn::Item::Mod(item) => item.content.as_ref().is_some_and(|(_, items)| {
            if owner.is_some_and(|owner| item.ident == owner) {
                exported(&item.vis) && items_declare(items, None, ident)
            } else {
                items_declare(items, owner, ident)
            }
        }),
        _ => false,
    })
}

pub(super) fn use_declares(tree: &syn::UseTree, ident: &str) -> bool {
    match tree {
        syn::UseTree::Name(item) => item.ident == ident,
        syn::UseTree::Rename(item) => item.rename == ident,
        syn::UseTree::Path(item) => use_declares(&item.tree, ident),
        syn::UseTree::Group(item) => item.items.iter().any(|item| use_declares(item, ident)),
        syn::UseTree::Glob(_) => false,
    }
}
