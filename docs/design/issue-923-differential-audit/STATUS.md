# Issue #923 — Differential Audit & Fix Campaign — STATUS (paused)

**Paused:** 2026-07-20. Written so a fresh Claude Code session (or any engineer)
with zero prior context can pick this up from the repo alone.

**Branch:** `claude/tcl-lsp-issue-923-qzkfqz` (already has 6 commits ahead of
the `2c7693b` base it forked from — 5 fixes + 1 merge from `origin/rust`).

**tl;dr:** A deep differential-audit campaign against real-world Tcl code
found ~66 confirmed LSP correctness bugs so far (22 in tcllib, 39+ across 7
other corpora, more still being found). 7 tcllib bugs are fixed, tested, and
merged into this branch. 14 more tcllib bugs are triaged with fix plans
(8 of them researched in detail). ~56 findings from the other corpora are
not yet even audited (the audit workflow was mid-run when paused). Nothing
is lost — the raw data, the exact scripts that produced it, and everything
needed to resume are in this directory.

---

## 1. The original mandate

Verbatim (this is the standing instruction driving all of this work):

> Dig deep into https://github.com/bitwisecook/tcl-lsp/issues/923 and tell me
> whats missing... verify against georgetree's and nico-robert's projects as
> well as tcllib and tk projects. verify everything is functioning properly.
> Consider tricky Tcl features like namespaces, rename, unknown, aliasing,
> safe-/sub-interpreters, tracing, tricky indirection, tclOO, upvar, uplevel,
> eval, the ::tcl and ::mathop namespaces, source, package loading,
> autoIndex, args in proc definitions and other difficult Tcl surfaces. Use
> plain, idiomatic, modern 2026 rust. This project is not yet in production
> so you can rearchitect and redesign things at will. Each fix must be
> considered deeply and the fixes must be looking at the general case, not
> just hacking in a specific check for the problem. Leverage the full
> compiler stack where possible, making fixes that utilise it to carry
> information correctly and bring information to the deepest layers with the
> richest information. The command registry, and other registries are the
> centralised information, you may add flags, structures, hooks or other
> required information across the registries to enable the compiler to make
> decisions without having that information encoded in the compiler itself,
> it must be fetched from the registry where that's possible and centralises
> information. No putting `if <command name string> then <action>` in the
> compiler/runtime stack. Create relevant TP/FP/TN/FN tests, lsp_e2e tests
> and vscode tests to cover all of this work. Do not add any new clippy
> allows with this work, clippy allows are a code smell and need to be
> properly fixed, the bar for them is very high, it must be shown that they
> are actually improving the readability of the code. C Tcl 9 is the truth
> oracle by default, or the version specific/dialect specific interpreter
> where possible.

Session mode: **Ultracode** was active (Workflow-tool multi-agent
orchestration expected/encouraged, cost not a primary constraint).

A later `/goal complete the exhaustive tests` made this a standing directive
(the session's Stop hook would not let the agent stop until it held) — that
goal is **not yet complete**; this pause is a deliberate user interrupt, not
goal completion.

---

## 2. Methodology established (follow this for all remaining work)

**Three-way differential testing**, per finding:

1. Mine a real, tricky pattern from an actual corpus file (not invented).
2. Build a minimal, faithful repro `.tcl` script.
3. Run it under a **real C Tcl interpreter** — `tclsh9.0` by default (Tcl
   9.0.4, built from source), `tclsh8.6` when version-sensitivity matters —
   to get **ground truth**. Never assume Tcl semantics; verify them.
4. Run the identical file through the built `tcl-lsp-server` via real LSP
   JSON-RPC (see `.claude/skills/lsp-client/lsp_client.py`, and note the fix
   already applied to it below).
5. Diff oracle vs LSP behaviour. Classify: **CONFIRMED** (real bug, provably
   diverges from tclsh), **REFUTED** (LSP is actually correct, or the
   "finding" doesn't reproduce), **PLAUSIBLE** (suspicious but not nailed
   down), **INCONCLUSIVE**.
6. For every CONFIRMED bug that gets fixed: implement it **registry-driven**
   (see §4), add unit tests covering **TP/FP/TN/FN** shapes, add an
   `lsp_e2e` test (real JSON-RPC round-trip, see
   `rust/tcl-lsp-server/tests/preview_tickets_e2e.rs` for ~14 worked
   examples), and a VS Code test where it adds distinct value (fixture under
   `editors/vscode/testFixture/`, test in
   `editors/vscode/src/test/previewTickets.test.ts`).
7. Run the validation gates before committing (see §6) — every commit on
   this branch so far is fully green: `cargo fmt --check`, targeted
   `clippy -D warnings` (zero new warnings, **zero new `#[allow]`**),
   `cargo xtask resolution-drift`, and the full test suite of every touched
   crate.

**Architecture bar** (from the mandate, non-negotiable): no
`if cmd_name == "foo"` string branching in the analyser/compiler. Fixes must
be driven by registry data (`tcl_registry::CommandSpec`/`SubCommand` fields,
hooks, traits) or by analysis **state** already recorded from a previous
pass (e.g. "is this command name a tracked interpreter handle", not "is this
command literally spelled `interp`"). See §5 for the specific shared
mechanisms this campaign already added — reuse them before inventing a new
one.

---

## 3. What's already fixed and merged (this branch)

Six commits ahead of the `2c7693b` fork point, all pushed:

| Commit | Finding(s) | Summary |
|---|---|---|
| `9448af9` | tcllib idx 127 | `param_name_spans_for_token`: braced param-list token span fed straight into the old span-scanner without accounting for the lexer's inner-end convention — every proc/method/lambda parameter after the **first** silently lost its declaration span. |
| `49f4eec` | tcllib idx 107, 115 | New `tcl_compiler::analyser::lookup_var_in_namespace` (namespace-path tree walk, reuses `advance_command_resolution_namespace`) — a fully- or relatively-qualified `$::ns::var` / `$ns::var` reference never resolved at all; only bare names were handled. |
| `37e6886` | tcllib idx 111 | (a) `interp create -safe NAME` never registered `NAME` as a known command (missing leading-option skip in the generic `defines_command_at` consumer — fixed via a new, reusable `CommandSpec`/`SubCommand::leading_option_word_count`). (b) `NAME eval { … }` (the interpreter's own object command) was never recognised as an isolated child-interp body, only literal `interp eval NAME { … }` was. |
| `26e5553` | tcllib idx 118, 119 | `namespace eval $name { … }` and `oo::define $class …` both keyed their scope/ClassDef by the dynamic argument's **raw written text**, so two unrelated call sites using the same variable name collided. Fixed with synthetic per-call-site keys (`@dynns@<offset>` / `@dynclass@<offset>`), mirroring the pre-existing `@interp@<path>` pattern. Also widened a stub `ClassDef`'s `body_span` (was name-token-sized, causing document-symbol range corruption independent of the merge bug). |
| (pre-existing, found+fixed same session before the above, folded into `9448af9`'s predecessor commit — see full history) | — | Nested definitions of a registry builtin (the "rename builtin away, install same-named shadow, restore it" idiom) permanently outranked the real builtin everywhere in the workspace. Gated via new `AnalysisResult::offset_is_inside_any_definition_body`. |
| `4fc2b84` | — | Merge of `origin/rust` (mainline) into this branch. **Not my work** — picked up commit `02386f6` "fix(lsp): resolve superclass/mixin/inherit as class references (issue #923) (#962)" which landed on mainline from a parallel effort on the same issue while this session was running. Merged cleanly, fully re-verified (all test suites, clippy, fmt, drift gate) after merge. |

Run `git log --oneline 2c7693b..HEAD` for the exact list; each commit
message has full rationale.

**Important:** other sessions/PRs are actively landing #923 fixes on
`origin/rust` in parallel (see the `02386f6` merge above). **Before resuming,
`git fetch origin rust` and check `git log origin/rust -20` for anything new,
then `git merge origin/rust` into this branch** (merge, not rebase — this
branch is already pushed) and re-run the full validation gates, exactly as
was done for `4fc2b84`. Expect file overlap in `handlers.rs`, `oo.rs`,
`types.rs`, `definition.rs`, `lib.rs` — a clean auto-merge is **not** proof
of correctness; rebuild and run every touched crate's full test suite before
trusting it.

---

## 4. Shared mechanisms added this campaign (reuse these first)

Before implementing any remaining fix, check whether it can reuse one of
these rather than inventing something new:

- **`tcl_compiler::analyser::lookup_var_in_namespace`**
  (`rust/tcl-compiler/src/analyser/scope.rs`) — given a fully-qualified
  namespace path and a base variable name, walks the whole scope tree
  (namespace can be `namespace eval`-reopened anywhere) and returns the
  `VarDef`. Never matches a proc's own locals even if its command-resolution
  namespace happens to coincide.
- **`CommandSpec::leading_option_word_count` /
  `SubCommand::leading_option_word_count`** (`rust/tcl-registry/src/spec.rs`)
  — given a declared `options: &[OptionSpec]` table and the argument words,
  returns how many leading words are option flags (handles unique-prefix
  matching and value-taking options), so a fixed-index argument consumer
  (`defines_command_at`, positional `arg_types` hints) can skip past
  `?-flag? ?-opt val?` prefixes generically instead of assuming position 0.
- **The `@interp@<path>` / `@dynns@<offset>` / `@dynclass@<offset>` pattern**
  (`rust/tcl-compiler/src/analyser/handlers.rs`) — the general answer to "this
  construct's identity can't be statically resolved to a real Tcl name, and
  two unrelated occurrences must never be treated as the same thing just
  because they look alike." Mint a synthetic key using the `@`-prefixed
  (unrepresentable in real Tcl) marker plus either the interpreter's
  qualified path or the argument token's own byte offset. Already used for:
  child-interpreter domains, dynamic `namespace eval` targets, dynamic
  `oo::define` targets. **Candidate for reuse**: idx 121 (dynamic TclOO
  constructor class — though that one may be better solved by consulting
  `const_contributors`, see its research plan) and potentially others.
- **`AnalysisResult::offset_is_inside_any_definition_body`**
  (`rust/tcl-compiler/src/analyser/types.rs`) — "is this byte offset inside
  any recorded proc/class body", i.e. "does this only exist conditionally,
  at call time, not load time." Used to stop a nested/shadow definition from
  outranking a real builtin or an unconditional top-level one.
- **`isolate_interp_eval_body`** (`rust/tcl-compiler/src/analyser/handlers.rs`)
  — shared body-isolation helper for both literal `interp eval PATH { … }`
  and handle-form `NAME eval { … }`. If a third spelling of "run this script
  in an isolated child-interpreter scope" turns up, extend this, not a new
  parallel implementation.
- **`innermost_namespace_at`** (`rust/tcl-lsp-core/src/definition.rs`) — thin
  wrapper over `tcl_compiler::analyser::command_resolution_namespace_at`,
  the one canonical implementation of "what namespace does an unqualified
  command/qualifier at this byte offset resolve against" (handles the
  `oo::class` method global-only special case, `uplevel #0` reset, deep
  nesting). Empirically verified (this session, against real tclsh) to be
  the *same* rule a variable qualifier resolves against too.

### Gotchas learned the hard way

- **Lexer span convention**: a braced/bracketed/quoted word token's span
  starts *at* the opening delimiter but ends *one byte short* of the closing
  one (`Token.content_offset` + `SourceMap::token_text()`/manual
  `content_offset`-based stripping is the only correct way to get the inner
  content — never hand-slice `source[tok.span]`). This exact bug (idx 127)
  cost real user-visible correctness; it's an easy trap to fall into again.
- **`resolution-drift` xtask gate**: flags *any* new `.name ==` /
  `.name == word`-shaped scan near `all_procs`/`all_classes`, **including in
  test code** — there is no test exemption. A deliberate, reviewed test
  assertion needs a `// drift-ok: <reason>` comment on the line or one of
  the two lines above it (see `two_dynamic_namespace_eval_blocks_...` test
  in `handlers.rs` for a worked example).
- **Disk space**: this environment's writable disk is a **fixed per-session
  allowance** (this session's was ~38G total, shared between `/tmp` and the
  repo checkout). `cargo build`/`test`/`clippy --workspace` pulls in a large,
  unrelated dependency tree (wasmtime, cranelift, an MCP server, a fuzzer, a
  debugger — this is a big workspace, ~37+ crates, not just the LSP). A
  cold full-workspace build alone can consume the **entire** allowance
  (hit ENOSPC twice this session). Mitigations that worked:
  - Prefer scoped builds while iterating: `cargo test -p tcl-compiler -p
    tcl-lsp-core -p tcl-lsp-server` etc., **not** `--workspace`, unless doing
    a final pre-commit sweep.
  - `cargo xtask <gate>` (e.g. `resolution-drift`) only pulls in
    `tcl-registry`+deps — much cheaper than a full build, use it liberally.
  - On ENOSPC: `rm -rf /home/user/tcl-lsp/target` — always safe, it's pure
    build cache, regenerates on next build. This alone freed ~28G both times.
  - `df -h /` before/after big builds to keep an eye on it; the reported
    `Avail` misleads at 0 with the size column still showing plenty — that's
    the allowance, not disk hardware, being exhausted.
- **`Workflow` tool + `args` parameter**: passing a large JSON array via the
  `args:` input parameter failed at runtime (`FINDINGS.map is not a
  function`) despite being passed correctly as a JSON array, not a
  stringified one — root cause not fully diagnosed, but the reliable
  workaround is to **embed the data directly in the script body** as a JS
  literal (`const FINDINGS = [...]`) rather than relying on `args`, then
  invoke via `Workflow({scriptPath: <file-you-wrote-yourself>})`. All four
  scripts in `scripts/` in this directory use (or should be adapted to use)
  this pattern.
- **`tclsh9.0`**: the built Tcl 9.0.4 binary needs `LD_LIBRARY_PATH` set to
  find `libtcl9.0.so` — there's a wrapper at `/usr/local/bin/tclsh9.0` that
  does this. **This wrapper will not exist in a fresh session/container** —
  see §7 for how to recreate the whole oracle environment.
- **`.claude/skills/lsp-client/lsp_client.py`**: fixed this session (commit
  `8a9a7cb`, already on the branch) to answer server-initiated LSP requests
  (`workspace/configuration`, etc.) — without this, `pull_and_apply_config`
  hangs forever and cross-file workspace indexing silently never runs,
  producing convincing-looking but **false** "cross-file resolution is
  broken" bugs. If cross-file LSP behaviour ever looks suspicious again,
  suspect the test harness before the server.

---

## 5. Data inventory (`data/`)

All the raw research output, so nothing has to be re-mined or re-audited
from scratch:

| File | Contents |
|---|---|
| `01-mined-findings-per-corpus.json` | Raw mining output, one entry per corpus (8 corpora), each with its list of mined tricky-pattern candidates. |
| `02-mined-findings-flattened-130.json` | Same data flattened to one list, 130 total candidate findings across all 8 corpora. |
| `03-tcllib-audit-input-25.json` | The 25 tcllib-corpus findings selected for the first audit wave (indices 105–129 in the flattened numbering). |
| `04-tcllib-audit-results-COMPLETE-25of25.json` | **Complete.** All 25 tcllib findings differentially audited: 22 CONFIRMED, 3 REFUTED. This is the source for the tcllib triage in §6. |
| `05-main-audit-input-105.json` | The 105 findings from the other 7 corpora (SpiceGenTcl, argparse, tclopt, ticklecharts, pix, tomato, tk) selected for the second audit wave (indices 0–104). |
| `06-main-audit-results-PARTIAL-49of105.json` | **Partial — 49 of 105 done** (idx 0–48; 49–104 not yet audited). 39 CONFIRMED, 10 REFUTED so far. **None of these 39 have been triaged or fixed yet** — this is entirely open work. |
| `07-remaining-tcllib-findings-14.json` | The 14 tcllib CONFIRMED findings not yet fixed (full detail: summary, failure_scenario, oracle_output, lsp_output, root_cause_hint, repro_path — repro files themselves are gone, scratchpad-only, but the hints are detailed enough to rebuild a repro in minutes). |
| `08-research-plans-PARTIAL-8of14.json` | **Partial — 8 of 14 done.** Refined, current-code-verified fix plans for 8 of the 14 remaining tcllib findings (idx 3, 9, 105, 106, 110, 113, 116, 120), produced by a research-only agent fan-out (no file edits) that re-checked each root-cause hint against the *current* (post-merge) code and proposed concrete changes + test scenarios. idx 18, 24, 121, 122, 125, 128 do not have refined plans yet — use `07`'s `root_cause_hint` field directly for those, which is still quite detailed.

---

## 6. Remaining work, prioritized

### 6a. tcllib — 14 CONFIRMED findings, not yet fixed

All in `data/07-remaining-tcllib-findings-14.json`; 8 have refined plans in
`data/08-research-plans-PARTIAL-8of14.json`. Suggested order (by severity,
then by whether a refined plan exists):

| idx | severity | feature | one-line summary | refined plan? |
|---|---|---|---|---|
| 105 | medium | namespaces | W123 false-positive + bad quickfix for bare `exists`/`get` inside a proc in `::tcl::dict`; separately W001/hover disagree on `dict getnull`. | yes |
| 106 | medium | namespaces | `namespace ensemble create -map {...}` targets never resolved for definition/hover/references. | yes |
| 3 | medium | rename | `rename $old newName` (or any `$`/`[`-containing rename arg) gives up entirely instead of trying constant-folding first. | yes |
| 110 | medium | unknown | `namespace eval $ns [list namespace unknown $handler]` (list-wrapped, not literal) not recognised as installing a namespace-unknown handler → false W123. | yes |
| 113 | medium | aliasing | Any bareword inside a TclOO class body matching a sibling method name resolves to it, even with no `link`/`forward`/`my` making it reachable. | yes |
| 9 | medium | safe_interp | `set mpip [interp create -safe]` (auto-generated name via a variable) never tracked, so `interp alias $mpip ...` cross-domain resolution never fires. | yes |
| 120 | medium | tclOO | A class's own bound command name (`ActiveRecord find ...` calling its own `classmethod`) never resolves — `receiver_instance_class` requires `instance_classes`/`created_instance_commands`, never populated for the class-defining call itself. | yes |
| 116 | low | tracing | `apply {argList body namespaceOverride}`'s runtime namespace override isn't modelled; bareword resolution uses purely lexical nesting. | yes |
| 121 | medium | tclOO | `set class ::Derived; set obj [$class create NAME]` — single-hop variable-indirected class name never resolved (needs `const_contributors`/SSA wiring, or the `@dyn...@` pattern). | no — use root_cause_hint |
| 122 | medium | upvar | W210 false-positive: call-by-name upvar writes from a user proc invoked inside an `if`/`while` **condition** aren't recognised (only 4 hardcoded builtins are, in `cmd_substitution_out_vars`). | no |
| 18 | medium | uplevel | W210 false-positive for `upvar`+`uplevel` custom-control-structure idiom, when the actual upvar is one proc-call hop away from the literal call site. | no |
| 125 | medium | eval | W220 false-positive: `{$var}` inside a double-quoted string mis-tokenized as non-substituting brace-quoted (re-lexing loses quote context). | no |
| 128 | medium | package_loading | `PackageResolver::parse_pkg_index` ignores `if {...} { return }` reachability guards in `pkgIndex.tcl`, over-suppressing W123. | no |
| 24 | medium | autoindex | `hover()` never falls back to the cross-document/autoload resolution tiers that `definition()`/`references()` already use. | no |

Each of these follows the exact same playbook as the 7 already-fixed
findings: read root_cause_hint (and the refined plan if present) → confirm
still-reproduces against current code → check `tclsh9.0`/`tclsh8.6` ground
truth if not already fully confirmed → registry-driven fix reusing §4's
mechanisms where applicable → unit tests (TP/FP/TN/FN) + lsp_e2e test →
validation gates → commit.

### 6b. Main audit wave — 56 of 105 findings not yet even audited

`03-differential-audit-main105-IN_PROGRESS.js` (in `scripts/`) is the exact
workflow script; it was killed mid-run when this session paused (49/105
complete, captured in `data/06-...-PARTIAL-49of105.json`). To resume:
re-invoke it fresh (embed-in-script pattern, see the Workflow gotcha above)
against `data/05-main-audit-input-105.json`, most likely skip re-auditing
idx 0–48 (already done, captured) and only run the remaining idx 49–104 —
or just re-run all 105 if starting a fresh Workflow run is simpler than
splicing partial results back in (the differential audit is not expensive
per-item, mostly agent time).

Once complete, the 39 (and counting) CONFIRMED findings from this wave are
**entirely untriaged** — nobody has looked at severity/priority/clustering
for them yet. Do that first (cluster by root cause the way idx 107+115 and
idx 118+119 were clustered and fixed together this session — look for
repeated root-cause hints across findings before fixing one at a time).

### 6c. Broader mandate coverage check

The mandate named specific "tricky Tcl feature" dimensions. Rough coverage
so far (from mined+fixed findings): namespaces ✓✓✓, rename (open, idx 3),
unknown (open, idx 110; interp-create angle done), aliasing (open, idx 113),
safe-/sub-interpreters ✓ (idx 111 done; idx 9's variable-indirected case
open), tracing (idx 115 done, idx 116 open), tricky indirection ✓✓ (idx
118/119 done), tclOO (open, idx 120/121), upvar (open, idx 122), uplevel
(open, idx 18), eval (open, idx 125), `::tcl`/`::tcl::mathop` namespaces ✓
(idx 127's host procs were the bug, mathop dispatch itself was already
correct), source (not specifically probed yet — consider mining more
`source`-heavy patterns), package loading (open, idx 128), autoIndex (open,
idx 24), proc args ✓ (idx 127). Once the remaining tcllib + main-wave
findings are exhausted, consider a dedicated mining pass for `source` and
`autoIndex` specifically if coverage still looks thin.

---

## 7. Recreating the verification environment

None of this survives a fresh container — re-run before auditing/fixing
anything new:

```sh
# Rust toolchain (need >= 1.97)
rustup update stable

# Tcl 9.0.4 oracle, built from source in-tree (tmp/ is gitignored, this
# does NOT survive a fresh checkout — this is exactly how it was done this
# session, verified from the actual build dir's git remote/tag):
mkdir -p /home/user/tcl-lsp/tmp && cd /home/user/tcl-lsp/tmp
git clone --branch core-9-0-4 https://github.com/tcltk/tcl tcl9.0.4
cd tcl9.0.4/unix && ./configure && make -j
# No `make install` needed/used — run it straight out of the build dir via
# a wrapper (this is the ACTUAL wrapper this session used, verbatim):
cat > /usr/local/bin/tclsh9.0 <<'EOF'
#!/bin/bash
export LD_LIBRARY_PATH="/home/user/tcl-lsp/tmp/tcl9.0.4/unix:${LD_LIBRARY_PATH:-}"
exec /home/user/tcl-lsp/tmp/tcl9.0.4/unix/tclsh "$@"
EOF
chmod +x /usr/local/bin/tclsh9.0

# Tcl 8.6 oracle — Debian package, already provides /usr/bin/tclsh8.6
apt-get install -y tcl8.6

# Reference corpora (only needed if mining NEW findings, not for fixing the
# already-mined ones in data/):
mkdir -p corpora && cd corpora
git clone https://github.com/georgtree/SpiceGenTcl
git clone https://github.com/georgtree/argparse
git clone https://github.com/georgtree/tclopt
git clone https://github.com/nico-robert/ticklecharts
git clone https://github.com/nico-robert/pix
git clone https://github.com/nico-robert/tomato
git clone https://github.com/tcltk/tk
git clone https://github.com/tcltk/tcllib  # tcllib-2.0

# Build the LSP server + CLI once
cd /home/user/tcl-lsp && cargo build -p tcl-lsp-server -p tcl-cli
```

The `.claude/skills/lsp-client/lsp_client.py` skill (already fixed and
committed on this branch) drives the built server over real JSON-RPC for
manual differential checks — see its own docstring for usage
(`python3 lsp_client.py diagnostics|definition|hover|references file.tcl
[line col]`).

---

## 8. Immediate next steps for whoever picks this up

1. `git fetch origin rust && git log origin/rust -10` — check for new #923
   work landed on mainline since `4fc2b84`; merge it in and re-validate if
   so (see §3's note).
2. Recreate the tclsh9.0/8.6 oracle environment (§7).
3. Pick the next tcllib finding from §6a (start with idx 105 or 106 — both
   have refined plans and are medium severity, well-specified).
4. Follow the playbook in §2/§4. Commit after each finding (or tightly
   related cluster of findings), same granularity as the 5 fixes already on
   this branch — don't batch unrelated fixes into one commit.
5. Periodically re-run `cargo test --workspace` (full, not scoped) and
   `cargo clippy --workspace --all-targets -- -D warnings` as a sweep, not
   just the scoped per-crate checks used while iterating — the mandate's
   validation bar is workspace-wide.
6. Once the 14 tcllib findings are exhausted, resume/re-run the main 105-item
   audit wave (§6b), triage its output, and repeat.
7. Keep the Stop-hook's implicit contract: uncommitted work is a liability —
   commit and push after every completed fix, don't let it accumulate.
