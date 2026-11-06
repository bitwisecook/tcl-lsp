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

//! A compile service restricted to one admitted root script.
//!
//! This is input restriction only: the canonical compiler's module is returned
//! unchanged, and the owning VM checks the module's closure itself. Every
//! other compilation, including nested scripts, substitution templates,
//! procedures and command plans, is refused.

use std::sync::Arc;

use tcl_bytecode::ModuleAsm;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{
    CompileError, CompileService, NativeCompilationEntry, ScriptCommandPlan,
    ScriptCompileTargetBytes, SourceImage,
};

use super::BytecodeCompileService;

/// The canonical compiler over one exact shared store, accepting only the
/// admitted candidate as a plain root script.
pub struct ApplicationInputCompileService {
    canonical: BytecodeCompileService,
    admitted: SourceImage,
}

impl ApplicationInputCompileService {
    /// Accept exactly `candidate`, compiled against `registry` itself under
    /// `profile`.
    #[must_use]
    pub fn new(
        profile: &'static tcl_dialect::DialectProfile,
        registry: Arc<CommandRegistry>,
        candidate: &str,
    ) -> Self {
        Self {
            canonical: BytecodeCompileService::for_shared_registry(profile, registry),
            admitted: SourceImage::native(candidate.as_bytes()),
        }
    }
}

fn refused() -> CompileError {
    CompileError::Unsupported("the application service compiles only its admitted script".into())
}

impl CompileService for ApplicationInputCompileService {
    type Module = ModuleAsm;

    fn compile(&self, _source: &str) -> Result<ModuleAsm, CompileError> {
        Err(refused())
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        if target.source != &self.admitted || !target.namespace.is_root() {
            return Err(refused());
        }
        self.canonical
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }

    fn script_command_plan_for_profile(
        &self,
        _source: &str,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        Err(refused())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_admitted_root_script_compiles() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let service = ApplicationInputCompileService::new(
            profile,
            Arc::new(CommandRegistry::build_default().project_for_profile(profile)),
            "puts 5",
        );
        assert!(service.compile("puts 5").is_err());
        assert!(service.compile_for_profile("puts 5", profile).is_err());
        assert!(service.compile_traced("puts 5").is_err());
        assert!(
            service
                .script_command_plan_for_profile("puts 5", profile)
                .is_err()
        );
        let entry = crate::test_support::captured_native_entry(profile);
        let root = tcl_runtime_api::ByteNamespacePath::root();
        for (source, admitted) in [("puts 5", true), ("puts 6", false)] {
            let image = SourceImage::native(source.as_bytes());
            assert_eq!(
                service
                    .compile_plain_script_bytes_with_entry(
                        ScriptCompileTargetBytes {
                            source: &image,
                            namespace: &root,
                        },
                        profile,
                        &entry,
                    )
                    .is_ok(),
                admitted,
                "{source}"
            );
            assert!(
                service
                    .compile_script_bytes_with_entry(
                        ScriptCompileTargetBytes {
                            source: &image,
                            namespace: &root,
                        },
                        profile,
                        &entry,
                    )
                    .is_err()
            );
        }
    }
}
