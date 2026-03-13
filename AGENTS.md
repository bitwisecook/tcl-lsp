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

## Documentation requirements

Any new or changed feature **must** include documentation updates in the same
change:

1. **README.md** — Update the relevant section of `README.md` to reflect the
   new or changed behaviour. Add new entries to feature lists, update usage
   examples, and revise any affected descriptions.
2. **KCS documents** — Create a new KCS note or update an existing one in
   `docs/kcs/` to cover the operational detail of the feature. Follow the
   existing KCS format and register new notes in the KCS index
   (`docs/kcs/README.md`).
3. **Screenshots** — Capture screenshots demonstrating the feature in action
   and add them to the repository. Reference them from `README.md` and/or the
   relevant KCS document. Screenshots should show realistic usage and clearly
   illustrate the feature's behaviour.

A PR that adds or modifies a feature without these documentation updates is
incomplete and must not be merged.

## Code style

- Python style is enforced by **Ruff** (`make lint-py` / `make format-py`).
- TypeScript style is enforced by **ESLint + Prettier** (`make lint-ts`).
- Use **UK spelling** in identifiers and comments (`normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous single-letter variables outside tiny loops.
- Prefer `match/case` for enum/token dispatch with 3+ branches.
- See `CONTRIBUTING.md` for the full style guide.

## KCS documentation

- KCS is the default documentation style for this project: prefer small, searchable KCS notes over large monolithic docs for operational guidance and contracts.
- Start with the KCS index: `docs/kcs/README.md`.
- For compiler internals and pass/fact contracts, use: `docs/kcs/compiler/README.md`.
- When changing behaviour in covered areas, update the relevant KCS note in the same change.

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
