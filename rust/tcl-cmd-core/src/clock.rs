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

//! `clock` — wall-clock readout + civil-calendar math, shared across runtimes.
//!
//! Neither runtime had `clock`; this implements it once over [`ValueOps`]. The
//! command stays host-free: the adapter reads the current time from the host's
//! [`Clock`](tcl_platform::Clock) and passes it in as [`Now`], plus a
//! `local_offset` callback (the local zone's offset at a given instant) for the
//! non-`-gmt` `format`/`add` paths. A host with no timezone database returns a
//! `0` offset, so local time then equals UTC.
//!
//! Implemented: `seconds`/`milliseconds`/`microseconds`/`clicks`, `format` (the
//! civil-date specifiers), `add` (count/unit arithmetic incl. calendar
//! months/years), and `scan -format` (the inverse — parse an input per a format).
//! Free-form `clock scan` (natural-language dates) is not yet supported. Mirrors
//! C's `clock.tcl` ensemble. The civil↔days conversions are Howard Hinnant's
//! branch-free algorithms (valid for the full proleptic Gregorian range).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::format_push_string,
    clippy::similar_names
)]

use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

use crate::error::CmdError;
use crate::prefix::OptionTable;

/// Retained private `ClockClientData` objects supplied by the runtime adapter.
pub struct FormatArgumentLiterals<V> {
    pub format: V,
    pub locale: V,
    pub timezone: V,
    pub gmt: V,
}

/// The conflict result uses the worker's retained original literal.
pub enum FormatArgumentError {
    Command(CmdError),
    GmtWithTimezone,
}

impl From<CmdError> for FormatArgumentError {
    fn from(error: CmdError) -> Self {
        Self::Command(error)
    }
}

impl From<tcl_syntax::value::ValueError> for FormatArgumentError {
    fn from(error: tcl_syntax::value::ValueError) -> Self {
        Self::Command(error.into())
    }
}

/// C8.6's private argument worker; supplied option values remain original objects.
///
/// # Errors
/// Arity, option, boolean and wide-integer conversions, then GMT/timezone conflict.
pub fn parse_format_args<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    literals: &FormatArgumentLiterals<O::Value>,
) -> Result<O::Value, FormatArgumentError> {
    if args.is_empty() || args.len().is_multiple_of(2) {
        return Err(CmdError::with_error_code(
            "wrong # args: should be \"clock format clockval ?-format string? ?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE?\"",
            "CLOCK wrongNumArgs",
        ).into());
    }
    let mut format = literals.format.clone();
    let mut locale = literals.locale.clone();
    let mut timezone = literals.timezone.clone();
    let mut gmt = false;
    let mut saw_gmt = false;
    let mut saw_timezone = false;
    let table = crate::prefix::OptionTable::abbreviating(
        "option",
        &["-format", "-gmt", "-locale", "-timezone"],
    );
    for pair in args[1..].as_chunks::<2>().0 {
        let index = match table.index_of_original(ops, &pair[0]) {
            Ok(index) => index,
            Err(error) => {
                if error.native_execution_refusal().is_some() {
                    return Err(error.into());
                }
                let option = ops.native_string_bytes(&pair[0])?;
                let option = option.split(|byte| *byte == 0).next().unwrap_or_default();
                let mut code = b"CLOCK badOption ".to_vec();
                tcl_syntax::list::append_list_element(&mut code, option, false);
                let mut details = error.into_byte_details();
                details.error_code = crate::error::CmdErrorCodeUpdate::Set(code);
                return Err(CmdError::from_byte_details(details).into());
            }
        };
        match index {
            0 => format = pair[1].clone(),
            1 => {
                gmt = ops.as_bool(&pair[1])?;
                saw_gmt = true;
            }
            2 => locale = pair[1].clone(),
            _ => {
                timezone = pair[1].clone();
                saw_timezone = true;
            }
        }
    }
    ops.as_int(&args[0])?;
    if saw_gmt && saw_timezone {
        return Err(FormatArgumentError::GmtWithTimezone);
    }
    if gmt {
        timezone = literals.gmt.clone();
    }
    Ok(ops.new_list(vec![format, locale, timezone]))
}

/// The clean integer-overflow error tclsh raises when a `clock` computation
/// leaves the wide range (e.g. `clock add 0 9223372036854775807 weeks`). Every
/// timestamp here is user-controlled, so the arithmetic below funnels through
/// `checked_*`/`i128` and returns this rather than panicking (debug) or wrapping
/// (release) — matching tclsh, which reports a catchable error, never a wrap.
fn overflow() -> CmdError {
    CmdError::new("integer value too large to represent")
}

/// The current wall-clock time, read by the adapter from the host's `Clock`.
pub struct Now {
    /// Seconds since the Unix epoch (`clock seconds`).
    pub secs: i64,
    /// Milliseconds since the Unix epoch (`clock milliseconds`).
    pub millis: i64,
    /// Microseconds since the Unix epoch (`clock microseconds`/`clicks`).
    pub micros: i64,
}

/// `clock subcommand ?arg ...?`. `args[0]` is the subcommand. `now` supplies the
/// wall clock; `local_offset(ts)` is the local zone's offset (seconds east of
/// UTC) at `ts`, used by `format`/`add` without `-gmt`.
///
/// # Errors
/// A bad subcommand, bad option, non-integer argument, or `clock scan`
/// (which this dispatcher does not implement).
pub fn dispatch<O, V>(
    ops: &mut O,
    args: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
    release: TclVersion,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((sub, rest)) = args.split_first() else {
        return Err(CmdError::wrong_args("clock subcommand ?arg ...?"));
    };
    let word = ops.try_as_str(sub)?.to_string();
    // `clock` is an ensemble, so an exact name wins and a unique prefix
    // resolves (`clock se` → `seconds`); the whole miss sentence — including
    // the ensemble's comma before `or` — belongs to `crate::ensemble`.
    let Some(index) = crate::ensemble::resolve_subcommand(CLOCK_SUBS, word.as_bytes(), true) else {
        return Err(CmdError::new(
            String::from_utf8_lossy(&crate::ensemble::unknown_subcommand_message(
                CLOCK_SUBS,
                word.as_bytes(),
                true,
                b"::tcl::clock",
            ))
            .into_owned(),
        ));
    };
    match CLOCK_SUBS[index] {
        "seconds" => nullary(ops, rest, "seconds", now.secs),
        "milliseconds" => nullary(ops, rest, "milliseconds", now.millis),
        "microseconds" => nullary(ops, rest, "microseconds", now.micros),
        "clicks" => clicks(ops, rest, now),
        "format" => format(ops, rest, now, local_offset),
        "add" => add(ops, rest, now, local_offset, release),
        // Unreachable: `CLOCK_SUBS` has exactly the seven arms here.
        _ => scan(ops, rest, now, local_offset),
    }
}

/// `clock`'s subcommand set, alphabetical as the ensemble sorts it. C also
/// carries `configure`, `mktime` and the internal `::tcl::clock` helpers behind
/// its script implementation; this core names only what it dispatches.
const CLOCK_SUBS: &[&str] = &[
    "add",
    "clicks",
    "format",
    "microseconds",
    "milliseconds",
    "scan",
    "seconds",
];

/// A no-argument time readout (`clock seconds`/`milliseconds`/`microseconds`).
fn nullary<O, V>(ops: &mut O, rest: &[V], name: &str, val: i64) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    if !rest.is_empty() {
        return Err(CmdError::wrong_args(&format!("clock {name}")));
    }
    Ok(ops.new_int(val))
}

/// `clock clicks ?-milliseconds|-microseconds?` — a high-resolution counter
/// (microseconds by default).
fn clicks<O, V>(ops: &mut O, rest: &[V], now: &Now) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    const SWITCHES: OptionTable =
        OptionTable::abbreviating("option", &["-milliseconds", "-microseconds"]);
    let val = match rest {
        [] => now.micros,
        [opt] => match SWITCHES.index_of_cmd(ops.try_as_str(opt)?.as_bytes())? {
            0 => now.millis,
            _ => now.micros,
        },
        _ => return Err(CmdError::wrong_args("clock clicks ?-switch?")),
    };
    Ok(ops.new_int(val))
}

const DEFAULT_FORMAT: &str = "%a %b %d %H:%M:%S %Z %Y";

/// The bounds `ClockParseFmtScnArgs` accepts for a clock value or `-base`
/// (`tclDate.h`: `TCL_MIN_SECONDS`/`TCL_MAX_SECONDS`), which also bound a
/// `clock add` count of hours, minutes or seconds.
const MAX_SECONDS: i64 = 0x00F0_0000_0000_0000;

/// Which of the three commands sharing C's `ClockParseFmtScnArgs` is parsing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClockOp {
    Format,
    Scan,
    Add,
}

impl ClockOp {
    /// The usage `Tcl_WrongNumArgs` reports (with `CLOCK wrongNumArgs`).
    fn usage(self) -> &'static str {
        match self {
            ClockOp::Format => {
                "clock format clockval|now ?-format string? ?-gmt boolean? \
                 ?-locale LOCALE? ?-timezone ZONE?"
            }
            ClockOp::Scan => {
                "clock scan string ?-base seconds? ?-format string? ?-gmt boolean? \
                 ?-locale LOCALE? ?-timezone ZONE? ?-validate boolean?"
            }
            ClockOp::Add => {
                "clock add clockval|now ?number units?...?-gmt boolean? \
                 ?-locale LOCALE? ?-timezone ZONE?"
            }
        }
    }

    /// The option list a rejected option word's message enumerates.
    fn choices(self) -> &'static str {
        match self {
            ClockOp::Format => "-format, -gmt, -locale, or -timezone",
            ClockOp::Scan => "-base, -format, -gmt, -locale, -timezone or -validate",
            ClockOp::Add => "-gmt, -locale, or -timezone",
        }
    }
}

const OPTIONS: OptionTable = OptionTable::abbreviating(
    "option",
    &[
        "-base",
        "-format",
        "-gmt",
        "-locale",
        "-timezone",
        "-validate",
    ],
);
const OPT_BASE: usize = 0;
const OPT_FORMAT: usize = 1;
const OPT_GMT: usize = 2;
const OPT_TIMEZONE: usize = 4;
const OPT_VALIDATE: usize = 5;

/// The options `ClockParseFmtScnArgs` extracted, with the clock value or
/// `-base` already read as seconds.
struct ClockArgs {
    base: i64,
    format: Option<String>,
    gmt: bool,
    validate: bool,
}

/// `CLOCK badOption <word>`, the identity every rejected option, clock value
/// and `-base` carries.
fn bad_option(message: impl Into<String>, word: &str) -> CmdError {
    CmdError::with_error_code(
        message,
        tcl_syntax::list::join_list(["CLOCK", "badOption", word]),
    )
}

/// C's `ClockParseFmtScnArgs`. `rest[0]` is the clock value (format/add) or
/// the input string (scan); the option words sit at the odd positions, and
/// `clock add` passes over any that read as wide integers (its counts).
fn parse_args<O, V>(ops: &mut O, rest: &[V], op: ClockOp, now: i64) -> Result<ClockArgs, CmdError>
where
    O: ValueOps<Value = V>,
{
    if rest.len().is_multiple_of(2) {
        return Err(CmdError::with_error_code(
            format!("wrong # args: should be \"{}\"", op.usage()),
            "CLOCK wrongNumArgs",
        ));
    }
    let mut args = ClockArgs {
        base: now,
        format: None,
        gmt: false,
        validate: true,
    };
    let mut base = (op != ClockOp::Scan).then_some(0);
    let mut saw: u32 = if op == ClockOp::Scan {
        0
    } else {
        1 << OPT_BASE
    };
    for i in (1..rest.len()).step_by(2) {
        if op == ClockOp::Add && ops.as_int(&rest[i]).is_ok() {
            continue;
        }
        let word = ops.try_as_str(&rest[i])?.to_string();
        let bad_msg = || {
            bad_option(
                format!("bad option \"{word}\": must be {}", op.choices()),
                &word,
            )
        };
        let Ok(index) = OPTIONS.index_of_cmd(word.as_bytes()) else {
            return Err(bad_msg());
        };
        if saw & (1 << index) != 0 {
            if index == OPT_BASE && op != ClockOp::Scan {
                return Err(bad_msg());
            }
            return Err(bad_option(
                format!("bad option \"{word}\": doubly present"),
                &word,
            ));
        }
        let value = &rest[i + 1];
        match index {
            OPT_FORMAT if op == ClockOp::Add => return Err(bad_msg()),
            OPT_FORMAT => args.format = Some(ops.try_as_str(value)?.to_string()),
            OPT_GMT => args.gmt = ops.as_bool(value)?,
            OPT_BASE => base = Some(i + 1),
            OPT_VALIDATE if op != ClockOp::Scan => return Err(bad_msg()),
            OPT_VALIDATE => args.validate = ops.as_bool(value)?,
            // `-locale` and `-timezone` are accepted, not yet honoured.
            _ => {}
        }
        saw |= 1 << index;
    }
    if saw & (1 << OPT_GMT) != 0 && saw & (1 << OPT_TIMEZONE) != 0 {
        return Err(CmdError::with_error_code(
            "cannot use -gmt and -timezone in same call",
            "CLOCK gmtWithTimezone",
        ));
    }
    if let Some(at) = base {
        args.base = read_seconds(ops, &rest[at], now)?;
    }
    Ok(args)
}

/// Read a clock value or `-base` as C does: a wide integer within
/// `±MAX_SECONDS`, or exactly `now`/`-now`.
fn read_seconds<O, V>(ops: &mut O, value: &V, now: i64) -> Result<i64, CmdError>
where
    O: ValueOps<Value = V>,
{
    let word = ops.try_as_str(value)?.to_string();
    match ops.as_int(value) {
        Ok(secs) if (-MAX_SECONDS..=MAX_SECONDS).contains(&secs) => Ok(secs),
        Ok(_) | Err(ValueError::IntegerOverflow) => {
            Err(bad_option("integer value too large to represent", &word))
        }
        Err(_) if word == "now" || word == "-now" => Ok(now),
        Err(_) => Err(bad_option(
            format!("bad seconds \"{word}\": must be now or integer"),
            &word,
        )),
    }
}

/// `clock format clockval|now ?-format string? ?-gmt boolean? ?-locale l? ?-timezone z?`.
fn format<O, V>(
    ops: &mut O,
    rest: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let args = parse_args(ops, rest, ClockOp::Format, now.secs)?;
    let clockval = args.base;
    let fmt = args.format.as_deref().unwrap_or(DEFAULT_FORMAT);
    let offset = if args.gmt { 0 } else { local_offset(clockval) };
    let local = clockval
        .checked_add(i64::from(offset))
        .ok_or_else(overflow)?;
    let civil = Civil::from_secs(local);
    Ok(ops.new_str(&render(&civil, fmt, offset, clockval)))
}

/// `clock add`'s units, in C's table order (`week` beside `weeks` so `week`
/// is no ambiguous prefix).
const UNITS: OptionTable = OptionTable::abbreviating(
    "unit",
    &[
        "years", "months", "week", "weeks", "days", "weekdays", "hours", "minutes", "seconds",
    ],
);

/// `word` as `release`'s `clock add` reads a unit. 9.0's C `clock add` takes
/// any unique prefix of [`UNITS`]; 8.5 and 8.6's script `clock add` takes only
/// each unit's singular or plural word and has no `weekdays` (measured: tclsh
/// 8.5.19 and 8.6.18 refuse `d`, `mo`, `weekdays` with `CLOCK badUnit`; 9.0.4
/// and 9.1.0 accept them).
fn unit_under(word: &str, release: TclVersion) -> Result<Unit, CmdError> {
    if release >= TclVersion::V9_0 {
        return Ok(Unit::from_index(UNITS.index_of_cmd(word.as_bytes())?));
    }
    let unit = match word {
        "year" | "years" => Unit::Years,
        "month" | "months" => Unit::Months,
        "week" | "weeks" => Unit::Weeks,
        "day" | "days" => Unit::Days,
        "hour" | "hours" => Unit::Hours,
        "minute" | "minutes" => Unit::Minutes,
        "second" | "seconds" => Unit::Seconds,
        _ => {
            return Err(CmdError::with_error_code(
                format!(
                    "unknown unit \"{word}\", must be  years, months, weeks, days, hours, minutes or seconds"
                ),
                tcl_syntax::list::join_list(["CLOCK", "badUnit", word]),
            ));
        }
    };
    Ok(unit)
}

/// A resolved `clock add` unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    Years,
    Months,
    Weeks,
    Days,
    Weekdays,
    Hours,
    Minutes,
    Seconds,
}

impl Unit {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Unit::Years,
            1 => Unit::Months,
            2 | 3 => Unit::Weeks,
            4 => Unit::Days,
            5 => Unit::Weekdays,
            6 => Unit::Hours,
            7 => Unit::Minutes,
            _ => Unit::Seconds,
        }
    }

    /// The largest count magnitude C accepts: a C `int` for the date units,
    /// the clock-value bound for the time units.
    fn max_count(self) -> i64 {
        match self {
            Unit::Hours | Unit::Minutes | Unit::Seconds => MAX_SECONDS,
            _ => 0x7fff_ffff,
        }
    }
}

/// `clock add clockval|now ?count unit ...? ?-gmt boolean? ?-locale l? ?-timezone z?`.
fn add<O, V>(
    ops: &mut O,
    rest: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
    release: TclVersion,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let args = parse_args(ops, rest, ClockOp::Add, now.secs)?;
    let mut t = args.base;
    for i in (1..rest.len()).step_by(2) {
        let Ok(count) = ops.as_int(&rest[i]) else {
            continue;
        };
        let unit = unit_under(&ops.try_as_str(&rest[i + 1])?, release)?;
        if !(-unit.max_count()..=unit.max_count()).contains(&count) {
            return Err(overflow());
        }
        if count != 0 {
            t = add_units(t, count, unit, args.gmt, local_offset)?;
        }
    }
    Ok(ops.new_int(t))
}

/// Upper bound (magnitude) on a civil year passed to `days_from_civil` in the
/// calendar-add path. Any year beyond this yields an epoch far outside the wide
/// range (a year holds ~3.15e7 seconds, so even `±3e11` years overflows `i64`
/// seconds), and `days_from_civil`'s own `i64` arithmetic would itself overflow
/// well before then — so a larger year is reported as a clean overflow rather
/// than wrapped. Comfortably below where `era * 146097` could exceed `i64`.
const MAX_CIVIL_YEAR: i64 = 300_000_000_000;

/// C's `ClockWeekdaysOffs`: the calendar days spanning `offs` weekdays from a
/// day whose ISO weekday (Monday 1 … Sunday 7) is `day_of_week`.
fn weekdays_offs(day_of_week: i64, offs: i64) -> i64 {
    let mut weeks = offs / 5;
    let mut offs = offs % 5;
    if offs < 0 {
        offs += 5;
        weeks -= 1;
    }
    offs += 7 * weeks;
    let mut result_day = day_of_week + offs.rem_euclid(7);
    if day_of_week > 5 {
        let adj = 5 - day_of_week;
        offs += adj;
        result_day += adj;
    }
    if result_day > 5 {
        offs += 2;
    }
    offs
}

/// Apply `count` of `unit` to the timestamp `t` (UTC seconds). Fixed units add a
/// constant; calendar units (months/years) go through the civil date in the
/// active zone, clamping the day to the target month's length.
fn add_units(
    t: i64,
    count: i64,
    unit: Unit,
    gmt: bool,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<i64, CmdError> {
    let offset = if gmt { 0 } else { i64::from(local_offset(t)) };
    let fixed = match unit {
        Unit::Seconds => Some((count, 1)),
        Unit::Minutes => Some((count, 60)),
        Unit::Hours => Some((count, 3600)),
        Unit::Days => Some((count, 86_400)),
        Unit::Weeks => Some((count, 7 * 86_400)),
        Unit::Weekdays => {
            let local = t.checked_add(offset).ok_or_else(overflow)?;
            let weekday = Civil::from_secs(local).weekday;
            let iso = if weekday == 0 { 7 } else { weekday };
            Some((weekdays_offs(iso, count), 86_400))
        }
        Unit::Months | Unit::Years => None,
    };
    if let Some((count, scale)) = fixed {
        // `count` and `t` are both user-controlled wides; compute `t + count*scale`
        // in i128 so neither the multiply nor the add can wrap before the range
        // check.
        let secs = i128::from(t) + i128::from(count) * i128::from(scale);
        return i64::try_from(secs).map_err(|_| overflow());
    }
    let months = if unit == Unit::Years {
        i128::from(count) * 12
    } else {
        i128::from(count)
    };
    // `t + offset` is bounded by the offset (seconds), but guard it anyway since
    // `t` may sit at the wide boundary.
    let local_in = t.checked_add(offset).ok_or_else(overflow)?;
    let c = Civil::from_secs(local_in);
    // The civil month tally and the resulting year are derived in i128 so a huge
    // `count` of months/years cannot overflow before we clamp it to a real date.
    let total = (i128::from(c.month) - 1) + months;
    let year_i128 = i128::from(c.year) + total.div_euclid(12);
    // `days_from_civil` does its own i64 arithmetic (`era * 146097`, `yoe * 365`),
    // which itself overflows for an astronomically large year — and such a year
    // could never yield an epoch inside the wide range anyway (the seconds in a
    // year past `MAX_CIVIL_YEAR` exceed `i64`). Reject it as a clean overflow
    // before the call rather than letting the helper wrap.
    if year_i128 > i128::from(MAX_CIVIL_YEAR) || year_i128 < -i128::from(MAX_CIVIL_YEAR) {
        return Err(overflow());
    }
    let year = year_i128 as i64; // bounded above, so the cast is exact
    // `rem_euclid(12)` is in `0..=11`, so the `+ 1` and cast are safe.
    let month = (total.rem_euclid(12) + 1) as u32;
    let day = c.day.min(days_in_month(year, month));
    // `days_from_civil` is now bounded; recompose in i128 and undo the zone
    // offset, both range-checked back into a wide.
    let local = i128::from(days_from_civil(year, month, day)) * 86_400 + i128::from(c.tod);
    let result = local - i128::from(offset);
    i64::try_from(result).map_err(|_| overflow())
}

/// `clock scan inputString -format formatString ?-base seconds? ?-gmt boolean?
/// ?-validate boolean? …` — parse `inputString` per `formatString` into an
/// epoch timestamp. Free-form scanning (no `-format`) is not yet supported.
fn scan<O, V>(
    ops: &mut O,
    rest: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let args = parse_args(ops, rest, ClockOp::Scan, now.secs)?;
    let input = ops.try_as_str(&rest[0])?.to_string();
    // Free-form (natural-language) scanning is not implemented; `-format` only.
    let Some(fmt) = args.format else {
        return Err(CmdError::new("free-form clock scan is not yet supported"));
    };
    let base = args.base;
    let base_offset = if args.gmt { 0 } else { local_offset(base) };
    let base_local = base
        .checked_add(i64::from(base_offset))
        .ok_or_else(overflow)?;
    let base_civil = Civil::from_secs(base_local);
    let fields = parse_with_format(&input, &fmt)?;
    if let Some(epoch) = fields.epoch {
        return Ok(ops.new_int(epoch));
    }
    let year = fields.year.unwrap_or(base_civil.year);
    let month = fields.month.unwrap_or(base_civil.month);
    let day = fields.day.unwrap_or(base_civil.day);
    // C's `ClockValidDate`, on by default: without it an out-of-range month
    // or day rolls over into the next (`days_from_civil` is linear in both).
    if args.validate {
        if !(1..=12).contains(&month) {
            return Err(invalid_input("month"));
        }
        if day == 0 || day > days_in_month(year, month) {
            return Err(invalid_input("day"));
        }
    }
    let hour = fields.hour.unwrap_or(0);
    let min = fields.min.unwrap_or(0);
    let sec = fields.sec.unwrap_or(0);
    // `year` (hence `days_from_civil`) and the `%H`/`%M`/`%S` fields all come from
    // the input string, so compose the local epoch in i128 to avoid a wrap at the
    // wide boundary, then range-check both it and the offset-adjusted result.
    let local_i128 = i128::from(days_from_civil(year, month, day)) * 86_400
        + i128::from(hour) * 3600
        + i128::from(min) * 60
        + i128::from(sec);
    let local = i64::try_from(local_i128).map_err(|_| overflow())?;
    let offset = if args.gmt { 0 } else { local_offset(local) };
    let result = i128::from(local) - i128::from(offset);
    Ok(ops.new_int(i64::try_from(result).map_err(|_| overflow())?))
}

/// `unable to convert input string: invalid <field>`, C's `ClockValidDate`
/// refusal (`CLOCK invInpStr <field>`).
fn invalid_input(field: &str) -> CmdError {
    CmdError::with_error_code(
        format!("unable to convert input string: invalid {field}"),
        tcl_syntax::list::join_list(["CLOCK", "invInpStr", field]),
    )
}

/// The date/time fields a `-format` scan extracted (each `None` if its specifier
/// was absent). `epoch` (from `%s`) short-circuits everything else.
#[derive(Default, Debug)]
struct ScanFields {
    year: Option<i64>,
    month: Option<u32>,
    day: Option<u32>,
    hour: Option<i64>,
    min: Option<i64>,
    sec: Option<i64>,
    epoch: Option<i64>,
}

/// `input string does not match supplied format` — the scan mismatch error.
fn no_match() -> CmdError {
    CmdError::with_error_code(
        "input string does not match supplied format",
        "CLOCK badInputString",
    )
}

/// Read up to `max` decimal digits (at least one) from `inb` at `*ip`.
///
/// `max` is as large as 19 (the `%s` epoch field), so the accumulator can leave
/// the wide range on attacker input (e.g. nineteen `9`s); fold the digits with
/// `checked_*` and surface the clean overflow error tclsh would, never wrapping.
fn read_uint(inb: &[u8], ip: &mut usize, max: usize) -> Result<i64, CmdError> {
    let start = *ip;
    let mut val: i64 = 0;
    while *ip < inb.len() && *ip - start < max && inb[*ip].is_ascii_digit() {
        val = val
            .checked_mul(10)
            .and_then(|v| v.checked_add(i64::from(inb[*ip] - b'0')))
            .ok_or_else(|| {
                CmdError::with_error_code(
                    "integer value too large to represent",
                    "CLOCK dateTooLarge",
                )
            })?;
        *ip += 1;
    }
    if *ip == start {
        return Err(no_match());
    }
    Ok(val)
}

/// Read an alphabetic month name (full or 3-letter abbreviation) → 1-12.
fn read_month_name(inb: &[u8], ip: &mut usize) -> Result<u32, CmdError> {
    let start = *ip;
    while *ip < inb.len() && inb[*ip].is_ascii_alphabetic() {
        *ip += 1;
    }
    let name = core::str::from_utf8(&inb[start..*ip]).map_err(|_| no_match())?;
    if name.is_empty() {
        return Err(no_match());
    }
    for (idx, m) in MONTHS.iter().enumerate() {
        if m.eq_ignore_ascii_case(name) || m[..3].eq_ignore_ascii_case(name) {
            return Ok(idx as u32 + 1);
        }
    }
    Err(no_match())
}

/// Parse `input` against the strftime-style `fmt`, extracting the date/time
/// fields. A literal char must match; a numeric specifier reads its digits; an
/// unrecognised value (e.g. month 13) is `unable to convert input string`.
fn parse_with_format(input: &str, fmt: &str) -> Result<ScanFields, CmdError> {
    let inb = input.as_bytes();
    let mut ip = 0;
    let mut f = ScanFields::default();
    let mut chars = fmt.chars();
    while let Some(fc) = chars.next() {
        if fc != '%' {
            // A literal format byte must match the input (a space matches a space).
            let mut buf = [0u8; 4];
            for b in fc.encode_utf8(&mut buf).as_bytes() {
                if inb.get(ip) != Some(b) {
                    return Err(no_match());
                }
                ip += 1;
            }
            continue;
        }
        let Some(mut spec) = chars.next() else {
            return Err(no_match());
        };
        if matches!(spec, 'E' | 'O') {
            let Some(modified) = chars.next() else {
                return Err(no_match());
            };
            if !modified.is_ascii() || !is_specifier(modified as u8) {
                return Err(no_match());
            }
            spec = modified;
        }
        match spec {
            'Y' => f.year = Some(read_uint(inb, &mut ip, 4)?),
            'y' => {
                let yy = read_uint(inb, &mut ip, 2)?;
                f.year = Some(if yy < 69 { 2000 + yy } else { 1900 + yy });
            }
            'm' => f.month = Some(read_uint(inb, &mut ip, 2)? as u32),
            'b' | 'B' | 'h' => f.month = Some(read_month_name(inb, &mut ip)?),
            'd' | 'e' => f.day = Some(read_uint(inb, &mut ip, 2)? as u32),
            'H' | 'k' | 'I' | 'l' => f.hour = Some(read_uint(inb, &mut ip, 2)?),
            'M' => f.min = Some(read_uint(inb, &mut ip, 2)?),
            'S' => f.sec = Some(read_uint(inb, &mut ip, 2)?),
            's' => {
                let neg = inb.get(ip) == Some(&b'-');
                if neg {
                    ip += 1;
                }
                let v = read_uint(inb, &mut ip, 19)?;
                f.epoch = Some(if neg { -v } else { v });
            }
            't' | 'n' => {
                while inb.get(ip).is_some_and(u8::is_ascii_whitespace) {
                    ip += 1;
                }
            }
            '%' => {
                if inb.get(ip) != Some(&b'%') {
                    return Err(no_match());
                }
                ip += 1;
            }
            _ => return Err(no_match()),
        }
    }
    // The format must consume the whole input (trailing whitespace excepted).
    while inb.get(ip).is_some_and(u8::is_ascii_whitespace) {
        ip += 1;
    }
    if ip != inb.len() {
        return Err(no_match());
    }
    Ok(f)
}

// civil date

/// A broken-down civil date/time (always relative to whatever zone the caller
/// applied via the offset).
struct Civil {
    year: i64,
    month: u32,
    day: u32,
    hour: i64,
    min: i64,
    sec: i64,
    /// Seconds-of-day (`hour*3600 + min*60 + sec`), kept for re-composition.
    tod: i64,
    /// 0 = Sunday … 6 = Saturday.
    weekday: i64,
    /// Day of year, 1-366.
    yearday: i64,
}

impl Civil {
    fn from_secs(secs: i64) -> Self {
        let days = secs.div_euclid(86_400);
        let tod = secs.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        // 1970-01-01 was a Thursday (weekday 4 with Sunday = 0).
        let weekday = (days.rem_euclid(7) + 4).rem_euclid(7);
        let yearday = days - days_from_civil(year, 1, 1) + 1;
        Self {
            year,
            month,
            day,
            hour: tod / 3600,
            min: (tod % 3600) / 60,
            sec: tod % 60,
            tod,
            weekday,
            yearday,
        }
    }
}

/// Days since the Unix epoch → `(year, month, day)` (Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `(year, month, day)` → days since the Unix epoch (Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mi = i64::from(m);
    let doy = (153 * (if mi > 2 { mi - 3 } else { mi + 9 }) + 2) / 5 + (i64::from(d) - 1); // [0,365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if m == 2 && is_leap(y) {
        29
    } else {
        DAYS[(m - 1) as usize]
    }
}

// strftime rendering

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Convert a 24-hour `hour` to its 12-hour-clock value (`%I`/`%l`).
fn hour12(hour: i64) -> i64 {
    let h = hour % 12;
    if h == 0 { 12 } else { h }
}

/// `+HHMM` / `-HHMM` (`%z`) for an offset in seconds east of UTC.
fn fmt_offset(offset: i32) -> String {
    let sign = if offset < 0 { '-' } else { '+' };
    let a = offset.abs();
    format!("{sign}{:02}{:02}", a / 3600, (a % 3600) / 60)
}

/// Render a [`Civil`] through a strftime-style format. Unknown specifiers pass
/// through verbatim (`%F` → `%F`), matching Tcl's `clock format`.
fn render(c: &Civil, fmt: &str, offset: i32, epoch: i64) -> String {
    let wd = WEEKDAYS[c.weekday as usize];
    let mon = MONTHS[(c.month - 1) as usize];
    let mut out = String::with_capacity(fmt.len() + 16);
    let mut chars = fmt.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let Some(mut spec) = chars.next() else {
            out.push('%');
            break;
        };
        if matches!(spec, 'E' | 'O') {
            let modifier = spec;
            let Some(modified) = chars.next() else {
                // Tcl treats a dangling locale modifier as an empty
                // conversion rather than rendering it literally.
                break;
            };
            if !modified.is_ascii() || !is_specifier(modified as u8) {
                out.push('%');
                out.push(modifier);
                out.push(modified);
                continue;
            }
            spec = modified;
        }
        match spec {
            'a' => out.push_str(&wd[..3]),
            'A' => out.push_str(wd),
            'b' | 'h' => out.push_str(&mon[..3]),
            'B' => out.push_str(mon),
            'C' => out.push_str(&format!("{:02}", c.year / 100)),
            'd' => out.push_str(&format!("{:02}", c.day)),
            'e' => out.push_str(&format!("{:2}", c.day)),
            'H' => out.push_str(&format!("{:02}", c.hour)),
            'I' => out.push_str(&format!("{:02}", hour12(c.hour))),
            'j' => out.push_str(&format!("{:03}", c.yearday)),
            'k' => out.push_str(&format!("{:2}", c.hour)),
            'l' => out.push_str(&format!("{:2}", hour12(c.hour))),
            'm' => out.push_str(&format!("{:02}", c.month)),
            'M' => out.push_str(&format!("{:02}", c.min)),
            'n' => out.push('\n'),
            'p' => out.push_str(if c.hour < 12 { "AM" } else { "PM" }),
            'R' => out.push_str(&format!("{:02}:{:02}", c.hour, c.min)),
            'r' => out.push_str(&format!(
                "{:02}:{:02}:{:02} {}",
                hour12(c.hour),
                c.min,
                c.sec,
                if c.hour < 12 { "AM" } else { "PM" }
            )),
            's' => out.push_str(&epoch.to_string()),
            'S' => out.push_str(&format!("{:02}", c.sec)),
            't' => out.push('\t'),
            'T' => out.push_str(&format!("{:02}:{:02}:{:02}", c.hour, c.min, c.sec)),
            'u' => out.push_str(&(if c.weekday == 0 { 7 } else { c.weekday }).to_string()),
            'w' => out.push_str(&c.weekday.to_string()),
            'y' => out.push_str(&format!("{:02}", c.year.rem_euclid(100))),
            'Y' => out.push_str(&c.year.to_string()),
            'z' => out.push_str(&fmt_offset(offset)),
            'Z' => {
                if offset == 0 {
                    out.push_str("GMT");
                } else {
                    out.push_str(&fmt_offset(offset));
                }
            }
            // Tcl's `%D`/`%x` use a **4-digit** year (not POSIX's 2-digit).
            'D' | 'x' => out.push_str(&format!("{:02}/{:02}/{}", c.month, c.day, c.year)),
            '%' => out.push('%'),
            // Unknown specifier: emit it verbatim (Tcl's behaviour).
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

/// One clock format conversion recognised by the shared renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Specifier {
    /// Byte offset of `%` in the format string.
    pub start: usize,
    /// Byte offset immediately after the conversion.
    pub end: usize,
    /// Optional locale modifier (`E` or `O`).
    pub modifier: Option<u8>,
    /// The conversion letter.
    pub letter: u8,
}

/// Whether `letter` is implemented by the shared clock renderer/parser.
/// Keeping this list next to `render` prevents editor-only POSIX additions
/// from being presented as Tcl features.
#[must_use]
pub fn is_specifier(letter: u8) -> bool {
    matches!(
        letter,
        b'a' | b'A'
            | b'b'
            | b'B'
            | b'C'
            | b'd'
            | b'D'
            | b'e'
            | b'H'
            | b'I'
            | b'j'
            | b'k'
            | b'l'
            | b'm'
            | b'M'
            | b'n'
            | b'p'
            | b'R'
            | b'r'
            | b's'
            | b'S'
            | b't'
            | b'T'
            | b'u'
            | b'w'
            | b'x'
            | b'y'
            | b'Y'
            | b'z'
            | b'Z'
            | b'%'
    )
}

/// Scan a clock format using the same supported conversion set as the
/// runtime. Tcl accepts `E` and `O` as locale modifiers only when a supported
/// conversion follows them; a dangling modifier is not a conversion.
#[must_use]
pub fn specifiers(fmt: &str) -> Vec<Specifier> {
    let bytes = fmt.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let modifier = if matches!(bytes.get(i), Some(b'E' | b'O'))
            && bytes.get(i + 1).is_some_and(|letter| is_specifier(*letter))
        {
            let modifier = bytes[i];
            i += 1;
            Some(modifier)
        } else {
            None
        };
        if let Some(&letter) = bytes.get(i)
            && is_specifier(letter)
        {
            i += 1;
            out.push(Specifier {
                start,
                end: i,
                modifier,
                letter,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trips_and_known_dates() {
        // 1970-01-01 is day 0, a Thursday.
        let c = Civil::from_secs(0);
        assert_eq!((c.year, c.month, c.day), (1970, 1, 1));
        assert_eq!(c.weekday, 4); // Thursday
        // 2023-11-14 22:13:20 UTC (the tclsh reference timestamp).
        let c = Civil::from_secs(1_700_000_000);
        assert_eq!((c.year, c.month, c.day), (2023, 11, 14));
        assert_eq!((c.hour, c.min, c.sec), (22, 13, 20));
        assert_eq!(c.weekday, 2); // Tuesday
        assert_eq!(c.yearday, 318);
    }

    #[test]
    fn days_civil_inverse() {
        for &z in &[-50_000_i64, -1, 0, 1, 19_000, 100_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
    }

    #[test]
    fn render_matches_tclsh_specifiers() {
        let c = Civil::from_secs(1_700_000_000);
        assert_eq!(
            render(&c, "%Y-%m-%d %H:%M:%S", 0, 1_700_000_000),
            "2023-11-14 22:13:20"
        );
        assert_eq!(
            render(&c, "%a %A %b %B %p", 0, 0),
            "Tue Tuesday Nov November PM"
        );
        assert_eq!(
            render(&c, "%j %u %w %e %I %z %%", 0, 0),
            "318 2 2 14 10 +0000 %"
        );
        assert_eq!(render(&c, "%T %R %D", 0, 0), "22:13:20 22:13 11/14/2023");
        assert_eq!(render(&c, "%s", 0, 1_700_000_000), "1700000000");
        assert_eq!(render(&c, "%EY %OS", 0, 0), "2023 20");
        assert_eq!(render(&c, "%E", 0, 0), "");
        // Unknown specifier passes through verbatim (Tcl does not support %F).
        assert_eq!(render(&c, "%F", 0, 0), "%F");
    }

    #[test]
    fn scan_with_format_parses_fields() {
        // Full date+time round-trips `format` (UTC).
        let f = parse_with_format("2023-11-14 22:13:20", "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!((f.year, f.month, f.day), (Some(2023), Some(11), Some(14)));
        assert_eq!((f.hour, f.min, f.sec), (Some(22), Some(13), Some(20)));
        let f = parse_with_format("2023 20", "%EY %OS").unwrap();
        assert_eq!((f.year, f.sec), (Some(2023), Some(20)));
        // Month name + the day defaults handled at the `scan` layer.
        let f = parse_with_format("Nov 14 2023", "%b %d %Y").unwrap();
        assert_eq!((f.year, f.month, f.day), (Some(2023), Some(11), Some(14)));
        // %s short-circuits to an absolute epoch.
        assert_eq!(
            parse_with_format("1700000000", "%s").unwrap().epoch,
            Some(1_700_000_000)
        );
        // A month past 12 is left for the validation step; a mismatch or an
        // unknown month name is `CLOCK badInputString`.
        assert_eq!(
            parse_with_format("2023-13-01", "%Y-%m-%d").unwrap().month,
            Some(13)
        );
        for (input, fmt) in [("xyz", "%Y"), ("Foo", "%b")] {
            let err = parse_with_format(input, fmt).unwrap_err();
            assert_eq!(
                err.message().unwrap(),
                "input string does not match supplied format"
            );
            assert_eq!(err.error_code().unwrap(), Some("CLOCK badInputString"));
        }
    }

    #[test]
    fn specifiers_include_complete_locale_modifiers() {
        assert_eq!(
            specifiers("%EY %OS %E"),
            vec![
                Specifier {
                    start: 0,
                    end: 3,
                    modifier: Some(b'E'),
                    letter: b'Y',
                },
                Specifier {
                    start: 4,
                    end: 7,
                    modifier: Some(b'O'),
                    letter: b'S',
                },
            ]
        );
    }

    #[test]
    fn add_calendar_months() {
        let off = |_: i64| 0;
        // +2 days and +3 calendar months from 2023-11-14.
        assert_eq!(
            add_units(1_700_000_000, 2, Unit::Days, true, &off).unwrap(),
            1_700_172_800
        );
        let plus3 = add_units(1_700_000_000, 3, Unit::Months, true, &off).unwrap();
        let c = Civil::from_secs(plus3);
        assert_eq!((c.year, c.month, c.day), (2024, 2, 14));
    }

    #[test]
    fn add_units_overflow_is_a_clean_error() {
        // `clock add 0 9223372036854775807 weeks` (and the calendar units)
        // must report the clean overflow error tclsh raises, not panic/wrap.
        let off = |_: i64| 0;
        let msg = "integer value too large to represent";
        assert_eq!(
            add_units(0, i64::MAX, Unit::Weeks, true, &off)
                .unwrap_err()
                .message()
                .unwrap(),
            msg
        );
        assert_eq!(
            add_units(i64::MAX, 1, Unit::Seconds, true, &off)
                .unwrap_err()
                .message()
                .unwrap(),
            msg
        );
        // Calendar months/years go through the i128 month tally + civil clamp.
        assert_eq!(
            add_units(0, i64::MAX, Unit::Months, true, &off)
                .unwrap_err()
                .message()
                .unwrap(),
            msg
        );
        assert_eq!(
            add_units(0, i64::MAX, Unit::Years, true, &off)
                .unwrap_err()
                .message()
                .unwrap(),
            msg
        );
        // A non-overflowing add still works on the i128 path.
        assert_eq!(
            add_units(0, 1, Unit::Weeks, true, &off).unwrap(),
            7 * 86_400
        );
    }

    /// tclsh 9.0.4/9.1.0 take any unique unit prefix and `weekdays`; 8.5.19
    /// and 8.6.18 take only the singular and plural words.
    #[test]
    fn units_read_as_each_release_reads_them() {
        for word in ["d", "mo", "y", "weekdays", "weekday", "days"] {
            assert!(unit_under(word, TclVersion::V9_0).is_ok(), "{word}");
        }
        let ambiguous = unit_under("w", TclVersion::V9_1).expect_err("w is ambiguous");
        assert_eq!(
            ambiguous.error_code().unwrap(),
            Some("TCL LOOKUP INDEX unit w")
        );
        for release in [TclVersion::V8_5, TclVersion::V8_6] {
            for word in ["day", "days", "month", "weeks", "second"] {
                assert!(unit_under(word, release).is_ok(), "{word} {release:?}");
            }
            for word in ["d", "mo", "weekdays", "w"] {
                let error = unit_under(word, release).expect_err(word);
                assert_eq!(
                    error.message().unwrap(),
                    format!(
                        "unknown unit \"{word}\", must be  years, months, weeks, days, hours, minutes or seconds"
                    )
                );
                assert_eq!(
                    error.error_code().unwrap(),
                    Some(format!("CLOCK badUnit {word}").as_str())
                );
            }
        }
    }

    #[test]
    fn weekdays_skip_the_weekend_as_c_does() {
        // ISO weekdays: Thursday 4, Sunday 7 (tclsh 9.0 `clock add … weekdays`).
        assert_eq!(weekdays_offs(4, 1), 1);
        assert_eq!(weekdays_offs(4, 5), 7);
        assert_eq!(weekdays_offs(4, -1), -1);
        assert_eq!(weekdays_offs(7, 1), 1);
        assert_eq!(weekdays_offs(5, 1), 3);
    }

    #[test]
    fn read_uint_overflow_is_a_clean_error() {
        // A 19-digit `%s` epoch beyond the wide range must not wrap the i64
        // accumulator — `read_uint` returns the clean overflow error.
        let mut ip = 0;
        let err = read_uint(b"9999999999999999999", &mut ip, 19).unwrap_err();
        assert_eq!(
            err.message().unwrap(),
            "integer value too large to represent"
        );
        assert_eq!(err.error_code().unwrap(), Some("CLOCK dateTooLarge"));
        // In-range values still parse.
        let mut ip = 0;
        assert_eq!(read_uint(b"2023", &mut ip, 4).unwrap(), 2023);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C86CalendarFields {
    pub seconds: i64,
    pub local_seconds: i64,
    pub tz_offset: i32,
    pub julian_day: i32,
    pub gregorian: i32,
    pub bce: bool,
    pub year: i32,
    pub day_of_year: i32,
    pub month: i32,
    pub day_of_month: i32,
    pub iso8601_year: i32,
    pub iso8601_week: i32,
    pub day_of_week: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct C86Date {
    jday: i32,
    bce: bool,
    gregorian: bool,
    year: i32,
    day: i32,
    month: i32,
    month_day: i32,
    iso_year: i32,
    iso_week: i32,
    weekday: i32,
}

type C86CalendarResult<T> = Result<T, tcl_syntax::value::ValueError>;

fn c86_calendar_unavailable() -> tcl_syntax::value::ValueError {
    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "unproved C86 GetDateFields arithmetic extent",
    )
}
fn c86_add(a: i32, b: i32) -> C86CalendarResult<i32> {
    a.checked_add(b).ok_or_else(c86_calendar_unavailable)
}
fn c86_sub(a: i32, b: i32) -> C86CalendarResult<i32> {
    a.checked_sub(b).ok_or_else(c86_calendar_unavailable)
}
fn c86_mul(a: i32, b: i32) -> C86CalendarResult<i32> {
    a.checked_mul(b).ok_or_else(c86_calendar_unavailable)
}

fn c86_era_year_day(fields: &mut C86Date, changeover: i32) -> C86CalendarResult<()> {
    let mut year = 1;
    let mut day;
    if fields.jday >= changeover {
        fields.gregorian = true;
        day = c86_sub(fields.jday, 1_721_426)?;
        let mut n = day / 146_097;
        day %= 146_097;
        if day < 0 {
            day = c86_add(day, 146_097)?;
            n = c86_sub(n, 1)?;
        }
        year = c86_add(year, c86_mul(400, n)?)?;
        n = day / 36_524;
        day %= 36_524;
        if n > 3 {
            n = 3;
            day = c86_add(day, 36_524)?;
        }
        year = c86_add(year, c86_mul(100, n)?)?;
    } else {
        fields.gregorian = false;
        day = c86_sub(fields.jday, 1_721_424)?;
    }
    let mut n = day / 1_461;
    day %= 1_461;
    if day < 0 {
        day = c86_add(day, 1_461)?;
        n = c86_sub(n, 1)?;
    }
    year = c86_add(year, c86_mul(4, n)?)?;
    n = day / 365;
    day %= 365;
    if n > 3 {
        n = 3;
        day = c86_add(day, 365)?;
    }
    year = c86_add(year, n)?;
    fields.bce = year <= 0;
    fields.year = if fields.bce { c86_sub(1, year)? } else { year };
    fields.day = c86_add(day, 1)?;
    Ok(())
}

fn c86_leap(fields: &C86Date) -> C86CalendarResult<bool> {
    let year = if fields.bce {
        c86_sub(1, fields.year)?
    } else {
        fields.year
    };
    Ok(year % 4 == 0 && (!fields.gregorian || year % 400 == 0 || year % 100 != 0))
}

fn c86_month_day(fields: &mut C86Date) -> C86CalendarResult<()> {
    const LENGTHS: [[i32; 12]; 2] = [
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    ];
    let lengths = &LENGTHS[usize::from(c86_leap(fields)?)];
    let mut day = fields.day;
    let mut month = 0;
    while month < 12 && day > lengths[month] {
        day = c86_sub(day, lengths[month])?;
        month += 1;
    }
    if month >= 12 || day < 1 {
        return Err(c86_calendar_unavailable());
    }
    fields.month = i32::try_from(month + 1).map_err(|_| c86_calendar_unavailable())?;
    fields.month_day = day;
    Ok(())
}

fn c86_month_day_to_jday(fields: &mut C86Date, changeover: i32) -> C86CalendarResult<()> {
    const PRIOR: [[i32; 13]; 2] = [
        [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365],
        [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366],
    ];
    let mut year = if fields.bce {
        c86_sub(1, fields.year)?
    } else {
        fields.year
    };
    let mm1 = c86_sub(fields.month, 1)?;
    let mut q = mm1 / 12;
    let mut r = mm1 % 12;
    if r < 0 {
        r = c86_add(r, 12)?;
        q = c86_sub(q, 1)?;
    }
    year = c86_add(year, q)?;
    let month = c86_add(r, 1)?;
    let ym1 = c86_sub(year, 1)?;
    fields.gregorian = true;
    fields.bce = year < 1;
    fields.year = if fields.bce { c86_sub(1, year)? } else { year };
    // C's negative-quarter workaround evaluates -ym1 as a signed int first.
    let mut q4 = if ym1 >= 0 {
        ym1 / 4
    } else {
        let positive = ym1.checked_neg().ok_or_else(c86_calendar_unavailable)?;
        -(positive / 4)
    };
    if ym1 % 4 < 0 {
        q4 = c86_sub(q4, 1)?;
    }
    let mut q100 = ym1 / 100;
    if ym1 % 100 < 0 {
        q100 = c86_sub(q100, 1)?;
    }
    let mut q400 = ym1 / 400;
    if ym1 % 400 < 0 {
        q400 = c86_sub(q400, 1)?;
    }
    let index = usize::try_from(c86_sub(month, 1)?).map_err(|_| c86_calendar_unavailable())?;
    let gregorian_prior = PRIOR[usize::from(c86_leap(fields)?)][index];
    let mut jday = c86_sub(1_721_426, 1)?;
    jday = c86_add(jday, fields.month_day)?;
    jday = c86_add(jday, gregorian_prior)?;
    jday = c86_add(jday, c86_mul(365, ym1)?)?;
    jday = c86_add(jday, q4)?;
    jday = c86_sub(jday, q100)?;
    jday = c86_add(jday, q400)?;
    if jday < changeover {
        fields.gregorian = false;
        jday = c86_sub(1_721_424, 1)?;
        jday = c86_add(jday, fields.month_day)?;
        jday = c86_add(jday, PRIOR[usize::from(year % 4 == 0)][index])?;
        jday = c86_add(jday, c86_mul(365, ym1)?)?;
        jday = c86_add(jday, q4)?;
    }
    fields.jday = jday;
    Ok(())
}

fn c86_weekday_on_or_before(weekday: i32, jday: i32) -> C86CalendarResult<i32> {
    let mut k = c86_add(weekday, 6)? % 7;
    if k < 0 {
        k = c86_add(k, 7)?;
    }
    c86_sub(jday, c86_sub(jday, k)? % 7)
}

fn c86_iso_to_jday(fields: &mut C86Date, changeover: i32) -> C86CalendarResult<()> {
    let mut first_week = C86Date {
        bce: fields.bce,
        year: fields.iso_year,
        month: 1,
        month_day: 4,
        ..Default::default()
    };
    c86_month_day_to_jday(&mut first_week, changeover)?;
    let monday = c86_weekday_on_or_before(1, first_week.jday)?;
    let jday = c86_add(monday, c86_mul(7, c86_sub(fields.iso_week, 1)?)?)?;
    fields.jday = c86_sub(c86_add(jday, fields.weekday)?, 1)?;
    Ok(())
}

fn c86_year_week_day(fields: &mut C86Date, changeover: i32) -> C86CalendarResult<()> {
    let mut temp = C86Date {
        jday: c86_sub(fields.jday, 3)?,
        ..Default::default()
    };
    c86_era_year_day(&mut temp, changeover)?;
    temp.iso_year = if temp.bce {
        c86_sub(temp.year, 1)?
    } else {
        c86_add(temp.year, 1)?
    };
    temp.iso_week = 1;
    temp.weekday = 1;
    c86_iso_to_jday(&mut temp, changeover)?;
    if fields.jday < temp.jday {
        temp.iso_year = if temp.bce {
            c86_add(temp.iso_year, 1)?
        } else {
            c86_sub(temp.iso_year, 1)?
        };
        c86_iso_to_jday(&mut temp, changeover)?;
    }
    fields.iso_year = temp.iso_year;
    let fiscal_day = c86_sub(fields.jday, temp.jday)?;
    fields.iso_week = c86_add(fiscal_day / 7, 1)?;
    fields.weekday = c86_add(fiscal_day, 1)? % 7;
    if fields.weekday < 1 {
        fields.weekday = c86_add(fields.weekday, 7)?;
    }
    Ok(())
}

/// Numeric calendar fields only; supplies no getters, objects, host timezone or admission.
pub fn c86_calendar_fields(
    seconds: i64,
    offset: i32,
    changeover: i32,
) -> C86CalendarResult<C86CalendarFields> {
    let local_seconds = seconds
        .checked_add(i64::from(offset))
        .ok_or_else(c86_calendar_unavailable)?;
    let quotient = local_seconds / 86_400;
    let floored = quotient
        .checked_sub(i64::from(local_seconds % 86_400 < 0))
        .ok_or_else(c86_calendar_unavailable)?;
    let jday = floored
        .checked_add(2_440_588)
        .ok_or_else(c86_calendar_unavailable)?;
    let mut date = C86Date {
        jday: i32::try_from(jday).map_err(|_| c86_calendar_unavailable())?,
        ..Default::default()
    };
    c86_era_year_day(&mut date, changeover)?;
    c86_month_day(&mut date)?;
    c86_year_week_day(&mut date, changeover)?;
    Ok(C86CalendarFields {
        seconds,
        local_seconds,
        tz_offset: offset,
        julian_day: date.jday,
        gregorian: i32::from(date.gregorian),
        bce: date.bce,
        year: date.year,
        day_of_year: date.day,
        month: date.month,
        day_of_month: date.month_day,
        iso8601_year: date.iso_year,
        iso8601_week: date.iso_week,
        day_of_week: date.weekday,
    })
}

/// Original C86 Dict keys, in the worker's insertion order.
pub const C86_DATE_FIELD_KEYS: [&[u8]; 14] = [
    b"localSeconds",
    b"seconds",
    b"tzName",
    b"tzOffset",
    b"julianDay",
    b"gregorian",
    b"era",
    b"year",
    b"dayOfYear",
    b"month",
    b"dayOfMonth",
    b"iso8601Year",
    b"iso8601Week",
    b"dayOfWeek",
];

pub struct DateFieldLiterals<V> {
    pub keys: [V; 14],
    pub ce: V,
    pub bce: V,
}

pub enum C86DateValue<V> {
    Wide(i64),
    Int(i32),
    Original(V),
}

/// Numeric constructors and original children retain distinct native roles.
pub fn c86_date_values<V: Clone>(
    fields: &C86CalendarFields,
    name: &V,
    literals: &DateFieldLiterals<V>,
) -> [C86DateValue<V>; 14] {
    use C86DateValue::{Int, Original, Wide};
    [
        Wide(fields.local_seconds),
        Wide(fields.seconds),
        Original(name.clone()),
        Int(fields.tz_offset),
        Int(fields.julian_day),
        Int(fields.gregorian),
        Original(if fields.bce {
            literals.bce.clone()
        } else {
            literals.ce.clone()
        }),
        Int(fields.year),
        Int(fields.day_of_year),
        Int(fields.month),
        Int(fields.day_of_month),
        Int(fields.iso8601_year),
        Int(fields.iso8601_week),
        Int(fields.day_of_week),
    ]
}

/// A reached original-object purpose; neutral List/Dict operations do not supply it.
pub trait NativeClockDateOps {
    type Value: Clone;
    type List;
    fn date_check(&self) -> Result<(), tcl_syntax::value::ValueError>;
    fn date_wide(&mut self, value: &Self::Value) -> Result<i64, tcl_syntax::value::ValueError>;
    fn date_int(&mut self, value: &Self::Value) -> Result<i32, tcl_syntax::value::ValueError>;
    fn date_is_bignum(
        &mut self,
        value: &Self::Value,
    ) -> Result<bool, tcl_syntax::value::ValueError>;
    fn date_elements(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::List, tcl_syntax::value::ValueError>;
    fn date_len(&self, list: &Self::List) -> Result<usize, tcl_syntax::value::ValueError>;
    fn date_child(
        &mut self,
        list: &Self::List,
        index: usize,
    ) -> Result<Self::Value, tcl_syntax::value::ValueError>;
    fn date_index_zero(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::Value, tcl_syntax::value::ValueError>;
    fn date_result(
        &mut self,
        fields: &C86CalendarFields,
        name: &Self::Value,
    ) -> Result<Self::Value, CmdError>;
}

pub enum DateFieldError {
    Command(CmdError),
    BignumSeconds,
}
impl From<CmdError> for DateFieldError {
    fn from(error: CmdError) -> Self {
        Self::Command(error)
    }
}
impl From<tcl_syntax::value::ValueError> for DateFieldError {
    fn from(error: tcl_syntax::value::ValueError) -> Self {
        Self::Command(error.into())
    }
}

fn reached_date<O: NativeClockDateOps, T>(
    ops: &O,
    result: Result<T, tcl_syntax::value::ValueError>,
) -> Result<T, DateFieldError> {
    ops.date_check()?;
    Ok(result?)
}

fn date_transition<O: NativeClockDateOps>(
    ops: &mut O,
    table: &O::List,
    index: usize,
) -> Result<i64, DateFieldError> {
    let result = ops.date_child(table, index);
    let row = reached_date(ops, result)?;
    let result = ops.date_index_zero(&row);
    let cell = reached_date(ops, result)?;
    let result = ops.date_wide(&cell);
    reached_date(ops, result)
}

/// Parse original seconds/changeover/table in the actual C86 worker order.
/// Empty-table C localtime and unsafe native arithmetic/storage extents refuse.
pub fn get_date_fields<O: NativeClockDateOps>(
    ops: &mut O,
    args: &[O::Value],
) -> Result<O::Value, DateFieldError> {
    use tcl_syntax::value::ValueError;
    ops.date_check()?;
    if args.len() != 3 {
        return Err(
            CmdError::wrong_args("::tcl::clock::GetDateFields seconds tzdata changeover").into(),
        );
    }
    let result = ops.date_wide(&args[0]);
    let seconds = reached_date(ops, result)?;
    let result = ops.date_int(&args[2]);
    let changeover = reached_date(ops, result)?;
    let result = ops.date_is_bignum(&args[0]);
    if reached_date(ops, result)? {
        return Err(DateFieldError::BignumSeconds);
    }
    let result = ops.date_elements(&args[1]);
    let table = reached_date(ops, result)?;
    let result = ops.date_len(&table);
    let rows = reached_date(ops, result)?;
    if rows == 0 {
        return Err(
            ValueError::CommandProtocolUnavailable("native C86 localtime conversion").into(),
        );
    }
    let rows = i32::try_from(rows)
        .map_err(|_| ValueError::CommandProtocolUnavailable("C86 timezone row-count extent"))?;
    let first = date_transition(ops, &table, 0)?;
    let mut lower = 0_i32;
    if seconds >= first {
        let mut upper = rows - 1;
        while lower < upper {
            let middle = lower
                .checked_add(upper)
                .and_then(|sum| sum.checked_add(1))
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "C86 transition-search arithmetic extent",
                ))?
                / 2;
            let tick = date_transition(ops, &table, middle as usize)?;
            if seconds >= tick {
                lower = middle;
            } else {
                upper = middle - 1;
            }
        }
    }
    let result = ops.date_child(&table, lower as usize);
    let selected = reached_date(ops, result)?;
    let result = ops.date_elements(&selected);
    let row = reached_date(ops, result)?;
    let result = ops.date_child(&row, 1);
    let offset_value = reached_date(ops, result)?;
    let result = ops.date_int(&offset_value);
    let offset = reached_date(ops, result)?;
    let result = ops.date_child(&row, 3);
    let name = reached_date(ops, result)?;
    let fields = c86_calendar_fields(seconds, offset, changeover)?;
    ops.date_check()?;
    let result = ops.date_result(&fields, &name);
    ops.date_check()?;
    result.map_err(Into::into)
}

#[cfg(test)]
mod c86_calendar_tests {
    use super::*;

    #[test]
    fn retained_c86_numeric_calendar_rows() {
        let protocol =
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let literals = DateFieldLiterals {
            keys: C86_DATE_FIELD_KEYS.map(Vec::from),
            ce: b"CE".to_vec(),
            bce: b"BCE".to_vec(),
        };
        let mut compared = 0;
        for row in include_str!("../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/capture-01/8.6.18/values/stdout.txt").lines() {
            if !row.starts_with("ROW ") { continue; }
            let words = tcl_syntax::list::split_native_list_bytes(row.as_bytes(), protocol).unwrap();
            if words[3].as_ref() != b"0" || words[1].as_ref() == b"empty-tzdata" { continue; }
            let expected = tcl_syntax::list::split_native_list_bytes(&words[5], protocol).unwrap().into_iter().map(|word| word.into_owned()).collect::<Vec<_>>();
            let value = |key: &[u8]| expected.chunks_exact(2).find(|pair| pair[0] == key).unwrap()[1].clone();
            let integer = |key: &[u8]| std::str::from_utf8(&value(key)).unwrap().parse::<i64>().unwrap();
            let changeover = match words[1].as_ref() {
                b"julian-calendar" => i32::MAX,
                b"gregorian-forced" => 0,
                b"changeover-before" | b"changeover-at" => 2299161,
                b"changeover-unsigned" => -1,
                _ => 2361222,
            };
            let fields = c86_calendar_fields(integer(b"seconds"), i32::try_from(integer(b"tzOffset")).unwrap(), changeover).unwrap();
            let actual = C86_DATE_FIELD_KEYS.into_iter().zip(c86_date_values(&fields,&value(b"tzName"),&literals))
                .flat_map(|(key, role)| {
                    let bytes = match role {
                        C86DateValue::Wide(number) => number.to_string().into_bytes(),
                        C86DateValue::Int(number) => number.to_string().into_bytes(),
                        C86DateValue::Original(bytes) => bytes,
                    };
                    [key.to_vec(), bytes]
                }).collect::<Vec<_>>();
            assert_eq!(actual, expected, "{}", row);
            compared += 1;
        }
        assert_eq!(compared, 16);
    }

    #[test]
    fn unmeasured_native_calendar_overflow_refuses() {
        for (seconds, offset) in [(i64::MAX, 1), (i64::MAX, 0)] {
            assert!(matches!(
                c86_calendar_fields(seconds, offset, 2361222),
                Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(_))
            ));
        }
    }
}
