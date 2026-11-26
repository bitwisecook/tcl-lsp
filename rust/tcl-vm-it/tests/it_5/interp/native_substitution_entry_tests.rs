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

//! Actual template entry is independently admitted from scripts and procedures.
use tcl_runtime_api::{
    ByteNamespacePath, CompileService, SourceImage,
    native_substitution::{
        NativeSelectedSubstitutionHandler, NativeSubstitutionCompilationEntry,
        NativeSubstitutionFlags, NativeSubstitutionTarget,
    },
};

#[test]
fn substitution_entry_requires_selected_handler_and_independent_source_context() {
    // Implementation proof: naming.substitution.entry-purpose-and-cache-currency
    // docs/design/analysis/name-resolution-proofs/substitution-entry-purpose-and-cache-currency.md
    struct ScriptOnly;
    impl CompileService for ScriptOnly {
        type Module = tcl_bytecode::ModuleAsm;
        fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            panic!("a template must never reach the ordinary script fallback")
        }
    }
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let snapshot = crate::fixture::root_compilation_entry(&mut vm);
        let selected = snapshot.commands.iter().find(|row| matches!(&row.implementation,
            tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry { identity, .. } if identity == "subst")).unwrap();
        let flags = NativeSubstitutionFlags::new(true, true, true);
        let handler = NativeSelectedSubstitutionHandler::capture(&snapshot, selected).unwrap();
        let entry =
            NativeSubstitutionCompilationEntry::from_selected_handler(&snapshot, handler, flags)
                .unwrap();
        let source = SourceImage::native(b"$x".as_slice());
        let namespace = ByteNamespacePath::root();
        let target = NativeSubstitutionTarget {
            source: &source,
            namespace: &namespace,
        };
        assert!(matches!(
            ScriptOnly.compile_substitution_with_entry(target, vm.dialect_profile(), entry),
            Err(tcl_runtime_api::CompileError::Unsupported(_))
        ));
        let compiler =
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(vm.dialect_profile());
        let module = compiler
            .compile_substitution_with_entry(target, vm.dialect_profile(), entry)
            .unwrap();
        assert_eq!(module.source, source);
        assert!(module.procedures.is_empty());
        let document = SourceImage::document("$x");
        assert!(
            compiler
                .compile_substitution_with_entry(
                    NativeSubstitutionTarget {
                        source: &document,
                        namespace: &namespace
                    },
                    vm.dialect_profile(),
                    entry
                )
                .is_err()
        );
        let mut wrong = selected.clone();
        wrong.implementation =
            tcl_runtime_api::native_compilation::NativeCommandImplementation::Opaque;
        assert!(NativeSelectedSubstitutionHandler::capture(&snapshot, &wrong).is_err());
        let mut foreign = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let foreign_snapshot = crate::fixture::root_compilation_entry(&mut foreign);
        assert!(
            NativeSubstitutionCompilationEntry::from_selected_handler(
                &foreign_snapshot,
                handler,
                flags
            )
            .is_err()
        );
        let mut unknown = snapshot.clone();
        unknown.closed = false;
        assert!(
            NativeSubstitutionCompilationEntry::from_selected_handler(&unknown, handler, flags)
                .is_err()
        );
        unknown = snapshot.clone();
        unknown.source_string_protocol = None;
        assert!(
            NativeSubstitutionCompilationEntry::from_selected_handler(&unknown, handler, flags)
                .is_err()
        );
        unknown = snapshot.clone();
        unknown.execution_point = None;
        assert!(
            NativeSubstitutionCompilationEntry::from_selected_handler(&unknown, handler, flags)
                .is_err()
        );
        unknown = snapshot.clone();
        unknown.source_string_protocol = Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V8_5,
        ));
        assert!(
            NativeSubstitutionCompilationEntry::from_selected_handler(&unknown, handler, flags)
                .is_err()
        );
    }
    for engine in ["tcl8.4", "tcl8.5", "jim"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let snapshot = crate::fixture::root_compilation_entry(&mut vm);
        if let Some(selected) = snapshot.commands.iter().find(|row| matches!(&row.implementation,
            tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry { identity, .. } if identity == "subst")) {
            assert!(NativeSelectedSubstitutionHandler::capture(&snapshot, selected).is_err());
        }
    }
}
