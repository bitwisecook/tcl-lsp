# v2.1.7

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is a correctness audit of **semantic highlighting**. Every token
the server emits was diffed against the retired 1.x implementation over ~30k
lines of real Tcl — the Tcl 8.6 and 9.0 core libraries, plus `ruff`, `zesty`,
`ticklecharts` and `tomato` — and every difference was chased to a cause. The
result restores everything the Rust rewrite had dropped, fixes a long list of
things *both* implementations got wrong, and brings back first-class
highlighting for BIG-IP configs and iApp APL presentations.

One fix reaches beyond highlighting: an object referenced only from a `switch`
arm was invisible to the BIG-IP reference graph, which `f5 grep`, `f5 irule
trace`, `f5 query rename` and the `bigip-cleanup` skill all read. See **Bug
Fixes**.

## New Features

- **BIG-IP config highlighting is back, and works for the first time.** A
  `bigip.conf` / `.scf` now gets a real token stream — partitions, pools,
  monitors, profiles, VLANs, interfaces, IP addresses, ports, route domains,
  FQDNs, usernames and encrypted values — instead of every line being painted
  as one flat string. `samples/bigip/bigip.conf` previously *crashed* the 1.x
  server outright and produced 272 whole-line strings under 2.1.6; it now
  yields 374 correctly-typed, non-overlapping tokens.
- **Embedded iRules inside a BIG-IP config are highlighted as Tcl.** A
  `ltm rule /Common/x { when HTTP_REQUEST { … } }` body is code, not config, and
  is now walked with the iRules dialect — object references included, so
  `pool /Common/api_pool` inside a config reads exactly as it does in a
  standalone `.irul`.
- **APL (iApp presentation) highlighting is back.** All ten `apl*` token types —
  sections, field types, field names, attributes, validators, directives,
  `define` names, the `optional` guard — and the `[ … ]` bracket expressions
  that embed Tcl are now walked as Tcl, as the feature has always documented.
- **Expect scripts highlight.** `expect { pat body … }` — the central construct
  of every Expect script — was rendering as flat per-line strings, its clause
  bodies never recursed. Patterns, `-re` regex mode, the `timeout` / `eof` /
  `full_buffer` keywords and every clause body now highlight, and
  `expect_before` / `expect_after` / `expect_user` / `expect_background` /
  `expect_tty` come with them.
- **TclOO reads as objects, not strings.** Method names carry the standard LSP
  `method` type at their declaration *and* at their call sites (`my m`,
  `$obj m`); class names carry `class`; `superclass` / `mixin` / `export` /
  `filter` / `forward` arguments are typed as the class or method they name.
  Proc, method and constructor parameters carry the standard `parameter` type
  again, so a theme can tell an argument from an ordinary local.

## Improvements

- **The token vocabulary is coherent and enforced.** 57 types — 16 standard LSP
  plus six closed families (regex, `format`/`scan`, `clock`, `binary`, BIG-IP,
  APL). Every type each dialect emits is either a standard LSP type or mapped to
  an editor scope, and two tests now hold that line — including for the narrow
  dialects (the versioned Tcls, the five EDA tools, Expect), which reach the same
  sub-tokenisers a plain `.tcl` file does.
- **Shared lexical rules are now shared.** Tcl's backslash-escape widths and the
  literal/escape split live once, beside the evaluator that defines them; the
  `{pattern body …}` clause-list split lives once, shared by the token walker and
  the iRules object walker. Each had grown private copies that had already
  drifted apart.
- **Clause lists are registry data.** `switch` and Expect's `expect` are the same
  construct, described once in the command registry rather than matched by name
  in the walker.

## Bug Fixes

- **Objects referenced from a `switch` arm were invisible to the BIG-IP
  reference graph.** A `switch` case list is not a body, so the iRules
  object-reference walker never descended into one — and a pool used only inside
  a `switch` arm is an entirely ordinary iRule:

  ```tcl
  switch -glob [HTTP::uri] {
      "/api/*" { pool /Common/api_pool }
      default  { pool /Common/web_pool }
  }
  ```

  `f5 grep` missed the iRule routing to a pool, `f5 irule trace` reported no pool
  references for it, `f5 query rename` counted 2 referrers where there were 3 —
  and **`bigip-cleanup` would have generated a delete for a live pool.** All
  three tools now see the full graph.
- **Quoted and braced literals lost their closing delimiter.** `"hello world"`
  was highlighted as `"hello world`, `{a b}` as `{a b`, `${var}` as `${var`. An
  *escaped* string lost both delimiters.
- **Backslash escape widths were wrong.** `\xhh`, `\uhhhh`, `\Uhhhhhhhh` and
  octal `\ooo` were all treated as a backslash plus one character, so `\x41` was
  highlighted as an escape `\x` followed by a string `41`.
- **Array references with a computed index painted as strings.** `set env($lo)`,
  `unset UnknownPending($name)` — a pattern used throughout Tcl's own `init.tcl`
  and `package.tcl`.
- **A regular expression's closing `)` was typed as a quantifier**, not a group
  delimiter.
- **`expr` emitted no token at all for `(`, `)`, `?` and `:`.**
- **An empty procedure body emitted its closing brace as a function.**
  `proc p {args} {}` produced a stray `}` token.
- **A quoted string containing `::` was typed as a namespace**, losing any escape
  inside it.
- **`defaultLibrary` was lost** on commands reached through a `namespace import`
  alias and on a built-in's namespace prefix (`tcl::mathop::`).
- **APL diagnostics were reported a column short** after any astral character —
  positions counted code points rather than UTF-16 units.
- **The JetBrains plugin failed to load on IntelliJ 2026.1+.** Compiled against
  the 2024.1 floor, it called platform LSP methods whose Kotlin default-argument
  bridges no longer exist in 2026.1 / 2026.2, so Marketplace verification
  reported two critical `unresolved method` problems and the plugin broke at
  runtime on current IDEs.

## Breaking Changes

- **New semantic token types.** The legend gains `parameter`, `method`, `class`,
  the twelve BIG-IP types and the ten `apl*` types. Themes that style tokens by
  type may want rules for them; all are either standard LSP types (styled out of
  the box) or already mapped to TextMate scopes by the shipped editor
  integrations.
- **`interface` is renamed `bigipInterface`.** The 1.x legend reused the standard
  LSP `interface` type for a BIG-IP network interface, which shadows its real
  meaning. A custom theme rule keyed on `interface:tcl-bigip` must be renamed.
- **Procedure and method parameters are now `parameter`, not `variable`.** A
  theme that colours `variable` and expects parameters to match will see them
  change; `parameter` is a standard LSP type and is styled by default.
- **`switch`'s `default` arm is now a `keyword`**, not a string — matching
  Expect's `timeout` / `eof`, which likewise match no text.
