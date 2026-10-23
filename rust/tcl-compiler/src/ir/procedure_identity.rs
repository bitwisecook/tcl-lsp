// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Module-owned body identities, separate from command names and Native tokens.

use super::Procedure;
pub use tcl_core_types::{ProcedureId, ProcedureIdIssuer};

/// Execution-frame purpose of a retained source body.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ProcedureBodyKind {
    #[default]
    /// Named procedure declaration.
    Named,
    /// Registry-selected deferred event handler.
    Event,
    /// Anonymous application body in its own frame.
    Lambda,
    /// Script executed in a selected namespace.
    NamespaceScript,
    /// Procedure body supplied by an installed implementation.
    Installed,
    /// Original declaration retained for source analysis.
    OriginalDeclaration,
}

/// Retain every body. Display-name ambiguity cannot choose an implementation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcedureInventory(std::collections::BTreeMap<ProcedureId, Procedure>);

impl ProcedureInventory {
    pub(crate) fn insert(&mut self, id: ProcedureId, procedure: Procedure) -> Option<Procedure> {
        self.0.insert(id, procedure)
    }
    /// Read one exact body; display labels do not select it.
    #[must_use]
    pub fn get(&self, id: &ProcedureId) -> Option<&Procedure> {
        self.0.get(id)
    }
    /// Mutate one exact body without changing its identity.
    pub fn get_mut(&mut self, id: &ProcedureId) -> Option<&mut Procedure> {
        self.0.get_mut(id)
    }
    /// Test exact body membership, including module ownership.
    #[must_use]
    pub fn contains_key(&self, id: &ProcedureId) -> bool {
        self.0.contains_key(id)
    }
    /// Count retained bodies, including repeated displays.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Whether the inventory retains no bodies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Remove all bodies without resetting the Module issuer.
    pub fn clear(&mut self) {
        self.0.clear();
    }
    /// Traverse every body, preserving repeated displays.
    pub fn values(&self) -> impl Iterator<Item = &Procedure> {
        self.0.values()
    }
    /// Mutate every body while retaining its issued identity.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Procedure> {
        self.0.values_mut()
    }
    /// Traverse bodies with their Module-issued IDs.
    pub fn iter(&self) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.0.iter()
    }
    /// Mutate bodies with their unchanged Module-issued IDs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ProcedureId, &mut Procedure)> {
        self.0.iter_mut()
    }
    /// Traverse retained IDs in issuance order.
    pub fn keys(&self) -> impl Iterator<Item = &ProcedureId> {
        self.0.keys()
    }
    /// Remove this exact body, preserving other equal displays.
    pub fn remove(&mut self, id: &ProcedureId) -> Option<Procedure> {
        self.0.remove(id)
    }
    /// Filter exact bodies without reissuing their IDs.
    pub fn retain(&mut self, mut retain: impl FnMut(&ProcedureId, &mut Procedure) -> bool) {
        self.0.retain(|id, procedure| retain(id, procedure));
    }
    /// Presentation rows retain duplicates and supply no declaration selection.
    pub fn display_entries(&self) -> impl Iterator<Item = (&String, &Procedure)> {
        self.0
            .values()
            .map(|procedure| (&procedure.qualified_name, procedure))
    }
    /// Reporting labels appearing exactly once; this projection selects no body.
    pub fn unique_displays(&self) -> impl Iterator<Item = &String> {
        self.0
            .values()
            .filter(|procedure| {
                self.0
                    .values()
                    .filter(|candidate| candidate.qualified_name == procedure.qualified_name)
                    .take(2)
                    .count()
                    == 1
            })
            .map(|procedure| &procedure.qualified_name)
    }
    /// Test display presence without selecting an implementation.
    #[must_use]
    pub fn has_display(&self, display: &str) -> bool {
        self.0
            .values()
            .any(|procedure| procedure.qualified_name == display)
    }
    /// Select one uniquely named authored test fixture.
    #[cfg(test)]
    pub(crate) fn unique_display(&self, display: &str) -> Option<(&ProcedureId, &Procedure)> {
        let mut matching = self
            .0
            .iter()
            .filter(|(_, procedure)| procedure.qualified_name == display);
        let selected = matching.next()?;
        matching.next().is_none().then_some(selected)
    }
    /// Match all bodies from this original allocating instruction.
    pub fn at_declaration<'a>(
        &'a self,
        site: &crate::command_binding::CommandAllocationSite,
    ) -> impl Iterator<Item = (&'a ProcedureId, &'a Procedure)> + use<'a> {
        let site = site.clone();
        self.0
            .iter()
            .filter(move |(_, procedure)| procedure.declaration_site.as_ref() == Some(&site))
    }
}

impl<'a> IntoIterator for &'a ProcedureInventory {
    type Item = (&'a ProcedureId, &'a Procedure);
    type IntoIter = std::collections::btree_map::Iter<'a, ProcedureId, Procedure>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a> IntoIterator for &'a mut ProcedureInventory {
    type Item = (&'a ProcedureId, &'a mut Procedure);
    type IntoIter = std::collections::btree_map::IterMut<'a, ProcedureId, Procedure>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

/// Installed lowerings carried by the readonly authored body unit of the same
/// evaluated `proc`. C Tcl creates one command per evaluated `proc`, so the
/// carrying unit is that command's one analysis unit; each carried lowering
/// keeps its own Module-issued ID, which remains its Source SSA identity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarriedInstalledLowerings(std::collections::BTreeMap<ProcedureId, CarriedLowering>);

#[derive(Debug, Clone, PartialEq)]
struct CarriedLowering {
    unit: ProcedureId,
    procedure: Procedure,
}

impl CarriedInstalledLowerings {
    pub(crate) fn insert(&mut self, id: ProcedureId, unit: ProcedureId, procedure: Procedure) {
        self.0.insert(id, CarriedLowering { unit, procedure });
    }
    /// Read one carried lowering under its own issued ID.
    #[must_use]
    pub fn get(&self, id: &ProcedureId) -> Option<&Procedure> {
        self.0.get(id).map(|carried| &carried.procedure)
    }
    /// Mutate one carried lowering without changing its identity or unit.
    pub fn get_mut(&mut self, id: &ProcedureId) -> Option<&mut Procedure> {
        self.0.get_mut(id).map(|carried| &mut carried.procedure)
    }
    /// The body unit that carries this lowering.
    #[must_use]
    pub fn unit_of(&self, id: &ProcedureId) -> Option<ProcedureId> {
        self.0.get(id).map(|carried| carried.unit)
    }
    /// Test exact lowering membership.
    #[must_use]
    pub fn contains_key(&self, id: &ProcedureId) -> bool {
        self.0.contains_key(id)
    }
    /// Count carried lowerings; they are not analysis units.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Whether no unit carries an Installed lowering.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Remove every carried lowering.
    pub fn clear(&mut self) {
        self.0.clear();
    }
    /// Remove one carried lowering, returning its carrying unit and body.
    pub fn remove(&mut self, id: &ProcedureId) -> Option<(ProcedureId, Procedure)> {
        self.0
            .remove(id)
            .map(|carried| (carried.unit, carried.procedure))
    }
    /// Traverse carried lowerings with their own issued IDs.
    pub fn lowerings(&self) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.0.iter().map(|(id, carried)| (id, &carried.procedure))
    }
    /// Traverse carried lowering IDs in issuance order.
    pub fn keys(&self) -> impl Iterator<Item = &ProcedureId> {
        self.0.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Module, Script};

    fn body(display: &str, text: &str) -> Procedure {
        Procedure {
            name: "p".into(),
            qualified_name: display.into(),
            publication: None,
            declaration_site: None,
            body_kind: ProcedureBodyKind::Named,
            params: vec![],
            span: tcl_lexer::Span::new(0, 0),
            body: Script::default(),
            params_raw: String::new(),
            body_source: Some(text.into()),
            body_offset: 0,
            namespace_scoped: false,
            base_priority: 500,
        }
    }

    #[test]
    fn repeated_displays_retain_both_bodies_and_refuse_label_selection() {
        let mut module = Module::default();
        let first = module.retain_procedure(body("::a:::p", "return LEFT"));
        let second = module.retain_procedure(body("::a:::p", "return RIGHT"));
        assert_ne!(first, second);
        assert_eq!(module.procedures.len(), 2);
        assert_eq!(
            module
                .procedures
                .get(&first)
                .unwrap()
                .body_source
                .as_deref(),
            Some("return LEFT")
        );
        assert_eq!(
            module
                .procedures
                .get(&second)
                .unwrap()
                .body_source
                .as_deref(),
            Some("return RIGHT")
        );
        assert!(module.procedures.unique_display("::a:::p").is_none());
        assert!(
            !module
                .procedures
                .unique_displays()
                .any(|name| name == "::a:::p")
        );
    }

    #[test]
    fn cloned_module_retains_existing_ids_and_issues_distinct_new_bodies() {
        let mut module = Module::default();
        let existing = module.retain_procedure(body("::p", "return FIRST"));
        let mut clone = module.clone();
        assert!(clone.procedures.get(&existing).is_some());
        let left = module.retain_procedure(body("::left", "return LEFT"));
        let right = clone.retain_procedure(body("::right", "return RIGHT"));
        assert_ne!(left, right);
        assert!(module.procedures.get(&right).is_none());
        assert!(clone.procedures.get(&left).is_none());
        let foreign = Module::default().retain_procedure(body("::p", "return FIRST"));
        assert!(module.procedures.get(&foreign).is_none());
    }
}

impl core::ops::Index<&ProcedureId> for ProcedureInventory {
    type Output = Procedure;
    fn index(&self, id: &ProcedureId) -> &Self::Output {
        &self.0[id]
    }
}
