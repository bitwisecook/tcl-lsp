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

All code in the compiler, analysis, registry, and runtime crates under `rust/`
must be human reviewed before merging.
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

- Rust style is enforced with `cargo fmt` + `clippy` (`-D warnings`). Run `make check-rust`.
- TypeScript style is enforced with ESLint and Prettier. Run `make lint-ts`.
- Python style (the remaining `f5report` / skills / Sublime code) is enforced with Ruff. Run `make lint-py` and `make format-py`.
- Use UK spelling in internal names and comments (for example `normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous one-letter variables outside tiny local loops.
  - Function parameters and return-position variables can be single letters when the type is explicit, the use is local to the function, and the letter means the same thing everywhere in the codebase.
  - Single letters are also acceptable as loop indices (`i`, `j`, `k`) in comprehensions or short `for` loops, or the conventional `_` throwaway.
  - If a single letter would mean two different things (e.g. `d` for both diagnostics and dominator candidates), use a two-letter identifier for the less common meaning. Each file that uses short names must declare them in a comment block near its imports.
- Domain abbreviations are acceptable when established and clear (`cfg`, `ssa`, `uri`).
- Prefer ASCII punctuation in comments/docs for consistency.
- Prefer `match/case` for enum/token dispatch with 3+ branches; use `if` for simple guards.
- Prefer `x & 1` over `x % 2` for odd/even checks on integers, and use
  truthiness (`if x & 1:` rather than `if x & 1 == 1:`).
- Put the project copyright/license header on our own original source files
  only: the full AGPL-3.0 notice with `Copyright (C) <year> James Deucker
  (bitwisecook)`, placed after any shebang or coding/`-*-` magic first line.
  Never add it to vendored or third-party code, which keeps its own original
  notices and license (for example `runtime/rust/vendor/` and
  `rust/tcl-regex/tests/data/reg.test`). Also skip generated files, test
  fixtures and golden corpora, and `.github/workflows/*`. See `DUAL-LICENSING.md`
  for the licensing model.

## Code reuse and deduplication

Do not duplicate utility functions across modules. If two or more files need
the same helper, extract it into an appropriate shared module.

- Command specs and the helpers that build them live in `rust/tcl-registry/src/`
  (`spec.rs` for `CommandSpec`, `commands/<dialect>/` for the per-command
  definitions).
- Compiler-internal helpers shared across passes live in
  `rust/tcl-compiler/src/ir_helpers.rs` and
  `rust/tcl-compiler/src/optimiser/helpers/`.
- Before adding a private helper, grep the tree for its body.
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


## Documentation style

The project has two kinds of written content, with different rules:

- **KCS notes** (`docs/kcs/`) — small, user-facing answers to one question
  each, written in plain British English. They are for people trying to
  get something done. There are four categories: Issue, Q&A, How-To, and
  Functionality.
- **Design docs** (`docs/design/`) — technical material describing how the
  system is built: architecture, contracts, interfaces, data-structure
  references. Technical jargon is allowed.

The authoritative split, the four KCS categories, and the nine-rule style
guide all live in [`AGENTS.md`](AGENTS.md) under "Knowledge base and
documentation". The full style guide with worked examples lives in
[`docs/kcs/STYLE.md`](docs/kcs/STYLE.md).

KCS templates are at [`docs/kcs/templates/`](docs/kcs/templates/README.md).
Design-doc templates are at
[`docs/design/templates/`](docs/design/templates/README.md).

When you document compiler behaviour or diagnostics contracts:

- Put the contract, data-structure reference, or pipeline narrative under
  `docs/design/compiler/` and link it from
  [`docs/design/compiler/README.md`](docs/design/compiler/README.md).
- If a contributor workflow or troubleshooting story needs documenting,
  write it as a KCS how-to or issue note under `docs/kcs/` and link it
  from [`docs/kcs/README.md`](docs/kcs/README.md).
- Keep [`docs/design/compiler-architecture.md`](docs/design/compiler-architecture.md)
  as orientation, diagrams, and links — not deep implementation policy.

If a PR changes compiler fact contracts, update at least one relevant
design doc and mention the update in the PR description.

### Review checklist for compiler fact-contract changes

When a PR changes compiler behaviour, diagnostics contracts, or
pass-produced facts, reviewers should explicitly ask:

- Did this change alter a compiler fact contract?
- If yes, which `docs/design/compiler/` doc was updated?
- If a new compiler design doc was added, is it linked from both
  [`docs/design/compiler/README.md`](docs/design/compiler/README.md) and
  the top-level [`docs/design/README.md`](docs/design/README.md)?

## Compiler pipeline

The compiler pipeline transforms source through several stages. Each module's
docstring should explain:

1. What the module computes and why.
2. Key domain terms -- target audience is a senior engineer who has not
   written a compiler. For example, explain what SSA is, what a lattice value
   represents, or why a barrier node exists.
3. How the module fits into the pipeline (what feeds it, what consumes its
   output).

The stages are:

```
Source -> Lexer (rust/tcl-lexer/src/lexer.rs)
      -> CST (rust/tcl-syntax/)
      -> IR Lowering (rust/tcl-compiler/src/lowering/)
      -> CFG Construction (rust/tcl-compiler/src/cfg.rs)
      -> SSA Construction (rust/tcl-compiler/src/ssa.rs)
      -> Core Analyses: SCCP, liveness, dead stores
         (rust/tcl-compiler/src/analyses.rs, dead_stores.rs, …)
      -> Diagnostics / codegen (rust/tcl-compiler/src/codegen/)
```

Individual classes and functions with domain-specific names (e.g. `IRBarrier`,
`LatticeValue`, `_sccp`) must include a one-sentence explanation of the concept,
not just the implementation.

## Swallowed errors must still be logged

A recovered error that leaves no trace makes production debugging extremely
difficult.  Whenever a fallback hides a failure, log it so operators can still
see what happened.

In Rust, an `Err` arm (or an `unwrap_or_default()`-style fallback on a fallible
call) that discards the error should say what failed first.  There is no `log` /
`tracing` dependency in the workspace: inside the LSP server, report through the
client with `client.log_message(MessageType::LOG, …)`; elsewhere (CLIs, xtask)
write to stderr with `eprintln!`.

```rust
let cfg = match load_config(path) {
    Ok(c) => c,
    Err(e) => {
        self.client
            .log_message(MessageType::LOG, format!("config unreadable, using defaults: {e}"))
            .await;
        Config::default()
    }
};
```

In the remaining Python (`f5report`, the Claude skills, the Sublime plugin), a
bare `except Exception:` must carry a `log.debug(..., exc_info=True)`:

```python
except Exception:
    log.debug("module_name: short description of what failed", exc_info=True)
    return fallback_value
```

## Command metadata belongs on `CommandSpec`

When code needs to classify commands (e.g. "is this a diagram-worthy action?",
"does this always mutate state?", "can this be translated to XC?"), the metadata
must live as a field on `CommandSpec` in `rust/tcl-registry/src/spec.rs`.

**Do not** create hardcoded `HashSet`/`match` literals of command names in
consumer crates. This scatters knowledge about commands across the codebase and
makes it easy for new commands to be missed.

The pattern to follow:

1. **Add a field** to `CommandSpec` (default `false` / `None`).
2. **Add query methods** to `CommandRegistry` in
   `rust/tcl-registry/src/registry.rs` — a single-command predicate (e.g.
   `is_diagram_action(name)`) and a bulk query where consumers need one.
3. **Set the flag** on each relevant command spec under
   `rust/tcl-registry/src/commands/` (`tcl/`, `irules/`, `iapps/`, …).
4. **Use the registry** in the consumer crate instead of a local set.

Existing examples of this pattern: `pure`, `commits_response`,
`http_namespace`, `diagram_action`, `drops_connection`, `always_mutating`,
`output_only`, `http_setter_by_arity`, `mutator_subcommands`,
`xc_translatable`.

## Body identification and command argument roles

The canonical source for identifying body, expression, and pattern argument
indices is `CommandRegistry` in `rust/tcl-registry/src/registry.rs` via
`arg_indices_for_role()` and `plain_body_arg_indices()`.  Other modules
(including the formatter) delegate to these rather than duplicating the
argument-walking logic.

If the formatter needs to restrict which bodies are expanded (e.g. the `for`
command only expands its main body, not `init`/`next`), add a
formatter-specific override under `rust/tcl-lsp-core/src/formatting/` before the
general delegation call.

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

## Source of truth

The product is a Cargo workspace — see [`AGENTS.md`](AGENTS.md) "Repository
layout" for the crate roles, and `[workspace] members` in the top-level
`Cargo.toml` for the authoritative list. The language server, both CLIs, and the
MCP server are all cargo bins; there is no Python in the shipped server.

Python remains only in `rust/bigip-report-gen/python` (the `f5report` package
backed by the native `_engine`), the Claude skills under `.claude/skills/`, and
the Sublime Text plugin. `make lint-py` / `make typecheck-py` cover exactly that
set (`git ls-files '*.py'`).

- `make package-vsix` stages the VSIX into an isolated packaging directory under
  `build/`, bundling one native `tcl-lsp-server` binary per platform.
- To point an editor at a working tree, build the server (`make rust-server`) and
  set the client's server-path setting to `target/release/tcl-lsp-server`.

## Dependency audit policy

- Release gating uses `npm audit --omit=dev`; this must remain clean.
- Findings that exist only in `devDependencies` are accepted and are not release-blocking for this project.
- Do not churn dependency updates solely to clear dev-only advisories unless explicitly requested by a maintainer.
