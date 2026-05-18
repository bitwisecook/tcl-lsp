# v1.10.4

## New Features

- **JetBrains plugin — discoverable command actions.** Seventeen new
  AnAction entries under **Tools → Tcl** (and Find Action) cover every
  LSP server command the VS Code extension already exposed: Apply All
  Optimisations, Minify Document, Apply Safe Quick Fixes; Render Tk
  Preview, Generate Mermaid Diagram, Translate iRule to F5 XC;
  Generate BIG-IP Cleanup Script, Extract Linked BIG-IP Objects,
  Rename BIG-IP Partition...; List iRule Events, List Known Tcl
  Packages, List Ensemble Subcommands..., Describe iRule Event /
  Command..., Search Tcl Help..., Suggest Packages for Symbol...,
  Show Effective Configuration. Document-modifying commands fire the
  LSP command and let lsp4j apply the returned `WorkspaceEdit`;
  result-producing commands open the output in a scratch editor with
  a sensible extension.
- **JetBrains plugin — marketplace logo and release notes.**
  `META-INF/pluginIcon.svg` and `pluginIcon_dark.svg` ship with the
  plugin, so the JetBrains Marketplace listing and the in-IDE Plugins
  window now show the project logo instead of the default
  puzzle-piece. The `<change-notes>` block is populated from
  `RELEASE_NOTES.md` via a small inline markdown→HTML pass, so every
  marketplace release advertises what actually changed.
- **OS keystore for JetBrains publish token.** `scripts/jetbrains_token.sh`
  resolves the Marketplace upload token from (1) `$JETBRAINS_TOKEN`,
  (2) macOS Keychain (`security find-generic-password`), or (3) Linux
  libsecret (`secret-tool`), in that order. `make publish-jetbrains`
  and `make publish-verify` both use the resolver, so the publish flow
  no longer requires an env var in every shell. Script header
  documents the one-time `add-generic-password` / `secret-tool store`
  invocations for each platform.

## Bug Fixes

- **`publish-verify` JetBrains liveness check.** `curl -fsS … || echo
  "000"` was concatenating the real HTTP code with `000` on any
  non-2xx (e.g. `404` became `404000` in the warning text). Dropped
  `-f` so the status code is reported verbatim, and a 404 from
  `/api/auth/me` is now a soft warning since `gradle publishPlugin`
  is the real source of truth.
