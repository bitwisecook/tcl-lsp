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

//! Original `proc` definitions under the general and native tiers: the body, the name word and a replaced or renamed definer.

use super::*;

#[test]
fn general_original_definition_materialises_its_body_once() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set reads 0\n\
                  proc body_value {} {incr ::reads; return {return MATERIALISED}}\n\
                  proc p {} [body_value]\n\
                  set answer [p]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_materialised_body",
            source,
            "list $answer $reads [string equal [info body p] {return MATERIALISED}]",
        ),
        "MATERIALISED 1 1",
    );
}

#[test]
fn general_original_definition_keeps_braced_body_text_inert() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set reads 0\n\
                  proc body_value {} {incr ::reads; return UNEXPECTED}\n\
                  proc p {} {return {[body_value]}}\n\
                  set answer [p]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_inert_body",
            source,
            "list [string equal $answer {[body_value]}] $reads [string equal [info body p] {return {[body_value]}}]",
        ),
        "1 0 1",
    );
}

#[test]
fn general_original_definition_evaluates_its_name_once() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set calls 0\nproc next_name {} {incr ::calls; return :audit}\n\
                  proc [next_name] {} {return COUNTED}\nset answer [:audit]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_counted_name",
            source,
            "list $answer $calls [info commands audit]",
        ),
        "COUNTED 1 {}",
    );
}

#[test]
fn general_original_definition_preserves_unknown_name_error() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "proc $unknown_name {} {return NEVER}\nset after SHOULD_NOT_RUN\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_unknown_name",
            source,
            "list [info exists after] [info exists unknown_name] [info commands :audit]",
        ),
        "0 0 {}",
    );
}

#[test]
fn general_original_definition_resolves_a_replaced_definer() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "rename proc stock_proc\nstock_proc proc args {return SHADOW}\n\
                  set answer [proc :audit {} {return ORIGINAL}]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_replaced_definer",
            source,
            "list $answer [info commands :audit]",
        ),
        "SHADOW {}",
    );
}

#[test]
fn native_original_definition_preserves_a_replaced_definer() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "rename proc stock_proc\nstock_proc proc args {return SHADOW}\n\
                  set answer [proc :audit {} {return ORIGINAL}]\n";
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let mut emitted = compile_wasm(
        &unit,
        &registry,
        WasmCompileOptions::runtime_linked().native_tier(),
    );
    assert_eq!(
        run_link_bytes(
            &runtime,
            "native_original_argv_replaced_definer",
            &emitted.to_bytes(),
            source,
            "list $answer [info commands :audit]",
        ),
        "SHADOW {}",
    );
}

#[test]
fn general_original_definition_preserves_a_renamed_stock_definer() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source =
        "rename proc stock_proc\nstock_proc :audit {} {return ORIGINAL}\nset answer [:audit]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_renamed_stock_definer",
            source,
            "list $answer [info commands audit]",
        ),
        "ORIGINAL {}",
    );
}

#[test]
fn general_original_definition_preserves_a_materialised_name_word() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set name :audit\nproc $name {} {return ORIGINAL}\nset answer [:audit]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_materialised_name",
            source,
            "list $answer [info commands audit]",
        ),
        "ORIGINAL {}",
    );
}
