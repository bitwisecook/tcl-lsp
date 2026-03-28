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

### Phase 4: Semantic Model — Analyser

Phase 4 ports the semantic analysis layer — the Analyser that consumes
`SegmentedCommand[]` and produces an `AnalysisResult` containing proc
definitions, variable definitions, scopes, diagnostics, regex patterns,
package tracking, command aliases, and unknown-handler analysis.

**Why this layer:** It sits directly above the segmenter in the data-flow
pipeline. Every LSP feature (completion, hover, diagnostics, semantic tokens)
depends on `AnalysisResult`. Porting it to C++ keeps the entire
tokenise→segment→analyse pipeline in C++ without crossing the Python boundary.

#### Architecture

The analyser is designed as idiomatic modern C++ — not a transliteration of
the Python analyser. The initial port was followed by a 13-commit refactoring
pass that replaced Python-shaped patterns with proper C++ idioms. The key
architectural decisions and patterns are documented below as guidance for all
future native code.

##### Ownership: `unique_ptr` trees, not manual `new`/`delete`

Scope tree children use `std::vector<std::unique_ptr<Scope>>`. This
eliminates explicit destructors, custom move constructors, and NOLINT
suppressions. `make_child_scope()` uses `std::make_unique` and returns a
raw non-owning pointer (safe: parent outlives children within a single
`AnalysisResult`).

**Rule:** Never use bare `new`/`delete` in new code. Ownership is always
expressed via `unique_ptr` (single owner) or `shared_ptr` (rare, shared
ownership). Non-owning access uses raw pointers or references.

##### Optional values: `std::optional`, not bool+value pairs

Python's `Optional[T]` with `is not None` checks maps to `std::optional<T>`,
not `bool has_X; T X;` pairs. Examples:
- `Scope::body_range` is `std::optional<Range>` (not `bool has_body_range`)
- `ParamDef::default_value` is `std::optional<std::string>` (not `bool has_default`)
- `AnalysisResult::unknown_proc_info_` is `std::optional<UnknownProcInfo>`

**Rule:** If a value may or may not be present, use `std::optional`. Never
use a bool sentinel with a default-constructed companion.

##### Error reporting: `std::expected`, not bare `std::optional`

When a function can fail for identifiable reasons, use `std::expected<T, E>`
with a typed error enum — not `std::optional<T>` which discards *why* it
failed. Example:

```cpp
enum class StubParseError : std::uint8_t {
    NOT_A_STUB, MISSING_NAME, MISSING_BRACES,
    INVALID_ARG_SYNTAX, INVALID_ROLE, INVALID_EXPR_KIND,
};

auto parse_stub_line(std::string_view line, Range range)
    -> std::expected<StubCommandDef, StubParseError>;
```

**Rule:** Use `std::optional` when absence is normal (lookup miss). Use
`std::expected` when failure has distinct causes that callers may want to
distinguish.

##### Type-safe enums for codes and categories

Diagnostic codes are a `DiagCode` enum class (not stringly-typed):

```cpp
enum class DiagCode : std::uint16_t {
    E001 = 1001,  // builtin arity
    E002 = 1002,  // proc too few args
    W123 = 2123,  // unresolved command
    W214 = 2214,  // unused proc parameter
    // ...
};
```

This gives compile-time completeness checking and eliminates typo bugs.
`to_string(DiagCode)` converts to the string form ("E001", "W123") for
LSP output and pybind11.

**Rule:** When a finite set of string constants is compared in multiple
places, replace with an enum class. Provide `to_string()` for serialisation.

##### Encapsulation: private data with `friend` access

`AnalysisResult` has all data members private, with `friend class Analyser`
for mutation during analysis. External consumers use const accessors
(`diagnostics()`, `all_procs()`, `regex_patterns()`, etc.). This enforces
the rule that only the Analyser builds the result.

**Rule:** Data types consumed by multiple subsystems should have private
members with const accessors. Use `friend` for the single class that
populates them, not public setters.

##### Zero-copy views: `std::span` for contiguous data

`SegmentedCommand::args()` and `arg_tokens()` return `std::span<const T>`
— zero-copy views into the underlying vectors. This eliminates vector copies
on every command dispatch (called 33+ times per command).

**Rule:** When returning a view into owned contiguous data, use `std::span`.
Callers must not outlive the owning container.

##### Command dispatch: static table, not if/elif chain

`process_command` uses two `static const` `unordered_map` tables mapping
command names to member function pointers:

```cpp
using ConsumingHandler = auto(Analyser::*)(const SegmentedCommand&, Scope*) -> bool;
using NonConsumingHandler = void(Analyser::*)(const SegmentedCommand&, Scope*);

static const std::unordered_map<std::string_view, ConsumingHandler> consuming_handlers{...};
static const std::unordered_map<std::string_view, NonConsumingHandler> non_consuming_handlers{...};
```

Consuming handlers return `true` if they fully handle the command (skipping
generic analysis). Non-consuming handlers augment the generic path.

**Rule:** When dispatching on a string to one of N handlers, use a static
table. Encode the dispatch semantics in the type system (consuming vs
non-consuming).

##### Session pattern: stateless Analyser

The Analyser separates immutable configuration (`registry_`,
`disabled_diagnostics_`) from per-analysis transient state via a nested
`Session` struct:

```cpp
struct Session {
    AnalysisResult result;
    Scope* current_scope = nullptr;
    int32_t conditional_depth = 0;
    std::string last_comment;
    AliasResolver alias_resolver;
    bool unresolved_commands_emitted = false;
};
Session s_;
```

Each `analyse()` call resets state with `s_ = Session{}` — a single
assignment that makes it impossible to forget resetting a field. This
clearly separates what survives across calls (config) from what is
per-analysis (session).

**Rule:** When a class accumulates transient state that must be reset
between uses, group it into a session/context struct and reset it
atomically.

##### Embedded state: scope-local data, not external maps

Analysis state that logically belongs to a scope lives in the `Scope`
struct, not in external maps keyed on `Scope*`:

- `Scope::const_strings` — constant string values for regex propagation
- `Scope::regex_vars` — variables known to hold regex patterns
- `Scope::cached_namespace` — lazily computed namespace path

This eliminates fragile pointer-identity maps and makes scope state
self-contained.

**Rule:** If data is conceptually per-scope (or per-node), embed it in the
node struct. External maps keyed on pointer identity are a code smell.

##### Extracted components: `AliasResolver`

Command alias resolution is encapsulated in a nested `AliasResolver` struct
with `register_alias()` and `resolve()` methods. This keeps alias state
and logic cohesive without polluting the main Analyser interface.

**Rule:** When a subset of state + methods forms a natural unit, extract it
as a nested struct or class. Prefer composition over monolithic classes.

##### Generic tree traversal: `visit_scope_tree`

A function template provides pre-order scope tree visitation:

```cpp
template <typename F>
void visit_scope_tree(Scope& root, F&& fn);      // mutable
void visit_scope_tree(const Scope& root, F&& fn); // const
```

Used by `copy_for_snapshot()` to rebuild flat indexes. Available for all
future scope-walking passes (diagnostics, completions, etc.).

**Rule:** If a tree/graph walk pattern appears in 2+ places (or will), add
a generic visitor. Don't duplicate the walk logic.

#### File organisation

When a module grows large, split it into multiple `.cpp` files that share a
single header. This improves readability and compile times without adding
abstraction. The analyser is split three ways:

| File | Purpose | Lines |
|---|---|---|
| `analyser.cpp` | Entry points, body/command iteration, noqa suppression | ~170 |
| `analyser_commands.cpp` | Command dispatch + all command-specific handlers | ~840 |
| `analyser_helpers.cpp` | Variable tracking, scope helpers, naming, diagnostics, expr, AliasResolver | ~540 |

This pattern applies generally: if a `.cpp` file exceeds ~500 lines, consider
splitting it along natural seam lines (e.g. handlers vs helpers vs entry
points). Each file should be cohesive — all functions in a file should relate
to each other.

#### Types

**Phase 4a — semantic value types:**

| Type | Header | Purpose |
|---|---|---|
| `ProcArgTrait` | `semantic_types.hpp` | Bitmask enum for proc parameter usage traits |
| `VarDef` | `semantic_types.hpp` | Variable definition with references |
| `ParamDef` | `semantic_types.hpp` | Proc parameter with `optional<string>` default |
| `ProcDef` | `semantic_types.hpp` | Procedure definition |
| `ScopeKind` | `semantic_types.hpp` | GLOBAL, NAMESPACE, PROC |
| `Scope` | `semantic_types.hpp` | Scope tree node (`unique_ptr` children, embedded const-string/regex state) |
| `visit_scope_tree` | `semantic_types.hpp` | Generic pre-order scope tree visitor (function template) |
| `DiagCode` | `diagnostic.hpp` | Type-safe diagnostic code enum (1xxx=errors, 2xxx=warnings) |
| `RegexPattern` | `auxiliary_types.hpp` | Source range known to contain regex |
| `CommandInvocation` | `auxiliary_types.hpp` | Command word observed during analysis |
| `PackageRequire` | `auxiliary_types.hpp` | `package require` invocation |
| `PackageProvide` | `auxiliary_types.hpp` | `package provide` invocation |
| `SourceTarget` | `auxiliary_types.hpp` | `source` command target path |
| `StubArgDef` | `auxiliary_types.hpp` | Parameter in stub command definition |
| `StubCommandDef` | `auxiliary_types.hpp` | Command stub from structured comment |
| `StubExprDef` | `auxiliary_types.hpp` | Expr function/operator stub |
| `StubParseError` | `stub_parser.hpp` | Typed error enum for stub parse failures |
| `UnknownProcInfo` | `auxiliary_types.hpp` | Analysis of user-defined `unknown` proc |
| `PackageContext` | `auxiliary_types.hpp` | Package confidence levels |
| `AnalysisResult` | `analysis_result.hpp` | Complete document analysis result (encapsulated, `friend Analyser`) |

**Phase 4b — command registry interface:**

| Type | Header | Purpose |
|---|---|---|
| `ArgRole` | `command_interface.hpp` | What role an argument plays (BODY, EXPR, PATTERN, etc.) |
| `Arity` | `command_interface.hpp` | Min/max argument count |
| `CommandSig` | `command_interface.hpp` | Simple command signature |
| `SubcommandSig` | `command_interface.hpp` | Command with subcommands |
| `CommandRegistryInterface` | `command_interface.hpp` | ABC for command metadata |
| `TestCommandRegistry` | `command_interface.hpp` | Minimal test-harness registry |

**Phase 4c–4d — modules:**

| Module | Purpose |
|---|---|
| `stub_parser.hpp/cpp` | Inline stub comment parser; returns `std::expected` |
| `param_list_parser.hpp/cpp` | Tcl parameter list parser |
| `analyser.hpp` + 3 `.cpp` files | Core analyser with Session pattern |

#### Analyser command handlers

| Handler | Tcl command | Notable features |
|---|---|---|
| `handle_proc` | `proc` | Scope creation, param defs, W113 shadow check, W214 unused params, unknown proc detection |
| `handle_set` | `set` | Variable definition (2 args), read (1 arg), const string tracking |
| `handle_variable_decl` | `variable`, `global` | Alternating name/value pairs |
| `handle_namespace_eval` | `namespace eval` | Namespace scope creation |
| `handle_foreach` | `foreach` | Per-variable ranges from var-list |
| `handle_for` | `for` | Init body, test expr, next body, main body |
| `handle_switch` | `switch` | Form 1 (inline pairs), Form 2 (braced body), -regexp regex tracking |
| `handle_catch` | `catch` | Conditional depth, result/options vars |
| `handle_try` | `try` | on/trap/finally handler scanning |
| `handle_if` | `if` | if/elseif/else with optional `then` keywords |
| `handle_while` | `while` | Expression + body analysis |
| `handle_dict` | `dict for` | Variable list + body analysis |
| `handle_interp_alias` | `interp alias` | Namespace-aware alias recording via `AliasResolver` |
| `handle_package` | `package` | require/provide tracking |
| `handle_source` | `source` | Literal vs dynamic path detection |
| `handle_expr` | `expr` | All args analysed as expressions |
| `analyse_body_args` | generic | Registry-based arg role analysis (BODY/EXPR/PATTERN/VAR_NAME) |

#### Diagnostics

Diagnostic codes use the `DiagCode` enum. Numeric encoding: 1xxx = errors,
2xxx = warnings/hints. `to_string(DiagCode)` produces the string form
("E001", "W123") for LSP and pybind11.

| Code | Severity | Description |
|---|---|---|
| E001 | Error | Built-in command arity violation (via registry) |
| E002 | Error | User proc: too few arguments |
| E003 | Error | User proc: too many arguments |
| E101 | Error | Missing open brace (from Python/pybind11) |
| E200 | Error | Missing close delimiter (generic) |
| E201 | Error | Unterminated bracket |
| E202 | Error | Unterminated quote |
| E203 | Error | Unterminated brace |
| E204 | Error | Recovery: extra characters after close-brace |
| E205 | Error | Recovery: extra characters after close-quote |
| E206 | Error | Recovery: missing close-brace for variable name |
| W113 | Warning | Proc shadows built-in command |
| W123 | Hint | Unresolved command (with "did you mean?" suggestions) |
| W214 | Hint | Unused proc parameter |

**Deferred to Phase 5–6:** W210 (read-before-set), W211 (unused variable),
W220 (dead assignment), H300 (possible paste error), I230/I231 (unreachable
branch/arm) — these require CFG/SSA analysis.

#### Memory management

- **Scope tree:** Root owned by `unique_ptr<Scope>` in `AnalysisResult`.
  Child scopes created with `make_unique<Scope>` via `make_child_scope()`,
  pushed into parent's `children` vector. Move-only semantics (copy deleted;
  use `copy_for_snapshot()` for explicit deep copies). Compiler-generated
  destructor recursively destroys the tree via unique_ptr chain.
- **Flat indexes:** `ProcDef*` and `VarDef*` in `all_procs_`/`all_variables_`
  point into scope-owned maps (valid for lifetime of scope tree).
- **Analysis result:** Move-only; deep copy via `copy_for_snapshot()` which
  uses `visit_scope_tree` to rebuild flat indexes into the copied tree.
- **Scope-embedded state:** `const_strings`, `regex_vars`, and
  `cached_namespace` live directly on `Scope`, not in external maps.

#### Test coverage (Catch2)

| Test file | Tests | Ported from |
|---|---|---|
| `test_semantic_types.cpp` | 7 | Type construction + scope tree |
| `test_analysis_result.cpp` | 8 | AnalysisResult methods |
| `test_param_list_parser.cpp` | 4 | Parameter list parsing |
| `test_stub_parser.cpp` | 12 | Stub comment parsing |
| `test_analyser_proc.cpp` | 10 | TestProcAnalysis + TestNamespaceAnalysis |
| `test_analyser_variable.cpp` | 8 | TestVariableAnalysis |
| `test_analyser_control_flow.cpp` | 12 | TestControlFlow |
| `test_analyser_regex.cpp` | 14 | TestRegexPatterns + TestRegexVariablePropagation |
| `test_analyser_package.cpp` | 12 | TestPackageRequire + TestSourceTargets |
| `test_analyser_alias.cpp` | 14 | TestInterpAlias |
| `test_analyser_w123.cpp` | 14 | TestW123UnresolvedCommand |
| `test_analyser_diagnostics.cpp` | 14 | TestDiagnostics + TestUnusedProcParameters |
| **Phase 4 total** | **129** | |
| **Cumulative total** | **248** | |

Plus 8,041 Python tests passing through pybind11 shim (unchanged).

#### Verification (Phase 4d checkpoint)

| Compiler | Build | Tests | ASan+UBSan | TSan | Valgrind |
|---|---|---|---|---|---|
| Clang 18 | clean | 28/28 | 28/28 | 28/28 | 28/28 |
| GCC 13 | clean | 28/28 | 28/28 | 28/28 | — |

Static analysis — all clean:

| Tool | Result |
|---|---|
| clang-tidy | 0 errors (5 NOLINT suppressions) |
| clang-format | compliant |

#### Remaining Phase 4 sub-phases

- 4e: Regex pattern tracking edge cases + package context analysis
- 4f: Full command alias resolution + unknown handler analysis (with IR lowering)
- 4g: Incremental analysis + snapshot/restore
- 4h: Shallow proc arg traits
- 4i: pybind11 bindings + Python integration
- 4j: Arity diagnostics via full registry bridge
- 4k: Upstream Tcl tests (proc.test, namespace.test)
- 4l: Documentation + benchmark

### Direction: Phase 5 and beyond

Phase 4's architecture establishes the patterns for all subsequent phases.
The key constraints going forward:

**The analyser is the foundation.** Every LSP feature (Phase 7) consumes
`AnalysisResult`. The result's encapsulated design (private data, const
accessors, `friend Analyser`) means new fields can be added without
changing the consumer API. New diagnostic passes add new `DiagCode` enum
values and emission sites — the type system enforces that all codes are
declared in one place.

**IR lowering (Phase 5) will consume the scope tree.** The `visit_scope_tree`
utility and the scope's embedded state (const_strings, regex_vars) provide
the interface. The IR module will read the scope tree via const visitors,
not by reaching into Analyser internals.

**CFG/SSA (Phase 6) enables data-flow diagnostics.** W210 (read-before-set),
W211 (unused variable), W220 (dead assignment) require the control flow graph
that Phase 6 builds. These will add new `DiagCode` values and new diagnostic
emission in dedicated pass functions — not by expanding the Analyser's
`process_command` dispatch.

**The Session pattern scales to incremental analysis (Phase 4g).** Because
transient state is isolated in `Session`, incremental re-analysis can
construct a partial session from a snapshot without touching the Analyser's
config. The `analyse_commands()` entry point already accepts pre-segmented
commands for this purpose.

**The `CommandRegistryInterface` ABC enables the Python bridge.** The
virtual dispatch overhead (once per command, not per token) is negligible.
The registry will be backed by the Python `CommandSpec` registry via
pybind11 until the full registry is ported to C++ in Phase 7.

**Architectural invariants for all new native code:**

1. Ownership via `unique_ptr`; no manual `new`/`delete`
2. `std::optional` for maybe-absent values; `std::expected` for failable operations
3. Enum classes for finite code/category sets; `to_string()` for serialisation
4. Encapsulated data types with const accessors and `friend` mutation
5. `std::span` for zero-copy views into contiguous owned data
6. Static dispatch tables for string-based command routing
7. Session/context structs for transient state that resets between uses
8. Scope-embedded state, not external pointer-keyed maps
9. Generic visitors for tree traversal
10. Every new diagnostic code added to the `DiagCode` enum (compile-time checked)

### LSP Library: lsp-framework (forked)

The native LSP server uses a fork of
[leon-bckl/lsp-framework](https://github.com/leon-bckl/lsp-framework) at
[bitwisecook/lsp-framework](https://github.com/bitwisecook/lsp-framework).

**What it is:** A C++20 library that auto-generates all LSP 3.17 types from the
official `metaModel.json` at build time. Zero external dependencies. Type-safe
handler registration using C++20 concepts. Built-in stdio and TCP socket
transports. MIT license.

**Why lsp-framework over alternatives:**

| Library | Decision | Reason |
|---|---|---|
| lsp-framework | **Selected** | C++20, zero deps, auto-generated types from meta model, clean API |
| LspCpp | Eliminated | boost + rapidjson + utfcpp + uri — too many large deps |
| bare-lsp | Eliminated | No license, 8 stars, abseil dep, demo-quality |
| Roll our own | Deferred | Transport/dispatcher are simple but type generation is significant toil; lsp-framework already solved it |

Priority ranking: quality > simplicity/readability > performance. lsp-framework
scored highest on quality (auto-generated types = no spec drift) and simplicity
(zero deps, fluent C++20 API).

#### Quality standard

**The fork must pass the same quality bar as tcl-lsp itself.** It is not
treated as a black-box dependency with suppressed warnings — it is held to
the full tcl-lsp compiler and sanitizer matrix:

| Requirement | Detail |
|---|---|
| **Compilers** | Clang 18, GCC 13, GCC 14 — all with `-Werror` |
| **Warning level** | 3 (all warnings) + hardening flags |
| **Hardening flags** | `-fstack-protector-strong`, `-D_FORTIFY_SOURCE=2`, `-D_GLIBCXX_ASSERTIONS`, `-Wconversion`, `-Wsign-conversion`, `-Wfloat-conversion`, `-Wvla`, `-Wdouble-promotion`, `-Wmissing-field-initializers`, `-Wnull-dereference`, `-Wformat=2`, `-Wunused-result`, `-Wimplicit-fallthrough` |
| **Sanitizers** | ASan+UBSan, ThreadSanitizer, Valgrind memcheck |
| **Static analysis** | clang-tidy (bugprone, modernize, performance, readability, cppcoreguidelines), cppcheck `--enable=all`, scan-build (9 checker categories), GCC `-fanalyzer` |
| **Formatting** | clang-format 18 enforced (upstream style: tabs, Attach braces) |
| **Tests** | Catch2 test suite covering JSON, JSON-RPC, transport, LSP types, MessageHandler, integration |

The fork's `Makefile` provides the same target structure as tcl-lsp (`make test-all`,
`make test-asan`, `make valgrind`, `make scan-build`, `make lint`, etc.).

#### Upstream synchronisation

**The fork must be kept up to date with upstream constantly.** Policy:

1. **Rebase on upstream regularly** — at minimum before each tcl-lsp release,
   and whenever upstream tags a new version. Use `git fetch upstream && git rebase upstream/master`.
2. **Upstream remote configured** — the fork must have `upstream` pointing at
   `leon-bckl/lsp-framework` in addition to `origin` at `bitwisecook/lsp-framework`.
3. **Minimise fork delta** — every fix that isn't tcl-lsp-specific should be
   submitted as a PR to upstream. The goal is zero functional divergence;
   the fork should only carry: Meson build, CI/CD, test suite, quality
   enforcement config files, and the \*BSD/int64 patches (until upstreamed).
4. **No private API forks** — if we need behaviour changes in the library,
   propose them upstream first. Only carry patches that upstream has declined
   or that are in-flight PRs.
5. **Track upstream releases** — when upstream tags a release, the fork should
   incorporate it within one week and verify all quality checks pass.

#### Fork improvements over upstream

1. **Meson build system** — ported from CMake for native tcl-lsp integration;
   pre-generated LSP types checked into `lsp/generated/` with forwarding headers
2. **\*BSD support** — FreeBSD/OpenBSD/NetBSD/DragonFlyBSD added to socket and
   process platform detection (`#if defined(__unix__) && !defined(_WIN32)`)
3. **CI/CD** — GitHub Actions matrix: Linux (GCC 13/14, Clang 18), macOS
   (Apple Clang), Windows (MSVC), FreeBSD (Clang). ASan+UBSan. clang-format check.
4. **Quality enforcement** — AGENTS.md, CLAUDE.md, Makefile (26 targets),
   .clang-format, .clang-tidy, .cppcheck-suppress, full hardening flags in
   meson.build with `-Werror` and `warning_level=3`
5. **Test suite** — Catch2 tests for JSON parser, JSON-RPC, transport,
   LSP type serialization, MessageHandler dispatch, and integration tests
6. **json::Integer → int64_t** — widened from int32_t to handle large request IDs;
   fixed cascading type truncation bugs in serialization.h, messagehandler.cpp
7. **Hardened build fixes** — `process.cpp` unconsumed `::write()` return,
   `socket.cpp` ssize_t→SizeType conversion, `jsonrpc.cpp` GCC false-positive
   `-Wmaybe-uninitialized` suppression (GCC bugzilla #106247)

#### Integration

Meson subproject via `wrap-git` pointing at the fork:

```ini
# native/subprojects/lsp-framework.wrap
[wrap-git]
url = https://github.com/bitwisecook/lsp-framework.git
revision = master
[provide]
lsp-framework = lsp_dep
```

The library builds with its own `-Werror` and hardening flags (not suppressed).
Our server code in `native/src/lsp/` also compiles with full `-Werror`.

#### Phases 7+8 collapse into Phase 7

With lsp-framework providing the protocol layer, LSP features and server
replacement happen together:
- 7a: Integrate lsp-framework subproject, verify `make test-all` passes
- 7b: Server skeleton (lifecycle, document sync, capabilities)
- 7c: Port semantic tokens (performance-critical, first feature)
- 7d: Port remaining features one at a time
- 7e: Remove pygls, lsprotocol, pybind11 shim

