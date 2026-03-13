# KCS: feature — iRule Extraction

## Summary

Extract iRules from BIG-IP configuration files into individual editor tabs or files.

## Surface

vscode-command

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

## File-path anchors

- `editors/vscode/src/extension.ts`
- `core/bigip/`

## Failure modes

- Extraction fails for non-standard config formatting.
- Linked object resolution misses references.

## Test anchors

- `tests/test_bigip_extraction.py`

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
