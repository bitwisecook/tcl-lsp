# C++ Rewrite — Design and Rationale

## Why

The tcl-lsp server is a mature ~40K-line Python codebase. Rewriting the core in
modern C++ (C++23/26) serves two goals:

1. **Performance**: The lexer, semantic analysis, and compilation pipeline are
   CPU-bound. C++ eliminates interpreter overhead, enables zero-copy token
   handling via `string_view`, and allows arena allocation for parse trees with
   bulk deallocation. The target architecture prioritises getting semantic tokens
   to the user immediately, then layers on deeper analysis asynchronously.

2. **Learning**: Building a real, idiomatic modern C++ codebase from the ground
   up — using C++23/26 features, the stdexec async model (P2300), and modern
   memory management patterns — while having a comprehensive test suite to
   verify correctness at every step.

## Design philosophy

**The C++ code is designed as idiomatic, clean, modern C++.** It is not a
transliteration of the Python. The Python's architecture (frozen dataclasses,
dict-heavy structures, GIL threading) does not dictate the C++ design.

The **pybind11 shim** is where the impedance mismatch between the C++ and
Python worlds is absorbed. The shim adapts clean C++ interfaces to match the
existing Python API that the rest of the codebase expects. As more layers are
ported, the shim moves forward and eventually disappears.

Concretely:
- C++ uses value types, `string_view`, spans, and arena allocation — not
  heap-allocated objects mimicking Python dataclasses
- C++ uses `std::expected` for errors — not exceptions mimicking Python's model
- C++ uses proper encapsulation with `[[nodiscard]]`, `const`, and move
  semantics — not public fields mimicking frozen dataclasses
- The async model uses stdexec senders/receivers — not threads mimicking
  Python's `asyncio.to_thread`

The shim handles conversions: `string_view` → Python `str`, `std::expected` →
Python exception, C++ value types → Python objects with `__hash__`/`__eq__`.

## Architecture

### Layered rewrite, bottom-up

The rewrite proceeds layer by layer from the bottom of the stack. At each step
the C++ layer is complete and tested, the shim adapts it to the Python
interface, and the full pytest suite passes.

```
Phase 1: Core types      SourcePosition, Range, DocumentBuffer
Phase 2: Lexer           TokenType, Token, TclLexer
Phase 3: Segmenter       SegmentedCommand, error recovery
Phase 4: Semantic model   ProcDef, VarDef, Scope, Diagnostic, AnalysisResult
Phase 5: IR + lowering   IRStatement variants, IRModule
Phase 6: CFG/SSA + async  Control flow, data flow, stdexec pipeline
Phase 7: LSP features    Completion, hover, semantic tokens, diagnostics
Phase 8: LSP server      Replace pygls entirely
```

### Async model

The target architecture uses stdexec (P2300) senders/receivers:

```
Document edit →
  tokenize + segment (fast, ~1ms) →
    semantic tokens (respond to editor immediately)
    analyse + lower to IR (background) →
      when_all: lint | optimiser | taint | shimmer
    each pass publishes diagnostics as it completes
```

New edits cancel in-flight analysis. Semantic tokens are always the priority.

### Memory model

- **Source text**: `std::string` owned by `DocumentBuffer`
- **Token text**: `std::string_view` into source — zero-copy within C++
- **Parse trees**: Arena-allocated (`std::pmr::monotonic_buffer_resource`),
  bulk-freed on document edit
- **Python boundary**: Strings copied (unavoidable; pybind11 converts
  `string_view` to Python `str`)
- **Immutability**: Achieved via `const`, value semantics, and encapsulation —
  not by copying Python's frozen dataclass pattern

### Concurrency

- `std::shared_mutex` for analysis snapshots (readers shared, writer exclusive)
- stdexec scheduler for thread pool management
- Per-document cancellation token for superceding edits

## Toolchain

| Choice | Decision | Rationale |
|---|---|---|
| Language | C++23/26 | Latest standard features: `expected`, `ranges`, `format`, spaceship operator, `deducing this` |
| Compiler | Clang 18+ | Best C++23 support, cross-platform, excellent tooling (clang-tidy, clang-format) |
| Build | Meson | Clean readable syntax, first-class pybind11 support, WrapDB for dependencies |
| Bindings | pybind11 | Native Python extension, exposes C++ types directly to Python |
| Testing | Catch2 (C++) + pytest (Python) | Catch2 for C++ unit tests; full pytest suite validates through pybind11 shim |
| Async | stdexec (P2300) | Standard-track sender/receiver model for composable async |
| Package mgmt | Meson WrapDB | pybind11, Catch2 available; stdexec via cmake subproject wrap |

## Branch strategy

- `main` — stable release branch
- `cpp` — long-lived feature branch for the rewrite (from main)
- Work branches off `cpp` for each phase
- Phases merge into `cpp`, which merges into `main` when stable

## Testing strategy

The full pytest suite is the source of truth throughout the rewrite. It
exercises the C++ code through the pybind11 shim, validating that the native
implementation is a correct drop-in replacement.

C++ unit tests (Catch2) provide fast feedback during development and test
C++-specific concerns (lifetime safety, `string_view` validity, arena
behaviour, benchmark comparisons).

Both test suites must pass at every step. The pytest suite is not removed until
all Python is gone.

## Benchmark results

Each phase records timing and memory measurements. Run with:

```bash
# Python-only baseline:
PYTHONPATH=. python3 scripts/bench_native_types.py

# With C++ native module:
PYTHONPATH=builddir/native:. python3 scripts/bench_native_types.py
```

### Phase 1: Core types (SourcePosition, Range, DocumentBuffer)

Test corpus: 230KB Tcl source, 10,000 lines. 100K iterations per operation.

**DocumentBuffer** (where computation lives — the big wins):

| Operation | Python | C++ | Speedup |
|---|---|---|---|
| `from_source` (230KB) | 71.1 ms | 138.9 µs | **512x** |
| `offset_to_position` | 5.5 µs | 1.3 µs | **4.2x** |
| `position_to_offset` | 7.5 µs | 1.1 µs | **6.8x** |
| `range_from_offsets` | 16.7 µs | 1.3 µs | **12.8x** |
| `chunk_line_range` | 6.0 µs | 1.3 µs | **4.6x** |

**Value types** (pybind11 wrapping overhead dominates at Python boundary):

| Operation | Python | C++ | Note |
|---|---|---|---|
| SourcePosition create | 2.3 µs | 3.0 µs | pybind11 wrapper cost |
| Range create | 1.9 µs | 2.5 µs | pybind11 wrapper cost |
| Range.zero() | 4.3 µs | 1.1 µs | 3.9x (avoids dataclass init) |

**Memory** (Python `tracemalloc` — C++ heap allocations invisible):

| Metric | Python | C++ |
|---|---|---|
| 10x large buffer (Python-tracked) | 3.89 MB | 992 B |

Note: the C++ DocumentBuffer allocates on the C++ heap which `tracemalloc`
cannot see. The 992B is only the pybind11 wrapper objects. True C++ memory
usage is ~230KB per buffer (source string + line starts vector), comparable
to Python but without the per-object overhead of Python's allocator.

**Key insight**: The value types (SourcePosition, Range) show no speedup
when accessed from Python because pybind11 wrapper creation dominates.
These will show their real benefit in Phase 2+ when the lexer stays entirely
in C++ and these types are passed by value within C++ without crossing the
Python boundary.

