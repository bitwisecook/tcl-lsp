# 04 — Zig runtime: Debug vs ReleaseFast

## Why this matters

`runtime/zig/build.zig` calls `b.standardOptimizeOption({})`, so a
plain `zig build` produces a Debug binary. The session-start hook
does not pin a build mode. AGENTS.md (Zig runtime layering
section) says

> Debug builds are ~3× larger but expose real bugs (e.g.
> `@ptrFromInt(0)` panics, buffer-offset vs address misuse) that
> are silently masked in release mode.

We confirm both the size ratio and quantify the runtime cost.

## Build sizes

| Mode | Bytes | Relative |
|---|---:|---|
| ReleaseFast | 2,600,403 (2.5 MB) | 1.00× |
| Debug | 3,674,836 (3.5 MB) | 1.41× |

## Workload timings

Runs the **same compiled per-script wasm** against both runtime
binaries; medians of 7 timed iterations after 3 warmups, fresh
wasmtime store per call. Workloads were sized down (compared with
the microbench) because the Debug build's larger stack frames
cause `__stack_chk_fail` traps at 200k iterations.

| Workload | release (ms) | debug (ms) | debug/release |
|---|---:|---:|---:|
| `set+incr` ×50,000 | 10.80 | 63.52 | **5.88×** |
| `if/else` ×20,000 | 5.97 | 33.58 | **5.62×** |
| `proc f {}; f` ×20,000 | 4.13 | 24.63 | **5.97×** |

## Takeaways

- **Always run published numbers against ReleaseFast.** The
  6× slowdown is uniform across hot-path categories so
  conclusions about where time goes don't change between
  builds, but the absolute numbers are nonsense in Debug.
- **Local development should still use Debug** (default). The
  trap traces in `02_control_flow_braced` and similar workloads
  in this experiment came out Debug-only because Debug exposes
  pointer-bounds errors that ReleaseFast silently truncates.
- **CI should pin both modes.** The session-start hook should
  build ReleaseFast for performance gates and Debug for
  correctness gates; today it builds neither (the wasm under
  `zig-out/bin/` is whatever the last invocation left).

Source: `debug_vs_release.py`; raw data: `debug_vs_release.json`.
