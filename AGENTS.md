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
tests/           Python test suite (pytest)
scripts/         Build and release automation
ai/              AI integrations (Claude skills, MCP server)
samples/         Sample Tcl and iRules code
```

## Prerequisites

- Python 3.10+ with [uv](https://docs.astral.sh/uv/)
- Node.js 20+ with npm

### Claude Code on the web — pre-installed toolchains and sources

The SessionStart hook in [`.claude/hooks/session-start.sh`](.claude/hooks/session-start.sh)
prepares remote sessions (containers where `CLAUDE_CODE_REMOTE=true`) with
the language toolchains and Tcl source trees the repo needs. It runs on
local machines as a no-op, so laptops are never touched. Everything listed
here is ready before Claude starts taking instructions — **no manual
`apt install` or curl step is required**.

| Tool / source    | Version       | Install path                    | On `PATH` as              |
|------------------|---------------|---------------------------------|---------------------------|
| rsync, xz-utils  | distro        | `/usr/bin/`                     | `rsync`, `xz`             |
| Zig              | 0.16.0        | `/opt/zig-0.16.0/`              | `/usr/local/bin/zig`      |
| Wasmtime         | v43.0.1       | `/opt/wasmtime-43.0.1/`         | `/usr/local/bin/wasmtime` |
| rustup + Rust    | stable 1.95.0 | `/root/.rustup`, `/root/.cargo` | `/usr/local/bin/{cargo,rustc,rustup,rustfmt,clippy-driver}` |
| Tcl 8.4 source   | 8.4.20        | `tmp/tcl8.4.20/`                | —                         |
| Tcl 8.5 source   | 8.5.19        | `tmp/tcl8.5.19/`                | —                         |
| Tcl 8.6 source   | 8.6.16        | `tmp/tcl8.6.16/`                | —                         |
| Tcl 9.0 source   | 9.0.3         | `tmp/tcl9.0.3/`                 | —                         |
| tcllib           | 2.0           | `tmp/tcllib-2.0/`               | —                         |

Notes on the fetched sources:

- Tcl and tcllib are full source trees (`generic/`, `unix/`, `win/`, `tests/`,
  `library/`, `doc/`, …) pulled as release tarballs from
  `codeload.github.com`. Tarballs are GitHub-CDN cached, smaller than a git
  clone, and friendlier to the upstream Tcl project than hitting
  `tcl.tk`/`sourceforge.net` on every cold session.
- Zig is fetched via the community mirror pool listed at
  [`community-mirrors.txt`](https://ziglang.org/download/community-mirrors.txt);
  the hook shuffles the pool, falls back to `ziglang.org` as the last resort,
  and verifies the x86_64-linux tarball against the published SHA-256.
- The hook is idempotent — warm containers re-run it and finish in seconds.

To bump any of these versions, edit the pinned variables at the top of
[`.claude/hooks/session-start.sh`](.claude/hooks/session-start.sh)
(`ZIG_VERSION`, `WASMTIME_VERSION`, `RUST_VERSION`, `TCLLIB_TAG` /
`TCLLIB_VERSION`) and, for Tcl, the version/tag maps in
[`.claude/skills/fetch-tcl-source/fetch_tcl_source.sh`](.claude/skills/fetch-tcl-source/fetch_tcl_source.sh).
For Zig, refresh `expected_sha` in the hook to match the new x86_64-linux
tarball's SHA-256 from `https://ziglang.org/download/index.json`.

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

## LSP feature toggles

Most LSP features use a simple runtime guard pattern: the handler is always
registered with `@server.feature(...)`, and the handler body checks
`feature_config.<feature>_enabled` before doing work (returning `None` when
disabled). This allows features to be toggled via `didChangeConfiguration`
without restarting.

A small set of features cannot follow this pattern because their handler
registration changes the `ServerCapabilities` advertised during `initialize`,
which alters client behaviour irreversibly for the session. These are listed
in `_RESTART_REQUIRED_TOGGLES` in `lsp/server.py` and are registered
conditionally at import time. Currently this set includes:

- **`pull_diagnostics_enabled`** — registers `textDocument/diagnostic` and
  `workspace/diagnostic` handlers, which flips `vscode-languageclient` into
  pull mode and disables the push pipeline.

Changing a restart-required toggle at runtime logs a warning but has no
effect until the server process is restarted.

## Lexer token types

The Tcl lexer (`core/parsing/lexer.py`) produces tokens with a `TokenType`
enum. Key conventions that affect downstream consumers:

- **`ESC`** — plain word fragment, possibly containing backslash escapes.
  This is the default type for unbraced, unsubstituted text. Standalone
  punctuation like `}` or `]` appearing outside of their structural role
  (i.e. as stray characters) also receives `TokenType.ESC`.
- **`STR`** — braced string `{...}`.
- **`CMD`** — command substitution `[...]`.
- **`VAR`** — variable substitution `$name` or `${name}`.
- **`SEP`** / **`EOL`** — whitespace separator / end-of-line.

When checking for stray punctuation (`}`, `]`), always check
`tok.type is TokenType.ESC` — not just `tok.text`. A `}` with type `STR`
is a structural brace, not a stray character.

See `docs/design/compiler/lexing-segmentation.md` for the full token type
table and lexer contracts.

## Zig runtime layering

The WASM runtime under `runtime/zig/` is organised as a stack of
small modules, each with one responsibility.  Callers should import
the specific module they need — the old "everything in `tcl_obj.zig`"
shape is dead.  `tcl_obj.zig` still re-exports the migrated symbols
(`obj.is_space`, `obj.list_elem_quote`, …) as a compat layer so older
callers don't break, but new code should go to the canonical module.

| Module | Owns | Reference Tcl 9 analogue |
|---|---|---|
| `tcl_chars.zig` | character classification (`is_space`, `is_scan_space`, `is_bareword`, `is_digit`, …) + byte-span comparators (`str_eq`, `str_cmp`, `mem_eq`) | `tclParse.c` `CHAR_TYPE` / `TclIsSpaceProc` / `TclIsBareword` |
| `tcl_bs.zig` | backslash decoder — `consume_bs_escape` for one ``\x`` escape; `decode_into` for whole-span decode; `encode_utf8` helper | `tclParse.c` `TclParseBackslash` / `Tcl_UtfBackslash` / `TclCopyAndCollapse` |
| `tcl_list_quote.zig` | output side of the list-string contract — `scan_element` + `convert_element` (COMPAT=1), `list_elem_quote` / `list_elem_quote_nth` | `tclUtil.c` `TclScanElement` / `TclConvertElement` / `Tcl_Merge` |
| `tcl_list_parse.zig` | input side — `count_elements`, `element_at`, `copy_unbraced_elem` | `tclUtil.c` `TclFindElement` / `Tcl_SplitList` |
| `tcl_parse.zig` | script / word tokeniser — `parse_command` (flat-array legacy API) and `ParseCommand` (Token-tree API with per-word `braced` flag) | `tclParse.c` `Tcl_ParseCommand` / `Tcl_ParseBraces` / `Tcl_ParseQuotedString` |
| `tcl_obj.zig` | TclObj memory model, type dispatch, `try_parse_int` / `try_parse_bool` | `tclObj.c` |
| `tcl_interp.zig` | eval loop + built-in command dispatch + proc frames (consumer) | `tclBasic.c` / `tclExecute.c` |
| `tcl_dispatch.zig` | host bridge for compiled-proc calls (consumer) | local shim |

A few invariants to preserve when adding features:

- **One canonical implementation per algorithm.**  List-element
  quoting, backslash decoding, whitespace classification — each
  lives in exactly one module.  The "third copy in `tcl_dispatch.zig`"
  bug that stripped newlines from braced proc bodies was exactly the
  kind of hazard this layering exists to prevent.
- **Character classification goes through `tcl_chars.zig`.**  Don't
  spell out `c == ' ' or c == '\t' …` inline — use `chars.is_space` /
  `chars.is_scan_space`.  Likewise for `is_digit`, `is_hex_digit`,
  `is_bareword`.
- **Braced-vs-unbraced is first-class.**  When a parser produces a
  word, callers must get the `braced` flag (either from
  `parse.Token.braced` or, in callers that still use the flat-array
  form, the old `word_braced[i]`).  Losing that flag along the path
  from parse to substitute is what causes ``\{`` / newline bugs in
  proc bodies passed through `uplevel`.

Reference Tcl source is fetched to `tmp/tcl9.0.3/` (and the matching 8.4,
8.5, 8.6 trees) by the SessionStart hook on web sessions — see
[Pre-installed toolchains and sources](#claude-code-on-the-web--pre-installed-toolchains-and-sources).
Locally, run `bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh 9.0`
(or `all`). `tmp/tcl9.0.3/generic/` carries the C parser / util files the
Zig ports mirror.

## Codegen and lowering fallback

Lowering hooks in `core/compiler/lowering_hooks/` convert high-level Tcl
commands into IR nodes. When a hook encounters a construct it cannot
safely specialise (e.g. `{*}` expansion in a structured command, or a
`subst` template with unsupported backslash forms), it **falls through to
the generic `IRCall`** rather than producing incorrect specialised IR.

This fallback-to-runtime pattern is intentional and preserves correctness.
Functions that return `None` to signal "I cannot handle this" (e.g.
`_parse_subst_template()` in `core/compiler/codegen/_helpers.py`) are not
incomplete — they are conservative by design. The runtime interpreter
handles the full Tcl specification; the compiler only inlines what it can
prove is safe.

See `docs/design/compiler/lowering-dispatch.md` for the dispatch hierarchy.

## Position infrastructure

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

### Argument role resolution order

Three mechanisms assign argument roles (BODY, EXPR, VAR_READ, VAR_WRITE, etc.)
to command arguments. They are evaluated in **priority order**:

1. **`arg_role_resolver`** (dynamic) — a callback that inspects the actual
   argument list and returns a role map. Used for variable-arity commands
   where roles depend on argument count or values (e.g. `set` distinguishes
   read vs write by whether a value argument is present; `if` maps bodies
   and expressions by keyword position).
2. **`arg_roles`** (static) — a fixed `dict[int, ArgRole]` on the spec.
   Sufficient when every call has the same argument layout.
3. **`assigns_variable_at`** (legacy shorthand) — marks a single argument
   index as a variable write. Overridden by the dynamic resolver when one
   exists.

The dynamic resolver takes priority over static fields. When reviewing a
command spec that has both `assigns_variable_at` and `arg_role_resolver`,
the resolver is the authority — the static field is a fallback for consumers
that do not call the resolver.

See `docs/design/compiler/command-registry.md` for the full field reference.

### Compound commands and multi-module handling

Tcl compound commands like `namespace upvar`, `namespace eval`,
`dict for`, `string map`, etc. are tokenised as a base command
(`namespace`, `dict`, `string`) with a subcommand argument. Different
analysis passes handle these at different levels:

- **Subcommand dispatch** in the registry uses `SubCommand` entries on the
  parent spec. The `arg_role_resolver` on the parent inspects the
  subcommand word to assign roles.
- **Variable scoping** (`core/analysis/var_scoping.py`) has explicit
  handling for compound forms like `namespace upvar`, `dict set`,
  `dict update`, etc. — these are not in `lsp/features/declaration.py`
  which only handles single-word commands (`global`, `variable`, `upvar`).
- **Lowering** (`core/compiler/lowering_hooks/`) has per-command hooks
  that understand subcommand structure.

When checking whether a compound command is handled, search all three
layers — not just the feature module closest to the symptom.

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
