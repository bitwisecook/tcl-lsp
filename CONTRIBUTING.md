# Contributing

## Upstream first

This project is licensed under the AGPL-3.0-or-later. If you fix a bug, add a
feature, or improve performance, please submit your changes as a pull request to
this repository rather than maintaining a private fork.

The AGPL already requires that derivative works are published under the same
license, so upstreaming your changes costs nothing extra and benefits everyone:
you get ongoing maintenance from the project, and the community gets your
improvements. Private forks that diverge over time become a burden for both
parties.

If your change is specific to an internal environment and genuinely cannot be
generalised, please open an issue describing the need so we can discuss how to
accommodate it in a way that works upstream.

## AI Use

All code in `core/` and `lsp/` must be human reviewed before merging to main.
Front-end code, editor integrations, CI/CD, build pipelines, AI integrations are
all vibe-coded. You may contribute to any area of this project using AI. AI generated code
must come with tests, and I would encourage you to at least use your organic brain
to come up with scenarios for the AI to generate tests to. AI still has a tendency
to cheat, generating bad code and bad tests if left to its own devices. Much like
AI, we tend to take shortcuts ourselves.

You MUST be honest about AI contributions including models used. It would be nice
to include prompts used so people can see what you did. Yes, I know I'm aware that
I didn't think about that from the start and include mine.

Models used so far: Claude Opus 4.6, Gemini 3.1 Pro, GPT-5.3-Codex.

## Style and formatting

- Python style is enforced with Ruff. Run `make lint-py` and `make format-py`.
- TypeScript style is enforced with ESLint and Prettier. Run `make lint-ts`.
- Use UK spelling in internal names and comments (for example `normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous one-letter variables outside tiny local loops.
  - Function parameters and return-position variables can be single letters when the type is explicit, the use is local to the function, and the letter means the same thing everywhere in the codebase.
  - Single letters are also acceptable as loop indices (`i`, `j`, `k`) in comprehensions or short `for` loops, or the conventional `_` throwaway.
  - If a single letter would mean two different things (e.g. `d` for both diagnostics and dominator candidates), use a two-letter identifier for the less common meaning. Each file that uses short names must declare them in a comment block near its imports.
- Domain abbreviations are acceptable when established and clear (`cfg`, `ssa`, `uri`).
- Prefer ASCII punctuation in comments/docs for consistency.
- Prefer `match/case` for enum/token dispatch with 3+ branches; use `if` for simple guards.

## Code reuse and deduplication

Do not duplicate utility functions across modules. If two or more files need
the same helper, extract it into an appropriate shared module.

- Identifier helpers live in `core/common/naming.py`.
- Command registry helpers live in `core/commands/registry/_base.py`.
  The `make_av(source)` factory there returns an `_av` closure bound to a
  documentation source string.  Use `_av = make_av(_SOURCE)` at module level
  in command definition files rather than defining a per-file `_av` function.
- Before adding a private helper prefixed with `_`, grep the tree for its body.
  If it already exists elsewhere, extract it into a shared location rather than
  copying it.

### Command parsing protocol

Several modules walk Tcl token streams using a shared accumulation pattern.
The three-list contract is:

| Variable | Type | Meaning |
|---|---|---|
| `argv` | `list[Token]` | First token of each whitespace-separated word |
| `argv_texts` | `list[str]` | Concatenated text of each word (may span multiple tokens) |
| `all_tokens` | `list[Token]` | Every token in the command span |

`argv[0]` / `argv_texts[0]` is the command name. `argv[1:]` / `argv_texts[1:]`
are arguments. The accumulator flushes on `TokenType.EOL`. When a token follows
`SEP` or `EOL`, it starts a new word; otherwise it is concatenated onto the
current word.

If you need this pattern, check whether an existing loop
(`compiler_checks._CompilerCheckRunner._process_text`, the analyser's main
loop, or the semantic-token emitter) already covers your use case before adding
another copy.


## Documentation style (KCS-first)

Project documentation should prefer small, searchable KCS notes over large monolithic narrative sections when the content is operational or high-churn.

When documenting compiler behaviour or diagnostics contracts:

- Add/update a focused note under `docs/kcs/compiler/` and link it from `docs/kcs/compiler/README.md`.
- Keep `docs/compiler-architecture.md` as orientation + diagrams + links, not deep implementation policy text.
- Use the templates in `docs/kcs/templates/` for consistent structure.

Each KCS note should include:

1. symptom,
2. operational context,
3. decision rules/contracts,
4. file-path anchors,
5. failure modes,
6. test anchors,
7. discoverability links.

If a PR changes compiler fact contracts, update at least one relevant KCS note and mention that update in the PR description.



### Review checklist for compiler fact-contract changes

When a PR changes compiler behaviour, diagnostics contracts, or pass-produced facts, reviewers should explicitly ask:

- Did this change alter a compiler fact contract?
- If yes, which `docs/kcs/compiler/` note was updated?
- If a new compiler KCS note was added, is it linked from both compiler and top-level KCS indexes?

## Compiler pipeline

The compiler pipeline transforms source through several stages. Each module's
docstring should explain:

1. What the module computes and why.
2. Key domain terms -- target audience is a senior Python engineer who has not
   written a compiler. For example, explain what SSA is, what a lattice value
   represents, or why a barrier node exists.
3. How the module fits into the pipeline (what feeds it, what consumes its
   output).

The stages are:

```
Source -> Lexer (parsing/lexer.py)
      -> IR Lowering (compiler/lowering.py)
      -> CFG Construction (compiler/cfg.py)
      -> SSA Construction (compiler/ssa.py)
      -> Core Analyses: SCCP, liveness, dead stores (compiler/core_analyses.py)
      -> Diagnostics
```

Individual classes and functions with domain-specific names (e.g. `IRBarrier`,
`LatticeValue`, `_sccp`) must include a one-sentence explanation of the concept,
not just the implementation.

## Exception handling and debug logging

Bare `except Exception:` handlers must include a `log.debug(...)` call with
`exc_info=True` so failures are visible at debug log level.  Follow the
pattern established in `workspace/scanner.py` and `workspace/package_resolver.py`:

```python
except Exception:
    log.debug("module_name: short description of what failed", exc_info=True)
    return fallback_value
```

Every module that catches exceptions needs a logger:

```python
import logging
log = logging.getLogger(__name__)
```

Silent swallowing makes production debugging extremely difficult.  The debug
level keeps normal output clean while still letting operators diagnose issues
with `--log-level DEBUG`.

## Command metadata belongs on `CommandSpec`

When code needs to classify commands (e.g. "is this a diagram-worthy action?",
"does this always mutate state?", "can this be translated to XC?"), the metadata
must live as a field on `CommandSpec` in `core/commands/registry/models.py`.

**Do not** create hardcoded `frozenset` or `set` literals of command names in
consumer modules. This scatters knowledge about commands across the codebase and
makes it easy for new commands to be missed.

The pattern to follow:

1. **Add a field** to `CommandSpec` (default `False` or `None`).
2. **Add query methods** to `CommandRegistry` — a single-command predicate
   (e.g. `is_diagram_action(name)`) and a bulk query
   (e.g. `diagram_action_commands()`).
3. **Set the flag** on each relevant command spec in
   `core/commands/registry/{irules,tcl,iapps}/`.
4. **Use the registry** in the consumer module instead of a local set.

Existing examples of this pattern: `pure`, `commits_response`,
`http_namespace`, `diagram_action`, `drops_connection`, `always_mutating`,
`output_only`, `http_setter_by_arity`, `mutator_subcommands`,
`xc_translatable`.

## Body identification and command argument roles

The canonical source for identifying body, expression, and pattern argument
indices is `commands/registry/runtime.py` via `body_arg_indices()`,
`expr_arg_indices()`, and `arg_indices_for_role()`.  Other modules (including
the formatter) delegate to these functions rather than duplicating the
argument-walking logic.

If the formatter needs to restrict which bodies are expanded (e.g. the `for`
command only expands its main body, not `init`/`next`), add a
formatter-specific override in `formatting/engine.py` before the general
delegation call.

## Dead code and docstring accuracy

- Remove dead code promptly. Do not leave stub functions, no-op registrations
  in dispatch tables, or unused helpers.
- If a function is planned but not yet implemented, mark it with
  `# TODO(author): description` and do not register it in dispatch tables.
- Docstrings must match implementation. If a method's behaviour changes, update
  the docstring in the same commit.

## Module-level state

Avoid mutable module-level state. If global state is necessary (e.g. the
command registry, server singletons), document the initialisation order and
thread-safety expectations in a comment at the definition site. Prefer passing
instances through constructors over importing module globals where practical.

## Python source of truth

The Python source of truth is split between `lsp/` (LSP runtime) and `core/`
(reusable parser/compiler/analysis modules).

- The extension package does not keep a mirrored Python tree under `editors/vscode/`.
- `make vsix` stages bundled zipapp artifacts into an isolated packaging directory under `build/`.
- If you need to point VS Code at a working tree directly, set `tclLsp.serverPath` to the repo root.

## Dependency audit policy

- Release gating uses `npm audit --omit=dev`; this must remain clean.
- Findings that exist only in `devDependencies` are accepted and are not release-blocking for this project.
- Do not churn dependency updates solely to clear dev-only advisories unless explicitly requested by a maintainer.
