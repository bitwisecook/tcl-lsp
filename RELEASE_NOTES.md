# v1.5.0

## New Features

- **Optimisation profiles** — Five named profiles (`off`, `readability`, `standard`, `full`, `aggressive`) control which optimiser passes run. The editor defaults to `readability` for real-time diagnostics; explicit actions (CLI, MCP, chat) default to `full`. The `aggressive` profile runs multi-pass optimisation to a fixpoint. Configurable via `tclLsp.optimiser.profile`; individual `O1xx` toggles now use a tristate (`true`/`false`/`null`) to override or inherit from the active profile.
- **W313: Destructive file operation with variable path** — New taint-aware diagnostic warns when `file delete`, `file rename`, or `file mkdir` receive an unsanitised path, detecting path-traversal risk. Suppressed when the path is both normalised and bounds-checked. Uses branch-dependent guard analysis to track which CFG paths are protected.
- **PATH_BOUNDED taint colour** — New taint lattice colour for paths that have been both normalised and verified to stay within an intended directory, recognised via `string match`, `string first`, and `string equal` branch guards in the CFG.
- **CONSTSET lattice for SCCP** — The constant propagation lattice now supports finite sets of constants alongside single constants, tracking `foreach` iteration variables, variable-mediated lists, and interprocedural constant returns, suppressing false W307/W123 diagnostics for command names resolved through variables.
- **Registry-based constant folding** — 37 pure Tcl commands (`list`, `string`, `lindex`, `llength`, `join`, `split`, `format`, `lrange`, `lreverse`, `lrepeat`, `concat`, and more) can now be constant-folded at compile time when all arguments are statically known.
- **Case-mismatch suggestions for variable diagnostics** — W210, W211, and W220 now suggest similarly-named variables when the issue is likely a capitalisation typo.
- **Unicode confusables detection for W108** — Non-ASCII character detection now uses the Unicode Consortium's confusables specification for homoglyph identification. Configurable via `tclLsp.style.nonAscii` with four modes: `strict`, `confusables` (default for Tcl), `common`, and `off`.
- **Namespace ensemble and TclOO command resolution** — `namespace ensemble create` commands are now tracked, and TclOO class names created via `oo::class create` are resolved as commands. Objects modified via `oo::objdefine` are tracked to suppress false W308 diagnostics.
- **Interpolated command name resolution** — Command names containing variable substitutions are now resolved via the CONSTSET lattice, suppressing false W123 when all possible values are known commands.

## Improvements

- **Command registry trait-driven check dispatch** — Analysis checks (eval injection, subst injection, open pipeline, encoding mismatch, etc.) are now dispatched based on boolean trait flags on `CommandSpec` rather than hardcoded command name sets.
- **Hardcoded command sets migrated to registry traits** — Path-returning commands, unescape commands, language keywords, top-level-only iRules commands, safe-on-uninit commands, pure-evaluation commands, variable-destroying commands, and more are all now declared on their `CommandSpec`.
- **Per-subcommand dialect gating** — Subcommands introduced in specific Tcl versions (e.g. `dict getwithdefault` in 9.0, `string is entier` in 8.6) are now gated per-dialect, preventing false positives in older dialect modes.
- **Shimmer detection improvements** — Shimmer annotations added for all `dict` and `string` subcommands, index arguments, and `split`. Duplicate shimmer warnings suppressed when a prior use in the same block already coerced the same SSA version.
- **W304 tristate severity model** — Missing `--` option terminator diagnostic now uses OFF/POSSIBLE/ALWAYS severity based on static analysis of the argument value.
- **Command arity and metadata corrections** — Verified and fixed arity, options, return types, and side effects across dozens of commands against the Tcl C source.
- **BIG-IP version comments** — 42 iRules registry files now include version availability comments.
- **`pkgIndex.tcl` implicit variable suppression** — `$dir` is now recognised as implicitly set by the package loader, suppressing false diagnostics.
- **Semantic tokens driven by registry** — Language keyword classification for semantic tokens is now derived from the `is_language_keyword` trait rather than a hardcoded set.
- **Optimiser decorator metadata** — Optimisation pass categories declared via `@opt()` decorator, enabling profile-based filtering.
- **Comprehensive VS Code extension test coverage** — New test suites covering call hierarchy, commands, configuration, document links, document symbols, extension activation, folding ranges, inlay hints, language registration, rename, selection range, signature help, and workspace symbols.

## Bug Fixes

- **O112 switch elimination now respects `-glob` mode** — Previously ignored glob patterns, incorrectly matching only exact strings. Also fixed fallthrough handling and `-nocase` support.
- **O116 constant folding no longer triggers false S100 shimmer warnings** — Folding `[list ...]` to a literal no longer emits spurious shimmer diagnostics.
- **`diagnostics_enabled` setting not applied on VS Code restart** — The setting is now re-read after analysis completes, so configuration changes take effect immediately.
- **Selection range containment violation** — The selection range provider now enforces the VS Code invariant that each outer range must strictly contain its predecessor.
- **W210 false positives for `lappend`/`append`/`dict set`/`incr` on uninitialised variables** — These commands safely initialise variables; the `safe_on_uninit` mechanism is now registry-driven with per-dialect gating.
- **Incorrect `pure=True` removed from `clock scan` and `string is`** — These commands have side effects and were incorrectly marked as pure.
- **iRules option metadata fixes** — Corrected `takes_value` flags, added missing options, and fixed synopses.
- **`--` option terminator support audited** — Comprehensive audit against the Tcl source and iRules documentation.
- **`serialize-javascript` dependency bumped to ^7.0.5** — Addresses a known vulnerability.

## Breaking Changes

- **Default editor optimiser profile changed to `readability`** — The editor now shows only readability-focused optimisation suggestions by default. Set `tclLsp.optimiser.profile` to `full` to restore the previous behaviour.
- **Individual optimiser toggles now default to `null` (inherit from profile)** — Previously defaulted to `true`. Explicitly setting `true`/`false` overrides the profile; `null` inherits.
