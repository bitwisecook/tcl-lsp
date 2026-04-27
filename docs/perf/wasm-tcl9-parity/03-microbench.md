# 03 — Per-primitive micro benchmarks

## Methodology

For each Tcl primitive we want to isolate, build a script of the
form

```tcl
set N <iters>
<warmup statement>
for {set i 0} {$i < $N} {incr i} { <op> }
```

…run it 9 times under wasm and 9 times under tclsh (2 warmups each),
take the median wall time, subtract the no-op baseline, divide by
N to get **per-op cost in nanoseconds**. Same source for both
backends — fair comparison.

Baselines used to subtract:

| | median |
|---|---:|
| wasmtime no-op | 8.62 ms |
| `tclsh` no-op | 122.49 ms |

## Results

| Op | N | wasm/op (ns) | tclsh/op (ns) | wasm vs tclsh |
|---|---:|---:|---:|---|
| `set v hello; set _ $v` | 100,000 | 232 | 380 | **1.64× faster** |
| `incr x` (loop) | 200,000 | 120 | 445 | **3.70× faster** |
| `expr {$t + $i * 3 - 1}` | 100,000 | TRAP | 380 | **traps — bump-allocator OOM** |
| `lappend L $i` + `foreach v $L` | 20,000 | TRAP | — | **traps — bump-allocator OOM** |
| `append s x; string length $s` | 5,000 | 2,568 | < noise | **O(N²) — see hot-spots** |
| `proc f {} {return 42}` + `f` | 50,000 | 153 | **48** | **3.19× slower** |
| `proc add3 {a b c} {expr…}` + `add3 …` | 50,000 | 252 | 320 | **1.27× faster** |
| `if {$i % 2 == 0} {…} else {…}` | 100,000 | 222 | 423 | **1.91× faster** |
| `foreach v $L {incr t}` (L=10 elems) | 20,000 | 106 | 2,724 | **25.8× faster** ⚠ |
| `dict set d k$i $i` + `dict get` | 5,000 | TRAP | — | **traps — bump-allocator OOM** |
| `::ns::do $i` (namespaced proc) | 20,000 | 261 | < noise | **fast on both** |

## Reading the table

- **Green wins** (`set`, `incr`, `if`, namespaced proc lookup,
  multi-arg proc): the precompiled wasm path beats Tcl's
  bytecode interpreter by 1.3 – 3.7×, exactly because we bypass
  the per-statement parse + bytecode-compile + dispatch loop.
- **One real loss** (no-arg `proc` call): tclsh dispatches a
  no-op proc in ≈ 50 ns; we take ≈ 153 ns. Nearly all of the
  overhead is `frame_push` zeroing 4 KB and the surrounding
  frame-set/restore work. Fix sketched in
  [`07-recommendations.md`](07-recommendations.md).
- **Three traps** (`expr`, `lappend`, `dict`): all the same root
  cause — `valtypes/tcl_obj.zig`'s bump allocator never grows
  linear memory and only recycles 24-byte `OBJ_SIZE`
  allocations. Workloads that allocate larger buffers
  (intermediate lists, dict pairs, expr temporaries) march
  upward through linear memory until they hit the 16 MB limit
  and trap with `out of bounds memory access`.
- **One suspicious win** (`foreach` 25.8×): the inner body
  `incr t` writes to a variable that's never read again, so the
  wasm codegen may be eliding the increment under
  store-to-deadvar. The tclsh number (272 ns/inner-incr) is
  consistent with its other `incr` numbers. Re-run with a
  trailing `puts $t` to confirm before claiming the speed-up.
- **`append s x` is O(N²) on wasm.** `tcl_cmd_append`
  (`runtime/zig/valtypes/tcl_string.zig:22`) allocates a fresh
  buffer and `memcpy`s both halves on every call. After 5 000
  iterations of growing a 1-char-at-a-time string we've copied
  ≈ 12 MB through the bump allocator. Tcl 9 uses
  `Tcl_AppendObjToObj` with geometric capacity growth →
  amortised O(1).

Source: `microbench.py`; raw data: `microbench_results.json`.
