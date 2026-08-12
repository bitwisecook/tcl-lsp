# Runtime/VM execution-layer duplication audit

Scope: `runtime/rust/`, `rust/tcl-vm*`, `rust/tcl-bytecode`, `rust/tcl-cmd-core`,
`rust/tcl-runtime-api`, `rust/tcl-host-native`, `rust/tcl-sandbox`, the
`command-backing` gate and its report.

## F1: `int`/`wide`/`entier` disagree between the VM and the WASM runtime on out-of-range float operands — the runtime raises a domain error where TIP 237 requires a bignum result

**Confidence:** high
**Category:** duplicated-implementation

**Implementation A:** `rust/tcl-vm/src/cmd_math.rs:121-134` (`exact_trunc`/`wide_window`, used by `int_window` at `cmd_math.rs:307-318`) and `rust/tcl-vm/src/cmd_math.rs:464-476` (`m_entier`)
**Implementation B / what it should be:** `runtime/rust/src/cmd_mathfunc.rs:130-166` (float-operand path falls into `dispatch`), backed by `rust/tcl-syntax/src/expr/mathfunc.rs:454-462` (`finite_trunc_to_i64`) and `:708-711` (the `"int" | "entier" | "wide"` arm)

**Divergence evidence:** `expr {entier(1e300)}` (or `int`/`wide` with the same operand):
- `tcl-vm`: `m_entier` (`cmd_math.rs:464`) takes the finite-`f64` path, calls `exact_trunc` (`cmd_math.rs:121-123`, explicitly commented "`int(1e300)`-style conversions see the full 10^300-scale value C does") and returns the full-precision bignum — the VM's own doc comment at `cmd_math.rs:459-463` cites this as TIP 237's mandated unbounded behaviour, and `int`/`wide` wrap it to a 64-bit window (`wide_window`, `cmd_math.rs:129-134`) rather than erroring.
- `runtime/rust`: the float operand (`1e300` is a `Num::Float`, not an integer object) skips the tower fast-path at `cmd_mathfunc.rs:135` (`is_integer(argv[1])` is false) and falls through to the **shared** `dispatch(&lname, &nums)` call at `cmd_mathfunc.rs:151`. `tcl_syntax::expr::mathfunc::dispatch`'s `"int" | "entier" | "wide"` arm (`mathfunc.rs:708-711`) calls `finite_trunc_to_i64`, which returns `None` for any magnitude outside `i64` (`mathfunc.rs:454-462`, and pinned by its own unit test `mathfunc.rs:794-797`: `dispatch("entier", &[Num::Float(1.0e20)]) == None`). `None` becomes `cmd_mathfunc.rs:163-166`'s `"domain error: argument not in valid range"` (`-errorcode ARITH DOMAIN`) — i.e. `expr {entier(1e300)}` **errors** in the WASM runtime instead of returning a 300-digit integer.
- The two engines also disagree on `int(1e20)`/`wide(1e20)`: VM silently returns a specific wrapped 64-bit value (mod 2^64, matching real tclsh's truncate-then-wrap); the runtime raises `ARITH DOMAIN`.

**Why it matters:** `entier`/`int`/`wide` are core `expr` functions and TIP 237 explicitly makes `entier()` unbounded; the shared `tcl_syntax::expr::mathfunc::dispatch` helper (used only by the WASM runtime's `format`-sibling `cmd_mathfunc.rs`, not by the VM) has no bignum branch for the `Num::Float` case, so any script doing arithmetic on a float literal or computed double outside `i64` range and then converting it to an integer produces a spurious runtime error under the WASM/compiler target while working correctly under the VM. This is a silent, input-dependent correctness divergence between the two engines the compiler's WASM codegen targets and the CLI/tests run against — exactly the shape PR #1371 flagged, except here the "shared" module is the one that is wrong, and the non-shared VM code is the one that is right.

**What cleanup looks like:** Give `tcl_syntax::expr::mathfunc::Num` (or `dispatch`'s `int`/`wide`/`entier`/`round` arms) a bignum-producing path — mirroring what `tcl-vm`'s `exact_trunc`/`wide_window`/`m_entier` already do — so both engines widen through the shared tower instead of the runtime silently domain-erroring. At minimum, `runtime/rust/src/cmd_mathfunc.rs` needs its own bignum fallback for the `Num::Float` case (parallel to the existing integer-operand fast path at `cmd_mathfunc.rs:135-143`) before this reaches the f64-limited shared `dispatch`.

**Scale:** small, surgical fix (one function family: `int`/`wide`/`entier`/`round` for float operands) but high-severity, since it changes *pass vs. fail* for valid `expr` scripts depending on which engine executes them.

---

## F2: `file` is a partial port in `runtime/rust` — 17 of the registry's 31 subcommands are unimplemented, invisible to the presence-based parity gate, and diverge from the VM

**Confidence:** high
**Category:** half-ported

**Implementation A:** `rust/tcl-registry/src/commands/tcl/file_.rs` — 31 `SubCommand` entries (`atime`, `attributes`, `channels`, `copy`, `delete`, `dirname`, `executable`, `exists`, `extension`, `home`, `isdirectory`, `isfile`, `join`, `link`, `lstat`, `mkdir`, `mtime`, `nativename`, `normalize`, `owned`, `pathtype`, `readable`, `readlink`, `rename`, `rootname`, `separator`, `size`, `split`, `stat`, `system`, `tail`, `tempdir`, `tempfile`, `tildeexpand`, `type`, `volumes`, `writable`; all `dialects: None` or `TCL86_PLUS`/`TCL90_PLUS`, i.e. all valid at Tcl 9.0)
**Implementation B / what it should be:** `runtime/rust/src/cmd_fs.rs:90-109` (`FILE_SUBCOMMANDS`, 20 entries) + the `match sub.as_slice()` at `cmd_fs.rs:130-249`, which implements only: `dirname`, `tail`, `rootname`, `extension`, `join`, `split`, `normalize`, `separator`, `nativename`, `exists`, `isdirectory`, `isfile`, `readable`, `writable`, `executable`, `pathtype`, `delete`, `mkdir`, `size`, `type`. Missing entirely: `atime`, `attributes`, `channels`, `copy`, `home`, `link`, `lstat`, `mtime`, `owned`, `readlink`, `rename`, `stat`, `system`, `tempdir`, `tempfile`, `tildeexpand`, `volumes` — all fall through to the `other =>` arm at `cmd_fs.rs:247-251` (`"unknown or ambiguous subcommand …"`).

**Divergence evidence:** `file mtime somepath` — `rust/tcl-vm/src/cmd_file.rs:245-248` implements `"mtime"` (dispatching to `file_mtime`); `runtime/rust/src/cmd_fs.rs` has no `mtime` arm and it is absent from `FILE_SUBCOMMANDS`, so the identical script raises `unknown or ambiguous subcommand "mtime": must be delete, dirname, executable, exists, extension, isdirectory, isfile, join, mkdir, nativename, normalize, pathtype, readable, rootname, separator, size, split, tail, type, or writable` (the literal error text at `cmd_fs.rs:249-250`) in the WASM runtime while succeeding in the VM. `copy`/`rename`/`stat`/`attributes` are unimplemented in *both* engines (`tcl-vm/src/cmd_file.rs`'s own `canonical_file_sub` table at lines 93-121 lists them for prefix-resolution parity with C but has no match arm for them either) — so those are a shared, if half-ported, gap; `mtime` is the one place the two engines visibly disagree today.

**Why it matters:** `cargo xtask command-backing` classifies `file` as a fully-backed `Handler` (`docs/generated/wasm-command-backing.md:194`: `` | `file` | handler | `` ``) because the gate only checks that `register_builtin(b"file", …)` exists (`rust/xtask/src/command_backing.rs` §`scan_handlers`) — it has no visibility into which of the ensemble's subcommands are actually implemented. A compiler-emitted WASM module calling any of `file copy`/`file rename`/`file stat`/`file attributes`/`file mtime`/`file atime`/`file link`/`file readlink`/`file owned`/`file volumes`/`file channels`/`file system`/`file tempdir`/`file tempfile`/`file home`/`file tildeexpand` will fail at runtime with "unknown or ambiguous subcommand" despite `file` reporting green in the parity report — this is precisely the presence-vs-completeness gap AGENTS.md's WASM-parity section describes the gate as unable to catch.

**What cleanup looks like:** Either extend the parity gate to check subcommand coverage against the registry's `SubCommand` lists (not just top-level command presence) so a partial ensemble port shows up as a real gap, or track `file`'s missing subcommands explicitly (e.g. a documented "known-gap" note analogous to `KNOWN_UNBACKED`, but subcommand-scoped) until they are implemented. Implementing the missing subcommands themselves (`copy`, `rename`, `stat`/`lstat`, `attributes`, `atime`/`mtime`) is the larger piece of work; at minimum `mtime` should be ported to `runtime/rust` since the VM already has a working reference implementation to port from.

**Scale:** medium — one file (`runtime/rust/src/cmd_fs.rs`), ~17 subcommands, several of which (`copy`, `rename`, `stat`) need real host-filesystem plumbing through `tcl_platform::Filesystem` and are not simple copy-paste ports.

---

## F3: `format` is reimplemented from scratch in `runtime/rust` instead of using the shared `tcl_cmd_core::format` the VM already migrated to, and is missing the Tcl 9 `%#d`/`%#i` alternate-form prefix as a result

**Confidence:** high
**Category:** duplicated-implementation

**Implementation A:** `rust/tcl-cmd-core/src/format.rs` (the shared, `ValueOps`-generic implementation, driven by `tcl_syntax::format::parse_spec`), consumed by `rust/tcl-vm/src/cmd_format.rs:94-97` (`tcl_cmd_core::format::format_cmd(vm, args)`) — the module doc at `cmd_format.rs:19-23` states "Both rendering directions now live in the shared `tcl_cmd_core`".
**Implementation B / what it should be:** `runtime/rust/src/cmd_format.rs` — an entirely independent, hand-rolled `format_cmd` (lines 70-285) with its own flag/width/precision parsing loop, not calling `tcl_cmd_core::format` or `tcl_syntax::format::parse_spec` at all (confirmed: `runtime/rust/src/cmd_format.rs` has no `tcl_cmd_core` reference anywhere in the file, unlike its sibling `cmd_scan.rs`, which *does* use the shared `tcl_cmd_core::scan` engine — `scan` was migrated, `format` was not).

**Divergence evidence:** `format %#d 42`:
- `tcl-cmd-core::format::render_spec` (`rust/tcl-cmd-core/src/format.rs:161-171`) explicitly adds the Tcl 9 `0d` alternate-form prefix for `%#d`/`%#i` (`digits.insert_str(0, "0d")` when `FmtFlags::HASH` is set and the verb is `d`/`i`) — this is regression-tested against real tclsh 9.0 at `rust/tcl-vm/tests/cmd_string_e2e.rs:895-906` (`bug_format_hash_decimal_prefix_tcl9`, asserting `format %#d 42` → `"0d42"`, with the test doc explicitly noting the *old*, wrong VM behaviour was `"42"` — i.e. this was already found and fixed once, on the VM/shared side).
- `runtime/rust/src/cmd_format.rs`'s `int_field` (lines 348-435) only adds an alternate-form prefix for `o`/`x`/`X`/`b` (`cmd_format.rs:401-412`: `match conv { b"o" => "0o", b"x"|b"X" => "0x", b"b" => "0b", _ => "" }` — the `_` arm silently drops the prefix for `d`/`i`). So `format %#d 42` under `runtime/rust` returns `"42"`, while the exact same script under `tcl-vm` (and, per the pinned test, real tclsh 9.0) returns `"0d42"`.

**Why it matters:** this is the PR #1371 shape precisely: one semantic question (`format`'s conversion rendering) answered in two independently-maintained places, and a bug that was found and fixed in the shared implementation (with a regression test) never propagated to the copy that didn't get migrated. A script compiled to WASM and a script run through the VM/CLI will silently render different output for `%#d`/`%#i` — a correctness divergence between the compiler's target runtime and the tool used to validate `format` behaviour day to day.

**What cleanup looks like:** Port `runtime/rust/src/cmd_format.rs` onto `tcl_cmd_core::format::format_cmd` the same way `cmd_scan.rs` already sits on `tcl_cmd_core::scan`, and onto `runtime/rust`'s existing `ValueOps` adapter for `*mut TclObj` (`runtime/rust/src/value_ops.rs`, already used by the other migrated `cmd_*.rs` files). This deletes ~450 lines of duplicated spec-parsing/rendering logic and closes the `%#d`/`%#i` gap (and forecloses any other float/width/precision edge case the two copies may have quietly diverged on) for free.

**Scale:** medium — one file, but a well-trodden migration path (identical to the `format`/`scan` split already completed for `tcl-vm` and for `runtime/rust`'s own `cmd_scan.rs`), so mechanical rather than exploratory.

---

## Areas checked with no reportable finding

- `string`, `list`, `dict`, `array`, `namespace`, `binary`, `clock`, `regex`, `switch`, `trace`, `lseq`, `mathop` command families: both `tcl-vm` and `runtime/rust` route through `tcl_cmd_core` (confirmed via `grep -l tcl_cmd_core` over both `src/` trees), and the `append`/`lappend`/`incr` variable-mutation cores are explicitly shared via `tcl_cmd_core::var` even in files that don't otherwise import the crate.
- `::tcl::mathfunc::*`/`::tcl::mathop::*` registration: both are driven off `tcl_syntax::expr::mathfunc`/`operators` tables rather than hand-typed name lists (the `command_backing.rs` header documents a prior staleness bug here, already fixed — issue #983/#987's unification).
- `string bytelength`'s absence from `runtime/rust/src/cmd_string.rs`'s `STRING_SUBCOMMANDS` looked like a gap on first read, but the registry marks it `dialects: Some(DialectSet::TCL8X)` (`rust/tcl-registry/src/commands/tcl/string_.rs:920-931`) — removed in Tcl 9.0, so its absence from a Tcl-9-targeting runtime is correct, not a gap.
- The `command-backing` gate itself (`rust/xtask/src/command_backing.rs`) is well-built for what it claims: it explicitly documents its own presence-only limitation, has drift tests for report staleness and stale classifications, and its `KNOWN_UNBACKED` list reads as genuinely tracked (each entry has a specific technical reason, several tied to numbered issues) rather than a dumping ground.
- No overstated claims found regarding the embedded Tcltest files vs. the C `test*` command surface, or regarding package-driven extension bundling, in the files reviewed under `runtime/rust/src/`.
