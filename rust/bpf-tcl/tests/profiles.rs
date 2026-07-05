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

//! The profile-based top layer (protocol facet): a `profile` selects a bundle
//! of named header fields, and a handler reads a field by name
//! (`tcp_dport dport`) which the front-end expands to `load<width>` at the
//! field's offset (auto-binding an implicit `__pkt` from `ctx`). End-to-end
//! under rbpf: a header-aware XDP filter returns XDP verdicts.

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

fn run(src: &str, packet: &mut [u8]) -> u64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    run_socket_filter(&obj, packet).expect("runs")
}

/// A 60-byte buffer (Ethernet 14 + IPv4 20 + TCP 20, padded) — big enough for
/// every `ipv4_tcp` field offset.
fn frame() -> Vec<u8> {
    vec![0u8; 60]
}

/// Built-in `ipv4_tcp`: drop TCP traffic to dport 22, pass everything else.
/// No explicit `setbuf` — the field expansion binds `__pkt` from `ctx` for us.
const SSH_FILTER: &str = "profile ipv4_tcp\n\
    when XDP {\n\
        ip_proto proto\n\
        if {$proto != 6} { pass }\n\
        tcp_dport dport\n\
        if {$dport == 22} { drop }\n\
        pass\n\
    }\n";

#[test]
fn ipv4_tcp_drops_ssh() {
    let mut pkt = frame();
    pkt[23] = 6; // IPPROTO_TCP
    pkt[36] = 0x16; // dport 22, native-endian
    pkt[37] = 0x00;
    assert_eq!(run(SSH_FILTER, &mut pkt), 1); // XDP_DROP
}

#[test]
fn ipv4_tcp_passes_http() {
    let mut pkt = frame();
    pkt[23] = 6; // IPPROTO_TCP
    pkt[36] = 0x50; // dport 80
    pkt[37] = 0x00;
    assert_eq!(run(SSH_FILTER, &mut pkt), 2); // XDP_PASS
}

#[test]
fn ipv4_tcp_passes_non_tcp() {
    let mut pkt = frame();
    pkt[23] = 17; // IPPROTO_UDP — bails before the tcp_dport load
    assert_eq!(run(SSH_FILTER, &mut pkt), 2); // XDP_PASS
}

/// A user-defined profile declaring its own field (`field NAME OFFSET WIDTH`).
const USER_PROFILE: &str = "profile myproto {\n\
        field http_alt 36 16\n\
    }\n\
    when XDP {\n\
        http_alt port\n\
        if {$port == 8080} { drop }\n\
        pass\n\
    }\n";

#[test]
fn user_profile_field_drops() {
    let mut pkt = frame();
    pkt[36] = 0x90; // 8080 = 0x1f90, native-endian
    pkt[37] = 0x1f;
    assert_eq!(run(USER_PROFILE, &mut pkt), 1); // XDP_DROP
}

#[test]
fn user_profile_field_passes() {
    let mut pkt = frame();
    pkt[36] = 0x50; // 80
    pkt[37] = 0x00;
    assert_eq!(run(USER_PROFILE, &mut pkt), 2); // XDP_PASS
}

#[test]
fn unknown_profile_rejected() {
    let err = compile_module("profile nosuchproto\nwhen XDP { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadProfile);
}

#[test]
fn two_profiles_rejected() {
    let err = compile_module("profile ipv4\nprofile tcp\nwhen XDP { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadProfile);
}

#[test]
fn bad_field_width_rejected() {
    let src = "profile p { field weird 36 12 }\nwhen XDP { pass }\n";
    let err = compile_module(src).unwrap_err();
    assert_eq!(err.code, BpfDiag::BadProfile);
}
