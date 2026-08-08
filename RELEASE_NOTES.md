# v2.1.17

**2.x alpha — pre-release channel.**

A pre-release on the **2.x** line, where the ongoing Rust rewrite of tcl-lsp
ships its alphas. It is opt-in: install it from the VS Code Marketplace
**pre-release** channel or the JetBrains Marketplace **eap** channel, or
download the pre-release VSIX / plugin / native binaries from this GitHub
release. The stable **1.x** line stays the default for everyone who has not
opted into pre-releases, and a `2.1.x` build never becomes the "latest" GitHub
release or the default Marketplace download.

A documentation and tooling release. No language-server behaviour changes: the
analyser, compiler and registry are untouched. What changes is the documentation
you read, the build targets you run, and the screenshots on the Marketplace
listing.

## Documentation

The README was 3088 lines with every feature at equal weight. It is now ordered
by what you actually reach for: seven headline features (diagnostics, semantic
highlighting, completions, hover, navigation, refactorings, formatting), then a
full index linking all 86 per-feature notes, then the deeper material.

Two lists the README never carried are now complete and generated from the
source of truth rather than written by hand — all **sixteen dialects** (the old
prose omitted `expect`, `bpf`, `tcl9.1`, `f5-tmsh`, and named the five EDA
vendors only collectively) and all **69 registry packages**.

F5 moves to a standalone **[README-f5.md](README-f5.md)**: the four F5 dialects,
the BIG-IP config model, the `f5` CLI, the query DSL with its worked how-tos,
the `f5q` Python bindings, the report generator, XC translation, and the iRule
Event Orchestrator.

The install guides were telling users to download Python `.pyz` zipapps that
have not shipped for several releases. Every install path now describes the
native per-triple binaries that releases actually publish, across
INSTALL-cli.md, INSTALL-editors.md, and the Helix, Sublime, JetBrains and Emacs
editor READMEs. CONTRIBUTING.md likewise still documented the retired Python
server — seven "concern packages" and an `.importlinter` file, none of which
exist — and now describes the Cargo workspace.

A new KCS rule: **a fixed bug that needs no reader action is not a KCS note.**
Recorded in STYLE.md, AGENTS.md and the KCS README, with the test to apply.

## Fixes

- **VS Code extension tests could not start on macOS.** The test host's IPC
  socket path exceeded the 103-byte `sun_path` limit, failing with a bare
  `EINVAL`. Both the single-root and multi-root runners now keep it well under.
- **`make` printed a warning and could run codegen twice.** Two rules used
  GNU Make 4.3 grouped-target syntax; macOS ships Make 3.81, which parses it as
  a third target named `&` and attaches the recipe to every output.
- **The diagram webview fetched Mermaid from a CDN** at render time, with a
  remote origin in its CSP, and silently degraded to raw source when offline.
  Mermaid is now bundled into the VSIX.
- **Chat picked an arbitrary language model** — the first the host returned —
  whenever the panel handed it the synthetic `auto` selector. It now prefers the
  largest context window.
- Chat's code-fence parsers accepted only a few exact tags, discarding
  well-formed answers labelled otherwise.

## Known issue

`@irule /create` and `/diagram` receive **empty model responses** — `/diagram`
logs `0 chars returned`. `/explain`, `/validate`, `/review` and `/help` answer
normally, so this is specific to the two commands that send the largest prompts.
Chat now says so plainly instead of ending on a silent "Generating Mermaid
diagram...". The `26-ai-create` and `28-ai-diagram` screenshots in this release
show that failure state honestly rather than a staged success.

## Screenshots

All 30 scenes retaken against the current build. The capture harness had three
faults that made it unusable unattended: an AI sign-in prompt that could hang a
run forever, a capture handshake that polled marker files, and two waits that
burned their full timeout on every run because they expected more diagnostics
and semantic tokens than the fixtures produce. Scene times fell from 32s, 22.6s
and 12.1s to 5.7s, 2.9s and 2.4s.

