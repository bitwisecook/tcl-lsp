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

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

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
/// A bad subcommand, bad option, non-integer argument, or `clock scan` (which is
/// not yet implemented).
pub fn dispatch<O, V>(
    ops: &mut O,
    args: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((sub, rest)) = args.split_first() else {
        return Err(CmdError::wrong_args("clock subcommand ?arg ...?"));
    };
    let sub = ops.as_str(sub).to_string();
    match sub.as_str() {
        "seconds" => nullary(ops, rest, "seconds", now.secs),
        "milliseconds" => nullary(ops, rest, "milliseconds", now.millis),
        "microseconds" => nullary(ops, rest, "microseconds", now.micros),
        "clicks" => clicks(ops, rest, now),
        "format" => format(ops, rest, local_offset),
        "add" => add(ops, rest, local_offset),
        "scan" => scan(ops, rest, now, local_offset),
        other => Err(CmdError::new(format!(
            "unknown or ambiguous subcommand \"{other}\": must be add, clicks, \
             format, microseconds, milliseconds, scan, or seconds"
        ))),
    }
}

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
    let val = match rest {
        [] => now.micros,
        [opt] => match &*ops.as_str(opt) {
            "-milliseconds" => now.millis,
            "-microseconds" => now.micros,
            other => {
                return Err(CmdError::bad_choice(
                    "option",
                    other,
                    "-milliseconds or -microseconds",
                ));
            }
        },
        _ => {
            return Err(CmdError::wrong_args(
                "clock clicks ?-milliseconds|-microseconds?",
            ));
        }
    };
    Ok(ops.new_int(val))
}

const DEFAULT_FORMAT: &str = "%a %b %d %H:%M:%S %Z %Y";

/// `clock format clockval ?-format string? ?-gmt boolean? ?-timezone z? ?-locale l?`.
fn format<O, V>(ops: &mut O, rest: &[V], local_offset: &dyn Fn(i64) -> i32) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((cv, opts)) = rest.split_first() else {
        return Err(CmdError::wrong_args(
            "clock format clockval ?-format string? ?-gmt boolean? ?-locale locale? ?-timezone zone?",
        ));
    };
    let clockval = ops.as_int(cv)?;
    let mut fmt = DEFAULT_FORMAT.to_string();
    let mut gmt = false;
    let mut i = 0;
    while i < opts.len() {
        let name = ops.as_str(&opts[i]).to_string();
        let value = opts
            .get(i + 1)
            .ok_or_else(|| CmdError::new(format!("value for \"{name}\" missing")))?;
        match name.as_str() {
            "-format" => fmt = ops.as_str(value).to_string(),
            "-gmt" => gmt = ops.as_bool(value)?,
            "-timezone" | "-locale" => {} // accepted, not yet honoured
            other => {
                return Err(CmdError::bad_choice(
                    "option",
                    other,
                    "-format, -gmt, -locale, or -timezone",
                ));
            }
        }
        i += 2;
    }
    let offset = if gmt { 0 } else { local_offset(clockval) };
    // `clockval` is user input; a near-`i64::MAX` value plus the zone offset must
    // not wrap before we break it into a civil date.
    let local = clockval
        .checked_add(i64::from(offset))
        .ok_or_else(overflow)?;
    let civil = Civil::from_secs(local);
    Ok(ops.new_str(&render(&civil, &fmt, offset, clockval)))
}

/// `clock add clockval ?count unit ...? ?-gmt boolean? ?-timezone z? ?-locale l?`.
fn add<O, V>(ops: &mut O, rest: &[V], local_offset: &dyn Fn(i64) -> i32) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((cv, rest)) = rest.split_first() else {
        return Err(CmdError::wrong_args(
            "clock add clockval ?number units ...? ?-option value?",
        ));
    };
    let mut t = ops.as_int(cv)?;
    let mut gmt = false;
    let mut i = 0;
    while i < rest.len() {
        let word = ops.as_str(&rest[i]).to_string();
        match word.as_str() {
            "-gmt" | "-timezone" | "-locale" | "-base" => {
                let value = rest
                    .get(i + 1)
                    .ok_or_else(|| CmdError::new(format!("value for \"{word}\" missing")))?;
                if word == "-gmt" {
                    gmt = ops.as_bool(value)?;
                }
                i += 2;
            }
            _ => {
                // A `count unit` pair (count may be negative).
                let count = ops.as_int(&rest[i])?;
                let unit = rest
                    .get(i + 1)
                    .ok_or_else(|| CmdError::new("missing unit of measure"))?;
                let unit = ops.as_str(unit).to_string();
                t = add_units(t, count, &unit, gmt, local_offset)?;
                i += 2;
            }
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

/// Apply `count` of `unit` to the timestamp `t` (UTC seconds). Fixed units add a
/// constant; calendar units (months/years) go through the civil date in the
/// active zone, clamping the day to the target month's length.
fn add_units(
    t: i64,
    count: i64,
    unit: &str,
    gmt: bool,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<i64, CmdError> {
    let fixed = match unit {
        "seconds" | "second" | "secs" | "sec" => Some(1),
        "minutes" | "minute" | "mins" | "min" => Some(60),
        "hours" | "hour" => Some(3600),
        "days" | "day" => Some(86_400),
        "weeks" | "week" => Some(7 * 86_400),
        _ => None,
    };
    if let Some(scale) = fixed {
        // `count` and `t` are both user-controlled wides; compute `t + count*scale`
        // in i128 so neither the multiply nor the add can wrap before the range
        // check (the precedent `lseq` uses i128 intermediates for the same reason).
        let secs = i128::from(t) + i128::from(count) * i128::from(scale);
        return i64::try_from(secs).map_err(|_| overflow());
    }
    let months = match unit {
        "months" | "month" => i128::from(count),
        // `count * 12` (years→months) can itself overflow i64 for an extreme
        // count, so keep the whole month tally in i128.
        "years" | "year" => i128::from(count) * 12,
        other => {
            return Err(CmdError::new(format!(
                "unknown unit of measure \"{other}\": must be \
                 seconds, minutes, hours, days, weeks, months, or years"
            )));
        }
    };
    let offset = if gmt { 0 } else { i64::from(local_offset(t)) };
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

/// `clock scan inputString -format formatString ?-gmt boolean? ?-base clock? …` —
/// parse `inputString` per `formatString` into an epoch timestamp. Free-form
/// scanning (no `-format`) is not yet supported.
fn scan<O, V>(
    ops: &mut O,
    rest: &[V],
    now: &Now,
    local_offset: &dyn Fn(i64) -> i32,
) -> Result<V, CmdError>
where
    O: ValueOps<Value = V>,
{
    let Some((input, opts)) = rest.split_first() else {
        return Err(CmdError::wrong_args(
            "clock scan inputString ?-base clockVal? ?-format string? ?-gmt boolean? ?-locale locale? ?-timezone zone?",
        ));
    };
    let input = ops.as_str(input).to_string();
    let mut fmt: Option<String> = None;
    let mut gmt = false;
    let mut base = now.secs;
    let mut i = 0;
    while i < opts.len() {
        let name = ops.as_str(&opts[i]).to_string();
        let value = opts
            .get(i + 1)
            .ok_or_else(|| CmdError::new(format!("value for \"{name}\" missing")))?;
        match name.as_str() {
            "-format" => fmt = Some(ops.as_str(value).to_string()),
            "-gmt" => gmt = ops.as_bool(value)?,
            "-base" => base = ops.as_int(value)?,
            "-timezone" | "-locale" => {}
            other => {
                return Err(CmdError::bad_choice(
                    "option",
                    other,
                    "-base, -format, -gmt, -locale, or -timezone",
                ));
            }
        }
        i += 2;
    }
    // Free-form (natural-language) scanning is not implemented; `-format` only.
    let Some(fmt) = fmt else {
        return Err(CmdError::new("free-form clock scan is not yet supported"));
    };
    let base_offset = if gmt { 0 } else { local_offset(base) };
    // `-base` is user input; guard the offset add before breaking it into a date.
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
    if day == 0 || day > days_in_month(year, month) {
        return Err(CmdError::new("unable to convert input string: invalid day"));
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
    let offset = if gmt { 0 } else { local_offset(local) };
    let result = i128::from(local) - i128::from(offset);
    Ok(ops.new_int(i64::try_from(result).map_err(|_| overflow())?))
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
    CmdError::new("input string does not match supplied format")
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
            .ok_or_else(overflow)?;
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
    Err(CmdError::new(
        "unable to convert input string: invalid month",
    ))
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
        let Some(spec) = chars.next() else {
            return Err(no_match());
        };
        match spec {
            'Y' => f.year = Some(read_uint(inb, &mut ip, 4)?),
            'y' => {
                let yy = read_uint(inb, &mut ip, 2)?;
                f.year = Some(if yy < 69 { 2000 + yy } else { 1900 + yy });
            }
            'm' => {
                let m = read_uint(inb, &mut ip, 2)?;
                if !(1..=12).contains(&m) {
                    return Err(CmdError::new(
                        "unable to convert input string: invalid month",
                    ));
                }
                f.month = Some(m as u32);
            }
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
        let Some(spec) = chars.next() else {
            out.push('%');
            break;
        };
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
/// runtime. `%E`/`%O` are intentionally not treated as locale modifiers: the
/// runtime does not implement them and emits them literally.
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
        if let Some(&letter) = bytes.get(i)
            && is_specifier(letter)
        {
            i += 1;
            out.push(Specifier {
                start,
                end: i,
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
        // Unknown specifier passes through verbatim (Tcl does not support %F).
        assert_eq!(render(&c, "%F", 0, 0), "%F");
    }

    #[test]
    fn scan_with_format_parses_fields() {
        // Full date+time round-trips `format` (UTC).
        let f = parse_with_format("2023-11-14 22:13:20", "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!((f.year, f.month, f.day), (Some(2023), Some(11), Some(14)));
        assert_eq!((f.hour, f.min, f.sec), (Some(22), Some(13), Some(20)));
        // Month name + the day defaults handled at the `scan` layer.
        let f = parse_with_format("Nov 14 2023", "%b %d %Y").unwrap();
        assert_eq!((f.year, f.month, f.day), (Some(2023), Some(11), Some(14)));
        // %s short-circuits to an absolute epoch.
        assert_eq!(
            parse_with_format("1700000000", "%s").unwrap().epoch,
            Some(1_700_000_000)
        );
        // Errors: bad month, then a literal/format mismatch.
        assert_eq!(
            parse_with_format("2023-13-01", "%Y-%m-%d")
                .unwrap_err()
                .message(),
            "unable to convert input string: invalid month"
        );
        assert_eq!(
            parse_with_format("xyz", "%Y").unwrap_err().message(),
            "input string does not match supplied format"
        );
    }

    #[test]
    fn add_calendar_months() {
        let off = |_: i64| 0;
        // +2 days and +3 calendar months from 2023-11-14.
        assert_eq!(
            add_units(1_700_000_000, 2, "days", true, &off).unwrap(),
            1_700_172_800
        );
        let plus3 = add_units(1_700_000_000, 3, "months", true, &off).unwrap();
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
            add_units(0, i64::MAX, "weeks", true, &off)
                .unwrap_err()
                .message(),
            msg
        );
        assert_eq!(
            add_units(i64::MAX, 1, "seconds", true, &off)
                .unwrap_err()
                .message(),
            msg
        );
        // Calendar months/years go through the i128 month tally + civil clamp.
        assert_eq!(
            add_units(0, i64::MAX, "months", true, &off)
                .unwrap_err()
                .message(),
            msg
        );
        assert_eq!(
            add_units(0, i64::MAX, "years", true, &off)
                .unwrap_err()
                .message(),
            msg
        );
        // A non-overflowing add still works (regression guard on the i128 path).
        assert_eq!(add_units(0, 1, "weeks", true, &off).unwrap(), 7 * 86_400);
    }

    #[test]
    fn read_uint_overflow_is_a_clean_error() {
        // A 19-digit `%s` epoch beyond the wide range must not wrap the i64
        // accumulator — `read_uint` returns the clean overflow error.
        let mut ip = 0;
        let err = read_uint(b"9999999999999999999", &mut ip, 19).unwrap_err();
        assert_eq!(err.message(), "integer value too large to represent");
        // In-range values still parse.
        let mut ip = 0;
        assert_eq!(read_uint(b"2023", &mut ip, 4).unwrap(), 2023);
    }
}
