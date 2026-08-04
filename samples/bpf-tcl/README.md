# BPF-Tcl userspace demos

These demos exercise both backend targets that exist today. The default `rbpf`
target executes real eBPF instructions over synthetic packet bytes in a
userspace virtual machine. The explicit `kernel-xdp` target supports a small,
map-free, verdict-only XDP subset that the Linux kernel can verifier-load and
test-run.

The clean-room instructions and full script were validated on Ubuntu 26.04
with Rust 1.97.0, GNU `readelf`, and LLVM 21.

The demos do **not** attach programs to a network interface or socket. Default
ELF output uses the simulator context ABI and is for inspection only; pass an
object to `bpftool prog load` only when it was compiled with `--target
kernel-xdp`. See the
[backend architecture and roadmap](../../docs/design/compiler/ebpf-backend.md).

## Install prerequisites on Debian or Ubuntu

Install the native build tools and ELF inspector:

```sh
sudo apt-get update
sudo apt-get install -y \
  curl ca-certificates build-essential pkg-config libssl-dev binutils git
```

The workspace follows the current stable Rust release. Distribution `cargo`
packages are often older than the `rust-version` in the top-level
`Cargo.toml`, so install Rust through `rustup`:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain stable
. "${HOME}/.cargo/env"
rustup update stable
rustc --version
cargo --version
```

On a recent Ubuntu release that packages `rustup`, this is also valid:

```sh
sudo apt-get install -y rustup
rustup default stable
```

`readelf` comes from `binutils`. LLVM disassembly is optional:

```sh
sudo apt-get install -y llvm
```

`bpftool` is not required for these demos. It can inspect the host's BPF
capabilities. The separate verdict-only kernel demo below requires it:

```sh
sudo apt-get install -y bpftool
```

## Get and compile the project

```sh
git clone https://github.com/bitwisecook/tcl-lsp.git
cd tcl-lsp

source scripts/dev/agent-build-env.sh
cargo build -p bpf-tcl --release
target/release/bpf-tcl --help
```

The compiled executable is `target/release/bpf-tcl`. The demo script below
builds and uses `target/debug/bpf-tcl` automatically instead, which is quicker
for development builds and behaves identically for the examples.

## Run everything

From the repository root on a Linux server:

```sh
bash samples/bpf-tcl/run-demos.sh
```

The script builds `bpf-tcl`, then demonstrates:

1. a socket filter that drops undersized packets and accepts complete packets;
2. an XDP handler using a user-defined packet profile and template;
3. an integer map that persists across repeated simulator invocations;
4. framework priority sorting and descriptive `attach` metadata;
5. emitted eBPF assembly; and
6. a map-free `EM_BPF` ELF object inspected with `readelf` when available.

Multi-byte packet fields are read in the declared byte order. Built-in and
user profile fields default to network order (big-endian), matching real
Ethernet/IP/TCP/UDP headers, so a `tcp_dport` comparison works against a
realistic packet on a little-endian host. A `load8`/`load16`/`load32` verb or a
`field` declaration may override the order with a trailing `be`, `le`, or
`native` word; `native` is a compatibility mode for synthetic host-order test
packets only.

## Run one example

```sh
source scripts/dev/agent-build-env.sh
cargo build -p bpf-tcl

target/debug/bpf-tcl check samples/bpf-tcl/xdp-marker.bpftcl
target/debug/bpf-tcl run samples/bpf-tcl/xdp-marker.bpftcl --packet 7f000102
target/debug/bpf-tcl run samples/bpf-tcl/xdp-marker.bpftcl --packet 01020304
```

Expected verdicts are `XDP_DROP` for a first byte of `0x7f`, and `XDP_PASS`
for a first byte of `0x01`.

Use `--repeat N` to execute one emitted program repeatedly while preserving its
simulated maps:

```sh
target/debug/bpf-tcl run samples/bpf-tcl/map-counter.bpftcl \
  --packet 0000000000000000 --repeat 3
```

The final output includes `map hits: 0=3`.

## Run the first real kernel XDP program

The explicit `kernel-xdp` target currently supports map-free, verdict-only XDP
handlers. It rejects packet/context access and maps until their verifier-proof
and relocation lowering is implemented. This gives the project a small but
genuine kernel-loaded vertical slice without silently emitting unsafe code.

Requirements:

- Linux with XDP and the `bpf()` system call enabled;
- root or sudo access with the required BPF capabilities;
- `bpftool`, `readelf`, Cargo, and a mounted BPF filesystem at `/sys/fs/bpf`.

Run:

```sh
bash samples/bpf-tcl/run-kernel-xdp-demo.sh
```

The script:

1. compiles `xdp-kernel-pass.bpftcl` with `--target kernel-xdp`;
2. inspects the ELF object;
3. verifier-loads and pins it temporarily with `bpftool`;
4. executes it over a synthetic 64-byte Ethernet frame using
   `BPF_PROG_TEST_RUN`;
5. expects return value `2` (`XDP_PASS`); and
6. removes the pin and all temporary files on exit.

It does not attach the program to a network interface. If the BPF filesystem is
not mounted, mount it once with:

```sh
sudo mount -t bpf bpf /sys/fs/bpf
```

## Compile a `.bpftcl` file

`check` runs the framework expansion and typed lowering without emitting an
object:

```sh
target/debug/bpf-tcl check samples/bpf-tcl/priority-bundle.bpftcl
```

`compile` can emit readable assembly, raw instruction bytes, hex, or a
structural ELF object. A bundle with multiple handlers needs `--program N` for
ELF because each program receives its own object:

```sh
target/debug/bpf-tcl compile samples/bpf-tcl/priority-bundle.bpftcl \
  --emit asm --program 0

target/debug/bpf-tcl compile samples/bpf-tcl/priority-bundle.bpftcl \
  --emit elf --program 0 -o /tmp/bpf-tcl-xdp.o

readelf -h -S -s /tmp/bpf-tcl-xdp.o
llvm-objdump -d /tmp/bpf-tcl-xdp.o  # optional
```

The ELF has an `EM_BPF` machine header, an `xdp` or `socket` program section, a
GPL licence section, and a function symbol. The default target is still
`rbpf`, so structural validity does not imply kernel loadability. Use
`--target kernel-xdp` only with the currently supported verdict-only XDP subset.

## Execute synthetic packets

`run` accepts packet bytes as hexadecimal. Whitespace inside the value is
ignored. It compiles the selected handler and executes the resulting eBPF under
the userspace VM:

```sh
target/debug/bpf-tcl run samples/bpf-tcl/socket-length.bpftcl \
  --packet 01020304
```

For a multi-handler file, use `--program N`. Program indexes are the order
printed by `check`, after priority sorting.

## Troubleshooting

- If Cargo says the compiler is too old, run `rustup update stable`, ensure
  `${HOME}/.cargo/bin` is on `PATH`, and retry.
- If `readelf` is missing, install `binutils`. If `llvm-objdump` is missing,
  install `llvm`; the demo script treats both inspectors as optional.
- No `sudo` is needed after prerequisites are installed. The userspace VM does
  not require `CAP_BPF`, `CAP_NET_ADMIN`, a mounted BPF filesystem, or kernel
  headers.
- An out-of-bounds synthetic packet load is an execution error. Keep explicit
  `pktlen` guards before fixed-offset loads.
