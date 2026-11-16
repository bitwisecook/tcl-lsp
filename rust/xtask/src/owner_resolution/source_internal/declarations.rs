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

//! What a source item declares: its name, its imports, the generics and attributes a route supports, and the scope its visibility grants.

use super::{Declaration, EXTERN_CALLER};
use std::collections::BTreeSet;
use syn::visit;
use syn::visit::Visit;

pub(super) fn item_name(item: &syn::Item) -> Option<String> {
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

pub(super) fn flatten_use(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut Vec<(String, Vec<String>)>,
) -> Option<()> {
    flatten_use_with_local_precedence(tree, prefix, out, false)
}

pub(super) fn flatten_use_with_local_precedence(
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

pub(super) fn source_constant_is_literal(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Lit(value) => super::super::route_attrs_are_plain(&value.attrs),
        syn::Expr::Reference(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && value.mutability.is_none()
                && source_constant_is_literal(&value.expr)
        }
        syn::Expr::MethodCall(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && value.method == "as_slice"
                && value.args.is_empty()
                && value.turbofish.is_none()
                && matches!(value.receiver.as_ref(), syn::Expr::Lit(literal)
                if matches!(literal.lit, syn::Lit::ByteStr(_))
                    && super::super::route_attrs_are_plain(&literal.attrs))
        }
        syn::Expr::Array(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && value.elems.iter().all(source_constant_is_literal)
        }
        syn::Expr::Tuple(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && value.elems.iter().all(source_constant_is_literal)
        }
        syn::Expr::Paren(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && source_constant_is_literal(&value.expr)
        }
        syn::Expr::Group(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && source_constant_is_literal(&value.expr)
        }
        syn::Expr::Repeat(value) => {
            super::super::route_attrs_are_plain(&value.attrs)
                && source_constant_is_literal(&value.expr)
                && source_constant_is_literal(&value.len)
        }
        _ => false,
    }
}

pub(super) fn source_attrs_preserve_routes(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().all(|attribute| {
        [
            "doc", "allow", "warn", "deny", "forbid", "expect", "must_use", "inline", "cold",
        ]
        .iter()
        .any(|name| attribute.path().is_ident(name))
    })
}

pub(super) fn source_type_attrs_preserve_routes(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().all(|attribute| {
        super::super::type_attrs_are_plain(std::slice::from_ref(attribute))
            || ["allow", "warn", "deny", "forbid", "expect"]
                .iter()
                .any(|name| attribute.path().is_ident(name))
    })
}

pub(super) fn supported_path_arguments(path: &syn::Path) -> bool {
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

pub(super) fn supported_generics(generics: &syn::Generics) -> bool {
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
                super::super::route_attrs_are_plain(&parameter.attrs)
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

pub(super) fn declaration<'a>(file: &'a syn::File, entry: &str) -> Option<Declaration<'a>> {
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
                    super::super::route_attrs_are_plain(&parameter.attrs)
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

pub(super) fn declaration_for_self_method<'a>(
    file: &'a syn::File,
    entry: &str,
) -> Option<Declaration<'a>> {
    declaration_in_mode(file, entry, false, false, true)
}

fn declaration_with_static_generics<'a>(
    file: &'a syn::File,
    entry: &str,
    static_consumer: bool,
) -> Option<Declaration<'a>> {
    declaration_in_mode(file, entry, static_consumer, false, false)
}

pub(super) fn declaration_in_mode<'a>(
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

pub(super) fn visibility_name(visibility: &syn::Visibility) -> String {
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

pub(super) fn visibility_scope(visibility: &syn::Visibility, owner: &str) -> Option<String> {
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

pub(super) fn accessible(visibility: &syn::Visibility, owner: &str, consumer: &str) -> bool {
    if consumer == EXTERN_CALLER {
        return matches!(visibility, syn::Visibility::Public(_));
    }
    visibility_scope(visibility, owner)
        .is_some_and(|scope| consumer == scope || consumer.starts_with(&format!("{scope}::")))
}

pub(super) fn pattern_bindings(pattern: &syn::Pat) -> Option<BTreeSet<String>> {
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
