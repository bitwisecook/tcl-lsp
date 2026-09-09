# Contributing

## Upstream first

Submit fixes and features as pull requests rather than maintaining a private
fork: the AGPL-3.0-or-later already requires derivative works to be published
under the same licence, so upstreaming costs nothing extra and spares both
sides a diverging fork. If a change is specific to an internal environment and
cannot be generalised, open an issue describing the need.

## AI Use

All code in the compiler, analysis, registry, and runtime crates under `rust/`
must be human reviewed before merging.
Front-end code, editor integrations, CI/CD, build pipelines, AI integrations are
all vibe-coded. You may contribute to any area of this project using AI. AI generated code
must come with tests, and I would encourage you to at least use your organic brain
to come up with scenarios for the AI to generate tests to. AI still has a tendency
to cheat, generating bad code and bad tests if left to its own devices. Much like
AI, we tend to take shortcuts ourselves.

You MUST be honest about AI contributions, including the models used;
including the prompts is welcome.

Models used so far: Claude Opus 4.6, Gemini 3.1 Pro, GPT-5.3-Codex.

## Style and formatting

- Rust style is enforced with `cargo fmt` + `clippy` (`-D warnings`), including
  the workspace-enabled `clippy::pedantic`. Run `make check-rust`.
- **Do not add `#[allow(...)]` / `#[expect(...)]` to silence a lint.** Fix the
  cause: `too_many_lines` → extract helpers; `similar_names` → rename;
  `too_many_arguments` → group into a config/options struct. Allow only when
  the lint is genuinely wrong *and* no reasonable refactor exists, with a
  one-line comment saying why. Pre-existing allows are not licence for more.
  The one clear pass: a config/options constructor whose many parameters
  *are* the config — grouping them only makes the API worse.
- Comments are plain and minimal, present only where the code does not
  convey the point. No banner comments (`// -----`, `// --- Text ---`) and no
  standalone separator lines; a plain `// Text` line instead.
- TypeScript style is enforced with ESLint and Prettier. Run `make lint-ts`.
- Python style (`f5report`, the repo skills and scripts, the Sublime plugin) is enforced with Ruff. Run `make lint-py` and `make format-py`.
- Use UK spelling in internal names and comments (for example `normalise`, `optimiser`, `analyse`).
- Keep names explicit; avoid ambiguous one-letter variables outside tiny local loops.
  - Function parameters and return-position variables can be single letters when the type is explicit, the use is local to the function, and the letter means the same thing everywhere in the codebase.
  - Single letters are also acceptable as loop indices (`i`, `j`, `k`) in comprehensions or short `for` loops, or the conventional `_` throwaway.
  - If a single letter would mean two different things (e.g. `d` for both diagnostics and dominator candidates), use a two-letter identifier for the less common meaning. Each file that uses short names must declare them in a comment block near its imports.
- Domain abbreviations are acceptable when established and clear (`cfg`, `ssa`, `uri`).
- Prefer ASCII punctuation in comments/docs for consistency.
- Prefer `match` for enum/token dispatch with 3+ arms; use `if` for simple guards.
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

### Command and word boundaries

Command and word ranges come from the red-green CST through the shared
segmenter; never re-derive them in a consumer — `cargo xtask
segmentation-drift` (in `make rust-check`) fails on a private re-implementation.
See [lexing-segmentation.md](docs/design/compiler/lexing-segmentation.md).


## Documentation style

The project has two kinds of written content, with different rules:

- **KCS notes** (`docs/kcs/`) — small, user-facing answers to one question
  each, written in plain British English, for people trying to get
  something done. Six categories: Issue, Q&A, How-To, Functionality,
  Diagnostic, and Optimisation.
- **Design docs** (`docs/design/`) — technical material describing how the
  system is built: architecture, contracts, interfaces, data-structure
  references. Technical jargon is allowed.

The split, the six categories, and the fourteen style rules live in
[`docs/kcs/STYLE.md`](docs/kcs/STYLE.md); templates are under
[`docs/kcs/templates/`](docs/kcs/templates/README.md) and
[`docs/design/templates/`](docs/design/templates/README.md). Complex terms go
in [`docs/GLOSSARY.md`](docs/GLOSSARY.md): KCS notes link it instead of
defining inline; design docs may do either.

| Content | Folder |
|---|---|
| answer to one user/contributor question | `docs/kcs/` |
| feature, command, or tool description | `docs/kcs/features/` |
| architecture and pipeline walkthroughs | `docs/design/` |
| compiler pass, stage, or analysis internals | `docs/design/compiler/` |
| module ownership or API contract | `docs/design/contracts/` |

### Documentation required for a PR

A PR that adds or changes a feature is incomplete without, in the same
change:

1. **README.md** — the relevant section reflects the new behaviour.
2. **KCS note** — created or updated from the matching template and linked
   from [`docs/kcs/README.md`](docs/kcs/README.md) (feature changes: the file
   under [`docs/kcs/features/`](docs/kcs/features/README.md)).
3. **Design doc** — if a contract, interface, or data structure changed, the
   owning file under `docs/design/` is updated and linked from
   [`docs/design/README.md`](docs/design/README.md).
4. **Glossary** — a new technical term gets a stable anchor in
   `docs/GLOSSARY.md`.
5. **Screenshots** — for user-visible changes, referenced from the KCS note
   and `README.md`.

`cargo xtask kcs-index-links` (in `make rust-check`) fails on an unindexed
note or a broken local link.

When you document compiler behaviour or diagnostics contracts:

- Put the contract, data-structure reference, or pipeline narrative under
  `docs/design/compiler/` and link it from
  [`docs/design/compiler/README.md`](docs/design/compiler/README.md).
- If a contributor workflow or troubleshooting story needs documenting,
  write it as a KCS how-to or issue note under `docs/kcs/` and link it
  from [`docs/kcs/README.md`](docs/kcs/README.md).
- Keep [`docs/design/compiler/architecture.md`](docs/design/compiler/architecture.md)
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

Types and functions with domain-specific names (e.g. `LatticeValue`, `sccp`)
must include a one-sentence explanation of the concept, not just the
implementation.

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

In Python (`f5report`, the repo skills, the Sublime plugin), a
bare `except Exception:` must carry a `log.debug(..., exc_info=True)`:

```python
except Exception:
    log.debug("module_name: short description of what failed", exc_info=True)
    return fallback_value
```

## Command metadata belongs on `CommandSpec`

When code needs to classify commands ("is this a diagram-worthy action?",
"does this always mutate state?", "can this be translated to XC?"), the
metadata lives on the command's `CommandSpec` in `rust/tcl-registry` — a
`Traits` flag in `traits.rs`, or a descriptor field in `spec.rs` when a flag
cannot express it. Never keep `HashSet`/`match` literals of command names in a
consumer crate.

1. **Add the flag or field** (`traits.rs` / `spec.rs`).
2. **Add a query** to `CommandRegistry` in `registry.rs` — a single-command
   predicate such as `is_diagram_action(name)`, and a bulk query where a
   consumer needs one.
3. **Set it** on each relevant spec under `commands/` (`tcl/`, `irules/`,
   `iapps/`, …).
4. **Use the registry** in the consumer.

The registry rule and its descriptor families are in
[AGENTS.md](AGENTS.md#the-registry-is-the-source-of-truth).

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
  `// TODO(author): description` and do not register it in dispatch tables.
- Docstrings must match implementation. If a method's behaviour changes, update
  the docstring in the same commit.

## Module-level state

Avoid mutable module-level state. If global state is necessary (e.g. the
command registry, server singletons), document the initialisation order and
thread-safety expectations in a comment at the definition site. Prefer passing
instances through constructors over importing module globals where practical.

## Source of truth

The product is a Cargo workspace — crate roles in
[project-layout.md](docs/design/contracts/project-layout.md), the authoritative
list in `[workspace] members` of the top-level `Cargo.toml`. The language
server, both CLIs, and the MCP server are cargo bins.

Python is limited to `rust/bigip-report-gen/python` (the `f5report` package
over the native `_engine`), a few repo skills and scripts, and the Sublime Text
plugin; `make lint-py` / `make typecheck-py` cover `git ls-files '*.py'`.

- `make package-vsix` stages the VSIX into an isolated packaging directory under
  `build/`, bundling one native `tcl-lsp-server` binary per platform.
- To point an editor at a working tree, build the server (`make rust-server`) and
  set the client's server-path setting to `target/release/tcl-lsp-server`.

## Dependency audit policy

- Release gating uses `npm audit --omit=dev`; this must remain clean.
- Findings that exist only in `devDependencies` are accepted and are not release-blocking for this project.
- Do not churn dependency updates solely to clear dev-only advisories unless explicitly requested by a maintainer.
