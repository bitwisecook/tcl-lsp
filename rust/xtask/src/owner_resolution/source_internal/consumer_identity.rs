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

//! The identity of a path, field, binding or expression in a consumer body.

use super::declarations::{
    accessible, declaration, flatten_use, item_name, pattern_bindings,
    source_attrs_preserve_routes, source_type_attrs_preserve_routes, supported_generics,
    supported_path_arguments,
};
use super::{ConsumerReferences, DeclaredFieldIdentity, TypedReference};
use std::collections::BTreeSet;
use syn::visit;
use syn::visit::Visit;

impl ConsumerReferences<'_> {
    /// `Option::map(Owner::method)` applies the owner path to the same single
    /// value the closure form binds.
    pub(super) fn option_map_applies_target(&self, call: &syn::ExprMethodCall) -> bool {
        if call.method == "map"
            && call.turbofish.is_none()
            && super::super::route_attrs_are_plain(&call.attrs)
            && call.args.len() == 1
            && let Some(syn::Expr::Path(path)) = call.args.first()
            && path.qself.is_none()
            && super::super::route_attrs_are_plain(&path.attrs)
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

    pub(super) fn clear_pattern(&mut self, pattern: &syn::Pat) {
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
                    || matches!(item, syn::Item::Use(import) if super::super::use_declares(&import.tree, "std"))
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
                        if !super::super::route_attrs_are_plain(&import.attrs) {
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

    pub(super) fn unwrapped_pointer(identity: &str) -> Option<&str> {
        ["Arc<", "Rc<", "Box<"]
            .into_iter()
            .find_map(|prefix| identity.strip_prefix(prefix)?.strip_suffix('>'))
    }

    pub(super) fn bind_option_pattern(&mut self, pattern: &syn::Pat, value: TypedReference) {
        let syn::Pat::TupleStruct(pattern) = pattern else {
            return;
        };
        if !super::super::route_attrs_are_plain(&pattern.attrs)
            || pattern.elems.len() != 1
            || !pattern.path.is_ident("Some")
            || self.shadowed.contains("Some")
            || self.modules.modules.get(&self.row.consumer_module).is_none_or(|module| {
                module.file.items.iter().any(|item| {
                    Self::standard_binding(item, "Some")
                        || matches!(item, syn::Item::Use(import) if super::super::use_declares(&import.tree, "Some"))
                })
            })
        {
            return;
        }
        let syn::Pat::Ident(binding) = &pattern.elems[0] else {
            return;
        };
        if binding.subpat.is_some() || !super::super::route_attrs_are_plain(&binding.attrs) {
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

    pub(super) fn bind_enum_pattern(&mut self, pattern: &syn::Pat, value: &TypedReference) {
        let syn::Pat::TupleStruct(pattern) = pattern else {
            return;
        };
        if !super::super::route_attrs_are_plain(&pattern.attrs) || pattern.elems.len() != 1 {
            return;
        }
        let syn::Pat::Ident(binding) = &pattern.elems[0] else {
            return;
        };
        if binding.subpat.is_some() || !super::super::route_attrs_are_plain(&binding.attrs) {
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

    pub(super) fn type_identity(&self, ty: &syn::Type) -> Option<String> {
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
            || !super::super::route_attrs_are_plain(&selected.attrs)
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
        if !super::super::route_attrs_are_plain(&receiver.attrs)
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
        if !super::super::route_attrs_are_plain(&reference.attrs)
            || reference.mutability.is_some() != mutable
        {
            return None;
        }
        let syn::Expr::Field(field) = reference.expr.as_ref() else {
            return None;
        };
        if !super::super::route_attrs_are_plain(&field.attrs)
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
                        && super::super::route_attrs_are_plain(&item.attrs)
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

    pub(super) fn field(
        &self,
        value: &TypedReference,
        field: &syn::Member,
    ) -> Option<TypedReference> {
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

    pub(super) fn expression_identity(&self, expression: &syn::Expr) -> Option<TypedReference> {
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
