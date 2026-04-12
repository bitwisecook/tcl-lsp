//! Main emitter loop and public API.
//!
//! Split across multiple files by responsibility, not reproducing
//! the Python mixin composition:
//!
//! - [`ordering`]  — CFG linearisation and loop body detection
//! - [`terminator`] — CFG terminator emission (goto/branch/return)
//! - [`proc_defs`] — interleaved proc definition emission
//! - [`loop_blocks`] — per-block handlers for foreach/while/for
//! - [`try_blocks`] — try/finally CFG pattern detection
//! - [`generate`] — top-level dispatcher
//! - [`bytecoded`] — registry-backed codegen hook dispatch
//!
//! Ported from `core/compiler/codegen/_emitter.py` and
//! `core/compiler/codegen/_bytecoded.py`.

#![allow(dead_code)]

pub mod bytecoded;
pub mod generate;
pub mod loop_blocks;
pub mod ordering;
pub mod proc_defs;
pub mod terminator;
pub mod try_blocks;

use std::collections::HashMap;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::{Module as IrModule, Procedure as IrProcedure};

use super::{CodegenCtx, FunctionAsm, ModuleAsm};

/// Generate bytecode assembly for a single CFG function.
///
/// When `is_proc` is true, variables are accessed via the LVT
/// (`loadScalar1`/`storeScalar1`). When `false` (top-level scripts),
/// variables are accessed via the stack (`loadStk`/`storeStk`).
#[must_use]
pub fn codegen_function(cfg: &CfgFunction, params: &[&str], is_proc: bool) -> FunctionAsm {
    codegen_function_with_procs(cfg, params, is_proc, &[])
}

/// Generate bytecode assembly for a CFG function, with pending proc defs.
///
/// Used by `codegen_module` to interleave proc definitions at their
/// source positions within the top-level script.
#[must_use]
pub fn codegen_function_with_procs(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    proc_defs: &[IrProcedure],
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params);
    generate::generate(&mut ctx, cfg, proc_defs)
}

/// Generate bytecode assembly for an entire module.
#[must_use]
pub fn codegen_module(cfg_module: &CfgModule, ir_module: &IrModule) -> ModuleAsm {
    let top = codegen_function(&cfg_module.top_level, &[], false);
    let mut procs: HashMap<String, FunctionAsm> = HashMap::new();
    for (qname, cfg_func) in &cfg_module.procedures {
        let ir_proc = ir_module.procedures.get(qname);
        // Skip procs defined inside namespace eval — tclsh compiles
        // them lazily at runtime, not at compile time.
        if let Some(p) = ir_proc {
            if p.namespace_scoped {
                continue;
            }
        }
        let params: Vec<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        procs.insert(qname.clone(), codegen_function(cfg_func, &params, true));
    }
    ModuleAsm {
        top_level: top,
        procedures: procs,
    }
}
