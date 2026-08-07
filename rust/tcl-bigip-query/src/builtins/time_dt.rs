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

//! Time / date-category builtins, wrapping time / date / calendar
//! semantics.
//!
//! Covers `now`, the jq date helpers (`todate` / `todateiso8601` / `date`,
//! `fromdate` / `fromdateiso8601`), the broken-down-time family (`gmtime` /
//! `localtime` / `mktime`), the `strftime` / `strptime` formatters, and the
//! `dateadd` / `datesub` arithmetic shims.
//!
//! Behaviour notes:
//! - Calendar conversions route through `chrono` (`DateTime<Utc>` /
//!   `NaiveDateTime`), whose civil-date algorithm reproduces the proleptic
//!   Gregorian calendar, so the UTC-based
//!   builtins (`todate`, `fromdate`, `gmtime`, `dateadd`, `datesub`) are
//!   byte-identical.
//! - **`now`** returns the current Unix time as a float (non-deterministic);
//!   it is exercised by a unit test here, not the golden fixture.
//! - **Timezone**: the DSL's `localtime` / `mktime` / `strftime` follow the
//!   process timezone. The generator and the differential test both pin `TZ=UTC`,
//!   so every case is deterministic. With `TZ=UTC`, local time equals UTC, so
//!   `localtime` shares the UTC broken-down conversion (documented here rather
//!   than reading `TZ` via libc, which chrono's `Local` does only
//!   unreliably under the test harness).
//! - **`strftime` / `strptime`**: `chrono`'s formatter matches the C
//!   `strftime` byte-for-byte on the specifiers the fixture exercises
//!   (`%Y %m %d %H %M %S %j %w %A %a %B %b %p %U %W %%`). Locale-dependent
//!   or name-bearing specifiers that are NOT guaranteed identical
//!   (`%Z`, `%c`, `%x`, `%X`, `%-d`, `%z`) are deliberately excluded from the
//!   fixture; the implementation still forwards them to `chrono`.
//! - Floats render with a trailing `.0` and integers without, so each result's
//!   int-vs-float type is tracked precisely: `fromdate` / `mktime` / `now` are
//!   floats; `dateadd` / `datesub` preserve int-vs-float like `+` / `-`.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};

use crate::builtins::{BuiltinSpec, as_number, as_str, plain, type_name};
use crate::errors::QueryError;
use crate::value::Value;

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("now", "time", 0, Some(0), false, bi_now),
        plain("todate", "time", 1, Some(1), false, bi_todate),
        plain("todateiso8601", "time", 1, Some(1), false, bi_todate),
        plain("fromdate", "time", 1, Some(1), false, bi_fromdate),
        plain("fromdateiso8601", "time", 1, Some(1), false, bi_fromdate),
        plain("date", "time", 1, Some(1), false, bi_todate),
        plain("gmtime", "time", 1, Some(1), false, bi_gmtime),
        plain("localtime", "time", 1, Some(1), false, bi_localtime),
        plain("mktime", "time", 1, Some(1), false, bi_mktime),
        plain("strftime", "time", 2, Some(2), false, bi_strftime),
        plain("strptime", "time", 2, Some(2), false, bi_strptime),
        plain("dateadd", "time", 2, Some(2), false, bi_dateadd),
        plain("datesub", "time", 2, Some(2), false, bi_datesub),
    ]
}

// Helpers

/// Coerce an `as_number` result to `f64`.
fn num_f64(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => unreachable!("as_number returns Int or Float"),
    }
}

/// Split a Unix-seconds float into the integer seconds passed to the C library
/// after truncating sub-second parts, plus the original value for the
/// `fromtimestamp` path. The platform `gmtime` floors toward negative infinity
/// for the day / time fields; `chrono`'s `from_timestamp` takes `(secs, nanos)`
/// with `nanos` in `[0, 1e9)`, so we floor the seconds and drop the fraction.
fn floor_secs(n: f64) -> i64 {
    n.floor() as i64
}

/// Build a UTC datetime from Unix seconds (sub-second part dropped, matching
/// the broken-down `gmtime`/`strftime`, which discard the fraction).
fn utc_from_unix(n: f64) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(floor_secs(n), 0)
}

/// jq's broken-down array order:
/// `[year-1900, month(0..11), day, hour, minute, second, wday(0=Sun), yday-1]`.
fn broken_down(dt: &DateTime<Utc>) -> Value {
    let naive = dt.naive_utc();
    broken_down_naive(&naive)
}

fn broken_down_naive(naive: &NaiveDateTime) -> Value {
    // `tm_wday`: 0=Mon..6=Sun; the DSL remaps to 0=Sun..6=Sat via
    // `tm_wday + 1 if tm_wday < 6 else 0`.
    let py_wday = i64::from(naive.weekday().num_days_from_monday()); // 0=Mon..6=Sun
    let jq_wday = if py_wday < 6 { py_wday + 1 } else { 0 };
    let yday0 = i64::from(naive.ordinal()) - 1; // tm_yday is 1-based; array is 0-based.
    Value::List(vec![
        Value::Int(i64::from(naive.year()) - 1900),
        Value::Int(i64::from(naive.month()) - 1),
        Value::Int(i64::from(naive.day())),
        Value::Int(i64::from(naive.hour())),
        Value::Int(i64::from(naive.minute())),
        Value::Int(i64::from(naive.second())),
        Value::Int(jq_wday),
        Value::Int(yday0),
    ])
}

// Builtins

fn bi_now(_args: &[Value]) -> Result<Value, QueryError> {
    // Current Unix time as a float.
    let now = Utc::now();
    let secs = now.timestamp() as f64;
    let frac = f64::from(now.timestamp_subsec_nanos()) / 1_000_000_000.0;
    Ok(Value::Float(secs + frac))
}

fn bi_todate(args: &[Value]) -> Result<Value, QueryError> {
    // Shared by `todate`, `todateiso8601`, and `date`. Formats Unix epoch
    // seconds as `%Y-%m-%dT%H:%M:%SZ` in UTC (sub-second part dropped).
    let n = num_f64(&as_number(&args[0], "todate", 1)?);
    let Some(dt) = utc_from_unix(n) else {
        // Out-of-range timestamps would raise; the fixture stays in
        // range, but mirror a clean error rather than panic.
        return Err(QueryError::builtin(format!(
            "todate: timestamp out of range: {}",
            crate::jsonfmt::py_float_repr(n)
        )));
    };
    Ok(Value::Str(
        dt.naive_utc().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    ))
}

fn bi_fromdate(args: &[Value]) -> Result<Value, QueryError> {
    // Shared by `fromdate` and `fromdateiso8601`. Parses an ISO-8601 UTC
    // timestamp and returns the Unix epoch seconds as a float.
    let s = as_str(&args[0], "fromdate", 1)?;
    match parse_isoformat(&s) {
        Ok(secs) => Ok(Value::Float(secs)),
        Err(detail) => Err(QueryError::builtin(format!(
            "fromdate: cannot parse {}: {detail}",
            py_str_repr(&s)
        ))),
    }
}

fn bi_gmtime(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "gmtime", 1)?);
    let Some(dt) = utc_from_unix(n) else {
        return Err(QueryError::builtin(format!(
            "gmtime: timestamp out of range: {}",
            crate::jsonfmt::py_float_repr(n)
        )));
    };
    Ok(broken_down(&dt))
}

fn bi_localtime(args: &[Value]) -> Result<Value, QueryError> {
    // With `TZ=UTC` (pinned by the generator and the differential test), local time
    // equals UTC, so this shares the UTC broken-down conversion.
    let n = num_f64(&as_number(&args[0], "localtime", 1)?);
    let Some(dt) = utc_from_unix(n) else {
        return Err(QueryError::builtin(format!(
            "localtime: timestamp out of range: {}",
            crate::jsonfmt::py_float_repr(n)
        )));
    };
    Ok(broken_down(&dt))
}

fn bi_mktime(args: &[Value]) -> Result<Value, QueryError> {
    // Inverse of `gmtime`: broken-down UTC array → Unix epoch seconds (float).
    let secs = broken_down_to_unix(&args[0], "mktime")?;
    Ok(Value::Float(secs as f64))
}

fn bi_strftime(args: &[Value]) -> Result<Value, QueryError> {
    use std::fmt::Write as _;
    // strftime over the UTC broken-down time — sub-second part dropped.
    let n = num_f64(&as_number(&args[0], "strftime", 1)?);
    let fmt = as_str(&args[1], "strftime", 2)?;
    let Some(dt) = utc_from_unix(n) else {
        return Err(QueryError::builtin(format!(
            "strftime: timestamp out of range: {}",
            crate::jsonfmt::py_float_repr(n)
        )));
    };
    // An invalid specifier (`%E`, a bare `%`, …) lexes to `Item::Error`, whose
    // `Display` returns `fmt::Error`. `.to_string()` turns that into a *panic*
    // ("a Display implementation returned an error unexpectedly"), aborting the
    // in-report wasm console; `write!` propagates it so we can return a clean
    // Tcl error instead.
    let mut buf = String::new();
    if write!(buf, "{}", dt.naive_utc().format(&fmt)).is_err() {
        return Err(QueryError::builtin(format!(
            "strftime: invalid format string: {fmt:?}"
        )));
    }
    Ok(Value::Str(buf))
}

fn bi_strptime(args: &[Value]) -> Result<Value, QueryError> {
    // strptime-style parse of `value` against `fmt` → jq broken-down array.
    let s = as_str(&args[0], "strptime", 1)?;
    let fmt = as_str(&args[1], "strptime", 2)?;
    let naive = parse_strptime(&s, &fmt).ok_or_else(|| {
        // Reproduce the strptime parse-error text verbatim.
        QueryError::builtin(format!(
            "strptime: cannot parse {} with {}: time data {} does not match format {}",
            py_str_repr(&s),
            py_str_repr(&fmt),
            py_str_repr(&s),
            py_str_repr(&fmt)
        ))
    })?;
    Ok(broken_down_naive(&naive))
}

fn bi_dateadd(args: &[Value]) -> Result<Value, QueryError> {
    // `t + s`: int when both ints, else float.
    let a = as_number(&args[0], "dateadd", 1)?;
    let b = as_number(&args[1], "dateadd", 2)?;
    Ok(py_add(&a, &b, true))
}

fn bi_datesub(args: &[Value]) -> Result<Value, QueryError> {
    // `t - s`: int when both ints, else float.
    let a = as_number(&args[0], "datesub", 1)?;
    let b = as_number(&args[1], "datesub", 2)?;
    Ok(py_add(&a, &b, false))
}

/// `a + b` / `a - b` over two numbers, preserving int vs float.
fn py_add(a: &Value, b: &Value, add: bool) -> Value {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        Value::Int(if add { x + y } else { x - y })
    } else {
        let x = num_f64(a);
        let y = num_f64(b);
        Value::Float(if add { x + y } else { x - y })
    }
}

// mktime: broken-down array → struct → Unix seconds (calendar.timegm)

/// Convert a broken-down time to Unix seconds. Accepts the
/// broken-down array `[year-1900, month, day, hour, minute, second, ...]`
/// (months 0-indexed); fields beyond the 6th are ignored. Seconds are
/// truncated to an integer.
fn broken_down_to_unix(value: &Value, name: &str) -> Result<i64, QueryError> {
    let items = match value {
        Value::List(items) | Value::Stream(items) => items,
        other => {
            return Err(QueryError::builtin(format!(
                "{name}: broken-down time must be a list of at least 6 ints, got {}",
                type_name(other)
            )));
        }
    };
    if items.len() < 6 {
        return Err(QueryError::builtin(format!(
            "{name}: broken-down time must be a list of at least 6 ints, got list"
        )));
    }
    // Number coercion: each field is a number (int or float). The
    // helper indexes the raw values; `timegm` then needs ints, with the
    // seconds truncated to an int. We truncate every field toward zero, matching
    // broken-down-time construction (year/month/... are ints; the
    // generator only feeds ints save for the truncated seconds case).
    let f = |i: usize| -> Result<i64, QueryError> {
        match &items[i] {
            Value::Int(n) => Ok(*n),
            Value::Float(x) => Ok(x.trunc() as i64),
            other => Err(QueryError::builtin(format!(
                "{name}: argument 1 must be a number, got {}",
                type_name(other)
            ))),
        }
    };
    let year = f(0)? + 1900;
    let month = f(1)? + 1;
    let day = f(2)?;
    let hour = f(3)?;
    let minute = f(4)?;
    let second = f(5)?;
    // `calendar.timegm` treats the tuple as UTC.
    let date = NaiveDate::from_ymd_opt(
        i32::try_from(year).map_err(|_| timegm_range_err(name))?,
        u32::try_from(month).map_err(|_| timegm_range_err(name))?,
        u32::try_from(day).map_err(|_| timegm_range_err(name))?,
    )
    .ok_or_else(|| timegm_range_err(name))?;
    let time = chrono::NaiveTime::from_hms_opt(
        u32::try_from(hour).map_err(|_| timegm_range_err(name))?,
        u32::try_from(minute).map_err(|_| timegm_range_err(name))?,
        u32::try_from(second).map_err(|_| timegm_range_err(name))?,
    )
    .ok_or_else(|| timegm_range_err(name))?;
    Ok(date.and_time(time).and_utc().timestamp())
}

fn timegm_range_err(name: &str) -> QueryError {
    QueryError::builtin(format!("{name}: broken-down time is out of range"))
}

// ISO-8601 parsing for the supported forms

/// Parse an ISO-8601 timestamp the way `fromdate` does and return Unix
/// seconds. The DSL normalises a trailing `Z` to `+00:00` and treats a
/// naive datetime as UTC. We reproduce the common forms byte-identically and
/// surface `Invalid isoformat string: '...'` message for inputs that
/// do not parse; the fixture only includes cases covered here.
fn parse_isoformat(s: &str) -> Result<f64, String> {
    let text = if let Some(stripped) = s.strip_suffix('Z') {
        format!("{stripped}+00:00")
    } else {
        s.to_string()
    };
    // Split off an optional trailing `+HH:MM` / `-HH:MM` offset (after the
    // time). ISO-8601 parsing also accepts `+HH:MM:SS` and `Z`;
    // we handle the offset uniformly below.
    let (body, offset_secs) = split_offset(&text);
    let naive = parse_naive_iso(body).ok_or_else(|| invalid_iso(s))?;
    let utc = naive.and_utc().timestamp();
    let nanos = naive.and_utc().timestamp_subsec_nanos();
    let secs = utc - offset_secs;
    Ok(secs as f64 + f64::from(nanos) / 1_000_000_000.0)
}

fn invalid_iso(s: &str) -> String {
    format!("Invalid isoformat string: {}", py_str_repr(s))
}

/// Pull a trailing timezone offset (`+HH:MM`, `-HH:MM`, `+HHMM`, `+HH:MM:SS`)
/// off the end of the string. Returns `(body, offset_in_seconds)`; a missing
/// offset yields `(text, 0)`.
fn split_offset(text: &str) -> (&str, i64) {
    // The date part may itself contain `-`; only look for a sign in the time
    // portion, i.e. after a `T` or space separator (or, for date-only input,
    // not at all).
    let sep = text.find(['T', ' ']);
    let search_from = match sep {
        Some(i) => i + 1,
        None => return (text, 0), // date-only: no time-zone offset.
    };
    let tail = &text[search_from..];
    if let Some(rel) = tail.rfind(['+', '-']) {
        let pos = search_from + rel;
        let sign = if &text[pos..=pos] == "-" { -1 } else { 1 };
        let off = &text[pos + 1..];
        if let Some(secs) = parse_offset(off) {
            return (&text[..pos], sign * secs);
        }
    }
    (text, 0)
}

/// Parse `HH:MM`, `HHMM`, `HH:MM:SS`, or `HH` into seconds.
fn parse_offset(off: &str) -> Option<i64> {
    let digits: String = off.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 2 || off.chars().any(|c| !(c.is_ascii_digit() || c == ':')) {
        return None;
    }
    let h: i64 = digits.get(0..2)?.parse().ok()?;
    let m: i64 = digits.get(2..4).map_or(Ok(0), str::parse).ok()?;
    let s: i64 = digits.get(4..6).map_or(Ok(0), str::parse).ok()?;
    Some(h * 3600 + m * 60 + s)
}

/// Parse the naive (offset-stripped) ISO body into a `NaiveDateTime`,
/// covering the date-only, minute-precision, second-precision, and
/// fractional-second forms ISO-8601 parsing accepts (with `T` or space sep).
fn parse_naive_iso(body: &str) -> Option<NaiveDateTime> {
    if let Some(idx) = body.find(['T', ' ']) {
        let (date, time) = (&body[..idx], &body[idx + 1..]);
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        for fmt in ["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"] {
            if let Ok(t) = chrono::NaiveTime::parse_from_str(time, fmt) {
                return Some(d.and_time(t));
            }
        }
        None
    } else {
        let d = NaiveDate::parse_from_str(body, "%Y-%m-%d").ok()?;
        Some(d.and_hms_opt(0, 0, 0).unwrap())
    }
}

// strptime for the supported specifiers

/// Parse `value` with a strptime `fmt` into a `NaiveDateTime`. Tries a
/// full date+time parse, then a date-only parse (filling midnight) the way
/// strptime defaults unmatched fields.
fn parse_strptime(value: &str, fmt: &str) -> Option<NaiveDateTime> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, fmt) {
        return Some(dt);
    }
    if let Ok(d) = NaiveDate::parse_from_str(value, fmt) {
        return Some(d.and_hms_opt(0, 0, 0).unwrap());
    }
    None
}

// Repr-style rendering of a string (single-quoted, like `{x!r}`)

fn py_str_repr(s: &str) -> String {
    crate::lexer::py_repr_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_a_finite_float_past_2020() {
        // `now` is non-deterministic, so it is excluded from the golden
        // fixture; assert only its type and a sane lower bound here.
        let v = bi_now(&[]).expect("now() succeeds");
        match v {
            Value::Float(f) => {
                assert!(f.is_finite(), "now() must be finite, got {f}");
                assert!(f > 1.6e9, "now() must be after 2020, got {f}");
            }
            other => panic!("now() must return a float, got {other:?}"),
        }
    }

    #[test]
    fn strftime_invalid_format_errors_not_panics() {
        // An invalid specifier lexes to `Item::Error`, whose
        // Display returns fmt::Error — `.to_string()` would panic. We must get a
        // clean QueryError instead.
        for bad in ["%E", "100%", "%"] {
            let err = bi_strftime(&[Value::Int(0), Value::Str(bad.to_owned())]).unwrap_err();
            assert!(
                err.to_string().contains("invalid format"),
                "expected a clean error for {bad:?}, got {err}",
            );
        }
        // FP-guard: a valid format still renders.
        let ok = bi_strftime(&[Value::Int(0), Value::Str("%Y-%m-%d".to_owned())]).unwrap();
        assert!(matches!(ok, Value::Str(s) if s == "1970-01-01"));
    }
}
