# AGENTS.md — development guide for AI agents

## Project overview

tcl-lsp is a Tcl Language Server Protocol implementation written in Python
(server) with editor integrations in TypeScript (VS Code), Rust (Zed), and
Gradle/Kotlin (JetBrains). It supports Tcl 8.4–9.0, F5 iRules/iApps, and EDA
tool dialects.

## Repository layout

```
lsp/             Python LSP server runtime and feature wiring
core/            Reusable Tcl parser/compiler/analysis modules
vm/              Bytecode VM, interpreter, and REPL
debugger/        Interactive Tcl debugger (CLI, VM/tclsh/tkinter backends)
editors/vscode/  VS Code extension (TypeScript)
editors/         Other editor integrations (Neovim, Zed, Emacs, Helix, Sublime, JetBrains)
explorer/        Web-based compiler explorer (Pyodide GUI)
rust/            Rust workspace for the incremental Python-to-Rust migration
                 rust/tcl-lexer/    pure Rust crate (no pyo3)
                 rust/tcl-lsp-rust/ PyO3 binding crate → `tcl_lsp_rust` wheel
tests/           Python test suite (pytest)
scripts/         Build and release automation
ai/              AI integrations (Claude skills, MCP server)
samples/         Sample Tcl and iRules code
```

## Prerequisites

- Python 3.10+ with [uv](https://docs.astral.sh/uv/)
- Node.js 20+ with npm
- Rust stable toolchain via [rustup](https://rustup.rs/); the floating
  `channel = "stable"` in `rust-toolchain.toml` is respected automatically

### Version requirements — sources of truth and update checklist

The **source of truth** for each minimum version:

| Requirement | Source of truth              | File                  |
|-------------|------------------------------|-----------------------|
| Python      | `requires-python`            | `pyproject.toml`      |
| Node.js     | CI matrix                    | `.github/workflows/ci.yml` |

When changing a minimum version, update **all** of these locations:

- `pyproject.toml` — `requires-python` and `[tool.ruff]` `target-version`
- `.github/workflows/ci.yml` — `python-version` matrix and `node-version` values
- `Makefile` — Prerequisites comment block at the top
- `AGENTS.md` — Prerequisites section (this file)
- `README.md` — Prerequisites / requirements section
- `editors/vscode/package.json` — `tclLsp.pythonPath` description text
- `editors/jetbrains/README.md` — Python version references
- `editors/neovim/README.md` — Python version in zipapp instructions

## Build system

The project uses GNU Make. Key targets:

| Target             | Purpose                                  |
|--------------------|------------------------------------------|
| `make prep-pr`     | **Fast pre-PR gate** (format + lint + typecheck + fast tests) — run this before every PR |
| `make test-slow`   | Slow tests: VS Code extension tests + smoke tests (zipapp + VSIX) |
| `make test`        | Run all tests (Python + VS Code extension) |
| `make test-py`     | Python test suite only                   |
| `make lint`        | All lint and style checks                |
| `make format-py`   | Auto-fix Python formatting with Ruff     |
| `make compile`     | Compile the TypeScript extension         |
| `make vsix`        | Build the .vsix VS Code extension        |
| `make rust-build`  | Build the `tcl_lsp_rust` wheel with maturin and install it into the uv venv |
| `make rust-test`   | Run `cargo test` on the Rust workspace   |
| `make rust-lint`   | `cargo fmt --check` + `cargo clippy -D warnings` |
| `make rust-format` | Auto-format Rust with `cargo fmt`        |

## Workflow requirements

**When a feature is complete, before suggesting creating a PR, always first
rebase off `main` and fix conflicts then run:**

```
make prep-pr
```

This target auto-formats code and then runs fast checks (no VS Code UI tests,
no smoke tests):

1. **Format** — Auto-fix Python (Ruff) and TypeScript (Prettier) formatting
2. **Lint** — Ruff check + format check + KCS docs validation
3. **Type-check** — ty (Python) + tsc (TypeScript)
4. **Fast tests** — Python pytest suite + optimiser coverage tests

Use `make test-slow` for VS Code extension tests and smoke tests
(zipapp + VSIX packaging).

All checks must pass before a PR is submitted. Do not skip individual steps.
Commit any formatting changes that `make prep-pr` applies before creating the PR.

## Knowledge base and documentation

The project has two kinds of written content with different purposes, tones,
and locations.

- **KCS notes** (`docs/kcs/`) are small, searchable answers to one question
  each, written in plain English for a named audience (user, contributor, or
  maintainer). They are for people who are trying to get something done.
- **Documentation** (`docs/design/`, `docs/GLOSSARY.md`) is technical
  material — design docs, contracts, interfaces, data-structure references,
  architecture narratives. It describes how the system is built and why.
  Technical jargon is allowed.

If you are not sure where something belongs: if it answers one question a
person would ask out loud, it is a KCS note; if it describes how a module is
structured, what its contract is, or what data flows through it, it is a
design doc.

### KCS — the four categories

Every KCS note is exactly one of these four types. Pick the category first,
then copy the matching template from [`docs/kcs/templates/`](docs/kcs/templates/README.md).

| Type | The question it answers | Template |
|---|---|---|
| **Issue** | Why is X not working, and how do I fix it? | [`kcs-template-issue.md`](docs/kcs/templates/kcs-template-issue.md) |
| **Q&A** | What is X? / When should I use Y? | [`kcs-template-qa.md`](docs/kcs/templates/kcs-template-qa.md) |
| **How-To** | How do I do X? | [`kcs-template-how-to.md`](docs/kcs/templates/kcs-template-how-to.md) |
| **Functionality** | What does command/feature/tool X do, and how do I use it? | [`kcs-template-functionality.md`](docs/kcs/templates/kcs-template-functionality.md) |

Every KCS note starts with a blockquote header naming its audience and
type:

```markdown
# KCS: <short title>

> **Audience:** User | Contributor | Maintainer
> **Type:** Issue | Q&A | How-To | Functionality
```

### KCS style rules

1. One KCS note answers **one** core question. If a note answers two
   questions, split it.
2. Name the audience explicitly at the top: **User**, **Contributor**, or
   **Maintainer**.
3. Write in **British English** (`colour`, `optimiser`, `analyse`).
4. Use the **Oxford comma**: "tokens, ranges, and diagnostics" — not
   "tokens, ranges and diagnostics".
5. Prefer short, plain sentences. Avoid long subordinate clauses.
6. **Do not use acronyms or specialist terms** without linking to the
   glossary. On first use within a note, use the plain name and link the
   glossary term: `[control-flow graph](docs/GLOSSARY.md#cfg)`.
7. Use **exact UI labels** when referring to buttons, menus, or commands.
8. Do not inline contract tables, data-structure references, or API
   signatures. Link to the relevant design doc instead.
9. Keep notes **short** — aim for one screen. If longer is required,
   consider whether it should be a design doc.
10. **Name the file after the question, not the implementation.** Use
    `kcs-issue-lsp-features-are-missing.md`, not
    `kcs-issue-vscode-lsp-startup-logs.md`. Functionality, diagnostic,
    and optimisation notes are named around their stable identifier:
    `kcs-feature-rename.md`, `kcs-diagnostic-w210-variable-read-before-set.md`,
    `kcs-optimisation-o105-constant-var-ref-propagation.md`.
11. **Functionality notes must include at least one concrete example**
    — a before/after code block for a transform, a code pointer
    showing where a diagnostic or hover appears, or a screenshot of a
    visual panel.
12. **Every note lists the editors and tools it applies to**, in an
    `## Applies to` section immediately after the audience/type
    header, as a comma-separated plain-text list (not bullets):
    `VS Code, Zed, JetBrains, Neovim, tcl-lsp CLI`. Use `all-editors`
    when the note runs everywhere; the build script expands it to
    the full LSP editor set. The canonical tag vocabulary covers
    editors (`vs-code`, `zed`, `jetbrains`, `neovim`, `helix`,
    `emacs`, `sublime-text`), tools (`tcl-lsp-cli`, `mcp`,
    `claude-skill`, `copilot-chat`), content kinds (`diagnostic`,
    `optimisation`, `warning`, `refactoring`, `analyser`,
    `transform`), and compiler passes (`lexing`, `lowering`, `cfg`,
    `ssa`, `sccp`, `liveness`, `type-infer`, `gvn`, `cse`, `dce`,
    `licm`, `instcombine`, `ipa`, `memssa`, `dataflow`, `taint`,
    `shimmer`, `tail-call`, `code-sinking`, `unused-procs`,
    `side-effects`, `exec-intent`, `rendered-props`, `const-fold`,
    `strength-reduce`, `codegen`). The vocabulary lives in
    [`core/help/kcs_db.py`](core/help/kcs_db.py) and is documented
    in [`docs/kcs/STYLE.md`](docs/kcs/STYLE.md) (rule 11). Per-code
    pages and compiler-internals feature pages must carry the
    compiler-pass tag of the pass that produces the code or the
    facts they consume.
13. **If the answer differs per editor or tool, split it into
    sub-headings** under the answer section, in the same order as
    `## Applies to`. Do not bury per-editor differences in inline
    asides.

For the full style guide with worked examples, see
[`docs/kcs/STYLE.md`](docs/kcs/STYLE.md).

### Documentation (non-KCS)

Design docs, contracts, and interface references live under
[`docs/design/`](docs/design/README.md). A design doc may be long, may use
technical jargon freely, and may include type signatures, contract tables,
ownership matrices, and file-path anchors. One contract per file is the
rule of thumb.

Complex terms go in [`docs/GLOSSARY.md`](docs/GLOSSARY.md). KCS notes link
to the glossary instead of defining terms inline; design docs may either
link or define locally.

### Where things live

| Content kind | Folder | Example |
|---|---|---|
| User/contributor answer to one question | `docs/kcs/` | `kcs-issue-lsp-features-are-missing.md` |
| Feature, command, or tool description | `docs/kcs/features/` | `kcs-feature-rename.md` |
| KCS style guide and templates | `docs/kcs/STYLE.md`, `docs/kcs/templates/` | — |
| Architecture and pipeline walkthroughs | `docs/design/` | `compiler-architecture.md` |
| Compiler pass, stage, or analysis internals | `docs/design/compiler/` | `cfg-construction.md` |
| Module ownership or API contract | `docs/design/contracts/` | `core-lsp-shared-utility.md` |
| Design-doc templates | `docs/design/templates/` | `template-contract.md` |
| Definitions of complex terms | `docs/GLOSSARY.md` | `CFG`, `SSA`, `lattice`, `shimmer` |

### Documentation required for a PR

Any new or changed feature **must** include documentation updates in the
same change:

1. **README.md** — update the relevant section to reflect the new or
   changed behaviour.
2. **KCS note** — create or update a note in `docs/kcs/` using the
   matching template, and add it to the relevant section of
   [`docs/kcs/README.md`](docs/kcs/README.md). For feature changes, update
   the file under [`docs/kcs/features/`](docs/kcs/features/README.md).
3. **Design doc** — if the change introduces or modifies a contract,
   interface, or data-structure, update the relevant file under
   [`docs/design/`](docs/design/README.md) and link it from
   [`docs/design/README.md`](docs/design/README.md).
4. **Glossary** — if the change introduces a new technical term, add it to
   [`docs/GLOSSARY.md`](docs/GLOSSARY.md) with a stable anchor.
5. **Screenshots** — capture screenshots for user-visible changes and
   reference them from the relevant KCS note and `README.md`.

A PR that adds or modifies a feature without these documentation updates
is incomplete and must not be merged.

## Code style

- Python style is enforced by **Ruff** (`make lint-py` / `make format-py`).
- TypeScript style is enforced by **ESLint + Prettier** (`make lint-ts`).
- Use **UK spelling** in identifiers and comments (`normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous single-letter variables outside tiny loops.
- Prefer `match/case` for enum/token dispatch with 3+ branches.
- **Comments** must be plain, minimal, and only present when they illuminate
  something the code itself does not convey. Do not use banner-style comments
  (`# -----------`, `# --- Text ---`, `# -- [section] ------`). Use a plain
  `# Text` comment instead. Never add standalone dash-separator lines.
- See `CONTRIBUTING.md` for the full style guide.

## Editor settings codegen

Whenever a diagnostic or optimisation is added, removed, or changed (code,
severity, message, or section), you **must** regenerate the editor settings
catalogues:

```
make gen-editor-settings
```

This updates the generated diagnostic tables in VS Code, Neovim, Zed, Emacs,
Helix, Sublime, and JetBrains editor integrations. Commit the regenerated files
alongside the diagnostic/optimisation change — CI will fail if they are stale.

## Rust workspace

The project is in the middle of an incremental Python-to-Rust migration that
starts at the lexer and works upward through the compiler and LSP server.
The full chunking strategy, rollout/rollback procedure, and naming
conventions live in `docs/kcs/kcs-rust-migration.md` — read it before
touching anything under `rust/` or the native-extension bits of the zipapp
builder.

The workspace is deliberately split in two:

- **`rust/tcl-lexer/`** — pure Rust crate, no `pyo3` dependency. Data
  structures, lifetimes, error types, and module boundaries are shaped for
  idiomatic Rust, not for mirroring the Python source. Downstream Rust
  consumers (future `tcl-compiler`, `tcl-lsp-server`, a standalone CLI) link
  against this crate directly.
- **`rust/tcl-lsp-rust/`** — PyO3 binding crate. This is the **only** place
  that knows about Python. It owns every `#[pyclass]` wrapper, `PyErr`
  translation, and back-compat shim needed to mimic the current Python API
  surface. The underlying Rust crates stay Python-agnostic.

**Rule: Python compatibility lives only in the binding layer.** If the
Python API demands something awkward (thread-local flags, class-level
state, mutable global configuration) the binding crate implements the
awkwardness and the pure-Rust crate gets a clean `&Config` or equivalent.
Do not plumb Python concerns through the core crate.

**Rule: restructure code when porting, do not transliterate.** The point of
moving to Rust is to benefit from enums, lifetimes, iterators, `Result`,
and zero-copy slices. A port that looks like Python with `;`s added has
missed the point. Reshape data structures, split/merge modules, and revise
names to match how a Rust developer would write the code today. Keep the
binding layer as the adapter.

`editors/zed/` is a pre-existing standalone Rust cdylib targeting
`wasm32-wasip2` and is intentionally **excluded** from the main Cargo
workspace. Leave its lockfile and target directory alone.

`DocumentBuffer` (`core/common/document_buffer.py`) is the per-document
position type. Use it instead of constructing `SourceMap` or calling
`source.split("\n")` in hot paths:

- `DocumentState.buffer` gives the canonical `DocumentBuffer` for an open
  document.
- `buf.lines` replaces `source.split("\n")` (cached, shared).
- `buf.offset_to_position()` / `buf.position_to_offset()` replace `SourceMap`
  construction (O(log n) bisect, no allocation).
- `buf.chunk_line_range()` replaces `_chunk_line_range(source, chunk)`
  (O(log n) instead of O(offset)).
- `position_from_offset()` (`core/common/ranges.py`) replaces
  `position_from_relative()` when a `line_starts` array is available
  (O(log n) instead of O(text_len)).

See `docs/kcs/kcs-core-lsp-shared-utility-contracts.md` for the full contract.

## Command registry

Command metadata lives on `CommandSpec` in `core/commands/registry/models.py`,
**not** in hardcoded sets scattered across consumer modules. Each command is
defined in its own file under `core/commands/registry/{irules,tcl,iapps}/`.

When a consumer needs to know something about a command (e.g. "is this an
action?", "does this mutate state?"), add a boolean field to `CommandSpec`, a
query method to `CommandRegistry`, and set the flag on the relevant command
specs. Do **not** create a `frozenset` of command names in the consumer module.

## Testing

- Test framework: **pytest** (configuration in `pyproject.toml`)
- Tests live in `tests/` — run with `make test-py` or `uv run pytest tests/ -q`
- VS Code extension tests: `make test-ext`
- **iRule test framework** (`core/irule_test/`): simulates TMM for testing iRules
  without hardware.  See `docs/kcs/kcs-irule-test-framework.md` for architecture.
  Codegen: `python -m core.irule_test.codegen_mock_stubs` (after registry changes)
- **xfail policy**: `pytest.mark.xfail` is only permitted as an intermediate
  state while a feature is under active development. Before a feature is
  considered ready for release, all underlying issues must be fixed and the
  xfail markers removed. Do not ship xfails — fix the root cause instead.

## Common tasks

**Fix lint issues automatically:**
```
make format-py
```

**Run just the Python tests:**
```
make test-py
```

**Run just the linters:**
```
make lint
```

**Type-check Python:**
```
make typecheck-py
```
