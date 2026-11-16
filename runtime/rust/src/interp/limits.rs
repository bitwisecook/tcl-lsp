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

//! `interp limit`, `interp recursionlimit` and `interp debug`: the limit set, the evaluation depth bounds and the time-limit tick.

use super::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::RecursionLimit;

/// `interp limit` configuration for one interpreter — the `commands` and `time`
/// limit types. The `time` limit is enforced (polled by the loop commands);
/// `commands` is stored for query/set only.
#[derive(Clone)]
pub(crate) struct LimitSet {
    cmd_command: Vec<u8>,
    cmd_granularity: i64,
    pub(super) cmd_value: Option<i64>,
    time_command: Vec<u8>,
    time_granularity: i64,
    /// Absolute wall-clock deadline as `(seconds, milliseconds)`; `None` unset.
    pub(super) time_value: Option<(i64, i64)>,
}

impl Default for LimitSet {
    fn default() -> Self {
        Self {
            cmd_command: Vec::new(),
            cmd_granularity: 1,
            cmd_value: None,
            time_command: Vec::new(),
            time_granularity: 10,
            time_value: None,
        }
    }
}

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
pub(super) const RECURSION_LIMIT: usize = 1000;

/// A native-stack safety net over **every** script-body evaluation and nested
/// command-dispatch entry —
/// control-flow bodies (`if`/`while`/`for`/`foreach`/…), proc bodies,
/// `eval`/`uplevel`/`source`, and command substitution — checked against
/// [`Interp::eval_depth`] in [`Interp::eval_script_mode`] and the actual nested
/// dispatch depth, independently of
/// [`RECURSION_LIMIT`]/[`Interp::recursion_limit`].
///
/// This is a genuinely different concern from `recursion_limit`:
/// `recursion_limit` is the user-configurable, Tcl-visible `interp
/// recursionlimit` budget (bounding *proc-call* nesting only, matching C
/// Tcl's `iPtr->numLevels`), whereas this crate's interpreter is a
/// tree-walking evaluator — unlike C Tcl's bytecode-compiled control
/// structures (which execute via a flat instruction loop, no per-nesting-
/// level native recursion), *every* nested body here costs one more group
/// of native Rust stack frames (`eval_command` → command dispatch →
/// `eval_control_body`/`run_proc` → `eval_framed`/`eval_shared_located_body`
/// → `eval_script_mode`, recursively). C Tcl has no equivalent native-stack
/// hazard for compiled control flow, so there is no directly-analogous
/// upstream constant to match here.
///
/// Empirically measured on this crate's native (non-WASM) build, run on a
/// plain 2 MiB thread stack (`cargo test`'s per-test default — the same
/// class of ambient stack budget the analyser also runs under): unguarded
/// nested `foreach` bodies overflow the stack (SIGABRT)
/// between depth 200 and 250, and — more surprisingly — plain unbounded
/// recursive *proc calls* overflow **before ever reaching the existing
/// `RECURSION_LIMIT` of 1000**, meaning that pre-existing, purely
/// Tcl-semantic cap was never actually a safe backstop against a native
/// crash on an ordinary thread stack, let alone the smaller stack a WASM
/// host may give this module (this crate is `#[cfg(not(target_arch =
/// "wasm32"))]`-agnostic and, per this crate's `Cargo.toml`, "eventually
/// builds for wasm32 as a cdylib" — a WASM host's stack budget is entirely
/// outside this crate's control, unlike a native embedding where a caller
/// can choose to run `eval_str` on a generously-sized thread the way
/// `tcl-lsp-server`/`tcl-debugger`/etc. do).
///
/// 128 is deliberately conservative — comfortably under half the measured
/// 2 MiB-stack crash threshold, so it holds real margin even against a
/// meaningfully smaller WASM stack, while still being far more headroom
/// than realistic (even generated/templated) Tcl needs: legitimate scripts
/// essentially never combine proc-call depth, control-flow nesting, and
/// command-substitution nesting to a combined total anywhere near 128.
/// Tripping it raises the same catchable `"too many nested evaluations
/// (infinite loop?)"` error `RECURSION_LIMIT` uses — the failure mode is
/// conceptually identical (too much nesting), just caught earlier for
/// native-safety reasons independent of the user-configurable budget.
pub(super) const NATIVE_EVAL_DEPTH_LIMIT: RecursionLimit = RecursionLimit(128);

#[cfg(debug_assertions)]
pub(super) fn trace_native_depth_refusal(phase: &str, eval: u32, dispatch: u32) {
    if std::env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref() == Ok("first-decline-v1") {
        eprintln!(
            "CCG_NATIVE_DEPTH_REFUSAL phase={phase} eval={eval} dispatch={dispatch} limit={}",
            NATIVE_EVAL_DEPTH_LIMIT.0
        );
    }
}

/// Build a Tcl dict (flat key/value list) object from `pairs`, releasing the
/// builder's references once the list has taken its own.
fn dict_obj(interp: &Interp, pairs: &[(&[u8], Vec<u8>)]) -> *mut TclObj {
    let mut elems: Vec<*mut TclObj> = Vec::with_capacity(pairs.len() * 2);
    for (k, v) in pairs {
        elems.push(obj::new_string_bytes(k));
        elems.push(obj::new_string_bytes(v));
    }
    let list = interp.new_list_object(&elems);
    for e in elems {
        drop_fresh(e);
    }
    list
}

/// Render an optional limit integer: the decimal bytes, or empty when unset.
fn opt_int(v: Option<i64>) -> Vec<u8> {
    v.map(|n| n.to_string().into_bytes()).unwrap_or_default()
}

/// Resolve an `interp limit` option by unambiguous prefix against `opts`
/// (C's `Tcl_GetIndexFromObj`) — through the one shared owner.
///
/// `OptionTable::abbreviating` is used rather than a hand-rolled
/// `starts_with` filter: a naive prefix filter can only ever report `bad
/// option`, but the empty word is a prefix of *every* option and C reports
/// that case as `ambiguous option ""`, not `bad option ""`. The option
/// table also owns the `", or"` enumeration in the error message, matching
/// `prefix::choice_list_bytes` rather than duplicating it.
fn resolve_limit_opt(arg: &[u8], opts: &[&[u8]]) -> Result<Vec<u8>, Vec<u8>> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", opts);
    match table.index_of(arg) {
        Ok(i) => Ok(opts[i].to_vec()),
        Err(m) => Err(m),
    }
}

/// `interp debug`'s one option word (`debugTypes[]`, `tclInterp.c`): C resolves
/// it with `Tcl_GetIndexFromObj(…, "debug option", 0)`, so `-f`/`-fr`
/// abbreviate and a miss is `bad debug option "…": must be -frame` — a
/// one-entry table is never `ambiguous`, not even for the empty word. Shared
/// with the bytecode VM through the one `tcl-cmd-core::prefix` matcher.
const DEBUG_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("debug option", &[b"-frame"]);

/// `interp limit`'s type word (`limitTypes[]`, `tclInterp.c`): the noun is
/// `limit type` and the flags are `0`, so `c`/`t` abbreviate and the empty
/// word — a prefix of both entries — is `ambiguous limit type ""`.
pub(crate) const LIMIT_TYPES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("limit type", &[b"commands", b"time"]);

/// An `interp limit` value outside its option's range (`TCL OPERATION INTERP
/// BADVALUE`).
fn bad_limit_value(message: &str) -> tcl_cmd_core::CmdError {
    tcl_cmd_core::CmdError::with_error_code(message, "TCL OPERATION INTERP BADVALUE")
}

impl Interp {
    /// `interp debug ?-frame ?bool??` on this interp. Returns the fresh result
    /// object (the `-frame N` dict, or the bool), or the error-message bytes.
    /// `-frame` is a one-way latch: setting it to false once true keeps it true.
    pub(crate) fn debug_apply(
        &mut self,
        opts: &[*mut TclObj],
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        let frame_byte: &[u8] = if self.debug_frame.get() { b"1" } else { b"0" };
        match opts.len() {
            0 => Ok(dict_obj(self, &[(b"-frame", frame_byte.to_vec())])),
            1 => {
                self.native_static_option_index(
                    opts[0],
                    DEBUG_OPTIONS.names(),
                    false,
                    "debug option",
                )?;
                Ok(obj::new_string_bytes(frame_byte))
            }
            _ => {
                self.native_static_option_index(
                    opts[0],
                    DEBUG_OPTIONS.names(),
                    false,
                    "debug option",
                )?;
                if crate::typed_value::native_boolean(opts[1], self.native_invocation_dialect())? {
                    self.invalidate_interpreter_policy();
                    self.debug_frame.set(true);
                }
                let frame_byte: &[u8] = if self.debug_frame.get() { b"1" } else { b"0" };
                Ok(obj::new_string_bytes(frame_byte))
            }
        }
    }

    /// `interp recursionlimit` get / set on this interp. `newlimit` is the
    /// optional new limit, read as C 9's `TclGetWideIntFromObj` reads it;
    /// returns the resulting limit, or the error the caller should raise (the
    /// typed read's, or `recursion limit must be > 0`). Each interp keeps its
    /// own limit, so a child raising its limit leaves the parent's untouched.
    pub(crate) fn recursion_limit_apply(
        &self,
        newlimit: Option<*mut TclObj>,
    ) -> Result<i64, tcl_cmd_core::CmdError> {
        match newlimit {
            None => Ok(self.recursion_limit.get() as i64),
            Some(word) => {
                let n = crate::typed_value::wide_int(word)?;
                if n <= 0 {
                    return Err(tcl_cmd_core::CmdError::with_error_code(
                        "recursion limit must be > 0",
                        "TCL OPERATION INTERP BADLIMIT",
                    ));
                }
                self.invalidate_interpreter_policy();
                self.recursion_limit.set(n as usize);
                Ok(n)
            }
        }
    }

    /// `interp limit limitType ?-option value …?` on this interp. Returns the
    /// fresh result object on success, or the error the caller should raise.
    pub(crate) fn limit_apply(
        &self,
        ltype: &[u8],
        opts: &[*mut TclObj],
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        match LIMIT_TYPES.index_of_cmd(ltype)? {
            0 => self.limit_commands(opts),
            _ => self.limit_time(opts),
        }
    }

    fn limit_commands(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-value"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            return Ok(dict_obj(
                self,
                &[
                    (b"-command", l.cmd_command.clone()),
                    (b"-granularity", l.cmd_granularity.to_string().into_bytes()),
                    (b"-value", opt_int(l.cmd_value)),
                ],
            ));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let l = self.limits.borrow();
            let val = match opt.as_slice() {
                b"-command" => l.cmd_command.clone(),
                b"-granularity" => l.cmd_granularity.to_string().into_bytes(),
                _ => opt_int(l.cmd_value),
            };
            return Ok(obj::new_string_bytes(&val));
        }
        // A trailing option with no value is a catchable error, not a silent
        // drop (`interp limit c commands -value 1 -granularity`).
        if opts.len() % 2 != 0 {
            return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                b"wrong # args: should be \"interp limit path commands ?-option value ...?\""
                    .to_vec(),
            ));
        }
        // The option loop may commit an earlier pair before a later pair is
        // rejected. Invalidate before its first possible policy write so that
        // partial-on-error mutation cannot leave an old token live.
        self.invalidate_interpreter_policy();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let val = opts[i + 1];
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().cmd_command = obj_bytes(val),
                b"-granularity" => {
                    let n = i64::from(crate::typed_value::int(val)?);
                    if n < 1 {
                        return Err(bad_limit_value("granularity must be at least 1"));
                    }
                    self.limits.borrow_mut().cmd_granularity = n;
                }
                _ => {
                    let n = crate::typed_value::wide_int(val)?;
                    if n < 0 {
                        return Err(bad_limit_value("command limit value must be at least 0"));
                    }
                    self.invalidate_ordinary_entry_observation();
                    self.limits.borrow_mut().cmd_value = Some(n);
                }
            }
            i += 2;
        }
        Ok(obj::new_string_bytes(b""))
    }

    fn limit_time(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-milliseconds", b"-seconds"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            let (secs, millis) = match l.time_value {
                Some((s, m)) => (s.to_string().into_bytes(), m.to_string().into_bytes()),
                None => (Vec::new(), Vec::new()),
            };
            return Ok(dict_obj(
                self,
                &[
                    (b"-command", l.time_command.clone()),
                    (b"-granularity", l.time_granularity.to_string().into_bytes()),
                    (b"-milliseconds", millis),
                    (b"-seconds", secs),
                ],
            ));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let l = self.limits.borrow();
            let val = match opt.as_slice() {
                b"-command" => l.time_command.clone(),
                b"-granularity" => l.time_granularity.to_string().into_bytes(),
                b"-seconds" => opt_int(l.time_value.map(|(s, _)| s)),
                _ => opt_int(l.time_value.map(|(_, m)| m)),
            };
            return Ok(obj::new_string_bytes(&val));
        }
        if opts.len() % 2 != 0 {
            return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                b"wrong # args: should be \"interp limit path time ?-option value ...?\"".to_vec(),
            ));
        }
        // As with command limits, parsing is incremental and can partially
        // mutate before returning an error on a later option.
        self.invalidate_interpreter_policy();
        let (mut sec, mut ms) = self.limits.borrow().time_value.unwrap_or((0, 0));
        let mut touched = self.limits.borrow().time_value.is_some();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let val = opts[i + 1];
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().time_command = obj_bytes(val),
                b"-granularity" => {
                    let n = i64::from(crate::typed_value::int(val)?);
                    if n < 1 {
                        return Err(bad_limit_value("granularity must be at least 1"));
                    }
                    self.limits.borrow_mut().time_granularity = n;
                }
                b"-seconds" => {
                    let n = crate::typed_value::wide_int(val)?;
                    if n < 0 {
                        return Err(bad_limit_value("seconds must be non-negative"));
                    }
                    sec = n;
                    touched = true;
                }
                _ => {
                    let n = crate::typed_value::wide_int(val)?;
                    if n < 0 {
                        return Err(bad_limit_value("milliseconds must be non-negative"));
                    }
                    ms = n;
                    touched = true;
                }
            }
            i += 2;
        }
        if touched {
            // Normalise excess milliseconds into seconds.
            sec = sec.saturating_add(ms.div_euclid(1000));
            ms = ms.rem_euclid(1000);
            self.invalidate_ordinary_entry_observation();
            self.limits.borrow_mut().time_value = Some((sec, ms));
        }
        Ok(obj::new_string_bytes(b""))
    }

    /// Whether this interp's `time` limit has elapsed (an absolute wall-clock
    /// deadline). `false` when no time limit is set.
    pub(crate) fn time_limit_exceeded(&self) -> bool {
        match self.limits.borrow().time_value {
            Some((secs, millis)) => {
                let deadline = i128::from(secs) * 1000 + i128::from(millis);
                self.host().clock().now_millis() >= deadline
            }
            None => false,
        }
    }

    /// Whether a `time` limit is configured at all — the cheap guard the loop
    /// commands check before paying for a wall-clock read.
    pub(crate) fn has_time_limit(&self) -> bool {
        self.limits.borrow().time_value.is_some()
    }

    /// Advance the limit-poll counter and, when a `time` limit is armed and the
    /// throttle window elapses, set the `time limit exceeded` error and return
    /// its `Code`. Called from the loop commands (`while`/`for`) each iteration
    /// so an unbounded loop under `interp limit $i time` terminates. A guarded
    /// no-op when no time limit is set.
    pub(crate) fn limit_check_tick(&mut self) -> Option<Code> {
        if let Some(code) = self.charge_tick() {
            return Some(code);
        }
        if !self.has_time_limit() {
            return None;
        }
        let t = self.limit_tick.get().wrapping_add(1);
        self.limit_tick.set(t);
        if t & 0x0FFF == 0 && self.time_limit_exceeded() {
            return Some(self.error(b"time limit exceeded"));
        }
        None
    }
}
