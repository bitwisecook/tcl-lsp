// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ir::ProcedureId;
use std::borrow::Cow;

pub(crate) trait FixtureBodyLabel {
    fn fixture_label(&self) -> Cow<'_, str>;
}
impl FixtureBodyLabel for crate::ir::Procedure {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.qualified_name)
    }
}
impl FixtureBodyLabel for crate::ir::MethodDef {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Owned(self.reporting_label())
    }
}
impl FixtureBodyLabel for crate::compilation_unit::FunctionUnit {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
impl FixtureBodyLabel for super::ProcSummary {
    fn fixture_label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.qualified_name)
    }
}

// A fixture label must identify one genuinely issued body before its ID is used.
pub(crate) fn fixture_id<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    label: &str,
) -> ProcedureId {
    fixture_id_if_present(rows, label)
        .unwrap_or_else(|| panic!("authored fixture body {label:?} absent"))
}
pub(crate) fn fixture_id_if_present<'a, T: FixtureBodyLabel + 'a>(
    rows: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    label: &str,
) -> Option<ProcedureId> {
    let mut matching = rows
        .into_iter()
        .filter(|(_, body)| body.fixture_label() == label);
    let first = matching.next().map(|(id, _)| *id);
    assert!(
        matching.next().is_none(),
        "authored fixture body {label:?} ambiguous"
    );
    first
}

pub(crate) fn fixture_proc<'a>(
    analysis: &'a super::InterproceduralAnalysis,
    label: &str,
) -> Option<&'a super::ProcSummary> {
    let id = fixture_id_if_present(&analysis.procedures, label)?;
    analysis.procedures.get(&id)
}
pub(crate) fn edge_labels(
    analysis: &super::InterproceduralAnalysis,
    edges: &[ProcedureId],
) -> Vec<String> {
    edges
        .iter()
        .map(|id| {
            analysis
                .procedures
                .get(id)
                .map(|body| body.qualified_name.clone())
                .or_else(|| {
                    analysis
                        .methods
                        .get(id)
                        .map(|body| body.base.qualified_name.clone())
                })
                .expect("fixture edge retains its exact body summary")
        })
        .collect()
}
