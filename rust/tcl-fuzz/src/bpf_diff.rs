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

//! eBPF value-differential arm.
//!
//! BPF-Tcl is a distinct Tcl dialect, so C Tcl is not its execution oracle.
//! This arm uses the registry's typed BPF descriptors to construct a bounded
//! socket-filter program, evaluates its small declared contract directly, and
//! compares that result with the *real* BPF-Tcl lowering, eBPF emitter, and
//! userspace eBPF VM.  Command spellings and the event name are resolved from
//! registry descriptors; changing a spelling in a spec therefore changes the
//! generated program without a fuzzer code edit.

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::compile_module;
use tcl_registry::CommandRegistry;
use tcl_registry::bpf_op::{
    BPF_EVENTS, BpfDeclKind, BpfEventCaps, BpfEventProgType, BpfOpKind, BpfVerdictKind,
};

/// One bounded packet-read case. `offset` always lies in [`Self::packet`], so
/// a discrepancy cannot be explained by an intentional out-of-bounds action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    /// The byte the program reads from the packet.
    pub offset: u8,
    /// The selected byte's value, and therefore the expected socket-filter
    /// verdict. Zero intentionally means drop.
    pub selected_byte: u8,
}

impl Case {
    /// A deterministic, in-bounds case from a campaign seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        // Keep the packet small but leave enough address variation to exercise
        // the emitter's immediate-offset encoding and bounds proof.
        Self {
            offset: (seed % 48) as u8,
            selected_byte: (seed >> 8) as u8,
        }
    }

    /// The reference packet. Its only semantic fact is the one this generated
    /// program observes: `packet[offset] == selected_byte`.
    #[must_use]
    pub fn packet(self) -> Vec<u8> {
        let mut packet = vec![0xA5; usize::from(self.offset) + 1];
        packet[usize::from(self.offset)] = self.selected_byte;
        packet
    }

    /// The dialect contract for this restricted generated shape.
    #[must_use]
    pub const fn expected_verdict(self) -> u64 {
        self.selected_byte as u64
    }
}

/// Resolve one BPF command spelling from its typed registry operation.
fn command(registry: &CommandRegistry, kind: BpfOpKind) -> Result<&'static str, String> {
    registry
        .bpf_command_for(kind)
        .ok_or_else(|| format!("registry has no BPF command for {kind:?}"))
}

/// Return the registry event that carries the socket-filter contract this arm
/// can execute in userspace. This chooses by program type, capability, and
/// verdict algebra — never by an event string.
fn socket_filter_event() -> Result<&'static str, String> {
    BPF_EVENTS
        .iter()
        .find(|event| {
            event.prog_type == BpfEventProgType::SocketFilter
                && event.is_codegen_ready()
                && event.capabilities.contains(BpfEventCaps::PKT_READ)
                && event.verdicts.contains(&BpfVerdictKind::Accept)
        })
        .map(|event| event.name)
        .ok_or_else(|| {
            "registry has no codegen-ready packet-reading socket-filter event".to_owned()
        })
}

/// Generate a valid BPF-Tcl program solely from typed registry descriptors.
///
/// The emitted program reads one byte and returns that byte through `accept`.
/// For `BPF_PROG_TYPE_SOCKET_FILTER`, the registry documents that return value
/// as the number of bytes accepted; a zero value consequently represents the
/// documented drop outcome.
pub fn source(case: Case) -> Result<String, String> {
    let mut registry = CommandRegistry::build_default();
    registry.load_bpf();
    let when = command(&registry, BpfOpKind::Framework(BpfDeclKind::When))?;
    let bind_packet = command(&registry, BpfOpKind::BindPacket)?;
    let load_byte = command(&registry, BpfOpKind::PacketLoad { width_bits: 8 })?;
    let accept = command(&registry, BpfOpKind::Verdict(BpfVerdictKind::Accept))?;
    let event = socket_filter_event()?;
    Ok(format!(
        "{when} {event} {{\n  {bind_packet} packet ctx\n  {load_byte} byte packet {}\n  {accept} {{$byte}}\n}}\n",
        case.offset
    ))
}

/// Execute one generated program through the real lowering and eBPF backend.
/// Returns `(actual, expected)` so callers can preserve the full mismatch.
pub fn check(case: Case) -> Result<(u64, u64), String> {
    let source = source(case)?;
    let module = compile_module(&source).map_err(|error| format!("BPF-Tcl lowering: {error}"))?;
    let program = module
        .programs
        .first()
        .ok_or_else(|| "BPF-Tcl lowering returned no program".to_owned())?;
    let object =
        emit_program(&program.program).map_err(|error| format!("eBPF emission: {error}"))?;
    let mut packet = case.packet();
    let actual =
        run_socket_filter(&object, &mut packet).map_err(|error| format!("eBPF run: {error}"))?;
    Ok((actual, case.expected_verdict()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tp_real_ebpf_program_returns_the_selected_packet_byte() {
        // TP: a non-zero packet byte is accepted as exactly that byte count.
        let case = Case {
            offset: 17,
            selected_byte: 91,
        };
        assert_eq!(check(case).expect("compiled BPF program"), (91, 91));
    }

    #[test]
    fn tn_zero_selected_byte_is_the_socket_filter_drop_verdict() {
        // TN: zero is the documented socket-filter drop value, not a runner
        // error or an implicit accept.
        let case = Case {
            offset: 0,
            selected_byte: 0,
        };
        assert_eq!(check(case).expect("compiled BPF program"), (0, 0));
    }

    #[test]
    fn fn_registry_spelling_changes_are_not_hidden_in_the_generator() {
        // FN prevention: verify source construction is possible without a
        // command spelling encoded in this fuzzer module.
        let generated = source(Case::from_seed(7)).expect("registry shape");
        assert!(generated.starts_with("when "));
        assert!(generated.contains("SOCKET_FILTER"));
    }
}
