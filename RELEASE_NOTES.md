# v1.9.0

## New Features

- New **`tcl f5`** CLI suite for working with BIG-IP configurations. The
  initial drop ships 17 top-level verbs plus 5 `irule` sub-verbs covering the
  full operator lifecycle:
  - **Acquire**: `fetch` (pull SCF/UCS from a live BIG-IP via REST or SSH),
    `extract` / `ucs2scf` (unpack a local UCS archive into SCF).
  - **Analyse**: `stats`, `graph` (DOT / JSON / Mermaid dependency graphs),
    `explain`, `diff` (semantic, ignores ordering and iRule whitespace),
    `validate` / `lint` (with a built-in rule registry, `irule lint`,
    SARIF output), `cleanup` (find and remove unreferenced objects), and
    `grep` (find every object related to a name, regex, or CIDR).
  - **Transform**: `rename`, `redact` (stable, reversible IP redaction with
    a sidecar map and CIDR-relationship preservation), `unredact`,
    `pcap-remap` (apply a redaction map to a PCAP / PCAPNG capture, including
    F5 trailer rewrites), `tmsh` / `scf2tmsh`, `split`, `merge`, `convert`.
  - **Round-trip**: `pull`, `push` (single-object iControl REST round-trip),
    plus `irule trace` and `irule extract` for iRule-focused inspection.
- New **`f5 explain-flow`** and **`f5 explain-pcap`** verbs trace a request
  or PCAP flow through the BIG-IP configuration — virtuals, profile chains,
  iRule decisions, LTM policy decisions, pools, and members — and narrate
  the path. Ships alongside an installable Claude skill at
  `ai/claude/skills/explain-flow/` that shares its narration prompt with
  the CLI.
- New **`f5 enrich-pcapng`** and **`f5 enrich-wireshark`** verbs annotate
  packet captures with BIG-IP object metadata.
- New **LTM policy parser and evaluator** — parses `ltm policy` stanzas
  into a structured model and evaluates rules against captured request
  state, with first-match / all-match / best-match strategies and a
  reasonable operand surface (HTTP host/URI/method/header, SNI, TCP
  address) and action surface (forward, redirect, URI replace, header
  insert/remove, TCP reset).
- New **`tcl completion`** verb emits bash / zsh / fish completion scripts
  for the `tcl` CLI, with a `--hint` flag for the install snippet.
- New **`string insert`** command and relative-qualified ensemble dispatch
  (`tcl::string::reverse` etc. now work without a leading `::`).
- **Tier-5 parity for `string is` and `format`** brings the WASM runtime up
  to Tcl 9.0 semantics: full class table for `string is` (including
  `dict`, `list`, `wordchar`, `entier`, `true`, `false`, prefix matching,
  `-strict`, `-failindex`); full `format` specifier surface (`#`, ` `,
  `b`, `u`, `p`, `h`, `hh`, `%c` Unicode, positional `%n$`, alt-form hex,
  Tcl-style `0` / `-` flag interaction, missing-arg / mid-spec / mix
  errors).

## Improvements

- Closed all 77 bytecode-identity xfails: proc lifting, `switch` lowering,
  and the rest of the codegen pipeline now match `tclsh 9.0` byte-for-byte
  across the reference snippet suite.
- WASM codegen now supports **dynamic variable names** — `set ::$n v`,
  `incr ::$n`, `lappend $local x` and friends route through the runtime
  variable resolver instead of being collapsed to a literal slot.
- Five Tier-2 WASM trap stems (`parse`, `error`, `cmdAH`, `cmdIL`, `list`)
  now run end to end, adding 4316 newly-passing tcltest cases against the
  Tcl 9 baseline (cmdAH alone contributes 3885).
- `test-slow` now provisions missing host dependencies (`tclsh 8.6` /
  `9.0`, Node / npm, `kotlinc`) idempotently across Debian/Ubuntu, RHEL
  family, and macOS, captures missing Tcl 9.0 bytecode reference files,
  and fetches dependencies in parallel.
- The MCP server (`ai/mcp/tcl_mcp_server.py`) exposes the new `f5` verbs
  to AI-assisted workflows.

## Bug Fixes

- **Per-workspace-folder configuration now actually applies (#407).**
  Previously the active dialect lived in a process-wide singleton, so a
  multi-folder workspace with each folder configuring its own
  `tclLsp.dialect` could only honour one value — the server picked one
  folder and emitted a "Per-folder dialects are not yet supported"
  warning for the rest. The dialect (and `extraCommands`,
  `style.nonAscii`, `libraryPaths`, `dialect_explicitly_set`) are now
  resolved per-document via the folder's `FeatureConfig`, with each LSP
  request handler opening a scoped runtime profile. Diagnostics,
  completion, hover, signature help, formatting, semantic tokens, the
  W108 non-ASCII check, and `package require` resolution all honour the
  folder's settings instead of a shared global. The warning loop in
  `lsp/settings.py` is gone.

- `return -code error` / compiled `INST_RETURN_IMM` — corrected the stack
  contract so the result value is read from `OBJ_UNDER_TOS` and the options
  dict from `OBJ_AT_TOS`, matching `tclsh`. Fixes empty-message errors
  from compiled `error` calls.
- `{*}` expansion inside command substitutions no longer drops arguments.
- `for` and `mathop` honour next-clause `break` / `continue` control codes,
  and shift-count validation matches the `INT_MAX` cap that Tcl 9 enforces
  in `INST_LSHIFT` / `INST_RSHIFT`.
- Array-reference codegen — bare `$arr(idx)` with a literal index now
  normalises to `${arr(idx)}` so it compiles to an array load, while
  substituted-index forms (`$a(\$cmd)`) still round-trip correctly.
- Indirect `set $expr value` now evaluates the variable name at runtime
  instead of being treated as a literal.
- `namespace origin` and `namespace forget` now match Tcl 9 semantics:
  `origin` walks the imported-command redirect chain to its source FQN,
  and `forget` properly tombstones the importing namespace's command
  table so a follow-up `rename` no longer fails with "command already
  exists".
- Variable-trace callbacks now receive the full op word (`read` / `write`
  / `unset` / `array`) instead of the internal single-character code, and
  callbacks that capture `$op` into long-lived variables no longer dangle
  into freed parser buffers.
- `info locals` returns the active proc frame's local names (it
  previously fell through to the empty-string fallback).

## Breaking Changes

- The `irule` CLI binary alias is removed. iRules-specific verbs
  (`event-order`, `event-info`) move to `f5 irule …` sub-verbs and default
  to the `f5-irules` dialect; generally useful verbs (`command-info`,
  `convert`, `help`, `diff`, the core analysis verbs) stay on `tcl` and
  continue to accept `--dialect f5-irules` for iRules input.
