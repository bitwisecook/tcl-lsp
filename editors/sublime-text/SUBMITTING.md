# Submitting TclLsp to Package Control

Everything needed to register this package on the default Package Control
channel, and what keeps it there. The submission itself is a **one-time**
pull request against
[`wbond/package_control_channel`](https://github.com/wbond/package_control_channel);
every release after that publishes itself.

## How distribution works

Package Control 4 can resolve a release from a **GitHub release asset**
rather than a git tag's source tarball. That is what this package uses:

- `build-sublime` in `.github/workflows/ci.yml` builds
  `TclLsp.sublime-package` on every `v*` tag and attaches it to the
  GitHub Release, signed and covered by the release's `SHA256SUMS`.
- The channel entry names that asset. Package Control serves the newest
  release carrying it — no mirror repo, no per-release channel PR, no
  marketplace token.
- The package is **platform-independent** (no bundled binary), so one
  asset serves every platform. `plugin.py` downloads the
  `tcl-lsp-server-<triple>` asset for the host on first use and verifies
  it against `SHA256SUMS`.

### Where this stands today

The current stable release is **v2.2.0** — the release that took the 2.x
line out of preview. It carries every `tcl-lsp-server-<triple>` asset and
the cosign-signed `SHA256SUMS` the plugin downloads and verifies against,
but its Sublime asset still uses the pre-submission name
(`tcl-lsp-sublime-2.2.0.sublime-package`, with a Linux binary inside).

So the channel PR waits for **the first stable release built from this
change** — any `2.2.x` patch or later even-minor tag — which is what
publishes `TclLsp.sublime-package`. Nothing else is outstanding: the
download path is already verified end to end against v2.2.0's assets (an
unstamped checkout resolves 2.2.0 from the GitHub API, fetches
`tcl-lsp-server-x86_64-unknown-linux-gnu`, verifies its checksum, stages
the spec packs beside it, and the resulting server answers an LSP
`initialize` with `TCL_LSP_SPEC_PACK_DIR` pointing at them).

### Pre-releases stay off the channel

Package Control ignores GitHub's `prerelease` flag — its GitHub client
skips drafts only. The odd-minor pre-release line
(`scripts/release/prerelease.sh`) is therefore kept off the channel by
asset **name**: a pre-release tag publishes
`TclLsp-prerelease.sublime-package`, which no channel entry matches, and
only a stable tag publishes `TclLsp.sublime-package`.

## The channel entry

Add this object to the `"packages"` array of
[`repository/t.json`](https://github.com/wbond/package_control_channel/blob/master/repository/t.json),
in alphabetical order by `name` (after `Tabnine`, before `Terraform` —
check the file, it moves):

```json
{
	"name": "TclLsp",
	"details": "https://github.com/bitwisecook/tcl-lsp",
	"readme": "https://raw.githubusercontent.com/bitwisecook/tcl-lsp/rust/editors/sublime-text/README.md",
	"issues": "https://github.com/bitwisecook/tcl-lsp/issues",
	"labels": ["language syntax", "auto-complete", "linting", "formatting", "code navigation", "snippets"],
	"releases": [
		{
			"asset": "TclLsp.sublime-package",
			"sublime_text": ">=4107",
			"python_versions": ["3.8"]
		}
	]
}
```

Notes on each key:

| Key | Why |
|---|---|
| `name` | The package folder name, and what `plugin.py`, `Main.sublime-menu` and `Default.sublime-commands` hard-code as `${packages}/TclLsp`. CamelCase, no `.`, no "Sublime", and deliberately *not* `Tcl` — Sublime Text ships a default package called `TCL`, which `Tcl` would shadow case-insensitively. |
| `details` | The monorepo. Package Control reads the description, author and homepage from it. |
| `readme` | The package README, not the monorepo one. Must be a raw URL, and it must name **`rust`**: `main` is the 1.x line and still serves the pre-rename README, which describes a bundled server and the old package name. Repoint it if the 2.x line ever becomes the default branch. |
| `labels` | Lower case, spaces not dashes, drawn from the suggested vocabulary in `example-repository.json`. |
| `asset` | Exact asset name on the GitHub Release. Also the filename a manual install needs, so the two paths agree. |
| `sublime_text` | `>=4107` — the build that introduced `.python-version` / plugin_host 3.8, which this package requires. |
| `python_versions` | `["3.8"]` — matches `.python-version`, and the sublimelsp/LSP package is 3.8-only. |

## Before opening the PR

1. **A stable release must already carry the asset.** Package Control
   resolves nothing otherwise, and the newest stable release predating
   this change carries only the old asset name. Check:

   ```bash
   gh release view "$(gh release list --repo bitwisecook/tcl-lsp \
       --exclude-drafts --json tagName,isPrerelease \
       --jq 'map(select(.isPrerelease | not)) | .[0].tagName')" \
       --repo bitwisecook/tcl-lsp --json assets \
       --jq '.assets[].name' | grep -x 'TclLsp.sublime-package'
   ```

   `make publish-verify` runs the same check as part of its Sublime
   section.

2. **Run the package reviewer** the channel's CI runs:

   ```bash
   make build-editor-sublime
   pipx run --spec \
       git+https://github.com/packagecontrol/st_package_reviewer \
       st_package_reviewer --repo-only build/sublime-stage
   ```

   `build/sublime-stage` is the exact tree that becomes the
   `.sublime-package`. Fix every failure; warnings are judgement calls
   (see [Known reviewer warnings](#known-reviewer-warnings)).

3. **Check the package name is still free** on
   [packagecontrol.io](https://packagecontrol.io/search/TclLsp) and in
   `repository/t.json`.

4. Open the PR with a one-paragraph description of what the package does
   and a link to the README. The channel's CI runs the reviewer against
   the resolved release.

## Known reviewer warnings

The current run reports **no failures** and 15 warnings, all expected:

| Warning | Why it stands |
|---|---|
| `'.sublime-syntax' … no '.tmLanguage' fallback` (×14) | `.sublime-syntax` has been supported since build 3092 and the entry requires 4107+. A `.tmLanguage` fallback would be dead weight. |
| `It looks like you're using platform-dependent code` (`plugin.py`, `sublime.platform()`) | The package itself is platform-independent — one asset for every platform — and the platform check only picks which `tcl-lsp-server` asset to download. So the channel entry needs no `platforms` key. |

Run the reviewer against `build/sublime-stage`, not `editors/sublime-text`:
the staged tree is what ships, and it is where `LICENSE.txt` (copied from
the repo root) and `server_version.json` (the release pin) exist. Against
the source directory the reviewer also reports a missing licence, which
the shipped package does not have.

## Guideline decisions baked into this package

- **No key bindings.** No `Default (*).sublime-keymap`, and no example
  keymap either — a package that ships bindings competes with the user's
  own. The README and
  [the KCS note](https://github.com/bitwisecook/tcl-lsp/blob/rust/docs/kcs/kcs-howto-bind-sublime-tcl-commands.md)
  list every bindable command instead.
- **No silent settings edits.** Disabling the built-in `TCL` package and
  enabling LSP's `semantic_highlighting` are both *offered* — once after
  installation, and on demand via **Tcl: Recommended Setup** — never
  applied behind the user's back.
- **No bundled executable**, so no `.no-sublime-package` and no
  platform-specific package. The server is downloaded per platform and
  checksum-verified.
- **`.python-version` = 3.8**, matching the `python_versions` in the
  channel entry and the LSP package's plugin host.
- **A top-level `LICENSE.txt`**, copied into the staged tree from the
  repo root at build time.
- **No `.pyc` or `__pycache__`**, stripped while staging, and no
  `package-metadata.json` (Package Control writes that itself). The
  packaging gate in the Makefile also fails the build if a native binary
  or a missing resource creeps in.

## After the PR merges

Nothing further per release. A stable tag builds, signs and attaches
`TclLsp.sublime-package`; Package Control's scraper picks it up within
about an hour. The channel entry only needs editing if the package name,
supported Sublime builds, or asset name change.
