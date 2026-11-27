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

//! Authored uplevel source words, independent of native literal registration.

use super::Value;
use crate::compiled::NativeCompilerPolicy;
use tcl_native_model::native_eval_object::{EvalObjectPurpose, NativeEvalObjectProtocol};
use tcl_syntax::value::ValueError;

#[derive(Clone, Copy)]
pub(crate) struct LogicalUpLevelSourceEntry {
    policy: NativeCompilerPolicy,
}

impl LogicalUpLevelSourceEntry {
    pub(crate) fn is_current(self, policy: &NativeCompilerPolicy) -> bool {
        self.policy == *policy
    }

    pub(crate) fn value(self, literal: &tcl_bytecode::NativeStringLiteral) -> Value {
        debug_assert!(self.policy.source_word_provider.is_some());
        Value::new_native_string_bytes(literal.bytes())
    }
}

impl crate::Vm {
    pub(crate) fn logical_uplevel_source_entry(
        &self,
        selected: NativeEvalObjectProtocol,
        purpose: EvalObjectPurpose,
    ) -> Result<Option<LogicalUpLevelSourceEntry>, ValueError> {
        let dialect = self.native_invocation_dialect();
        if purpose != EvalObjectPurpose::UpLevel || dialect.native_eval_object_protocol().is_some()
        {
            return Ok(None);
        }
        let Some(provider) = self.logical_providers.eval_object else {
            return Ok(None);
        };
        if dialect.eval_object_protocol(Some(provider)) != Some(selected)
            || selected.compiles_source(purpose)
        {
            return Ok(None);
        }
        let unavailable =
            || ValueError::CommandProtocolUnavailable("logical uplevel source word issuer");
        let words = self
            .logical_providers
            .source_words
            .ok_or_else(unavailable)?;
        let source = dialect
            .logical_source_string_protocol(words, self.native_execution_profile())
            .ok_or_else(unavailable)?;
        if self.source_string_protocol() != Some(source) {
            return Err(unavailable());
        }
        let engine = self.actual_engine_profile.ok_or_else(unavailable)?;
        if self
            .native_core_pin
            .as_ref()
            .is_none_or(|core| !std::ptr::eq(core.profile, engine))
        {
            return Err(unavailable());
        }
        Ok(Some(LogicalUpLevelSourceEntry {
            policy: self.native_compiler_policy(),
        }))
    }
}
