// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::body_identity::{fixture_id, fixture_id_if_present};

pub struct FixtureEscapes {
    pub module: tcl_compiler::ir::Module,
    pub summaries: std::collections::HashMap<
        tcl_core_types::BodyFrameId,
        tcl_compiler::var_escape::ProcEscapeSummary,
    >,
}

impl FixtureEscapes {
    pub fn new(
        module: tcl_compiler::ir::Module,
        summaries: std::collections::HashMap<
            tcl_core_types::BodyFrameId,
            tcl_compiler::var_escape::ProcEscapeSummary,
        >,
    ) -> Self {
        Self { module, summaries }
    }
    pub fn get(&self, display: &str) -> Option<&tcl_compiler::var_escape::ProcEscapeSummary> {
        let id = if display == tcl_compiler::var_escape::TOP_LEVEL_QNAME {
            tcl_core_types::BodyFrameId::Top(self.module.procedure_ids.module_id())
        } else {
            tcl_core_types::BodyFrameId::Body(fixture_id_if_present(
                &self.module.procedures,
                display,
            )?)
        };
        self.summaries.get(&id)
    }
    pub fn contains_key(&self, display: &str) -> bool {
        self.get(display).is_some()
    }
}

impl std::ops::Index<&str> for FixtureEscapes {
    type Output = tcl_compiler::var_escape::ProcEscapeSummary;
    fn index(&self, display: &str) -> &Self::Output {
        let id = if display == tcl_compiler::var_escape::TOP_LEVEL_QNAME {
            tcl_core_types::BodyFrameId::Top(self.module.procedure_ids.module_id())
        } else {
            tcl_core_types::BodyFrameId::Body(fixture_id(&self.module.procedures, display))
        };
        &self.summaries[&id]
    }
}
