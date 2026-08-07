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
///
/// Every variant except [`BpfVerdictKind::Next`] is **terminal**: it ends the
/// handler and, under handler composition (issue #1204), stops the chain. `Next`
/// is the sole **non-terminal** outcome — an explicit "continue to the next
/// handler" that the composition model recognises (see
/// [`BpfVerdictKind::is_terminal`]).
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
    /// `next` — the explicit, non-terminal continuation outcome. The handler
    /// returns without a decision so the next handler in priority order runs;
    /// if every handler yields `next`, the event's declared default verdict
    /// applies. Valid in every program type.
    Next,
}

impl BpfVerdictKind {
    /// Whether returning this verdict ends handler-chain evaluation. Every
    /// verdict is terminal except [`BpfVerdictKind::Next`] (an explicit
    /// continuation).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Next)
    }

    /// The DSL verb that produces this verdict.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Drop => "drop",
            Self::Pass => "pass",
            Self::Tx => "tx",
            Self::Next => "next",
        }
    }
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

/// Which program types an operation is valid in — a bitset over
/// [`BpfEventProgType`]'s families, since a verdict verb like `pass` is valid
/// on more than one program type (XDP, TC, and the cgroup sock-addr hooks all
/// accept `pass`/`drop`) without being valid on every type (`accept` stays
/// socket-filter-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfProgTypeSet(u8);

impl BpfProgTypeSet {
    const SOCKET_FILTER: u8 = 1;
    const XDP: u8 = 1 << 1;
    const TC: u8 = 1 << 2;
    const CGROUP: u8 = 1 << 3;

    /// Socket filters only (`accept`).
    pub const SOCKET_FILTER_ONLY: Self = Self(Self::SOCKET_FILTER);
    /// XDP only (`tx`).
    pub const XDP_ONLY: Self = Self(Self::XDP);
    /// XDP, TC, and the cgroup sock-addr hooks (`pass`/`drop`) — every
    /// program type with a `pass`-shaped verdict, i.e. every type but the
    /// socket filter (which uses `accept` instead).
    pub const PASS_LIKE: Self = Self(Self::XDP | Self::TC | Self::CGROUP);
    /// Valid in every program type.
    pub const ALL: Self = Self(Self::SOCKET_FILTER | Self::XDP | Self::TC | Self::CGROUP);

    /// Whether `prog_type` is a member of this set.
    #[must_use]
    pub const fn contains(self, prog_type: BpfEventProgType) -> bool {
        let bit = match prog_type {
            BpfEventProgType::SocketFilter => Self::SOCKET_FILTER,
            BpfEventProgType::Xdp => Self::XDP,
            BpfEventProgType::TcIngress | BpfEventProgType::TcEgress => Self::TC,
            BpfEventProgType::CgroupInet4Connect | BpfEventProgType::CgroupInet4Bind => {
                Self::CGROUP
            }
        };
        self.0 & bit == bit
    }
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
            prog_types: BpfProgTypeSet::ALL,
        }
    }

    /// A gated core operation with the given effects, valid everywhere.
    #[must_use]
    pub const fn gated(kind: BpfOpKind, effects: BpfEffects) -> Self {
        Self {
            kind,
            effects,
            prog_types: BpfProgTypeSet::ALL,
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
            prog_types: BpfProgTypeSet::ALL,
        }
    }
}

/// The eBPF program type an event maps to. The registry keeps its own small
/// enum so descriptor data stays dependency-free; `bpf-tcl-ir` maps the
/// codegen-ready variants onto its IR `ProgType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfEventProgType {
    /// `BPF_PROG_TYPE_SOCKET_FILTER`.
    SocketFilter,
    /// `BPF_PROG_TYPE_XDP`.
    Xdp,
    /// `BPF_PROG_TYPE_SCHED_CLS` on the ingress hook (`__sk_buff`).
    TcIngress,
    /// `BPF_PROG_TYPE_SCHED_CLS` on the egress hook (`__sk_buff`).
    TcEgress,
    /// `BPF_PROG_TYPE_CGROUP_SOCK_ADDR`, `connect4` attach (`bpf_sock_addr`).
    CgroupInet4Connect,
    /// `BPF_PROG_TYPE_CGROUP_SOCK_ADDR`, `bind4` attach (`bpf_sock_addr`).
    CgroupInet4Bind,
}

/// How far the compiler backend supports an event today. Every event is fully
/// *described* by its schema; a [`BpfCodegen::Ready`] event additionally lowers
/// to a compiled program. All six current events are `Ready` (issue #1203's
/// XDP/socket-filter targets, issue #1310's TC/cgroup targets); the
/// `Described`-but-not-`Ready` state stays in the model for a future event
/// whose schema lands before its codegen does — the front-end then resolves
/// the schema and reports a precise "described, codegen pending" diagnostic
/// rather than an "unknown event".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfCodegen {
    /// The event lowers to a compiled program on at least one target.
    Ready,
    /// The event is schema-described; its codegen is not yet implemented.
    Described,
}

/// The kernel context object a handler of an event receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfContext {
    /// The C context struct name (`xdp_md`, `__sk_buff`, `bpf_sock_addr`).
    pub struct_name: &'static str,
    /// The typed, fixed-offset fields a handler may read from the context.
    pub fields: &'static [BpfCtxField],
}

/// The byte order a context or packet field is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfFieldOrder {
    /// Host byte order (a scalar the kernel already presents natively).
    Host,
    /// Network byte order (big-endian on the wire; needs a byte swap on LE).
    Network,
}

/// One readable, fixed-layout field of an event's context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfCtxField {
    /// Field name as a handler would refer to it.
    pub name: &'static str,
    /// Byte offset within the context struct.
    pub offset: u16,
    /// Field width in bits (8, 16, 32, or 64).
    pub width_bits: u16,
    /// Byte order the field is interpreted in.
    pub order: BpfFieldOrder,
    /// One-line description.
    pub description: &'static str,
}

/// The helper/access capabilities a handler of an event is permitted to use.
/// A registry-described allowlist: a handler that needs a capability its event
/// does not grant is a schema violation, checkable without naming a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfEventCaps(u16);

impl BpfEventCaps {
    /// No capabilities beyond returning a verdict.
    pub const NONE: Self = Self(0);
    /// May read packet bytes / length.
    pub const PKT_READ: Self = Self(1);
    /// May read map values.
    pub const MAP_READ: Self = Self(1 << 1);
    /// May write map values.
    pub const MAP_WRITE: Self = Self(1 << 2);
    /// May emit records to a userspace ring buffer.
    pub const RINGBUF_OUTPUT: Self = Self(1 << 3);
    /// May read the event's typed context fields.
    pub const CTX_READ: Self = Self(1 << 4);

    /// The union of two capability sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every bit of `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

/// The kind of value an `attach` parameter carries. Drives validation of an
/// event's attachment parameter schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfAttachParamKind {
    /// A network interface name (`eth0`, `lo`) — resolved to an ifindex.
    Interface,
    /// A cgroup v2 directory path (`/sys/fs/cgroup/…`).
    CgroupPath,
    /// A traffic-control direction word (`ingress`/`egress`), fixed by the event.
    Direction,
}

/// One parameter an event's `attach` declaration accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfAttachParam {
    /// Parameter name (documentation/diagnostics).
    pub name: &'static str,
    /// The kind of value it carries.
    pub kind: BpfAttachParamKind,
    /// Whether the parameter is mandatory.
    pub required: bool,
    /// One-line description.
    pub description: &'static str,
}

/// What an event's program produces: a packet decision (a verdict), userspace
/// records (ring-buffer entries), or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfEventOutput {
    /// Returns a packet verdict only.
    PacketDecision,
    /// Emits typed userspace records only (observability events).
    UserspaceRecord,
    /// Both a packet decision and userspace records.
    Both,
}

/// The minimum kernel / BTF requirements to load and attach an event's program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpfKernelReq {
    /// Minimum kernel major version.
    pub min_major: u8,
    /// Minimum kernel minor version.
    pub min_minor: u8,
    /// Whether the event needs kernel BTF to attach (CO-RE / typed links).
    pub requires_btf: bool,
    /// Whether a first-class `bpf_link` attach is available (vs. a
    /// type-specific fallback such as netlink `tc` or `setsockopt`).
    pub bpf_link: bool,
}

/// A registry-described BPF event: the full typed contract behind
/// `when <EVENT> …`. This is the single source of event truth — the compiler,
/// loader, and documentation read it and never branch on an event name.
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
    /// The typed context a handler receives.
    pub context: BpfContext,
    /// Helper/access capabilities a handler is permitted to use.
    pub capabilities: BpfEventCaps,
    /// Terminal verdicts a handler of this event may return (the verdict
    /// algebra). `next` (the non-terminal continuation) is always permitted
    /// and is *not* listed here.
    pub verdicts: &'static [BpfVerdictKind],
    /// The verdict applied when no handler in the chain terminates.
    pub default_verdict: BpfVerdictKind,
    /// The `attach` parameter schema.
    pub attach_params: &'static [BpfAttachParam],
    /// Minimum kernel / BTF requirements.
    pub kernel: BpfKernelReq,
    /// Whether the program emits packet decisions, userspace records, or both.
    pub output: BpfEventOutput,
    /// How far compiler codegen supports this event today.
    pub codegen: BpfCodegen,
    /// One-line description for help/diagnostics.
    pub description: &'static str,
}

impl BpfEventSpec {
    /// Whether this event currently lowers to a compiled program.
    #[must_use]
    pub const fn is_codegen_ready(self) -> bool {
        matches!(self.codegen, BpfCodegen::Ready)
    }

    /// Whether `verb` names a verdict this event permits (terminal verdicts
    /// from the schema, plus the always-available non-terminal `next`).
    #[must_use]
    pub fn permits_verdict(&self, verb: &str) -> bool {
        if verb == BpfVerdictKind::Next.verb() {
            return true;
        }
        self.verdicts.iter().any(|v| v.verb() == verb)
    }

    /// Resolve a named context field.
    #[must_use]
    pub fn context_field(&self, name: &str) -> Option<&'static BpfCtxField> {
        self.context.fields.iter().find(|f| f.name == name)
    }

    /// The permitted-capability names, for `check`/help output.
    #[must_use]
    pub fn capability_names(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (bit, name) in [
            (BpfEventCaps::PKT_READ, "pkt-read"),
            (BpfEventCaps::MAP_READ, "map-read"),
            (BpfEventCaps::MAP_WRITE, "map-write"),
            (BpfEventCaps::RINGBUF_OUTPUT, "ringbuf"),
            (BpfEventCaps::CTX_READ, "ctx-read"),
        ] {
            if self.capabilities.contains(bit) {
                out.push(name);
            }
        }
        out
    }

    /// A stable label for the event's output kind.
    #[must_use]
    pub fn output_label(&self) -> &'static str {
        match self.output {
            BpfEventOutput::PacketDecision => "packet-decision",
            BpfEventOutput::UserspaceRecord => "userspace-record",
            BpfEventOutput::Both => "packet-decision+userspace-record",
        }
    }

    /// A one-line kernel-requirement summary (`>=5.6, BTF, bpf_link`).
    #[must_use]
    pub fn kernel_summary(&self) -> String {
        let mut s = format!(">={}.{}", self.kernel.min_major, self.kernel.min_minor);
        if self.kernel.requires_btf {
            s.push_str(", BTF");
        }
        s.push_str(if self.kernel.bpf_link {
            ", bpf_link"
        } else {
            ", type-specific attach"
        });
        s
    }
}

/// The Ethernet/IPv4/TCP `__sk_buff` metadata fields a socket-filter / TC
/// handler can read directly (a small, safe subset — the packet body is read
/// with `load8/16/32`).
const SK_BUFF_FIELDS: &[BpfCtxField] = &[
    BpfCtxField {
        name: "len",
        offset: 0,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "total packet length in bytes",
    },
    BpfCtxField {
        name: "protocol",
        offset: 16,
        width_bits: 32,
        order: BpfFieldOrder::Network,
        description: "L3 protocol (network byte order)",
    },
    BpfCtxField {
        name: "ifindex",
        offset: 24,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "interface index the packet arrived on / leaves by",
    },
    BpfCtxField {
        name: "mark",
        offset: 20,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "skb mark (readable/settable fwmark)",
    },
];

/// The `xdp_md` metadata fields.
const XDP_MD_FIELDS: &[BpfCtxField] = &[
    BpfCtxField {
        name: "ingress_ifindex",
        offset: 12,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "interface index the frame arrived on",
    },
    BpfCtxField {
        name: "rx_queue_index",
        offset: 16,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "RX queue index",
    },
];

/// The `bpf_sock_addr` fields a cgroup connect/bind hook inspects.
const SOCK_ADDR_FIELDS: &[BpfCtxField] = &[
    BpfCtxField {
        name: "user_ip4",
        offset: 4,
        width_bits: 32,
        order: BpfFieldOrder::Network,
        description: "destination/bind IPv4 address (network byte order)",
    },
    BpfCtxField {
        name: "user_port",
        offset: 24,
        width_bits: 32,
        order: BpfFieldOrder::Network,
        description: "destination/bind port (network byte order, low 16 bits)",
    },
    BpfCtxField {
        name: "family",
        offset: 0,
        width_bits: 32,
        order: BpfFieldOrder::Host,
        description: "address family (AF_INET)",
    },
];

const IFACE_PARAM: &[BpfAttachParam] = &[BpfAttachParam {
    name: "interface",
    kind: BpfAttachParamKind::Interface,
    required: true,
    description: "network interface to attach to",
}];

const CGROUP_PARAM: &[BpfAttachParam] = &[BpfAttachParam {
    name: "cgroup",
    kind: BpfAttachParamKind::CgroupPath,
    required: true,
    description: "cgroup v2 directory the policy applies to",
}];

/// Every BPF event the framework recognises, in documentation order. All six
/// are codegen-ready: `SOCKET_FILTER`/`XDP` (issue #1203), then
/// `TC_INGRESS`/`TC_EGRESS`/`CGROUP_CONNECT4`/`CGROUP_BIND4` (issue #1310's
/// `SCHED_CLS`/`CGROUP_SOCK_ADDR` lowering, completing issue #1204's event
/// framework).
pub const BPF_EVENTS: &[BpfEventSpec] = &[
    BpfEventSpec {
        name: "SOCKET_FILTER",
        aliases: &["SOCKET"],
        prog_type: BpfEventProgType::SocketFilter,
        elf_section: "socket",
        context: BpfContext {
            struct_name: "__sk_buff",
            fields: SK_BUFF_FIELDS,
        },
        capabilities: BpfEventCaps::PKT_READ
            .union(BpfEventCaps::MAP_READ)
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        verdicts: &[BpfVerdictKind::Accept, BpfVerdictKind::Drop],
        default_verdict: BpfVerdictKind::Accept,
        attach_params: IFACE_PARAM,
        kernel: BpfKernelReq {
            min_major: 3,
            min_minor: 19,
            requires_btf: false,
            bpf_link: false,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "socket filter: verdict is the number of bytes to accept (0 drops)",
    },
    BpfEventSpec {
        name: "XDP",
        aliases: &[],
        prog_type: BpfEventProgType::Xdp,
        elf_section: "xdp",
        context: BpfContext {
            struct_name: "xdp_md",
            fields: XDP_MD_FIELDS,
        },
        capabilities: BpfEventCaps::PKT_READ
            .union(BpfEventCaps::MAP_READ)
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        verdicts: &[
            BpfVerdictKind::Pass,
            BpfVerdictKind::Drop,
            BpfVerdictKind::Tx,
        ],
        default_verdict: BpfVerdictKind::Pass,
        attach_params: IFACE_PARAM,
        kernel: BpfKernelReq {
            min_major: 4,
            min_minor: 8,
            requires_btf: false,
            bpf_link: true,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "XDP ingress: verdict is an XDP action (PASS/DROP/TX)",
    },
    BpfEventSpec {
        name: "TC_INGRESS",
        aliases: &["TC"],
        prog_type: BpfEventProgType::TcIngress,
        elf_section: "tc",
        context: BpfContext {
            struct_name: "__sk_buff",
            fields: SK_BUFF_FIELDS,
        },
        capabilities: BpfEventCaps::PKT_READ
            .union(BpfEventCaps::MAP_READ)
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        // TC classifier verdicts reuse pass/drop naming (TC_ACT_OK / TC_ACT_SHOT).
        verdicts: &[BpfVerdictKind::Pass, BpfVerdictKind::Drop],
        default_verdict: BpfVerdictKind::Pass,
        attach_params: IFACE_PARAM,
        kernel: BpfKernelReq {
            min_major: 6,
            min_minor: 6,
            requires_btf: true,
            bpf_link: true,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "TC ingress classifier: verdict is a TC action (OK/SHOT)",
    },
    BpfEventSpec {
        name: "TC_EGRESS",
        aliases: &[],
        prog_type: BpfEventProgType::TcEgress,
        elf_section: "tc",
        context: BpfContext {
            struct_name: "__sk_buff",
            fields: SK_BUFF_FIELDS,
        },
        capabilities: BpfEventCaps::PKT_READ
            .union(BpfEventCaps::MAP_READ)
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        verdicts: &[BpfVerdictKind::Pass, BpfVerdictKind::Drop],
        default_verdict: BpfVerdictKind::Pass,
        attach_params: IFACE_PARAM,
        kernel: BpfKernelReq {
            min_major: 6,
            min_minor: 6,
            requires_btf: true,
            bpf_link: true,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "TC egress classifier: verdict is a TC action (OK/SHOT)",
    },
    BpfEventSpec {
        name: "CGROUP_CONNECT4",
        aliases: &[],
        prog_type: BpfEventProgType::CgroupInet4Connect,
        elf_section: "cgroup/connect4",
        context: BpfContext {
            struct_name: "bpf_sock_addr",
            fields: SOCK_ADDR_FIELDS,
        },
        capabilities: BpfEventCaps::MAP_READ
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        // A cgroup sock_addr hook returns 1 (allow) or 0 (deny); modelled with
        // pass = allow, drop = deny.
        verdicts: &[BpfVerdictKind::Pass, BpfVerdictKind::Drop],
        default_verdict: BpfVerdictKind::Pass,
        attach_params: CGROUP_PARAM,
        kernel: BpfKernelReq {
            min_major: 4,
            min_minor: 17,
            requires_btf: true,
            bpf_link: true,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "cgroup IPv4 connect policy: verdict allows (pass) or denies (drop) the connect",
    },
    BpfEventSpec {
        name: "CGROUP_BIND4",
        aliases: &[],
        prog_type: BpfEventProgType::CgroupInet4Bind,
        elf_section: "cgroup/bind4",
        context: BpfContext {
            struct_name: "bpf_sock_addr",
            fields: SOCK_ADDR_FIELDS,
        },
        capabilities: BpfEventCaps::MAP_READ
            .union(BpfEventCaps::MAP_WRITE)
            .union(BpfEventCaps::CTX_READ),
        verdicts: &[BpfVerdictKind::Pass, BpfVerdictKind::Drop],
        default_verdict: BpfVerdictKind::Pass,
        attach_params: CGROUP_PARAM,
        kernel: BpfKernelReq {
            min_major: 4,
            min_minor: 17,
            requires_btf: true,
            bpf_link: true,
        },
        output: BpfEventOutput::PacketDecision,
        codegen: BpfCodegen::Ready,
        description: "cgroup IPv4 bind policy: verdict allows (pass) or denies (drop) the bind",
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

/// The canonical names of events that currently lower to a compiled program.
#[must_use]
pub fn codegen_ready_event_names() -> Vec<&'static str> {
    BPF_EVENTS
        .iter()
        .filter(|e| e.is_codegen_ready())
        .map(|e| e.name)
        .collect()
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

    #[test]
    fn tc_and_cgroup_events_are_schema_described_and_codegen_ready() {
        // Issue #1204: TC ingress/egress and cgroup connect/bind exist as typed
        // registry contracts, resolvable by name/alias. Issue #1310: their
        // codegen (SCHED_CLS / CGROUP_SOCK_ADDR) is now implemented too.
        for name in ["TC_INGRESS", "TC_EGRESS", "CGROUP_CONNECT4", "CGROUP_BIND4"] {
            let e = lookup_bpf_event(name).unwrap_or_else(|| panic!("{name} missing"));
            assert_eq!(
                e.codegen,
                BpfCodegen::Ready,
                "{name} should be codegen-ready"
            );
            assert!(!e.context.fields.is_empty(), "{name} has no context fields");
            assert!(!e.attach_params.is_empty(), "{name} has no attach params");
        }
        assert_eq!(lookup_bpf_event("tc").map(|e| e.name), Some("TC_INGRESS"));
    }

    #[test]
    fn every_event_has_a_consistent_verdict_algebra() {
        for e in BPF_EVENTS {
            // The default verdict must be one the event actually permits, and
            // must be terminal (a chain cannot default to "continue").
            assert!(
                e.verdicts.contains(&e.default_verdict),
                "{}: default verdict not in its verdict set",
                e.name
            );
            assert!(
                e.default_verdict.is_terminal(),
                "{}: default verdict must be terminal",
                e.name
            );
            // `next` is always permitted and never listed as a terminal verdict.
            assert!(e.permits_verdict("next"), "{} must permit next", e.name);
            assert!(
                !e.verdicts.contains(&BpfVerdictKind::Next),
                "{}: next must not be listed as terminal",
                e.name
            );
            // Context reads imply the CTX_READ capability.
            assert!(
                e.capabilities.contains(BpfEventCaps::CTX_READ),
                "{}: context fields need CTX_READ",
                e.name
            );
        }
    }

    #[test]
    fn codegen_ready_events_are_every_described_event() {
        // Issue #1310: TC and cgroup codegen (SCHED_CLS / CGROUP_SOCK_ADDR
        // lowering) landed, so every registry-described event is now
        // codegen-ready — this list grows only when a new event is added,
        // not by dropping an existing one back to `Described`.
        assert_eq!(
            codegen_ready_event_names(),
            vec![
                "SOCKET_FILTER",
                "XDP",
                "TC_INGRESS",
                "TC_EGRESS",
                "CGROUP_CONNECT4",
                "CGROUP_BIND4",
            ]
        );
    }

    #[test]
    fn next_is_the_only_non_terminal_verdict() {
        assert!(!BpfVerdictKind::Next.is_terminal());
        for v in [
            BpfVerdictKind::Accept,
            BpfVerdictKind::Drop,
            BpfVerdictKind::Pass,
            BpfVerdictKind::Tx,
        ] {
            assert!(v.is_terminal(), "{v:?} should be terminal");
        }
    }
}
