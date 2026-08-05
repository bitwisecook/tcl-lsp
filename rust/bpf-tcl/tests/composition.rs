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

//! Handler composition, executed end to end (issue #1204).
//!
//! Two `when XDP` handlers compose deterministically: an early "deny" handler
//! terminates on a blocked port, while an "audit" handler continues with `next`
//! and lets a later handler decide. Each handler is compiled independently and
//! run under the `rbpf` harness; the per-handler outcomes feed the pure
//! composition model ([`bpf_tcl_ir::compose`]) — the verified-program-chain
//! deployment semantics, run in userspace without a kernel.

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{ChainResult, Outcome, compile_module, compose, event_chains};

/// XDP action numbers (as the codegen emits them).
const XDP_DROP: i64 = 1;
const XDP_PASS: i64 = 2;

/// Build a 40-byte frame whose 16-bit big-endian field at offset 36 is `dport`.
fn frame_with_dport(dport: u16) -> Vec<u8> {
    let mut p = vec![0u8; 40];
    p[36] = (dport >> 8) as u8;
    p[37] = (dport & 0xff) as u8;
    p
}

/// Run every handler of the (single-event) module over `packet` in chain order,
/// classify each outcome, and compose them into a final decision.
fn run_chain(src: &str, packet: &[u8]) -> ChainResult {
    let module = compile_module(src).expect("compiles");
    let chains = event_chains(&module);
    assert_eq!(chains.len(), 1, "test fixtures use a single event");
    let mut outcomes = Vec::new();
    for &idx in &chains[0].handler_indices {
        let obj = emit_program(&module.programs[idx].program).expect("emits");
        let mut pkt = packet.to_vec();
        let r0 = run_socket_filter(&obj, &mut pkt).expect("runs");
        outcomes.push(Outcome::classify(r0));
    }
    compose(&outcomes)
}

/// The composed chain: handler 0 (priority 10) denies port 22 and otherwise
/// continues; handler 1 (priority 20) passes everything.
const DENY_THEN_AUDIT: &str = "\
when XDP priority 10 {
  setbuf p ctx
  pktlen n p
  if {$n < 38} { next }
  load16 dport p 36 be
  if {$dport == 22} { drop }
  next
}
when XDP priority 20 {
  setbuf p ctx
  pass
}
";

#[test]
fn early_deny_handler_terminates_the_chain() {
    // Port 22 is blocked: the priority-10 handler drops and the chain stops
    // there — the audit handler never decides.
    let result = run_chain(DENY_THEN_AUDIT, &frame_with_dport(22));
    assert_eq!(
        result,
        ChainResult::Decided {
            index: 0,
            verdict: XDP_DROP
        }
    );
}

#[test]
fn continuation_reaches_the_later_handler() {
    // Port 80 is allowed: the priority-10 handler yields `next`, so the
    // priority-20 audit handler decides (pass).
    let result = run_chain(DENY_THEN_AUDIT, &frame_with_dport(80));
    assert_eq!(
        result,
        ChainResult::Decided {
            index: 1,
            verdict: XDP_PASS
        }
    );
}

#[test]
fn all_next_falls_through_to_the_event_default() {
    // Both handlers continue: composition yields Default, and the event schema
    // says XDP defaults to pass.
    let src = "\
when XDP priority 10 { next }
when XDP priority 20 { next }
";
    let result = run_chain(src, &frame_with_dport(80));
    assert_eq!(result, ChainResult::Default);

    let default = bpf_tcl_ir::event::event_spec("XDP")
        .unwrap()
        .default_verdict;
    assert_eq!(default.verb(), "pass");
}

#[test]
fn composition_order_is_priority_not_source_order() {
    // Declared audit-first in source, but priority puts deny first; a blocked
    // port must still be denied by the lower-priority handler.
    let src = "\
when XDP priority 20 {
  setbuf p ctx
  pass
}
when XDP priority 10 {
  setbuf p ctx
  pktlen n p
  if {$n < 38} { next }
  load16 dport p 36 be
  if {$dport == 22} { drop }
  next
}
";
    let result = run_chain(src, &frame_with_dport(22));
    assert_eq!(
        result,
        ChainResult::Decided {
            index: 0,
            verdict: XDP_DROP
        }
    );
}
