# KCS: How do I publish tcl-lsp to each editor marketplace?

> **Audience:** Maintainer
> **Type:** How-To

## Applies to

VS Code, Zed, JetBrains, Sublime Text, Neovim, Helix

## Question

How do I publish tcl-lsp to each editor's official extension registry or
plugin marketplace?

## Before you start

- Be on `main` at the tag you want to publish (`make release-tag V=X.Y.Z`
  has run successfully and CI has uploaded every artefact to the GitHub
  release).
- Have `gh` CLI installed and authenticated (`gh auth status`).
- Have the relevant credentials for each marketplace (see per-section
  prerequisites below).

## Answer

Every editor's registry follows the same shape: a one-time registration
step, then per-release publishes. The one-time step is the hard part —
the per-release publish is wrapped in a `make publish-<editor>` target
that you run from your laptop after `make release-tag`.

**No tooling in this repository pushes to external repos or opens PRs
against them.** The `make publish-*` targets that touch external
registries (Zed, Sublime Package Control, JetBrains Marketplace) either:

- call a marketplace API directly on a registry we own credentials for
  (VS Code via `vsce`, JetBrains via `gradlew publishPlugin`), or
- prepare the change in a local checkout and stop there, leaving the
  push + PR for you to inspect and raise yourself (Zed).

The release skill drives the per-release flow automatically; the
one-time registrations below are PRs you raise yourself.

### VS Code Marketplace

**Per-release (already automated):** `make publish-vsix` runs
`vsce publish` with the cached publisher PAT.

**First-time:**

1. Create a publisher at <https://marketplace.visualstudio.com/manage>
   (publisher name in this repo is `bitwisecook`, configured via
   `VSCE_PUBLISHER` in the Makefile).
2. Generate a PAT (Personal Access Token) at <https://dev.azure.com>
   with **Marketplace > Manage** scope.
3. Run `npx @vscode/vsce login bitwisecook` and paste the PAT.
4. Run `make publish-vsix`.

### JetBrains Marketplace

**Per-release:** `make publish-jetbrains` runs
`./gradlew publishPlugin` using the `JETBRAINS_TOKEN` env var.

**First-time:**

The marketplace requires the initial version to be uploaded
**interactively via the web UI**; token-based `publishPlugin` only
works for *subsequent* versions of an already-published plugin.

1. Build the plugin locally: `make jetbrains` →
   `build/tcl-lsp-jetbrains-X.Y.Z.zip`.
2. Sign in at <https://plugins.jetbrains.com/author/me> with the account
   that owns the `com.tcllsp` plugin namespace.
3. Click **Upload plugin** → pick the `.zip` from step 1.
4. Fill in the plugin metadata (description, vendor, category, license).
   The plugin ID `com.tcllsp.jetbrains` is set in
   `editors/jetbrains/build.gradle.kts`; pick **Other** → **Tcl** as the
   category.
5. Submit for moderation. Approval typically takes a few business days.
6. After approval, create a token at
   <https://plugins.jetbrains.com/author/me/tokens> with **Plugin
   uploader** scope.
7. Export it: `export JETBRAINS_TOKEN=...` and re-run
   `make publish-jetbrains`. Subsequent releases need only step 7.

### Sublime Text (Package Control)

**Per-release:** `make publish-sublime` verifies the GitHub Release
carries a `.sublime-package` asset. Package Control polls the channel
and serves new tagged releases automatically — there is no per-release
marketplace API call.

**First-time (you raise this PR yourself):**

1. Fork <https://github.com/wbond/package_control_channel>.
2. Open `repository/t.json` in the fork.
3. Insert the JSON object from
   `scripts/publish_helpers/sublime_channel_entry.json` into the
   `packages` array, in alphabetical order by `name`.
4. Run the channel repo's local schema validator (`python -m unittest
   tests.test_channel`) to catch mistakes early.
5. Push the branch and open the PR against
   `wbond/package_control_channel` yourself.

   Suggested PR title: `Add Tcl package`

   Suggested PR body:

   > Adds the **Tcl** package, providing Tcl, iRules, iApps, F5
   > BIG-IP/TMSH, and EDA tool language support powered by the
   > [tcl-lsp](https://github.com/bitwisecook/tcl-lsp) language
   > server. The package is distributed as a `Tcl.sublime-package`
   > asset attached to each GitHub release, so Package Control
   > follows tags and serves the asset directly.

6. The maintainers review for fit and naming; once merged, the
   `packagecontrol.io` website picks up the entry within a day and
   subsequent tagged releases serve automatically via the
   `Tcl.sublime-package` asset attached to each release. No further
   action from us per release.

### Zed (Extensions registry)

**Per-release:** `make publish-zed` runs `scripts/publish_zed.sh`, which:

1. Clones (or refreshes) a local checkout of
   `zed-industries/extensions` at
   `$HOME/.cache/tcl-lsp/zed-extensions` (override with
   `ZED_EXTENSIONS_CHECKOUT=<path>`).
2. Advances the `extensions/tcl` submodule pointer to the new tag.
3. Bumps the `version = "..."` field in the `[tcl]` block of
   `extensions.toml`.
4. Stages the changes on a new branch and **stops** — printing the
   exact commit / push / `gh pr create` commands for you to run after
   reviewing the diff. The script never pushes to your fork and never
   opens a PR.

Override the suggested fork name shown in the final summary with
`ZED_EXTENSIONS_FORK=owner/repo`.

**First-time (you raise this PR yourself):**

1. Fork <https://github.com/zed-industries/extensions>:
   `gh repo fork zed-industries/extensions --remote=false`.
2. Clone the fork: `git clone --recurse-submodules
   git@github.com:<you>/extensions.git`.
3. Add tcl-lsp as a submodule pointing at this repo:
   ```bash
   cd extensions
   git submodule add https://github.com/bitwisecook/tcl-lsp.git extensions/tcl
   cd extensions/tcl && git checkout vX.Y.Z && cd ../..
   ```
4. Insert the following block into `extensions.toml`, alphabetically by
   the section name:
   ```toml
   [tcl]
   submodule = "extensions/tcl"
   version = "X.Y.Z"
   path = "editors/zed"
   ```
   The `path` field tells Zed the extension lives under
   `editors/zed/` inside the submodule, not at its root.

   Suggested PR title: `Add tcl extension`

   Suggested PR body:

   > Adds the Tcl Language Support extension powered by
   > [tcl-lsp](https://github.com/bitwisecook/tcl-lsp). Provides
   > syntax, semantic-token highlighting, diagnostics, completions,
   > hover, go-to-definition, formatting, code actions, and an
   > MCP context server for Tcl 8.4–9.0, F5 iRules/iApps/TMSH, and
   > EDA tool dialects. The extension lives under `editors/zed/` in
   > the submodule (`path = "editors/zed"`).

5. Commit, push to your fork, and open the PR against
   `zed-industries/extensions` yourself. After merge, every subsequent
   release is just `make publish-zed` (which prepares the change
   locally — you push and open each follow-up PR yourself).

### Neovim (nvim-lspconfig)

**Per-release:** `make publish-neovim` is a no-op (informational).
nvim-lspconfig is an upstream LSP **config**, not an extension; the
config is stable across our releases and doesn't need re-publishing.

**First-time (you raise this PR yourself):**

1. Fork <https://github.com/neovim/nvim-lspconfig>.
2. Copy `editors/neovim/lspconfig.lua` from this repo into
   `lua/lspconfig/configs/tcl_lsp.lua` in the fork.
3. Update the `doc/server_configurations.md` file with a brief entry
   linking back to the tcl-lsp repo (see other entries for the format).
4. Run nvim-lspconfig's own lint / test scripts (CI config explains the
   commands).
5. Open the PR against `neovim/nvim-lspconfig` yourself.

   Suggested PR title: `feat(tcl_lsp): add Tcl Language Server config`

   Suggested PR body:

   > Adds a config for [tcl-lsp](https://github.com/bitwisecook/tcl-lsp),
   > a Tcl language server supporting Tcl 8.4–9.0, F5 iRules / iApps /
   > TMSH, and EDA tool dialects. The server ships as a self-contained
   > Python zipapp; this config expects `tcl-lsp-server.pyz` on the
   > user's PATH (with the executable bit set).

   After merge, users install tcl-lsp via:

   ```lua
   require('lspconfig').tcl_lsp.setup({})
   ```

   and pull the `tcl-lsp-server.pyz` zipapp from any of our GitHub
   releases.

### Helix

**Per-release:** `make publish-helix` is a no-op (informational). Helix
ships language-server entries in its own `languages.toml`; that entry
points at the canonical `tcl-lsp-server.pyz` zipapp name, so it does not
move from release to release.

**First-time (you raise this PR yourself):**

1. Fork <https://github.com/helix-editor/helix>.
2. Merge the contents of `editors/helix/languages.toml` from this repo
   into the `languages.toml` at the root of the fork. Specifically:
   - Add the `[language-server.tcl-lsp]` block.
   - Add the `[[language]]` block with `name = "tcl"`.
3. Run `cargo run -- --health tcl` locally to confirm the entry is
   well-formed.
4. Open the PR against `helix-editor/helix` yourself.

   Suggested PR title: `feat(lang): add Tcl support via tcl-lsp`

   Suggested PR body:

   > Adds Tcl, iRules, iApps, F5 BIG-IP, EDA, and Expect support to
   > Helix via [tcl-lsp](https://github.com/bitwisecook/tcl-lsp).
   > The language server ships as a single-file Python zipapp; users
   > only need `tcl-lsp-server.pyz` on their PATH after this entry
   > merges.

   After merge, users install tcl-lsp by dropping the
   `tcl-lsp-server.pyz` zipapp on their PATH.

## How to tell it worked

- **VS Code:** the plugin appears at
  `https://marketplace.visualstudio.com/items?itemName=bitwisecook.tcl-lsp-vscode`
  with the new version.
- **JetBrains:** the plugin page at
  `https://plugins.jetbrains.com/plugin/com.tcllsp.jetbrains` shows the
  new version after approval.
- **Sublime Text:** `https://packagecontrol.io/packages/Tcl` shows the
  new version within ~1 hour of tag push.
- **Zed:** the bump PR merges in `zed-industries/extensions` and the new
  version appears in Zed's extension panel within an hour.
- **Neovim / Helix:** the upstream PR merges; users running newer
  versions of nvim-lspconfig / Helix get tcl-lsp without any further
  action from us.

## Related

- [KCS index](README.md)
- [Release skill](../../.claude/skills/release/SKILL.md)
- [Editor settings codegen](../../AGENTS.md)
