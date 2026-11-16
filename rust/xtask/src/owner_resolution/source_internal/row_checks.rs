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

//! The check of one manifest row: its owner declaration, the owner's accessibility and the consumer's reference to it.

use super::declarations::{
    accessible, declaration, declaration_for_self_method, declaration_in_mode, visibility_name,
};
use super::{
    ConsumerReferences, CrateModules, Declaration, FieldTypeReferences, Row, SourceModule,
    TypedReference,
};
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::Visit;

impl CrateModules {
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
            || !super::super::file_attrs_preserve_routes(&owner_module.file.attrs)
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
                if !super::super::route_attrs_are_plain(&parameter.attrs) {
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
    pub(super) fn check_foreign_static_row(&self, row: &Row, target: &str) -> bool {
        let Some(consumer_module) = self.modules.get(&row.consumer_module) else {
            return false;
        };
        if !super::super::file_attrs_preserve_routes(&consumer_module.file.attrs) {
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

    pub(super) fn check_row(&self, row: &Row) -> bool {
        let Some(owner) = self.row_owner_declaration(row) else {
            return false;
        };
        let Some(consumer_module) = self.modules.get(&row.consumer_module) else {
            return false;
        };
        if !super::super::file_attrs_preserve_routes(&consumer_module.file.attrs) {
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
                    syn::FnArg::Typed(argument) => {
                        !super::super::route_attrs_are_plain(&argument.attrs)
                    }
                    syn::FnArg::Receiver(argument) => {
                        !super::super::route_attrs_are_plain(&argument.attrs)
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
