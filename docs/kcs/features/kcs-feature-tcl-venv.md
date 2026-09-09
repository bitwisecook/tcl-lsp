# KCS: feature — tcl venv

> **Audience:** User
> **Type:** Functionality

## Summary

Create and manage isolated Tcl virtual environments that pin a specific
tclsh version and keep project packages separate from the system.

## Applies to

tcl-lsp CLI

## Question

What does `tcl venv` do, and how do I use it?

## How to use

### tcl-lsp CLI

```sh
tcl venv create .venv            # create a venv with the default tclsh
tcl venv create .venv --tcl 9.0  # pin tclsh 9.0
source .venv/bin/activate        # activate (bash/zsh)
source .venv/bin/activate.fish   # activate (fish)
eval "$(tcl venv activate)"     # cross-shell activation
tcl venv info .venv              # show venv details
tcl venv list                    # list discoverable venvs
tcl venv update .venv --tcl 9.0  # re-link to a different tclsh
tcl venv run .venv -- tclsh script.tcl  # run without activation
tcl venv deactivate              # print deactivation snippet
tcl venv delete .venv            # remove the venv
```

### How activation works

Activation prepends `<venv>/bin` to `PATH`, sets `TCLLIBPATH` to
`<venv>/lib`, sets `TCL_VENV`, and modifies the shell prompt. The
`deactivate` function restores the original environment.

The `<venv>/bin/tclsh` wrapper always sets `TCLLIBPATH` before running
the pinned interpreter, so non-interactive use (CI, Makefiles) works
without manual activation.

## Options

- `--tcl VERSION` — pin a specific Tcl version (create/update).
- `--system-site-packages` — allow fallback to host `auto_path` (create).
- `--prompt NAME` — shell prompt label (create). Defaults to the venv directory name.
- `--force` — overwrite existing directory (create) or force delete.
- `--json` — emit JSON output (every subcommand except `activate`, `deactivate`, and `run`).
- `--shell bash|zsh|fish|csh|powershell` — shell flavour (activate).

## Example

```sh
$ tcl venv create .venv
  ✓ created /home/user/myapp/.venv
  activate: source /home/user/myapp/.venv/bin/activate

$ tcl venv info .venv
  created                         2026-09-08T17:40:25Z
  include-system-site-packages    false
  prompt                          .venv
  tcl_executable                  /usr/local/bin/tclsh9.0
  tcl_version                     9.0.4
  venv_tool                       tcl-lsp

$ source .venv/bin/activate
(.venv) $ tcl pkg install
(.venv) $ deactivate
$
```

## Related

- [KCS feature index](README.md)
- [tcl pkg](kcs-feature-tcl-pkg.md) — package management
- [Design: tclpkg contracts](../../design/contracts/tclpkg-contracts.md#virtual-environments-venvrs)
