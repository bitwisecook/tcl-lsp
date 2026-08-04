# KCS: feature — BPF-Tcl low-level packet language

> **Audience:** User
> **Type:** Functionality

## Summary

BPF-Tcl is a small, statically typed packet-programming language with Tcl
syntax that compiles to eBPF and runs under a userspace simulator (with a
verdict-only kernel-XDP slice).

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
`pass` / `drop` / `tx` for XDP. Build the CLI with `make rust-clis` (the
binary is `bpf-tcl`), then run `bpf-tcl check FILE`, `bpf-tcl compile FILE`,
or `bpf-tcl run FILE --packet HEX`.

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
`kernel-xdp` target (`--target kernel-xdp`) currently compiles only map-free,
verdict-only XDP handlers; kernel packet access, kernel maps, and live
attachment are follow-on work. See
[`docs/design/compiler/ebpf-backend.md`](../../design/compiler/ebpf-backend.md)
for the full architecture and roadmap.
