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

//! The walk over a consumer body that records each reference to the owner.

use super::declarations::{
    flatten_use, source_attrs_preserve_routes, source_constant_is_literal, supported_path_arguments,
};
use super::{ConsumerReferences, TypedReference};
use syn::visit;
use syn::visit::Visit;

impl<'ast> Visit<'ast> for ConsumerReferences<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.row.route == "static"
            && super::super::route_attrs_are_plain(&call.attrs)
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
            && super::super::route_attrs_are_plain(&call.attrs)
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
            && super::super::route_attrs_are_plain(&call.attrs)
            && call.args.len() == 1
            && let Some(mut value) = self.expression_identity(&call.receiver)
            && let Some(inner) = value
                .identity
                .strip_prefix("Option<")
                .and_then(|value| value.strip_suffix('>'))
            && let Some(syn::Expr::Closure(closure)) = call.args.first()
            && closure.inputs.len() == 1
            && super::super::route_attrs_are_plain(&closure.attrs)
            && let Some(syn::Pat::Ident(parameter)) = closure.inputs.first()
            && super::super::route_attrs_are_plain(&parameter.attrs)
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
                    if !super::super::route_attrs_are_plain(&closure.attrs) {
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
            && super::super::route_attrs_are_plain(&expression.attrs)
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
        if !super::super::route_attrs_are_plain(&local.attrs) {
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
        if !super::super::route_attrs_are_plain(&closure.attrs) {
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
        if !super::super::route_attrs_are_plain(&expression.attrs) {
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
            && super::super::route_attrs_are_plain(&binding.attrs)
        {
            self.locals.insert(binding.ident.to_string(), value);
        }
        self.visit_block(&expression.body);
        self.locals = previous;
        self.shadowed = shadowed;
    }
    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if !super::super::route_attrs_are_plain(&expression.attrs) {
            self.invalid = true;
            return;
        }
        self.visit_expr(&expression.expr);
        let selected = self.expression_identity(&expression.expr);
        for arm in &expression.arms {
            let previous = self.locals.clone();
            let shadowed = self.shadowed.clone();
            if !super::super::route_attrs_are_plain(&arm.attrs) {
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
        if !super::super::route_attrs_are_plain(&expression.attrs) {
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
        if !super::super::route_attrs_are_plain(&expression.attrs) {
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
        if !super::super::route_attrs_are_plain(&expression.attrs) {
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
                if !super::super::route_attrs_are_plain(&constant.attrs)
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
                if !super::super::route_attrs_are_plain(&import.attrs)
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
