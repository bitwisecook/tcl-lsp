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

//! The capability/policy facet of the profile-based top layer: top-level
//! `allow`/`deny` declarations sandbox a program's operations. The check runs
//! over the *expanded* handler body, so a template or profile field cannot
//! smuggle a forbidden operation past the policy.

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

fn verdict(src: &str, len: usize) -> u64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    let mut pkt = vec![0u8; len];
    run_socket_filter(&obj, &mut pkt).expect("runs")
}

// ---- deny: default-allow, forbids the named verbs ------------------------

#[test]
fn deny_unused_verb_compiles() {
    // `map_set` is denied but never used — the program is fine.
    let src = "deny map_set\n\
        when SOCKET_FILTER {\n\
            setbuf p ctx\n\
            pktlen len p\n\
            accept {$len}\n\
        }\n";
    assert_eq!(verdict(src, 24), 24);
}

#[test]
fn deny_used_verb_rejected() {
    let src = "deny map_set\n\
        when SOCKET_FILTER {\n\
            map ctr hash 8 8 16\n\
            map_get n ctr {0}\n\
            map_set ctr {0} {$n}\n\
            accept\n\
        }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

#[test]
fn deny_can_forbid_a_verdict() {
    // A profile that may inspect but never drop.
    let src = "deny drop\n\
        when SOCKET_FILTER { drop }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

// ---- allow: restricts the gated set, verdicts/scalars stay free ----------

#[test]
fn allow_lists_the_only_gated_verbs() {
    // Only `map_get` is allowed; `map_set` (a gated verb) is therefore denied.
    let src = "allow map_get\n\
        when SOCKET_FILTER {\n\
            map m hash 8 8 16\n\
            map_get n m {0}\n\
            map_set m {0} {$n}\n\
            accept\n\
        }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

#[test]
fn allow_permits_listed_verbs_and_verdicts() {
    // `load16`/`map_get` listed; `map` decl + `setbuf`/`setint` are structural;
    // `accept` is a verdict — all permitted.
    let src = "allow map_get load16\n\
        when SOCKET_FILTER {\n\
            setbuf p ctx\n\
            load16 x p 0\n\
            map m hash 8 8 16\n\
            map_get n m {0}\n\
            accept {$x}\n\
        }\n";
    // load16 at offset 0 of a zeroed packet → 0 → accept 0.
    assert_eq!(verdict(src, 16), 0);
}

// ---- the policy governs *expanded* operations ----------------------------

#[test]
fn template_cannot_smuggle_a_denied_verb() {
    // The `use` splices in `map_set`, which the post-expansion check catches.
    let src = "deny map_set\n\
        template sneaky {} {\n\
            map m2 hash 8 8 8\n\
            map_set m2 {0} {1}\n\
        }\n\
        when SOCKET_FILTER { use sneaky\n accept }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

#[test]
fn profile_field_read_is_governed_as_a_load() {
    // `tcp_dport` expands to `load16`; denying loads forbids the field read.
    let src = "profile ipv4_tcp\n\
        deny load16\n\
        when XDP {\n\
            tcp_dport d\n\
            if {$d == 22} { drop }\n\
            pass\n\
        }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

// ---- malformed declarations ----------------------------------------------

#[test]
fn deny_unknown_verb_rejected() {
    let err = compile_module("deny frobnicate\nwhen SOCKET_FILTER { accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::CapabilityDenied);
}

#[test]
fn no_policy_is_unrestricted() {
    // No allow/deny — everything still works (the policy is a no-op).
    let src = "when SOCKET_FILTER {\n\
        map m hash 8 8 16\n\
        map_set m {0} {7}\n\
        map_get v m {0}\n\
        accept {$v}\n\
    }\n";
    assert_eq!(verdict(src, 8), 7);
}
