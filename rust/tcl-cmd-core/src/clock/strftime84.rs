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

//! Tcl 8.4's `clock`, the C `Tcl_ClockObjCmd` (`tclClock.c` at 8.4.20): four
//! subcommands, `strftime` formatting, and `clock scan` without `-format`.

use super::{Civil, CmdError, DEFAULT_FORMAT, Now, OptionTable, ValueOps, overflow, render};
use tcl_dialect::ClockFormatGrammar;

const SUBCOMMANDS: OptionTable =
    OptionTable::abbreviating("option", &["clicks", "format", "scan", "seconds"]);
const FORMAT_SWITCHES: OptionTable = OptionTable::abbreviating("switch", &["-format", "-gmt"]);
const SCAN_SWITCHES: OptionTable = OptionTable::abbreviating("switch", &["-base", "-gmt"]);
const FORMAT_USAGE: &str = "clock format clockval ?-format string? ?-gmt boolean?";
const SCAN_USAGE: &str = "clock scan dateString ?-base clockValue? ?-gmt boolean?";

/// `Tcl_GetIndexFromObj`'s message; 8.4 stores no error code with it.
fn lookup(table: &OptionTable, word: &str) -> Result<usize, CmdError> {
    table.index_of(word.as_bytes()).map_err(CmdError::new_bytes)
}

pub(super) fn dispatch<O, V>(
    ops: &mut O,
    args: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((sub, rest)) = args.split_first() else {
        return Err(CmdError::wrong_args("clock option ?arg ...?"));
    };
    let word = ops.try_as_str(sub)?.to_string();
    match lookup(&SUBCOMMANDS, &word)? {
        0 => clicks(ops, rest, now),
        1 => format(ops, rest, local_offset),
        2 => scan(ops, rest),
        _ if rest.is_empty() => Ok(ops.new_int(now.secs)),
        _ => Err(CmdError::wrong_args("clock seconds")),
    }
}

/// `clock clicks ?-milliseconds?`: any prefix of two or more bytes.
fn clicks<O, V>(ops: &mut O, rest: &[V], now: &Now) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    match rest {
        [] => Ok(ops.new_int(now.micros)),
        [switch] => {
            let word = ops.try_as_str(switch)?.to_string();
            if word.len() >= 2 && "-milliseconds".starts_with(word.as_str()) {
                Ok(ops.new_int(now.millis))
            } else {
                Err(CmdError::new(format!(
                    "bad switch \"{word}\": must be -milliseconds"
                )))
            }
        }
        _ => Err(CmdError::wrong_args("clock clicks ?-milliseconds?")),
    }
}

/// The switch pairs after the first word, read as C reads them: each
/// switch is looked up before the count is checked.
fn switches<O, V>(
    ops: &mut O,
    rest: &[V],
    table: &OptionTable,
    usage: &str,
) -> Result<Vec<(usize, usize)>, CmdError>
where
    O: ValueOps<Value = V>,
{
    if rest.is_empty() || rest.len() > 5 {
        return Err(CmdError::wrong_args(usage));
    }
    let mut pairs = Vec::new();
    let mut at = 1;
    while rest.len() - at > 1 {
        let word = ops.try_as_str(&rest[at])?.to_string();
        pairs.push((lookup(table, &word)?, at + 1));
        at += 2;
    }
    if at != rest.len() {
        return Err(CmdError::wrong_args(usage));
    }
    Ok(pairs)
}

fn format<O, V>(ops: &mut O, rest: &[V], local_offset: &dyn Fn(i64) -> i32) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    if rest.is_empty() || rest.len() > 5 {
        return Err(CmdError::wrong_args(FORMAT_USAGE));
    }
    let clockval = ops.as_int(&rest[0])?;
    let mut fmt = DEFAULT_FORMAT.to_string();
    let mut gmt = false;
    for (switch, value) in switches(ops, rest, &FORMAT_SWITCHES, FORMAT_USAGE)? {
        if switch == 0 {
            fmt = ops.try_as_str(&rest[value])?.to_string();
        } else {
            gmt = ops.as_bool(&rest[value])?;
        }
    }
    let offset = if gmt { 0 } else { local_offset(clockval) };
    let local = clockval
        .checked_add(i64::from(offset))
        .ok_or_else(overflow)?;
    let civil = Civil::from_secs(local);
    Ok(ops.new_str(&render(
        &civil,
        &fmt,
        offset,
        clockval,
        ClockFormatGrammar::Strftime84,
    )))
}

/// `clock scan dateString ?-base clockValue? ?-gmt boolean?`: free-form
/// only, which this core does not parse.
fn scan<O, V>(ops: &mut O, rest: &[V]) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    for (switch, value) in switches(ops, rest, &SCAN_SWITCHES, SCAN_USAGE)? {
        if switch == 0 {
            ops.as_int(&rest[value])?;
        } else {
            ops.as_bool(&rest[value])?;
        }
    }
    Err(CmdError::new("free-form clock scan is not yet supported"))
}
