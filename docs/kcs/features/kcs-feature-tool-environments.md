# KCS: feature — Tool Environments

> **Audience:** User
> **Type:** Functionality

## Summary

Vivado, Quartus, Questa, Libero, Synopsys, and Cadence are a Tcl release plus library packages, and this note says what that means and how to change or hide it.

## Applies to

all-editors, tcl-lsp-cli

## Question

Why does the editor say Vivado is Tcl 8.5 plus packages?

## How to use

### What a tool environment is

The EDA tools run a stock Tcl interpreter. Their own commands arrive as
library packages loaded into it. tcl-lsp describes them the same way.
"Xilinx Vivado" is Tcl 8.5 plus the `vivado`, `sdc`, and `upf` packages. It is
not a separate language. Pickers list these under **Tcl + packages**, next to
the **Dialects** group that holds the languages.

### Your setup keeps working

Nothing you wrote stops working. The ids (`xilinx-eda-tcl`,
`intel-quartus-eda-tcl`, `mentor-eda-tcl`, `microchip-libero-eda-tcl`,
`synopsys-eda-tcl`, and `cadence-eda-tcl`), the `tclLsp.dialect` setting, the
`# tcl-dialect:` comment, and `--dialect` all resolve as before. Files such as
`.xdc` still open in their tool's environment. The first time one opens in a
session, the language server shows a short message. **Learn more** opens this
note. **Don't show again** hides it for that tool in every editor.

### Pick a plain Tcl release instead

Set `tclLsp.dialect` to `tcl8.5` (or another release), or put
`# tcl-dialect: tcl8.5` in the first five lines of the file. The tool's own
commands are then reported as unknown, because the packages are not loaded.

### Short names

These names work in a `# tcl-dialect:` comment and after `--dialect`. Settings
lists show the full ids.

| Name | Environment |
|---|---|
| `vivado` | `xilinx-eda-tcl` |
| `quartus` | `intel-quartus-eda-tcl` |
| `questa`, `modelsim` | `mentor-eda-tcl` |
| `libero` | `microchip-libero-eda-tcl` |
| `dc_shell`, `primetime` | `synopsys-eda-tcl` |
| `genus`, `innovus` | `cadence-eda-tcl` |

### Turn the message off

**VS Code.** Untick **Tcl Lsp › Notifications: Environment Kind** in Settings.

**Other editors.** Set `tclLsp.notifications.environmentKind` to `false` in the
editor's `tclLsp` settings.

**Any editor, through the config file.** Add this to `config.ini`. It suits
editors with no settings screen.

```ini
[notifications]
environment_kind = false
```

The server reads `config.ini` when the editor answers its `workspace/configuration`
request, so an editor that declines that request does not read
`[notifications] environment_kind`; use the editor setting there.

An editor that sends the setting itself wins over `config.ini`. Your **Don't
show again** choices are saved in `notices.ini` under your state folder
(`~/.local/state/tcl-lsp/` on Linux). Delete a line, or the file, to see the
message again.

## Options

- `tclLsp.notifications.environmentKind` — show the message. Default `true`.
- `[notifications] environment_kind` — the same switch in `config.ini`.

## Example

Opening `top.xdc` shows this message once per session:

> Xilinx Vivado (xilinx-eda-tcl) is Tcl 8.5 plus the vivado, sdc and upf
> packages. Tool support is a set of library packages on a Tcl release, not a
> separate dialect; your selection keeps working as before.

To read the same file as plain Tcl 8.5, start it with:

```tcl
# tcl-dialect: tcl8.5
create_clock -period 10 [get_ports clk]
```

## Related

- [KCS feature index](README.md)
- [Dialect Selection](kcs-feature-dialect-selection.md) — every way to choose a [dialect](../../GLOSSARY.md#dialect)
- [Extension Settings and Server Control](kcs-feature-extension-settings.md)
- [How does tcl-lsp load configuration?](../kcs-qa-how-tcl-lsp-loads-configuration.md)
- [Environment selection](../../design/contracts/environment-selection.md) — the design behind the grouping and the message
- [Configuration file reference](../../design/contracts/xdg-config.md) — where `config.ini` and `notices.ini` live
