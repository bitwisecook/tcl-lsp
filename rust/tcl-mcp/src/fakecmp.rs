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

//! fakeCMP TMM-distribution helpers.
//!
//! Pure integer arithmetic — a modified FNV-1a masked to 31 bits — that maps a
//! connection 4-tuple to a TMM id. Kept byte-for-byte in sync with the Tcl
//! `_fakecmp_hash` in `rust/tcl-irule-test/tcl/orchestrator.tcl`, so this hash
//! and the runtime orchestrator distribute connections identically (agreement
//! is tested).

use serde_json::{Map, Value, json};

/// The fakeCMP hash: FNV-1a over the 4-tuple octets/ports, masked to 31 bits,
/// modulo `tmm_count`. `src`/`dst` are dotted IPv4 strings (already validated).
fn fakecmp_hash(src: &str, src_port: u32, dst: &str, dst_port: u32, tmm_count: u64) -> u64 {
    let mut h: u64 = 0x811C_9DC5;
    let mut step = |byte: u64| h = ((h ^ byte) * 0x0100_0193) & 0x7FFF_FFFF;
    for octet in src.split('.') {
        step(octet.parse::<u64>().unwrap_or(0));
    }
    step(u64::from(src_port & 0xFF));
    step(u64::from((src_port >> 8) & 0xFF));
    for octet in dst.split('.') {
        step(octet.parse::<u64>().unwrap_or(0));
    }
    step(u64::from(dst_port & 0xFF));
    step(u64::from((dst_port >> 8) & 0xFF));
    h % tmm_count
}

/// `Some(error-dict)` when `addr` is not a dotted-quad IPv4 in range.
fn validate_ipv4(addr: &str, field: &str) -> Option<Value> {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() != 4 {
        return Some(field_error(
            field,
            &format!("expected IPv4 (a.b.c.d), got '{addr}'"),
            addr,
        ));
    }
    for p in parts {
        match p.parse::<i64>() {
            Err(_) => {
                return Some(field_error(
                    field,
                    &format!("non-integer octet '{p}'"),
                    addr,
                ));
            }
            Ok(v) if !(0..=255).contains(&v) => {
                return Some(field_error(
                    field,
                    &format!("octet {v} out of range (0-255)"),
                    addr,
                ));
            }
            Ok(_) => {}
        }
    }
    None
}

/// `Some(error-dict)` when `port` is outside `0..=65535`.
fn validate_port(port: i64, field: &str) -> Option<Value> {
    if (0..=65535).contains(&port) {
        None
    } else {
        Some(field_error(field, "must be 0-65535", &port.to_string()))
    }
}

fn field_error(field: &str, message: &str, value: &str) -> Value {
    json!({ "field": field, "message": message, "value": value })
}

/// `{error, fields}` — the fakeCMP validation-failure shape.
fn validation_error(field_errors: Vec<Value>) -> Value {
    let summary = field_errors
        .iter()
        .map(|e| {
            format!(
                "{}: {}",
                e.get("field").and_then(Value::as_str).unwrap_or(""),
                e.get("message").and_then(Value::as_str).unwrap_or(""),
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    json!({ "error": summary, "fields": Value::Array(field_errors) })
}

/// Integer arg, or `None` when absent / not an integer.
fn arg_int(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(Value::as_i64)
}

fn arg_str_or<'a>(args: &'a Value, key: &str, default: &'a str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or(default)
}

/// Raw display value for an argument (for error `value` fields).
fn arg_display(args: &Value, key: &str) -> String {
    match args.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => "None".to_owned(),
    }
}

/// `fakecmp_which_tmm` — which TMM a specific 4-tuple hashes to.
pub fn which_tmm(args: &Value) -> Value {
    let tmm_count = arg_int(args, "tmm_count");
    let src_addr = arg_str_or(args, "src_addr", "");
    let dst_addr = arg_str_or(args, "dst_addr", "");
    let src_port = arg_int(args, "src_port");
    let dst_port = arg_int(args, "dst_port");

    let mut errors = Vec::new();
    if tmm_count.is_none_or(|c| c < 2) {
        errors.push(field_error(
            "tmm_count",
            "must be >= 2",
            &arg_display(args, "tmm_count"),
        ));
    }
    if let Some(e) = validate_ipv4(src_addr, "src_addr") {
        errors.push(e);
    }
    if let Some(e) = validate_port(src_port.unwrap_or(-1), "src_port") {
        errors.push(e);
    }
    if let Some(e) = validate_ipv4(dst_addr, "dst_addr") {
        errors.push(e);
    }
    if let Some(e) = validate_port(dst_port.unwrap_or(-1), "dst_port") {
        errors.push(e);
    }
    if !errors.is_empty() {
        return validation_error(errors);
    }

    let (tmm_count, src_port, dst_port) =
        (tmm_count.unwrap(), src_port.unwrap(), dst_port.unwrap());
    // All three were validated in range above, so the conversions can't fail.
    let tmm_id = fakecmp_hash(
        src_addr,
        u32::try_from(src_port).unwrap_or(0),
        dst_addr,
        u32::try_from(dst_port).unwrap_or(0),
        u64::try_from(tmm_count).unwrap_or(2),
    );
    json!({
        "tmm_id": tmm_id,
        "tmm_count": tmm_count,
        "tuple": format!("{src_addr}:{src_port} -> {dst_addr}:{dst_port}"),
    })
}

/// Upper bound on `suggest_sources`' `tmm_count`: one `Vec` bucket is allocated
/// per TMM id, so an unbounded value (a client sending e.g. `1_000_000_000_000`)
/// would try to allocate terabytes and abort the whole MCP server. The
/// candidate-tuple ceiling — 254 source octets × 254 source ports — is the most
/// buckets that can ever be filled anyway (issue 199).
const MAX_TMM_COUNT: i64 = 254 * 254;

/// `fakecmp_suggest_sources` — source tuples that spread across every TMM.
pub fn suggest_sources(args: &Value) -> Value {
    let tmm_count = arg_int(args, "tmm_count");
    let count = args.get("count").and_then(Value::as_i64).unwrap_or(1);
    let dst_addr = arg_str_or(args, "dst_addr", "192.168.1.100");
    let dst_port = args.get("dst_port").and_then(Value::as_i64).unwrap_or(443);

    let mut errors = Vec::new();
    if tmm_count.is_none_or(|c| !(2..=MAX_TMM_COUNT).contains(&c)) {
        errors.push(field_error(
            "tmm_count",
            &format!("must be between 2 and {MAX_TMM_COUNT}"),
            &arg_display(args, "tmm_count"),
        ));
    }
    if count < 1 {
        errors.push(field_error("count", "must be >= 1", &count.to_string()));
    }
    if let Some(e) = validate_ipv4(dst_addr, "dst_addr") {
        errors.push(e);
    }
    if let Some(e) = validate_port(dst_port, "dst_port") {
        errors.push(e);
    }
    if !errors.is_empty() {
        return validation_error(errors);
    }

    let tmm_count = tmm_count.unwrap();
    // Validated (`tmm_count >= 2`, `count >= 1`, port in range) so no cast fails.
    let tmm_u = u64::try_from(tmm_count).unwrap_or(2);
    let count_u = u64::try_from(count).unwrap_or(1);
    let dst_port_u = u32::try_from(dst_port).unwrap_or(0);

    // One bucket per TMM id, filled to `count` entries each.
    let mut plan: Vec<Vec<Value>> = vec![Vec::new(); usize::try_from(tmm_u).unwrap_or(0)];
    let need = tmm_u * count_u;
    let mut found: u64 = 0;
    'outer: for octet in 1..255u32 {
        for port in 10001..10255u32 {
            if found >= need {
                break 'outer;
            }
            let addr = format!("10.0.0.{octet}");
            let tmm = usize::try_from(fakecmp_hash(&addr, port, dst_addr, dst_port_u, tmm_u))
                .unwrap_or(0);
            if u64::try_from(plan[tmm].len()).unwrap_or(0) < count_u {
                plan[tmm].push(json!({ "addr": addr, "port": port }));
                found += 1;
            }
        }
    }

    // Integer TMM ids as (string) object keys, in 0..tmm_count order.
    let mut plan_obj = Map::new();
    for (id, bucket) in plan.into_iter().enumerate() {
        plan_obj.insert(id.to_string(), Value::Array(bucket));
    }
    json!({
        "tmm_count": tmm_count,
        "plan": Value::Object(plan_obj),
        "usage_hint": "In your test, iterate over the plan and set \
             ::orch::configure -client_addr $addr -client_port $port \
             before each ::orch::run_http_request call.",
    })
}

#[cfg(test)]
mod suggest_sources_tests {
    use super::suggest_sources;
    use serde_json::json;

    #[test]
    fn pathological_tmm_count_is_rejected_not_allocated() {
        // A huge tmm_count must produce a validation error rather than
        // attempting a terabyte per-TMM bucket allocation that OOM-aborts the
        // server (issue 199).
        let out = suggest_sources(&json!({ "tmm_count": 1_000_000_000_000i64, "count": 1 }));
        let s = out.to_string();
        assert!(s.contains("tmm_count"), "expected a tmm_count error: {s}");
        assert!(
            s.contains("between 2 and"),
            "expected the range message: {s}"
        );
        // It must NOT have built a plan.
        assert!(!s.contains("\"plan\""), "no plan should be produced: {s}");
    }

    #[test]
    fn ordinary_tmm_count_produces_a_plan() {
        let out = suggest_sources(&json!({ "tmm_count": 4, "count": 1 }));
        assert!(
            out.get("plan").is_some(),
            "a valid request yields a plan: {out}"
        );
    }
}
