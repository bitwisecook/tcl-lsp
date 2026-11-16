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

//! The local variable table: names to compiled local slots.

use std::collections::HashMap;

/// Maps variable names to local variable table (LVT) slot indices.
#[derive(Debug, Clone, Default)]
pub struct LocalVarTable {
    compiler_protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    slots: Vec<tcl_runtime_api::NameBytes>,
    pub(super) index: HashMap<tcl_runtime_api::NameBytes, usize>,
    /// Slots the *compiler* allocated for its own bookkeeping — a `dict for`
    /// iterator, a `catch` result temporary — rather than for a variable the
    /// source names.
    ///
    /// They share the table with source locals but are not the source's, and
    /// a consumer asking "does this body name that variable?" must not be
    /// fooled by one. The generated names carry a `#` prefix to avoid
    /// colliding in practice, but that prefix is legal in a Tcl variable name,
    /// so the marker is recorded rather than inferred.
    synthetic: std::collections::HashSet<usize>,
}

impl LocalVarTable {
    /// Configure the independently selected compiler-name comparison.
    /// Existing primary keys and declaration order remain unchanged.
    pub fn set_native_protocol(
        &mut self,
        protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    ) {
        self.compiler_protocol = protocol;
    }

    /// Retained compiler selection; absence uses exact artifact-name lookup.
    #[must_use]
    pub const fn native_protocol(
        &self,
    ) -> Option<tcl_syntax::naming::NativeCompiledVariableProtocol> {
        self.compiler_protocol
    }

    /// Find a source local through the configured compiler comparison.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.find_bytes(name.as_bytes())
    }

    /// Find a source local without decoding its original counted name.
    #[must_use]
    pub fn find_bytes(&self, name: &[u8]) -> Option<usize> {
        match self.compiler_protocol {
            Some(protocol) => self.find_native(protocol, name),
            None => self.index.get(name).copied(),
        }
    }

    /// Create a new LVT, optionally pre-populating with procedure parameters.
    #[must_use]
    pub fn new(params: &[&str]) -> Self {
        let mut lvt = Self::default();
        for &p in params {
            lvt.intern(p);
        }
        lvt
    }

    /// Allocate a distinct slot index for compiler bookkeeping.
    ///
    /// See [`LocalVarTable::synthetic`]; [`Self::is_source_local`] is how a
    /// consumer tells the two apart.
    pub fn intern_synthetic(&mut self, name: &str) -> usize {
        let slot = self.slots.len();
        self.slots.push(tcl_runtime_api::NameBytes::from(name));
        self.synthetic.insert(slot);
        slot
    }

    /// Reserve a distinct native temporary with no source name or lookup key.
    /// Empty source names remain independent named slots.
    pub fn intern_anonymous(&mut self) -> usize {
        let slot = self.slots.len();
        self.slots.push(tcl_runtime_api::NameBytes::default());
        self.synthetic.insert(slot);
        slot
    }

    /// Whether the compiled source names a local called `name`.
    ///
    /// False both for a name with no slot at all and for one whose only slot
    /// is a compiler temporary.
    #[must_use]
    pub fn is_source_local(&self, name: &str) -> bool {
        self.is_source_local_bytes(name.as_bytes())
    }

    /// Get or create a slot index for `name`.
    pub fn intern(&mut self, name: &str) -> usize {
        self.intern_bytes(name.as_bytes())
    }

    /// Get or create a local slot for an exact native storage key.
    pub fn intern_bytes(&mut self, name: &[u8]) -> usize {
        if let Some(protocol) = self.compiler_protocol {
            return self.intern_native(protocol, name);
        }
        if let Some(&idx) = self.index.get(name) {
            return idx;
        }
        let idx = self.slots.len();
        let name = tcl_runtime_api::NameBytes::from(name);
        self.slots.push(name.clone());
        self.index.insert(name, idx);
        idx
    }

    /// Create a table from actual bound formal storage keys in declaration order.
    #[must_use]
    pub fn from_names(params: &[tcl_runtime_api::NameBytes]) -> Self {
        let mut table = Self::default();
        for name in params {
            table.intern_bytes(name.as_bytes());
        }
        table
    }

    /// Retain every bound formal declaration slot, including repeated names.
    /// The selected compiler comparator governs later lookup; declaration
    /// storage never discards a name because it compares equal to another slot.
    #[must_use]
    pub fn from_native_names(params: &[tcl_runtime_api::NameBytes]) -> Self {
        let mut table = Self::default();
        for name in params {
            table.append_native_source(name.as_bytes());
        }
        table
    }

    /// Preserve physical local-cache indices, including unnamed temporary slots.
    /// Names and `None` markers come from the retained native layout; this
    /// constructor supplies no compiler policy or execution-frame authority.
    #[must_use]
    pub fn from_native_slot_names(names: &[Option<tcl_runtime_api::NameBytes>]) -> Self {
        let mut table = Self::default();
        for name in names {
            if let Some(name) = name {
                table.append_native_source(name.as_bytes());
            } else {
                let slot = table.slots.len();
                table.slots.push(tcl_runtime_api::NameBytes::default());
                table.synthetic.insert(slot);
            }
        }
        table
    }

    /// Snapshot the ordered native local layout without inventing source names
    /// for compiler temporary slots.
    #[must_use]
    pub fn native_slot_names(&self) -> Vec<Option<tcl_runtime_api::NameBytes>> {
        self.slots
            .iter()
            .enumerate()
            .map(|(slot, name)| (!self.synthetic.contains(&slot)).then(|| name.clone()))
            .collect()
    }

    fn append_native_source(&mut self, name: &[u8]) -> usize {
        let slot = self.slots.len();
        let name = tcl_runtime_api::NameBytes::from(name);
        self.slots.push(name.clone());
        self.index.entry(name).or_insert(slot);
        slot
    }

    /// Select the first nonsynthetic source slot under the actual compiler rule.
    /// A comparison match never changes either original primary name.
    #[must_use]
    pub fn find_native(
        &self,
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
        name: &[u8],
    ) -> Option<usize> {
        self.slots.iter().enumerate().find_map(|(slot, original)| {
            (!self.synthetic.contains(&slot)
                && protocol.compiled_local_names_equal(original.as_bytes(), name))
            .then_some(slot)
        })
    }

    /// Reuse the selected source slot or append the complete original byte name.
    pub fn intern_native(
        &mut self,
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
        name: &[u8],
    ) -> usize {
        self.find_native(protocol, name)
            .unwrap_or_else(|| self.append_native_source(name))
    }

    /// Whether the selected native compiler comparison finds a source local.
    #[must_use]
    pub fn is_source_local_native(
        &self,
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
        name: &[u8],
    ) -> bool {
        self.find_native(protocol, name).is_some()
    }

    /// Whether an exact native source key has a nonsynthetic local slot.
    #[must_use]
    pub fn is_source_local_bytes(&self, name: &[u8]) -> bool {
        self.find_bytes(name).is_some()
    }

    /// Return original native storage keys in local slot order.
    #[must_use]
    pub fn entries(&self) -> &[tcl_runtime_api::NameBytes] {
        &self.slots
    }

    /// Number of slots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}
