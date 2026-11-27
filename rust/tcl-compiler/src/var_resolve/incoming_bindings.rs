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

//! Binding an activation's incoming values: unknown and literal arguments and authenticated original formals.

use super::canonical_keys::canonical_binding_value_key;
use super::observer_projection::project_access;
use super::original_bytes::resolve_evaluated_variable_input;
use super::place_resolution::{resolve_literal_access, resolve_substitution_access};
use super::scalar_binding::bind_cell_identity;
use super::{ContentsOrigin, ContentsPresence, ResolveContext};
use tcl_core_types::NameBytes;
use tcl_names::place;
use tcl_names::place::{CellGeneration, CellIdentity, CellOwner, PlaceKind};
use tcl_registry::CommandRegistry;

impl ResolveContext {
    /// Bind an unknown ordinary incoming activation value.
    /// Only the current physical activation's direct slot receives incoming
    /// provenance. Static, namespace, foreign and observed aliases retain unknown
    /// store provenance; this does not turn later unknown writes into live-ins.
    pub fn bind_unknown_incoming(&mut self, variable: &str, registry: &CommandRegistry) {
        self.define_unknown_contents(variable, registry);
        self.record_incoming_binding_origin(variable, registry);
    }

    /// Prove that an authored substitution reads this activation's incoming slot.
    /// The caller validates the ordinary formal binding plan separately. This
    /// preserves the physical cell and rejects later writes, aliases to other
    /// storage, uncertain observers and missing values.
    #[must_use]
    pub fn incoming_activation_slot(
        &self,
        spelling: &str,
        slot: &str,
        registry: &CommandRegistry,
    ) -> Option<CellIdentity> {
        let place = resolve_substitution_access(
            spelling,
            self,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let cell = place.cell.as_ref()?;
        (!place.dynamic
            && !place.observed
            && place.kind == PlaceKind::Scalar
            && place.index.is_none()
            && place.name == slot
            && cell.name == slot
            && cell.generation != CellGeneration::Unknown
            && matches!(&cell.owner, CellOwner::Activation(identity)
                if self.activation.as_ref() == Some(identity))
            && self.contents_presence(&place) == ContentsPresence::Defined
            && self.read_contents_origin(&place, registry) == ContentsOrigin::Incoming)
            .then(|| cell.clone())
    }

    /// Bind a literal ordinary incoming value without inheriting an older store origin.
    /// A formal that writes static or foreign storage retains the literal bytes
    /// but cannot represent that write as this activation's SSA live-in.
    pub fn bind_literal_incoming(
        &mut self,
        variable: &str,
        value: &str,
        registry: &CommandRegistry,
    ) {
        self.define_literal(variable, value, registry);
        self.record_incoming_binding_origin(variable, registry);
    }

    /// Install an authenticated decoded formal name. C formals use their
    /// declaration storage key, independently of runtime name lookup. Jim
    /// uses its general setter grammar, including qualified and indexed names.
    /// The caller owns original `ParamList` topology and argument arity.
    pub(crate) fn bind_original_formal_incoming(
        &mut self,
        name: &[u8],
        value: Option<&str>,
        original_value: Option<
            &crate::command_binding::original_name_value::OriginalProducedNameValue,
        >,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        if self
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)?
            .recipe()
            != protocol
        {
            return None;
        }
        let mut target = if protocol.is_jim084() {
            resolve_evaluated_variable_input(
                tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                self,
                false,
                registry,
                tcl_registry::TraceOperation::Write,
            )
        } else {
            let identity = self.activation.as_ref()?;
            let mut target = place::scalar(
                std::str::from_utf8(name).unwrap_or_default(),
                place::LOCAL_NS,
                false,
            );
            target.cell = Some(CellIdentity {
                owner: CellOwner::Activation(identity.clone()),
                name: NameBytes::from(name),
                generation: CellGeneration::Incoming,
                interpreter: self.interpreter.clone(),
                storage_domain: None,
                execution: self.execution,
            });
            bind_cell_identity(&mut target, self);
            project_access(target, self, tcl_registry::TraceOperation::Write)
        };
        if target.dynamic || target.observed || target.kind == PlaceKind::Unknown {
            return None;
        }
        if self.store_would_error(&target) {
            return Some(false);
        }
        if protocol.is_jim084() && target.index.is_some() {
            let mut root = target.base();
            root.kind = PlaceKind::Scalar;
            if self.original_dictionary_root_valid(&root, registry) == Some(false) {
                return Some(false);
            }
        }
        // Installing a formal is not a subsequent source store and therefore
        // does not receive WrittenAt provenance or a fabricated source offset.
        target.observed = false;
        let key = canonical_binding_value_key(&target)?;
        self.forget_literal_value(&key);
        self.value_representations.remove(&key);
        self.publish_defined_slot(&target, &key);
        if let Some(value) = value {
            self.store_literal_value(key.clone(), value.to_owned());
        }
        let incoming = target.index.is_none()
            && matches!(target.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::Activation(identity)) if self.activation.as_ref() == Some(identity));
        self.contents_origins.insert(
            key,
            if incoming {
                ContentsOrigin::Incoming
            } else {
                ContentsOrigin::Unknown
            },
        );
        if incoming
            && let (Some(text), Some(original)) = (value, original_value)
            && text.as_bytes() == original.bytes()
        {
            self.retain_original_name_value(&target, original, registry);
        }
        Some(true)
    }

    fn record_incoming_binding_origin(&mut self, variable: &str, registry: &CommandRegistry) {
        let target = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let Some(name) = canonical_binding_value_key(&target) else {
            return;
        };
        let incoming = !target.dynamic
            && !target.observed
            && target.kind == PlaceKind::Scalar
            && target.index.is_none()
            && matches!(
                target.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::Activation(identity)) if self.activation.as_ref() == Some(identity)
            );
        self.contents_origins.insert(
            name,
            if incoming {
                ContentsOrigin::Incoming
            } else {
                ContentsOrigin::Unknown
            },
        );
    }
}
