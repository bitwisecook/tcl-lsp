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

//! Resource limits and confinement: the `time`, `commands` and value-size limits, `interp limit`, and confining stores to the running procedure.

use super::{LimitSet, VarBinding, VarTableOwner, Vm, err};
use crate::value::Value;
use crate::vars::VarState;
use tcl_runtime_api::Completion;

/// The child-as-command (`$child sub …`) subcommand words, in C table order
/// (`options[]` in `NRChildCmd`, `tclInterp.c`), resolved with
/// `Tcl_GetIndexFromObj(…, "option", 0)` — so `ev` abbreviates `eval` and the
/// empty word is `ambiguous option ""`.
///
/// C's table is `alias, aliases, bgerror, debug, eval, expose, hide, hidden,
/// issafe, invokehidden, limit, marktrusted, recursionlimit`; like the
/// `interp` ensemble's own table this engine names only what it dispatches,
/// dropping `alias`, `aliases`, and `bgerror`.
/// `interp limit`'s type word (`limitTypes[]`, `tclInterp.c`): the noun is
/// `limit type` and the flags are `0`, so `c`/`t` abbreviate and the empty
/// word — a prefix of both entries — is `ambiguous limit type ""`.
pub(crate) const LIMIT_TYPES: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("limit type", &["commands", "time"]);

/// Resolve an `interp limit` option by unambiguous prefix against `opts`,
/// matching C's `Tcl_GetIndexFromObj` — through the one shared owner.
///
/// A naive `starts_with` prefix filter can only ever say `bad option`: the
/// empty word is a prefix of *every* option, so it would report
/// `bad option ""` where C reports `ambiguous option ""`.
/// `OptionTable::abbreviating` owns both verdicts and the `", or"`
/// enumeration.
fn resolve_limit_opt<'a>(arg: &str, opts: &'a [&'a str]) -> Result<&'a str, String> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", opts);
    match table.index_of(arg.as_bytes()) {
        Ok(i) => Ok(opts[i]),
        Err(m) => Err(String::from_utf8_lossy(&m).into_owned()),
    }
}

/// Parse an `interp limit` integer option value with C's `Tcl_GetIntFromObj`
/// `expected integer but got "X"` wording.
/// A limit word through the selected wide integer getter, with its typed
/// failure code.
fn limit_integer(vm: &mut Vm, value: &Value) -> Result<i64, tcl_cmd_core::CmdError> {
    tcl_syntax::value::ValueOps::as_int(vm, value).map_err(Into::into)
}

/// An `interp limit` value outside its option's range (`TCL OPERATION INTERP
/// BADVALUE`).
fn bad_limit_value(message: &str) -> tcl_cmd_core::CmdError {
    tcl_cmd_core::CmdError::with_error_code(message, "TCL OPERATION INTERP BADVALUE")
}

/// Which limit an evaluation outran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// The `commands` limit.
    Commands,
    /// The `time` limit.
    WallClock,
    /// The value-size limit.
    ValueSize,
}

impl LimitKind {
    /// The error an evaluation that outran this limit fails with.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Commands => "command count limit exceeded",
            Self::WallClock => "time limit exceeded",
            Self::ValueSize => "value size limit exceeded",
        }
    }
}

impl Default for LimitSet {
    fn default() -> Self {
        Self {
            cmd_command: Value::string(""),
            cmd_granularity: 1,
            cmd_value: None,
            time_command: Value::string(""),
            time_granularity: 10,
            time_value: None,
            value_bytes: None,
            confined_stores: false,
            exceeded: None,
        }
    }
}

impl Vm {
    /// Whether this interp's `time` limit has elapsed. The stored time value is
    /// an absolute wall-clock deadline (`-seconds`/`-milliseconds`); the
    /// execution trampoline polls this so an unbounded bytecode loop still
    /// honours `interp limit $i time`. Returns `false` when no time limit is set.
    pub(crate) fn time_limit_exceeded(&self) -> bool {
        match self.limits.time_value {
            Some((secs, millis)) => {
                let deadline = i128::from(secs) * 1000 + i128::from(millis);
                self.host_rc().clock().now_millis() >= deadline
            }
            None => false,
        }
    }

    /// Whether a `time` limit is configured at all — the cheap guard the
    /// trampoline checks before paying for a wall-clock read.
    pub(crate) fn has_time_limit(&self) -> bool {
        self.limits.time_value.is_some()
    }

    /// Advance the limit-poll counter and, when a `time` limit is armed and the
    /// throttle window elapses, return the `time limit exceeded` error. Called
    /// from the bytecode trampoline (per tick) and the loop commands (per
    /// iteration) so both pure-bytecode and command-driven infinite loops are
    /// trapped. A no-op (beyond the guard) when no time limit is set.
    pub(crate) fn limit_check_tick(&mut self) -> Option<Completion<Value>> {
        if !self.has_time_limit() {
            return None;
        }
        self.counters.limit_tick = self.counters.limit_tick.wrapping_add(1);
        // Poll roughly every 4096 ticks (the low 12 bits clear).
        if self.counters.limit_tick.trailing_zeros() >= 12 && self.time_limit_exceeded() {
            return Some(self.exceed_limit_inner(LimitKind::WallClock));
        }
        None
    }

    /// Set (or clear) the absolute wall-clock deadline the `time` limit polls
    /// against, in host-clock milliseconds. The embedder-facing wrapper is
    /// [`Vm::set_wall_clock_budget`].
    pub(crate) fn set_time_limit_deadline(&mut self, deadline_millis: Option<i128>) {
        self.native_compiler_pass_owner
            .invalidate_ordinary_entry_observation();
        self.forget_exceeded(LimitKind::WallClock);
        self.limits.time_value = deadline_millis.map(|deadline| {
            let seconds = i64::try_from(deadline / 1000).unwrap_or(i64::MAX);
            let millis = i64::try_from(deadline % 1000).unwrap_or(0);
            (seconds, millis)
        });
    }

    /// Charge one command against this interp's `commands` limit, returning
    /// the `command count limit exceeded` completion when the budget is spent.
    ///
    /// Called from both command funnels — the trampoline's
    /// [`dispatch_words`](crate::exec) and the native
    /// [`invoke_command`](Vm::invoke_command) re-entry — so a body that loops
    /// in bytecode and one that loops through native re-entry are charged
    /// alike. Unlike C Tcl this checks every command rather than every
    /// `-granularity`th: the check is an increment and a compare, and an
    /// exactly-enforced budget is what the `SpecTcl` containment guarantee needs
    /// from a hook body.
    pub(crate) fn charge_command(&mut self) -> Option<Completion<Value>> {
        self.counters.commands_run = self.counters.commands_run.saturating_add(1);
        if let Some(kind) = self.limits.exceeded {
            return Some(err(kind.message()));
        }
        let limit = self.limits.cmd_value?;
        let limit = u64::try_from(limit).unwrap_or(0);
        (self.counters.commands_run > limit).then(|| self.exceed_limit_inner(LimitKind::Commands))
    }

    /// Record that the running evaluation outran `kind`, and answer its error.
    pub(crate) fn exceed_limit_inner(&mut self, kind: LimitKind) -> Completion<Value> {
        self.limits.exceeded = Some(kind);
        err(kind.message())
    }

    /// The limit the running (or last) evaluation outran, if it outran one.
    pub(crate) fn limit_exceeded_value(&self) -> Option<LimitKind> {
        self.limits.exceeded
    }

    /// Forget a recorded breach of `kind`: the limit was set again, so the
    /// evaluation stands against the new value (`Tcl_LimitSetCommands`).
    fn forget_exceeded(&mut self, kind: LimitKind) {
        if self.limits.exceeded == Some(kind) {
            self.limits.exceeded = None;
        }
    }

    /// Refuse a single allocation of `bytes` when it would exceed the armed
    /// value-size limit, returning the `value size limit exceeded` completion.
    ///
    /// The `commands` and `time` limits cannot bound this. `string repeat`
    /// builds its result in one opcode: it dispatches one command and returns
    /// in microseconds, so both budgets are still nearly full while the
    /// process has already asked the allocator for gigabytes. Left unbounded
    /// that is an OOM abort — the one failure the sandbox cannot contain and
    /// report, because it takes the whole server with it rather than the hook.
    ///
    /// Checked *before* allocating, so the refusal costs nothing and the
    /// memory is never requested.
    pub(crate) fn charge_allocation(&mut self, bytes: u64) -> Option<Completion<Value>> {
        let limit = self.limits.value_bytes?;
        (bytes > limit).then(|| self.exceed_limit_inner(LimitKind::ValueSize))
    }

    /// The value-size limit, if one is armed.
    pub(crate) fn value_size_limit_value(&self) -> Option<u64> {
        self.limits.value_bytes
    }

    /// Arm (or disarm) the value-size limit.
    pub(crate) fn set_value_size_limit_value(&mut self, limit: Option<u64>) {
        self.forget_exceeded(LimitKind::ValueSize);
        self.limits.value_bytes = limit;
    }

    /// Whether stores are confined to the running procedure's own frame.
    pub(crate) fn stores_confined_value(&self) -> bool {
        self.limits.confined_stores
    }

    /// Confine (or release) stores to the running procedure's own frame.
    pub(crate) fn set_stores_confined_value(&mut self, confined: bool) {
        self.limits.confined_stores = confined;
        if confined {
            self.scrub_host_globals_for_confinement();
        }
    }

    /// Remove every global the host's bootstrap wrote — `::env`,
    /// `::tcl_platform` and the library paths — so a body whose stores are
    /// confined reads no host environment either, and its answer depends on
    /// its arguments and the pinned release alone. Reading one raises, as
    /// reading any unset variable does.
    pub(super) fn scrub_host_globals_for_confinement(&mut self) {
        for name in tcl_platform::bootstrap::HOST_ARRAYS
            .iter()
            .chain(tcl_platform::bootstrap::HOST_PATH_GLOBALS)
        {
            self.unset_global_raw(name);
        }
    }

    /// Whether a store to `name`, resolved from level `start`, would land
    /// outside the running procedure's own frame while stores are confined:
    /// a `::`-qualified or namespace-resolved name, a global at level 0, a
    /// local linked to another frame's variable, or any level but the
    /// running one. Reads are never checked.
    pub(crate) fn store_confined(&self, name: &str, start: usize) -> bool {
        self.store_confined_bytes(name.as_bytes(), start)
            .unwrap_or(true)
    }

    pub(super) fn store_confined_bytes(
        &self,
        name: &[u8],
        start: usize,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if !self.limits.confined_stores {
            return Ok(false);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "confined variable naming",
            ))?
            .recipe();
        let input = protocol.combined_variable_input(name);
        Ok(self
            .var_binding_from_bytes(input.root().selected(), start)
            .is_none_or(|binding| self.store_binding_confined(&binding, start)))
    }

    pub(super) fn store_binding_confined(&self, binding: &VarBinding, start: usize) -> bool {
        self.limits.confined_stores
            && !(start != 0
                && start == self.current_level()
                && matches!(binding.owner, VarTableOwner::Frame(level) | VarTableOwner::CompiledLocal { level, .. } if level == start)
                && self.raw_variable_binding(binding).is_none_or(|id| {
                    !matches!(
                        self.var_arena.get(id).map(crate::vars::VarCell::state),
                        Some(VarState::Link(_) | VarState::NameLink(_))
                    )
                }))
    }

    pub(super) fn confined_store_error(name: &[u8], unset: bool) -> Completion<Value> {
        crate::command::err_with_code(
            [
                if unset {
                    b"can't unset \"".as_slice()
                } else {
                    b"can't set \"".as_slice()
                },
                name,
                b"\": stores are confined to the activation",
            ]
            .concat(),
            if unset {
                b"TCL UNSET VARNAME".as_slice()
            } else {
                b"TCL WRITE VARNAME".as_slice()
            },
        )
    }

    pub(super) fn confine_store_binding_bytes(
        &self,
        name: &[u8],
        binding: &VarBinding,
        start: usize,
        unset: bool,
    ) -> Result<(), Completion<Value>> {
        if self.store_binding_confined(binding, start) {
            return Err(Self::confined_store_error(name, unset));
        }
        Ok(())
    }

    pub(super) fn confine_store_parts_bytes(
        &mut self,
        root: &[u8],
        element: Option<&[u8]>,
        start: usize,
        unset: bool,
    ) -> Result<(), Completion<Value>> {
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if !self.limits.confined_stores {
            return Ok(());
        }
        let Some(protocol) = self.name_policy_protocol() else {
            return Err(self.refuse_host_command("confined variable naming is unavailable".into()));
        };
        let input = protocol.recipe().separate_variable_input(root, element);
        let binding = self.var_binding_from_bytes(input.root().selected(), start);
        if binding
            .as_ref()
            .is_none_or(|binding| self.store_binding_confined(binding, start))
        {
            return Err(Self::confined_store_error(root, unset));
        }
        Ok(())
    }

    pub(super) fn confine_store_bytes(
        &mut self,
        name: &[u8],
        start: usize,
        unset: bool,
    ) -> Result<(), Completion<Value>> {
        match self.store_confined_bytes(name, start) {
            Ok(false) => Ok(()),
            Ok(true) => Err(Self::confined_store_error(name, unset)),
            Err(error) => Err(crate::command::completion_from_cmd_error(
                self,
                error.into(),
            )),
        }
    }

    /// Refuse a store to `name`, resolved from level `start`, that
    /// [`Self::store_confined`] says lands outside the activation — an
    /// array's creation included. The refusal is an ordinary Tcl error
    /// raised before anything is written.
    pub(crate) fn confine_store(&self, name: &str, start: usize) -> Result<(), Completion<Value>> {
        if !self.store_confined(name, start) {
            return Ok(());
        }
        Err(crate::command::err_with_code(
            format!("can't set \"{name}\": stores are confined to the activation"),
            "TCL WRITE VARNAME",
        ))
    }

    /// Refuse `rand()` and `srand()` while stores are confined. The
    /// generator's seed is interpreter state every invocation shares:
    /// `srand` writes it and `rand` reads and advances what an earlier
    /// invocation left, so either would make one evaluation's answer
    /// depend on another's. Under 8.4 the functions are `expr` builtins no
    /// command restriction removes, so the refusal is here, where the draw
    /// is made. The refusal is an ordinary Tcl error.
    pub(crate) fn confine_generator(&self, function: &str) -> Result<(), Completion<Value>> {
        if !self.limits.confined_stores {
            return Ok(());
        }
        Err(err(format!(
            "can't call \"{function}\": stores are confined to the activation and the \
             generator's seed is not"
        )))
    }

    /// The `commands` limit value, if one is armed.
    pub(crate) fn command_limit_value(&self) -> Option<i64> {
        self.limits.cmd_value
    }

    /// Arm (or disarm) the `commands` limit.
    pub(crate) fn set_command_limit_value(&mut self, limit: Option<i64>) {
        self.native_compiler_pass_owner
            .invalidate_ordinary_entry_observation();
        self.forget_exceeded(LimitKind::Commands);
        self.limits.cmd_value = limit;
    }

    /// Whether a `commands` limit is armed.
    pub(crate) fn command_limit_armed(&self) -> bool {
        self.limits.cmd_value.is_some()
    }

    /// Commands dispatched since the counter was last reset.
    pub(crate) fn command_count(&self) -> u64 {
        self.counters.commands_run
    }

    /// Zero the command counter and forget a breach — an embedder's
    /// per-invocation fuel refill.
    pub(crate) fn reset_command_count_inner(&mut self) {
        self.counters.commands_run = 0;
        self.limits.exceeded = None;
    }

    /// `interp limit limitType ?-option value …?` on this interp (query / set;
    /// enforced for `time`).
    pub(crate) fn limit_apply(
        &mut self,
        ltype: &str,
        args: &[Value],
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        match LIMIT_TYPES.index_of_str(ltype)? {
            0 => self.limit_commands(args),
            _ => self.limit_time(args),
        }
    }

    fn limit_commands(&mut self, args: &[Value]) -> Result<Value, tcl_cmd_core::CmdError> {
        const OPTS: &[&str] = &["-command", "-granularity", "-value"];
        let l = &self.limits;
        let query = |opt: &str| match opt {
            "-command" => l.cmd_command.clone(),
            "-granularity" => Value::int(l.cmd_granularity),
            _ => l.cmd_value.map_or_else(|| Value::string(""), Value::int),
        };
        match args {
            [] => Ok(Value::list(vec![
                Value::string("-command"),
                l.cmd_command.clone(),
                Value::string("-granularity"),
                Value::int(l.cmd_granularity),
                Value::string("-value"),
                l.cmd_value.map_or_else(|| Value::string(""), Value::int),
            ])),
            [opt] => Ok(query(
                resolve_limit_opt(&opt.to_str(), OPTS).map_err(tcl_cmd_core::CmdError::new)?,
            )),
            _ => {
                for pair in args.chunks(2) {
                    // A trailing option with no value is a catchable error, not a
                    // panic (`interp limit c commands -value 1 -granularity`).
                    let [opt, val] = pair else {
                        return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                            "wrong # args: should be \"interp limit path commands \
                                    ?-option value ...?\""
                                .as_bytes(),
                        ));
                    };
                    let opt = resolve_limit_opt(&opt.to_str(), OPTS)
                        .map_err(tcl_cmd_core::CmdError::new)?;
                    match opt {
                        "-command" => self.limits.cmd_command = val.clone(),
                        "-granularity" => {
                            let n = i64::from(tcl_syntax::value::narrow_to_c_int(limit_integer(
                                self, val,
                            )?)?);
                            if n < 1 {
                                return Err(bad_limit_value("granularity must be at least 1"));
                            }
                            self.limits.cmd_granularity = n;
                        }
                        _ => {
                            let n = limit_integer(self, val)?;
                            if n < 0 {
                                return Err(bad_limit_value(
                                    "command limit value must be at least 0",
                                ));
                            }
                            self.forget_exceeded(LimitKind::Commands);
                            self.native_compiler_pass_owner
                                .invalidate_ordinary_entry_observation();
                            self.limits.cmd_value = Some(n);
                        }
                    }
                }
                Ok(Value::string(""))
            }
        }
    }

    fn limit_time(&mut self, args: &[Value]) -> Result<Value, tcl_cmd_core::CmdError> {
        const OPTS: &[&str] = &["-command", "-granularity", "-milliseconds", "-seconds"];
        let l = &self.limits;
        let query = |opt: &str| match opt {
            "-command" => l.time_command.clone(),
            "-granularity" => Value::int(l.time_granularity),
            "-seconds" => l
                .time_value
                .map_or_else(|| Value::string(""), |(s, _)| Value::int(s)),
            _ => l
                .time_value
                .map_or_else(|| Value::string(""), |(_, m)| Value::int(m)),
        };
        match args {
            [] => Ok(Value::list(vec![
                Value::string("-command"),
                l.time_command.clone(),
                Value::string("-granularity"),
                Value::int(l.time_granularity),
                Value::string("-milliseconds"),
                l.time_value
                    .map_or_else(|| Value::string(""), |(_, m)| Value::int(m)),
                Value::string("-seconds"),
                l.time_value
                    .map_or_else(|| Value::string(""), |(s, _)| Value::int(s)),
            ])),
            [opt] => Ok(query(
                resolve_limit_opt(&opt.to_str(), OPTS).map_err(tcl_cmd_core::CmdError::new)?,
            )),
            _ => {
                let (mut sec, mut ms) = self.limits.time_value.unwrap_or((0, 0));
                let mut touched = self.limits.time_value.is_some();
                for pair in args.chunks(2) {
                    let [opt, val] = pair else {
                        return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                            "wrong # args: should be \"interp limit path time \
                                    ?-option value ...?\""
                                .as_bytes(),
                        ));
                    };
                    let opt = resolve_limit_opt(&opt.to_str(), OPTS)
                        .map_err(tcl_cmd_core::CmdError::new)?;
                    match opt {
                        "-command" => self.limits.time_command = val.clone(),
                        "-granularity" => {
                            let n = i64::from(tcl_syntax::value::narrow_to_c_int(limit_integer(
                                self, val,
                            )?)?);
                            if n < 1 {
                                return Err(bad_limit_value("granularity must be at least 1"));
                            }
                            self.limits.time_granularity = n;
                        }
                        "-seconds" => {
                            let n = limit_integer(self, val)?;
                            if n < 0 {
                                return Err(bad_limit_value("seconds must be non-negative"));
                            }
                            sec = n;
                            touched = true;
                        }
                        _ => {
                            let n = limit_integer(self, val)?;
                            if n < 0 {
                                return Err(bad_limit_value("milliseconds must be non-negative"));
                            }
                            ms = n;
                            touched = true;
                        }
                    }
                }
                if touched {
                    // Normalise excess milliseconds into seconds.
                    sec = sec.saturating_add(ms.div_euclid(1000));
                    ms = ms.rem_euclid(1000);
                    self.forget_exceeded(LimitKind::WallClock);
                    self.native_compiler_pass_owner
                        .invalidate_ordinary_entry_observation();
                    self.limits.time_value = Some((sec, ms));
                }
                Ok(Value::string(""))
            }
        }
    }
}
