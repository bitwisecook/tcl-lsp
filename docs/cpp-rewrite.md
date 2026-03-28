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

### Naming conventions

The C++ code uses idiomatic C++ naming — **not** a mirror of the Python names.
The Python is going away; the C++ must stand on its own as a clean, readable
codebase. The pybind11 shim translates between C++ and Python names where they
differ.

| Element | C++ convention | Example |
|---|---|---|
| Types / classes | `CamelCase` | `SourcePosition`, `TclLexer` |
| Functions / methods | `lower_case` | `next_token()`, `offset_to_position()` |
| Variables / members | `lower_case` | `line_starts_`, `expand_syntax` |
| Private members | `lower_case_` (trailing `_`) | `source_`, `version_` |
| Enum constants | `UPPER_CASE` | `TokenType::ESC`, `TokenType::CMD` |
| Namespaces | `lower_case` | `tcl_lsp` |
| Constants | `lower_case` or `UPPER_CASE` | context-dependent |
| Files | `lower_case.hpp` / `.cpp` | `source_position.hpp`, `lexer.cpp` |

When a Python name doesn't match C++ conventions, the shim maps it:

```cpp
// C++ (clean, idiomatic)
auto TclLexer::next_token() -> std::expected<Token, LexError>;

// pybind11 shim (maps to Python interface)
cls.def("next_token", &TclLexer::next_token);  // same name here
cls.def("tokenize_all", ...);  // shim may rename if Python expects different name
```

The key rule: **design the C++ API first for C++ consumers**, then adapt
in the shim. Never compromise C++ naming to match Python conventions.

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
| Compilers | Clang 18+, GCC 13+, GCC 14+ | All native code must build clean under all three with `-Werror` and zero analyzer findings |
| Build | Meson | Clean readable syntax, first-class pybind11 support, WrapDB for dependencies |
| Bindings | pybind11 | Native Python extension, exposes C++ types directly to Python |
| Testing | Catch2 (C++) + pytest (Python) | Catch2 for C++ unit tests; full pytest suite validates through pybind11 shim |
| Async | stdexec (P2300) | Standard-track sender/receiver model for composable async |
| Package mgmt | Meson WrapDB | pybind11, Catch2 available; stdexec via cmake subproject wrap |

## Code quality tooling

Seven layers of analysis catch bugs at different levels — all wired into
`make prep-pr` (except Valgrind, which is a periodic deep-check):

### Static analysis (compile-time)

| Tool | Purpose | Config file |
|---|---|---|
| **clang-format 18** | Code formatting (LLVM-based, 100 col, 4-space indent) | `.clang-format` |
| **clang-tidy 18** | Linting, modernization, bug detection | `.clang-tidy` |
| **cppcheck 2.13+** | Additional static analysis (different bug patterns) | `.cppcheck-suppress` |
| **Clang Static Analyzer** | Path-sensitive analysis (null deref, dead stores, logic) | — |

### Runtime analysis (sanitizers)

| Tool | Catches | Notes |
|---|---|---|
| **AddressSanitizer (ASan)** | Buffer overflow, use-after-free, stack overflow, leaks | Runs every PR |
| **UndefinedBehaviourSanitizer (UBSan)** | Signed overflow, null deref, alignment, shift errors | Combined with ASan |
| **ThreadSanitizer (TSan)** | Data races, deadlocks, thread-safety bugs | Separate build (incompatible with ASan) |
| **Valgrind memcheck** | Uninitialised reads, invalid access, leaks | Periodic deep check (`make native-valgrind`) |

### Compiler hardening flags

The Meson build enables these unconditionally (see `meson.build`):
- `-fstack-protector-strong` — stack buffer overflow detection
- `-D_FORTIFY_SOURCE=2` — fortified libc functions (memcpy, strcpy bounds checking)
- `-D_GLIBCXX_ASSERTIONS` — libstdc++ bounds checking (operator[], iterators)
- `-Wconversion`, `-Wsign-compare` — catch implicit narrowing/sign issues
- `-Wnull-dereference`, `-Wformat=2` — null safety and format string security
- `-Wvla`, `-Wdouble-promotion`, `-Wimplicit-fallthrough` — common C++ pitfalls

### Dual-compiler requirement

All native C++ code (`native/src/`, `native/include/`, `native/tests/`) must
build with **zero warnings under `-Werror`** on all of:

- **Clang 18+** — primary development compiler, source of clang-tidy/clang-format
- **GCC 13** — baseline GCC, different warning heuristics and template diagnostics
- **GCC 14** — latest available, adds `-fanalyzer` improvements and `-fhardened`

The pybind11 bindings (`native/bindings/`) are temporary shim code — excluded
from clang-tidy, cppcheck, and GCC's `-fanalyzer`. GCC-specific false
positives in pybind11 template code are suppressed in the bindings build only.
External libraries (Catch2, pybind11) are not our code and not analysed.

### Installing prerequisites

```bash
# Ubuntu 24.04
apt install clang-18 clang-format-18 clang-tidy-18 clang-tools-18 \
  libclang-rt-18-dev gcc-13 g++-13 gcc-14 g++-14 cppcheck valgrind

# Python (for pybind11 bindings — optional for pure C++ analysis)
pip install pybind11 meson ninja
```

### Makefile targets

#### Clang 18 (primary)

| Target | In prep-pr? | Purpose |
|---|---|---|
| `make format-cpp` | via `make format` | Auto-format all C++ files |
| `make lint-cpp` | Yes | clang-tidy + cppcheck + format check |
| `make native-test` | Yes | Catch2 unit tests (Clang, `-Werror`) |
| `make native-test-asan` | Yes | Tests under Clang ASan + UBSan |
| `make native-test-tsan` | Yes | Tests under Clang TSan |
| `make native-scan-build` | Yes | Clang Static Analyzer (path-sensitive) |
| `make native-test-cfi` | Yes* | Control Flow Integrity (requires compiler-rt) |
| `make native-fuzz` | Yes* | libFuzzer harness (requires compiler-rt) |
| `make native-valgrind` | No | Valgrind memcheck (periodic deep check) |

*Only when `libclang-rt-18-dev` is installed.

#### GCC 13 + GCC 14

| Target | In prep-pr? | Purpose |
|---|---|---|
| `make native-test-gcc13` | Yes | Catch2 unit tests (GCC 13, `-Werror`) |
| `make native-test-gcc14` | Yes | Catch2 unit tests (GCC 14, `-Werror`) |
| `make native-test-gcc13-asan` | Yes | Tests under GCC 13 ASan + UBSan |
| `make native-test-gcc14-asan` | Yes | Tests under GCC 14 ASan + UBSan |
| `make native-gcc-analyze` | Yes | GCC static analyzer (`-fanalyzer`) |

GCC targets auto-skip when the compiler isn't installed (prints a message, does
not fail). The GCC `-fanalyzer` target uses the highest available GCC version
(prefers 14 for its improved checks: infinite-loop detection, overlapping-buffer
checks, enabled-by-default taint analysis).

#### What each tool catches

| Tool | Bug class | Unique value |
|---|---|---|
| **clang-tidy** | AST patterns, modernisation, const-correctness | Broadest lint coverage |
| **cppcheck** | Different static patterns (useStlAlgorithm, etc.) | Catches what clang-tidy misses |
| **Clang scan-build** | Path-sensitive: null deref, dead stores, logic | Execution path analysis |
| **GCC -fanalyzer** | Path-sensitive: different model than scan-build | GCC-specific: taint, infinite loops |
| **ASan + UBSan** | Runtime memory + undefined behaviour | Buffer overflows, use-after-free, signed overflow |
| **TSan** | Runtime thread safety | Data races, deadlocks |
| **Valgrind** | Runtime memory (no recompile needed) | Uninitialised reads, leaks |
| **CFI** | Runtime control flow | vtable corruption, type confusion |

## Branch strategy

- `main` — stable release branch
- `cpp` — long-lived feature branch for the rewrite (from main)
- Work branches off `cpp` for each phase
- Phases merge into `cpp`, which merges into `main` when stable

## Testing strategy

**Every Python test must be ported to C++ as its layer is rewritten.** When a
phase ports a Python module to C++, every pytest that exercises that module
must get a corresponding Catch2 test. No exceptions — the C++ test suite must
have at least the same coverage as the Python tests it replaces.

The porting process for each phase:

1. **Identify all pytest tests** that exercise the layer being ported. Use
   `grep`, test markers, and import analysis to find every test.
2. **Port each test to Catch2**, preserving the intent and edge cases. The
   C++ test may use different assertions or structure, but must cover the
   same behaviour.
3. **Verify parity**: the C++ Catch2 tests and the Python pytest tests must
   both pass against the same C++ implementation (via pybind11 shim).
4. **The Python tests remain** until all Python is gone — they serve as a
   cross-validation layer. But the C++ tests are the long-term owners.

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

Memory tracked via three complementary methods:
- **tracemalloc**: Python heap only (blind to C++ allocations)
- **mallinfo2**: C++ heap only (Linux, via pybind11-exposed `memory_stats()`)
- **RSS delta**: Process-level (captures both heaps)

**DocumentBuffer** (where computation lives — the big wins):

| Operation | Python | C++ | Speedup |
|---|---|---|---|
| `from_source` (230KB) | 6.1 ms | 188 µs | **33x** |
| `offset_to_position` | 1.0 µs | 445 ns | **2.3x** |
| `position_to_offset` | 548 ns | 227 ns | **2.4x** |
| `range_from_offsets` | 2.7 µs | 443 ns | **6.1x** |
| `chunk_line_range` | 1.0 µs | 331 ns | **3.0x** |

**Value types** (pybind11 wrapping overhead dominates at Python boundary):

| Operation | Python | C++ | Note |
|---|---|---|---|
| SourcePosition create | 469 ns | 713 ns | pybind11 wrapper cost |
| Range create | 1.1 µs | 1.4 µs | pybind11 wrapper cost |

**Memory** (10x large buffers, 230KB each):

| Metric | Python | C++ |
|---|---|---|
| Python heap (tracemalloc) | 3.89 MB | 944 B |
| C++ heap delta (mallinfo2) | 782 KB | 2.63 MB |
| Total in-process | ~4.67 MB | ~2.63 MB |

The C++ buffers use more C++ heap than Python's C heap overhead because each
buffer owns a `std::string` copy and a `vector<int32_t>` line-starts index.
Python's overhead is split across its own heap (3.89 MB for the objects) plus
C-level allocations for the underlying string/tuple data (782 KB).

Total memory per buffer is comparable (~260 KB C++ vs ~467 KB Python), but
the C++ version has no per-object Python overhead (no refcount, no type
pointer, no `__dict__`, no slots metadata). This difference compounds as
more types move to C++ in later phases.

**Key insight**: The value types (SourcePosition, Range) show no speedup
when accessed from Python because pybind11 wrapper creation dominates.
These will show their real benefit in Phase 2+ when the lexer stays entirely
in C++ and these types are passed by value within C++ without crossing the
Python boundary.

### LSP server baseline (pre-rewrite)

Captured via `scripts/perf_track.py bench` (3 iterations, median). These are
the end-to-end LSP timings before any native code is wired in, serving as the
reference point for measuring rewrite impact.

**Open-to-tokens (OTT)** — wall-clock from `didOpen` to `semanticTokens/full` response:

| File | Lines | Tokens | OTT | sem_tokens | Diags | Opts |
|---|---|---|---|---|---|---|
| irules_tcp | 139 | 265 | 116 ms | 39 ms | 1 | 1 |
| long_code | 539 | 1,557 | 302 ms | 137 ms | 92 | 18 |
| references | 350 | 368 | 107 ms | 36 ms | 0 | 1 |

**Server-side timing breakdown** (from `[timing]` log lines):

| Phase | irules_tcp | long_code | references |
|---|---|---|---|
| `collect_files` | 25 ms | 26 ms | 26 ms |
| `workspace_state.update` | ~0 ms | ~0 ms | ~0 ms |
| `semantic_tokens_full` | 39 ms | 137 ms | 36 ms |

The `workspace_state.update` shows ~0ms here because the benchmarking client
sends `semanticTokens/full` before the background analysis completes — the
semantic tokens code path handles this gracefully by doing a synchronous
analysis. The total `semantic_tokens_full` time therefore includes the
tokenisation and analysis that would normally be in the background update.

Stored in `perf_history.sqlite3` as version `cpp-phase1-baseline`. Re-run
after each phase to track impact:

```bash
python3 scripts/perf_track.py bench --version "cpp-phaseN-label"
python3 scripts/perf_track.py list
python3 scripts/perf_track.py graph  # generates docs/perf/*.png
```

### Phase 2: Lexer + Token Types

**Lexer tokenisation** (Python `TclLexer` vs C++ `NativeTclLexer`):

| Source | Size | Tokens | Python | C++ | Speedup |
|---|---|---|---|---|---|
| Small (1 line) | 22 B | 6 | 21.5 µs | 2.3 µs | **9.4x** |
| Medium (200 lines) | 2 KB | 606 | 2.25 ms | 175 µs | **12.8x** |
| Complex (800 lines) | 20 KB | 681 | 4.57 ms | 254 µs | **18.0x** |
| Large (10K lines) | 230 KB | 60,000 | 217 ms | 19.6 ms | **11.0x** |

Run with:

```bash
PYTHONPATH=builddir/native:. python3 scripts/bench_lexer.py
```

**Key insights**:
- The C++ lexer delivers 9-18x speedup across all source sizes
- Best speedup (18x) on realistic complex Tcl code — the fast-path
  character scanning in C++ eliminates Python's per-character overhead
- Token counts match exactly between Python and C++ implementations
- The pybind11 boundary cost (creating ~60K Python Token objects for the
  large test) accounts for most of the C++ time; the raw C++ tokenisation
  is significantly faster than what the boundary cost shows
- The real benefit will be seen in Phase 3+ when the segmenter consumes
  tokens entirely in C++ without crossing the Python boundary

**LSP server timings** (Phase 2, stored as `cpp-phase2-lexer`):

| File | Lines | Tokens | OTT |
|---|---|---|---|
| irules_tcp | 139 | 265 | 113 ms |
| long_code | 539 | 1,557 | 252 ms |
| references | 350 | 368 | 99 ms |

Note: These timings are similar to baseline because the native lexer is
not yet wired into the LSP pipeline — it's exposed as `NativeTclLexer` in
the native module but the Python code still uses its own `TclLexer`. The
speedup will be visible once the segmenter and semantic tokens code path
call the native lexer.

### Phase 3: Command Segmenter + Error Recovery

Phase 3 ports the command segmenter and error recovery pipeline to C++.
This is the layer that consumes the flat token stream from `TclLexer` and
groups tokens into per-command `SegmentedCommand` structures at EOL/semicolon
boundaries. It also includes incremental chunking (`TopLevelChunk` with
hash-based dirty tracking) and full E201/E202/E203 error recovery via ghost
token injection (zero-width tokens inserted to close unterminated delimiters).

**New types:**

| Type | Header | Purpose |
|---|---|---|
| `Severity` | `core/diagnostic.hpp` | LSP diagnostic severity (`uint8_t` enum) |
| `CodeFix` | `core/diagnostic.hpp` | Quick-fix suggestion (range + new text) |
| `Diagnostic` | `core/diagnostic.hpp` | Error/warning with optional fixes |
| `UnclosedDelimiter` | `parsing/segmenter.hpp` | Which delimiter was left open |
| `SegmentedCommand` | `parsing/segmenter.hpp` | Single parsed Tcl command |
| `TopLevelChunk` | `parsing/segmenter.hpp` | Source region for incremental analysis |
| `GhostToken` | `parsing/recovery.hpp` | Zero-width token for recovery (called "ghost" to avoid C++ `virtual` confusion) |
| `RecoveryResult` | `parsing/recovery.hpp` | Commands + diagnostics from recovery |

**API surface:**

| Function | Purpose |
|---|---|
| `segment_commands()` | Main entry point: tokenise + segment + optional recovery |
| `segment_top_level_chunks()` | Split source into hashable chunks for incremental re-analysis |
| `find_first_dirty_chunk()` | Pairwise hash comparison between old and new chunk lists |
| `compute_ghost_insertions()` | First-pass detection of missing delimiters (Python shim exposes as `compute_virtual_insertions`) |
| `segment_with_recovery()` | Full two-pass pipeline: detect → inject → re-parse |
| `has_suspicious_token()` | Check last command for unclosed delimiter tokens |
| `find_recovery_offset()` | Scan token text for known command to resume parsing |
| `position_from_relative()` | O(n) newline walk for absolute position from relative offset |

**Error recovery detectors:**

| Code | Condition | Heuristics |
|---|---|---|
| E201 | Unterminated `[` | comment-break, command-break, brace-break, no-heuristic |
| E202 | Unterminated `"` | newline with known command, no-heuristic |
| E203 | Unterminated `{` | de-indented known command, no-heuristic |

**Test coverage (Catch2):**

| Test file | Tests | Scope |
|---|---|---|
| `test_segmenter.cpp` | 15 | Core segmentation (commands, words, comments) |
| `test_segmenter_recovery.cpp` | 17 | Recovery + suspicious token + find_recovery_offset |
| `test_segmenter_chunks.cpp` | 12 | TopLevelChunk + find_first_dirty_chunk |
| `test_recovery_e201.cpp` | 14 | E201 heuristics + is_unterminated_cmd |
| `test_recovery_e202.cpp` | 8 | E202 heuristics + is_suspicious_quote |
| `test_recovery_e203.cpp` | 8 | E203 heuristics + is_suspicious_str |
| `test_recovery_ghost.cpp` | 9 | Ghost token lexer integration + pipeline |
| `test_upstream_parse.cpp` | 23 | Ported from Tcl upstream parse.test |
| **Total** | **106** | |

Plus 127 Python tests (75 segmenter + 52 recovery) passing through pybind11 shim.

**Tri-compiler verification:**

All native C++ code builds clean under Clang 18, GCC 13, and GCC 14 with
`-Werror` and all hardening flags. All compilers pass their sanitiser suites:

| Compiler | Build | ASan+UBSan | TSan | Valgrind |
|---|---|---|---|---|
| Clang 18 | clean | 16/16 | 16/16 | 16/16 |
| GCC 13 | clean | 16/16 | — | — |
| GCC 14 | clean | 16/16 | — | — |

Static analysis — all clean on our code (shim and external libraries excluded):

| Tool | Result |
|---|---|
| clang-tidy | 0 errors (5 NOLINT suppressions) |
| cppcheck | 0 errors |
| clang-format | compliant |
| Clang scan-build | 0 bugs found |
| GCC 14 -fanalyzer | 0 findings |

**pybind11 bindings:**

All Phase 3 types and functions are exposed to Python via the `_tcl_lsp_native`
module. The `core/_native.py` shim conditionally imports them (falls back to
pure-Python when native module is unavailable). Available as:
`NativeSegmentedCommand`, `TopLevelChunk`, `Diagnostic`, `CodeFix`, `Severity`,
`UnclosedDelimiter`, `segment_commands`, `segment_top_level_chunks`,
`find_first_dirty_chunk`, `compute_virtual_insertions`, `segment_with_recovery`,
`position_from_relative`.

**Naming convention — "ghost" vs "virtual":**
The C++ code uses "ghost token" (`GhostToken`, `compute_ghost_insertions`,
`ghost_insertions`) to avoid confusion with C++ `virtual` keyword semantics.
The Python code retains the original "virtual token" naming (`VirtualToken`,
`compute_virtual_insertions`, `virtual_insertions`). The pybind11 shim
translates between the two: C++ `compute_ghost_insertions()` is exposed to
Python as `compute_virtual_insertions()`, and the `ghost_insertions` parameter
is exposed as `virtual_insertions`.

