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

//! Typed BPF-Tcl lowering descriptors and event schemas.
//!
//! Every BPF-Tcl command spec carries a [`BpfOpSpec`] describing *what the
//! command is* to the BPF-Tcl compiler: which typed core operation it lowers
//! to, what effects it has (packet read, map read/write, termination), and
//! which program types its verdicts are compatible with.  The BPF-Tcl
//! front-end (`bpf-tcl-ir`) dispatches on this descriptor — never on the
//! command name — so the registry, lowering, capability policy, and generated
//! documentation cannot drift (issue #1202).
//!
//! The same module carries the [`BpfEventSpec`] table: the BPF-native event
//! space `when <EVENT> …` resolves against (issue #1204's registry-described
//! events).  Each event pairs a name and aliases with its program type, ELF
//! section convention, verdict set, and default verdict.

/// Effect classification for a BPF-Tcl operation.  Consumed by the
/// capability (`allow`/`deny`) policy: an operation with any effect bit set
/// is *gated*; a terminating operation is a *verdict*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfEffects(u8);

impl BpfEffects {
    /// No effects — a structural operation (`setint`, `setbuf`, `map` decl).
    pub const NONE: Self = Self(0);
    /// Reads packet bytes or packet length.
    pub const PKT_READ: Self = Self(1);
    /// Reads a map value.
    pub const MAP_READ: Self = Self(1 << 1);
    /// Writes a map value.
    pub const MAP_WRITE: Self = Self(1 << 2);
    /// Terminates the program with a verdict.
    pub const TERMINATES: Self = Self(1 << 3);

    /// The union of two effect sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every bit of `other` is present in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether any bit of `other` is present in `self`.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether this operation touches the packet or a map — the set the
    /// `allow` capability list restricts.
    #[must_use]
    pub const fn is_gated(self) -> bool {
        self.intersects(Self(
            Self::PKT_READ.0 | Self::MAP_READ.0 | Self::MAP_WRITE.0,
        ))
    }

    /// Whether this operation is a verdict (terminates the program).
    #[must_use]
    pub const fn is_verdict(self) -> bool {
        self.contains(Self::TERMINATES)
    }
}

/// The scalar width a `set*` verb commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfScalarWidth {
    /// Full 64-bit value (`setint`).
    I64,
    /// Sign-extended low 32 bits (`seti32`).
    I32SignExtended,
    /// Zero-extended low 32 bits (`setu32`).
    U32ZeroExtended,
}

/// A verdict family member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfVerdictKind {
    /// Socket filter `accept ?N?` — accept N bytes (defaults to whole packet).
    Accept,
    /// `drop` — 0 for a socket filter, `XDP_DROP` for XDP.
    Drop,
    /// XDP `pass` — `XDP_PASS`.
    Pass,
    /// XDP `tx` — `XDP_TX`.
    Tx,
}

/// A framework declaration kind — a statement consumed by the front-end
/// *before* core lowering (it never reaches BPF-IR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfDeclKind {
    /// `when EVENT ?priority N? { body }` — an event handler.
    When,
    /// `profile NAME ?{ field … }?` — the packet-layout profile.
    Profile,
    /// `field NAME OFFSET WIDTHBITS ?ORDER?` — a profile field (only valid
    /// inside a `profile` body).
    Field,
    /// `template NAME {params} {body}` — a parameterised macro.
    Template,
    /// `use NAME k=v …` — a template expansion site.
    Use,
    /// `allow CMD …` — capability allowlist.
    Allow,
    /// `deny CMD …` — capability denylist.
    Deny,
    /// `attach KIND TARGET` — deployment metadata.
    Attach,
}

/// Which program types an operation is valid in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgTypeSet {
    /// Valid in every program type.
    All,
    /// Socket filters only (`accept`).
    SocketFilterOnly,
    /// XDP only (`pass`, `tx`).
    XdpOnly,
}

/// What a BPF-Tcl command *is* — the typed core operation or framework
/// declaration it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfOpKind {
    /// `setint`/`seti32`/`setu32 NAME {EXPR}` — evaluate and commit a scalar.
    ScalarSet(BpfScalarWidth),
    /// `setbuf NAME ctx` — bind the packet buffer.
    BindPacket,
    /// `loadN DST SRC OFFSET ?be|le|native?` — fixed-offset packet load.
    PacketLoad {
        /// Load width in bits (8, 16, or 32).
        width_bits: u8,
    },
    /// `pktlen DST SRC` — the packet length.
    PacketLen,
    /// `map NAME hash|array KEYSZ VALSZ MAX ?shared|percpu?` — declaration.
    MapDeclare,
    /// `map_get DST NAME {KEY}` — value for key (0 when absent).
    MapGet,
    /// `map_set NAME {KEY} {VAL}` — store a value.
    MapSet,
    /// `map_has DST NAME {KEY}` — 1 when the key is present, else 0
    /// (distinguishes a missing key from a stored zero).
    MapHas,
    /// A terminating verdict.
    Verdict(BpfVerdictKind),
    /// `loop N VAR { body }` — bounded loop, unrolled before CFG construction.
    LoopMacro,
    /// A framework declaration consumed before core lowering.
    Framework(BpfDeclKind),
}

/// The registry-owned lowering contract for one BPF-Tcl command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfOpSpec {
    /// The operation this command lowers to.
    pub kind: BpfOpKind,
    /// Effect classification (drives the capability policy).
    pub effects: BpfEffects,
    /// Program types the operation is valid in.
    pub prog_types: BpfProgTypeSet,
}

impl BpfOpSpec {
    /// A structural core operation: no effects, valid everywhere.
    #[must_use]
    pub const fn structural(kind: BpfOpKind) -> Self {
        Self {
            kind,
            effects: BpfEffects::NONE,
            prog_types: BpfProgTypeSet::All,
        }
    }

    /// A gated core operation with the given effects, valid everywhere.
    #[must_use]
    pub const fn gated(kind: BpfOpKind, effects: BpfEffects) -> Self {
        Self {
            kind,
            effects,
            prog_types: BpfProgTypeSet::All,
        }
    }

    /// A verdict for the given program-type set.
    #[must_use]
    pub const fn verdict(kind: BpfVerdictKind, prog_types: BpfProgTypeSet) -> Self {
        Self {
            kind: BpfOpKind::Verdict(kind),
            effects: BpfEffects::TERMINATES,
            prog_types,
        }
    }

    /// A framework declaration.
    #[must_use]
    pub const fn framework(decl: BpfDeclKind) -> Self {
        Self {
            kind: BpfOpKind::Framework(decl),
            effects: BpfEffects::NONE,
            prog_types: BpfProgTypeSet::All,
        }
    }
}

/// The eBPF program type an event maps to. The registry keeps its own small
/// enum so descriptor data stays dependency-free; `bpf-tcl-ir` maps it onto
/// its IR `ProgType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfEventProgType {
    /// `BPF_PROG_TYPE_SOCKET_FILTER`.
    SocketFilter,
    /// `BPF_PROG_TYPE_XDP`.
    Xdp,
}

/// A registry-described BPF event: the contract behind `when <EVENT> …`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfEventSpec {
    /// Canonical event name (as documented).
    pub name: &'static str,
    /// Accepted aliases (case-insensitive, like the canonical name).
    pub aliases: &'static [&'static str],
    /// The program type handlers of this event compile to.
    pub prog_type: BpfEventProgType,
    /// The libbpf `SEC(...)` section convention for emitted ELF objects.
    pub elf_section: &'static str,
    /// Verdicts a handler of this event may return.
    pub verdicts: &'static [BpfVerdictKind],
    /// One-line description for help/diagnostics.
    pub description: &'static str,
}

/// Every BPF event the framework recognises, in documentation order.
pub const BPF_EVENTS: &[BpfEventSpec] = &[
    BpfEventSpec {
        name: "SOCKET_FILTER",
        aliases: &["SOCKET"],
        prog_type: BpfEventProgType::SocketFilter,
        elf_section: "socket",
        verdicts: &[BpfVerdictKind::Accept, BpfVerdictKind::Drop],
        description: "socket filter: verdict is the number of bytes to accept (0 drops)",
    },
    BpfEventSpec {
        name: "XDP",
        aliases: &[],
        prog_type: BpfEventProgType::Xdp,
        elf_section: "xdp",
        verdicts: &[
            BpfVerdictKind::Pass,
            BpfVerdictKind::Drop,
            BpfVerdictKind::Tx,
        ],
        description: "XDP ingress: verdict is an XDP action (PASS/DROP/TX)",
    },
];

/// Resolve an event name or alias (case-insensitive) to its spec.
#[must_use]
pub fn lookup_bpf_event(name: &str) -> Option<&'static BpfEventSpec> {
    let upper = name.to_ascii_uppercase();
    BPF_EVENTS.iter().find(|e| {
        e.name == upper
            || e.aliases
                .iter()
                .any(|a| a.eq_ignore_ascii_case(upper.as_str()))
    })
}

/// The canonical event names, for diagnostics and help text.
#[must_use]
pub fn bpf_event_names() -> Vec<&'static str> {
    BPF_EVENTS.iter().map(|e| e.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_classification() {
        assert!(BpfEffects::PKT_READ.is_gated());
        assert!(BpfEffects::MAP_READ.is_gated());
        assert!(BpfEffects::MAP_WRITE.is_gated());
        assert!(!BpfEffects::TERMINATES.is_gated());
        assert!(BpfEffects::TERMINATES.is_verdict());
        assert!(!BpfEffects::NONE.is_gated());
        assert!(!BpfEffects::NONE.is_verdict());
        assert!(
            BpfEffects::PKT_READ
                .union(BpfEffects::TERMINATES)
                .is_verdict()
        );
    }

    #[test]
    fn event_lookup_accepts_aliases_case_insensitively() {
        assert_eq!(
            lookup_bpf_event("socket").map(|e| e.name),
            Some("SOCKET_FILTER")
        );
        assert_eq!(
            lookup_bpf_event("socket_filter").map(|e| e.name),
            Some("SOCKET_FILTER")
        );
        assert_eq!(lookup_bpf_event("xdp").map(|e| e.name), Some("XDP"));
        assert_eq!(lookup_bpf_event("wat"), None);
    }

    #[test]
    fn every_event_declares_a_drop_verdict_and_a_section() {
        for e in BPF_EVENTS {
            assert!(
                e.verdicts.contains(&BpfVerdictKind::Drop),
                "{} must allow drop",
                e.name
            );
            assert!(!e.elf_section.is_empty());
            assert!(!e.description.is_empty());
        }
    }
}
