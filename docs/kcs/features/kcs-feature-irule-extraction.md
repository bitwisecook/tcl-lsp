# KCS: feature — iRule Extraction

> **Audience:** User
> **Type:** Functionality

## Summary

Extract iRules from BIG-IP configuration files into individual editor tabs or files.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Open iRule in Editor`, `Tcl: Open iRule from Config...`, `Tcl: Extract All iRules to Files...`, `Tcl: Extract Linked BIG-IP Objects` |

## How to use

- **VS Code**: Open a `bigip.conf` file and use the command palette:
  - `Open iRule in Editor` — extract the iRule at the cursor.
  - `Open iRule from Config...` — pick an iRule from a list.
  - `Extract All iRules to Files...` — save all iRules to individual files.
  - `Extract Linked BIG-IP Objects` — show related virtual servers, pools, etc.

## Operational context

BIG-IP configuration files embed iRules as `ltm rule` blocks. These commands parse the config file, extract the Tcl source from each rule block, and open it in the correct dialect for full LSP support.

Saving an opened iRule writes its body back into the configuration file.
The server finds the same `ltm rule` / `gtm rule` stanza again — by its
header, path and the body you opened — so edits elsewhere in the file are
fine, and it applies the write only to the version of the file it read.

## Failure modes

- Extraction fails for non-standard config formatting.
- Linked object resolution misses references.
- A save is refused, with a warning, when the rule itself changed in the
  configuration file since it was opened (open it again), when the edited
  body no longer parses back as the same rule (usually unbalanced braces),
  or when the configuration file changed while the write was being applied
  (save again). The configuration file is left unchanged.

## Example

Given a BIG-IP `bigip.conf` with:

```
ltm rule /Common/redirect_http {
    when HTTP_REQUEST {
        if { [TCP::local_port] == 80 } {
            HTTP::redirect "https://[HTTP::host][HTTP::uri]"
        }
    }
}
```

Placing the cursor anywhere inside the `ltm rule` block and
running **Tcl: Open iRule in Editor** opens a new editor tab with
just the iRule body as an `.irul` file — full LSP features apply
and the file is ready to edit or save.

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
