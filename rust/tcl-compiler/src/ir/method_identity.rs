// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained method body identities, independent of receiver and display names.

use super::{MethodDef, MethodKind, ProcedureId};

/// Structured authoring metadata for conservative redefinition groups.
/// This coordinate grants no object allocation, method token or dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCoordinate {
    /// Retained class presentation label; never parsed back into an allocation.
    pub class_label: String,
    /// Original member presentation label, including literal separators.
    pub member_label: String,
    /// Constructor, destructor and callable-member purposes remain distinct.
    pub kind: MethodKind,
}

impl MethodDef {
    /// Structured metadata; concatenated labels never identify a method body.
    #[must_use]
    pub fn coordinate(&self) -> MethodCoordinate {
        MethodCoordinate {
            class_label: self.class_name.clone(),
            member_label: self.method_name.clone(),
            kind: self.kind,
        }
    }
    /// Presentation only; this label may collide with another declaration.
    #[must_use]
    pub fn reporting_label(&self) -> String {
        format!("{}::{}", self.class_name, self.method_name)
    }
}

/// Every method body retained under its issuing Module's body ID.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MethodInventory(std::collections::BTreeMap<ProcedureId, MethodDef>);

impl MethodInventory {
    pub(crate) fn insert(&mut self, id: ProcedureId, method: MethodDef) -> Option<MethodDef> {
        self.0.insert(id, method)
    }
    /// Read the exact retained body; a name cannot select this ID.
    #[must_use]
    pub fn get(&self, id: &ProcedureId) -> Option<&MethodDef> {
        self.0.get(id)
    }
    /// Mutate one retained body without changing its identity.
    pub fn get_mut(&mut self, id: &ProcedureId) -> Option<&mut MethodDef> {
        self.0.get_mut(id)
    }
    /// Test exact method-body membership, including its issuer.
    #[must_use]
    pub fn contains_key(&self, id: &ProcedureId) -> bool {
        self.0.contains_key(id)
    }
    /// Retained definition count, including redefinitions and colliding displays.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Whether no method bodies are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Traverse every method with its issued ID.
    pub fn iter(&self) -> impl Iterator<Item = (&ProcedureId, &MethodDef)> {
        self.0.iter()
    }
    /// Traverse every exact body for mutation.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ProcedureId, &mut MethodDef)> {
        self.0.iter_mut()
    }
    /// Traverse exact IDs in allocation order.
    pub fn keys(&self) -> impl Iterator<Item = &ProcedureId> {
        self.0.keys()
    }
    /// Traverse every retained body.
    pub fn values(&self) -> impl Iterator<Item = &MethodDef> {
        self.0.values()
    }
    /// Mutate every retained body without reissuing IDs.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut MethodDef> {
        self.0.values_mut()
    }
    pub(crate) fn for_original_entry(
        &self,
        entry: &crate::command_binding::SourceReceiverMethodEntry,
    ) -> Option<(ProcedureId, &MethodDef)> {
        if entry.is_native_class_delegate() {
            return None;
        }
        let kind = match entry.receiver() {
            crate::command_binding::SourceMethodReceiver::Instance => MethodKind::Method,
            crate::command_binding::SourceMethodReceiver::Class => MethodKind::ClassMethod,
        };
        let mut matches = self.0.iter().filter(|(_, method)| {
            method.kind == kind
                && method
                    .span
                    .is_some_and(|span| span.start() == entry.declaration().offset)
                && method.body.executed_source.as_deref() == Some(entry.body())
                && method
                    .params
                    .iter()
                    .map(String::as_str)
                    .eq(entry.formals().iter().map(|(name, _)| name.as_str()))
        });
        let (id, method) = matches.next()?;
        matches.next().is_none().then_some((*id, method))
    }

    /// Reporting entries retain every colliding display and body.
    pub fn display_entries(&self) -> impl Iterator<Item = (String, &MethodDef)> {
        self.0
            .values()
            .map(|method| (method.reporting_label(), method))
    }
    /// Select one uniquely named authored test fixture.
    #[cfg(test)]
    pub(crate) fn unique_display(&self, label: &str) -> Option<(ProcedureId, &MethodDef)> {
        let mut matches = self
            .0
            .iter()
            .filter(|(_, method)| method.reporting_label() == label);
        let (id, method) = matches.next()?;
        matches.next().is_none().then_some((*id, method))
    }
}
impl<'a> IntoIterator for &'a MethodInventory {
    type Item = (&'a ProcedureId, &'a MethodDef);
    type IntoIter = std::collections::btree_map::Iter<'a, ProcedureId, MethodDef>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
