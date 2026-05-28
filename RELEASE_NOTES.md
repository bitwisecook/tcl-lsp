# v1.10.9

## New Features

- **Folding for backslash line-continuation commands (#493).** Commands
  spread across multiple physical lines via `\<newline>` continuations
  (e.g. a long proc call with each argument on its own line) are now
  foldable, collapsing to the opening line. Restores the legacy Tcl
  editor plugin behaviour.
- **`#region` / `#endregion` marker folding** (kind=`region`). Works
  in every LSP client — Zed, Neovim, Emacs, Sublime, Helix included —
  rather than depending on the VS Code language-configuration file.
- **Import-group folding** (kind=`imports`). Consecutive
  `package require` / `source` / `load` runs collapse to a single
  fold, matching the convention used by other language servers.

## Improvements

- **Unified block folding.** A single syntactic block collector now
  folds every multi-line braced `{...}` token (proc / namespace /
  control-flow bodies *and* plain data literals — lists, dicts, switch
  arms), bracketed `[...]` command substitutions, and quoted
  `"..."` strings. Resolves the inconsistency where a proc body folded
  but an identical-looking multi-line list did not, and lets
  continuations inside `[...]` fold via their enclosing token.
- **Folding is now purely syntactic.** The semantic-analysis
  dependency was removed, so folding works on incomplete or
  syntactically-broken files where the analyser would have bailed out.
- **Comment-block folds yield to region markers.** A `#region` /
  `#endregion` inside a comment block now produces a Region fold
  instead of being absorbed silently.

## Bug Fixes

- **Folding dropped a degenerate fold on a dangling `\` at EOF.**
  A trailing backslash on the last line no longer produces a fold
  over the empty space past it.
- **`ping(ip)` in f5-query could return `ok=True` with `rtt_ms=None`.**
  Under load (and on macOS where the BSD `ping -W` flag is per-packet
  milliseconds rather than per-deadline seconds), the per-reply
  `time=NN ms` token could be missing even when the subprocess exited
  cleanly. The parser now falls back to the summary line
  (`min/avg/max[/...] = X/Y/Z[/...] ms`) that both BSD and Linux
  `ping` emit on success, so the rtt is always populated when `ok` is
  true.
- **`make publish-{verify,sublime,zed}` looked in the wrong directory.**
  The three release scripts under `scripts/release/` computed `$ROOT`
  as `$(dirname $0)/..` — one level too shallow after the seven-concern
  package reorg moved them out of `scripts/`. `publish-verify`
  consequently reported every editor `[fail]` on a clean tree, and
  `publish-sublime` aborted post-build with `build/sublime-stage not
  found`. Both now resolve to the repo root.
