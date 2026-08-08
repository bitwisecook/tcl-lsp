# KCS: feature — BPF-Tcl low-level packet language

> **Audience:** User
> **Type:** Functionality

## Summary

BPF-Tcl is a small, statically typed packet-programming language with Tcl
syntax that compiles to eBPF and runs under a userspace simulator, and also
emits Linux-loadable XDP, socket-filter, TC (ingress/egress), and cgroup
(connect4/bind4) objects with real context access, bounds-checked packet
reads (where the context has a packet body), and maps.

## Applies to

tcl-lsp CLI

## Question

What does BPF-Tcl do, and how do I write a typed packet handler?

## How to use

Write a `.bpftcl` file. Each `when EVENT { … }` block (or
`when EVENT priority N { … }`) is one program. Inside a handler you bind the
packet with `setbuf`, read its length with `pktlen`, read fixed-offset fields
with `load8` / `load16` / `load32`, keep state in a `map`, and end every path
with an explicit verdict — `accept` / `drop` for a socket filter, or
`pass` / `drop` / `tx` for XDP, TC ingress/egress, and cgroup connect4/bind4
(`tx` is XDP-only; a cgroup handler has no packet to read, so it can only use
`map`/verdict verbs, never `setbuf`). Build the CLI with `make rust-clis` (the
binary is `bpf-tcl`), then run `bpf-tcl check FILE`, `bpf-tcl compile FILE`,
or `bpf-tcl run FILE --packet HEX`. `bpf-tcl compile FILE --emit elf` writes an
ELF object; add `--target kernel-xdp`, `--target kernel-socket`,
`--target kernel-tc`, or `--target kernel-cgroup-sockaddr` for a
Linux-loadable object (the default `--target rbpf` is a simulator artefact for
inspection).

Key contracts to know:

- **The header is strict.** A `when` header must be exactly `when EVENT { … }`
  or `when EVENT priority N { … }`. A non-integer priority, an unknown header
  word, or a substituted event is an error — never silently normalised.
- **Every path needs a verdict.** A handler path that reaches the end without
  a verdict is rejected; the compiler never inserts one for you.
- **Byte order is explicit.** Multi-byte fields default to network order
  (big-endian), matching real headers. Override with a trailing `be`, `le`, or
  `native` word on a `load*` verb or a profile `field`. `native` is a
  compatibility mode for synthetic host-order test packets only.
- **Maps are typed.** `map NAME hash|array KEYSZ VALSZ MAX ?shared|percpu?`.
  `map_get` reads a value (0 when absent); `map_has` reports whether the key is
  present, so you can tell a missing key from a stored zero. A `hash` map
  enforces its capacity; an `array` map is preallocated and zero-filled.
- **Integer division is truncated, not floored.** `/` and `%` follow eBPF
  signed truncated-toward-zero division, which diverges from Tcl floor division
  only when exactly one operand is negative.

### Example

```tcl
# Drop TCP traffic to port 22, accept everything else.
profile ipv4_tcp

when SOCKET_FILTER priority 100 {
    ip_proto proto
    if {$proto != 6} { accept }
    tcp_dport dport
    if {$dport == 22} { drop }
    accept
}
```

```sh
bpf-tcl check drop-ssh.bpftcl
bpf-tcl run   drop-ssh.bpftcl --packet 00...   # simulate over a hex packet
```

## Limits

The default target is the userspace `rbpf` simulator. The explicit
`kernel-xdp`, `kernel-socket`, `kernel-tc`, and `kernel-cgroup-sockaddr`
targets emit Linux-loadable objects with `struct xdp_md` / `struct __sk_buff`
(shared by socket-filter and TC) / `struct bpf_sock_addr` (cgroup) context
access, verifier-safe packet bounds proofs (where the context has a packet
body), and BTF-defined maps with relocations. The emitted objects are
validated in the test suite with `readelf`/`llvm-objdump` and an in-repo
verifier model; the actual `bpf()` kernel load needs root and a live kernel,
so it runs behind `#[ignore]`d tests (`rust/bpf-tcl/tests/kernel_load.rs`,
`rust/bpf-tcl/tests/kernel_attach.rs`). The `kernel-tc` target's real-kernel
`tc filter add ... bpf da obj ... sec tc` attach has been run and verified
against a live kernel; the `kernel-cgroup-sockaddr` target's real-kernel
`bpftool cgroup attach` path has not (no environment with `bpftool` installed
was available when this was implemented), though its codegen is covered by the
unit/e2e test suites and the in-repo verifier model. Live attachment beyond
these gated tests (links, pins, interface configuration in a production
deployment tool) is still follow-on work. See
[`docs/design/compiler/ebpf-backend.md`](../../design/compiler/ebpf-backend.md)
for the full architecture and roadmap.
