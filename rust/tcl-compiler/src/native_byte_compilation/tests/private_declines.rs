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

//! Declines that stay private to one worker and admit no other ensemble worker.

use super::*;

#[test]
fn dynamic_string_is_decline_is_private_and_does_not_admit_ensemble_workers() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::is".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker]);
    install_original_target_fixture(&mut entry);
    let direct = source_module(b"private $class -strict $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &direct.top_level.statements[0] else {
        panic!("original private call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    let public = source_module(b"ensemble member $class -strict $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &public.top_level.statements[0] else {
        panic!("original public call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .ensemble
        .as_mut()
        .unwrap()
        .map[0]
        .1
        .push(Some("prefix".into()));
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
}

#[test]
fn dynamic_format_word_decline_keeps_original_dispatch_owner() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut format = binding(
        &entry.namespaces[0],
        b"format",
        NativeCompilerHookPresence::Present,
    );
    format.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "format".into(),
        ensemble: None,
    });
    entry.commands.push(format);
    let module = source_module(b"format [side format] [side value]", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original format invocation")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].token += 1;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].has_execution_trace = true;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].has_execution_trace = false;
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .registry_identity = "::foreign::format".into();
    assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
}

#[test]
fn dynamic_format_word_zero_two_values_keep_original_owner() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut format = binding(
        &entry.namespaces[0],
        b"format",
        NativeCompilerHookPresence::Present,
    );
    format.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "format".into(),
        ensemble: None,
    });
    entry.commands.push(format);
    for source in [
        b"format [side format]".as_slice(),
        b"format [side format] [side a] [side b]".as_slice(),
    ] {
        let mut entry = entry.clone();
        let module = source_module(source, &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original format invocation")
        };
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].token += 1;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].has_execution_trace = true;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].has_execution_trace = false;
        entry.commands[0]
            .compiler
            .as_mut()
            .unwrap()
            .registry_identity = "::foreign::format".into();
        assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
    }
}

#[test]
fn ascii_failindex_decline_is_private_and_does_not_admit_ensemble_workers() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::is".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker]);
    install_original_target_fixture(&mut entry);
    let direct = source_module(b"private ascii -failindex index $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &direct.top_level.statements[0] else {
        panic!("original private call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    let public = source_module(b"ensemble member ascii -failindex index $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &public.top_level.statements[0] else {
        panic!("original public call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .ensemble
        .as_mut()
        .unwrap()
        .map[0]
        .1
        .push(Some("prefix".into()));
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
}
