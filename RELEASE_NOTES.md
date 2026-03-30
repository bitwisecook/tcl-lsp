# v1.3.1

## New Features
- **W124 — SSA-traced IP address validation**: validates IPv4/IPv6 address literals discovered via SCCP constant propagation, with related-information links from definition to use sites.
- **W126 — Channel type checking**: detects non-channel values (e.g. integers, plain strings) passed to channel argument positions in commands like `close`, `gets`, `puts`, `flush`, `eof`, `seek`, `tell`, `fcopy`, `fileevent`, `fconfigure`, `fblocked`, and `HSL::send`.
- **Channel tracking infrastructure**: `TclType.CHANNEL` type, `ArgRole.CHANNEL` annotations, and `TaintColour.CHANNEL` allow the type system to trace I/O handles from `open`, `socket`, `chan create`, and `HSL::open` through to channel-consuming commands.

## Improvements
- **Semantic token reclassification**: built-in commands (e.g. `set`, `puts`, `string`, `regexp`) are now emitted as `function` with the `defaultLibrary` modifier instead of `keyword`, aligning with the LSP semantic token specification and enabling distinct theme colouring for language keywords vs library functions.
- **TextMate grammar overhaul**: braced content now recurses as full Tcl source (highlighting proc/if/while bodies); `proc`/`method` names are captured as `entity.name.function`; added `when`, `on`, `trap`, `finally`, TclOO, and `oo::*` keywords; simplified variable scopes.
- **Option-aware arity checking**: commands that declare leading options (e.g. `puts -nonewline`) now correctly skip option flags before counting positional arguments, eliminating false-positive arity errors.
- **Faster file-open responsiveness**: `didOpen` and dialect changes now use async tasks to unblock the event loop, and syntax-only semantic tokens are eagerly precomputed so the editor gets instant highlighting before heavy analysis completes.
- **Diagnostic related information**: the LSP now emits `relatedInformation` on diagnostics that carry use-site references (currently W124).

## Bug Fixes
- Fixed `--` only being treated as an option terminator when the command explicitly declares it, preventing false arity errors on commands that accept `--`.
- Fixed W122 (regex-based IP check) false positives on version numbers following `/` (e.g. `Chrome/28.0.1550.0`); narrowed diagnostic range to the matched quad.
- W122 is now suppressed on lines where the more precise W124 (SSA-based) check fires.
- Fixed `class search` argument position detection in W306 (literal expected) — the class name is now correctly identified as the first positional argument rather than the last.
- Fixed CFG definition tracking for variables assigned inside command substitutions within expressions (e.g. `[set full_tag [string tolower $x]]` inside `lsearch`).
- Fixed backslash-newline continuation between words being treated as a word boundary instead of whitespace in command segmentation.
- Fixed empty command substitution `[]` being incorrectly flagged as unterminated in the recovery parser.
- Fixed constant-hoisting optimiser incorrectly suggesting hoisting of empty/default-value initialisations (`""`, `{}`, `[list]`) that must reset per-request.
- Fixed RULE_INIT `static::` definitions not being recognised as cross-event variables by the optimiser, preventing false "unused variable" or "hoist" suggestions.
- Rename now correctly preserves `$`, `${...}`, and namespace qualifiers at variable reference and proc call sites.
