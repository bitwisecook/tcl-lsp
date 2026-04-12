# KCS: feature — Chat Slash Commands

> **Audience:** User
> **Type:** Functionality

## Summary

Slash commands inside the VS Code Copilot Chat participants (`@irule`, `@tcl`, `@tk`) that create, explain, fix, validate, optimise, and review Tcl and iRules code with LSP-backed analysis.

## Applies to

VS Code Copilot Chat

## Question

What slash commands are available in the Copilot Chat participants, and what does each one do?

## How to use

Type `@irule`, `@tcl`, or `@tk` in the Copilot Chat panel, followed by a `/` and the command name. Each participant supports a different subset of commands.

### Commands available in all participants

| Command | What it does |
|---------|-------------|
| `/create` | Generate code from a plain-English description. The agentic loop validates the result against the LSP and iterates until diagnostics are clean (up to five rounds). |
| `/explain` | Explain what the code in the active editor or an attached `#file` does, covering event handlers, data flow, and security concerns. |
| `/fix` | Grab the active document, wait for LSP diagnostics, then iteratively fix every error and warning. |
| `/help` | Show the feature catalogue for the active participant. |

### Commands available in `@irule` and `@tcl`

| Command | What it does |
|---------|-------------|
| `/validate` | Run a full LSP analysis and present a colour-coded report grouped by severity and category (errors, security, taint, thread safety, control flow, performance, style, optimiser). |
| `/optimise` | Apply optimiser suggestions at the selected profile level, then explain why each rewrite is safe. |

### Commands available in `@irule` only

| Command | What it does |
|---------|-------------|
| `/review` | Security and safety review of the iRule. |
| `/convert` | Detect legacy patterns eligible for modernisation. |
| `/scaffold` | Generate an iRule skeleton from a list of events. |
| `/datagroup` | Suggest data-group extraction opportunities. |
| `/diff` | Explain the differences between two versions of an iRule. |
| `/event` | Show valid commands for a given iRule event. |
| `/migrate` | Convert an nginx, Apache, or HAProxy configuration to an iRule. |
| `/diagram` | Generate a Mermaid flowchart of the iRule's control flow. |

## Example

Typing the following into the Copilot Chat panel:

> `@tcl /create a proc that reverses a list in place`

The participant generates code, writes it to a temporary document, waits for LSP diagnostics, and iterates until the analysis is clean:

```tcl
proc lreverse_inplace {listVar} {
    upvar 1 $listVar lst
    set lst [lreverse $lst]
}
```

The response includes the number of validation iterations and a button to insert the code into a new file.

## Related

- [KCS feature index](README.md)
- [@irule Chat Participant](kcs-feature-ai-chat-irule.md)
- [@tcl Chat Participant](kcs-feature-ai-chat-tcl.md)
- [@tk Chat Participant](kcs-feature-ai-chat-tk.md)
- [Diagnostics](kcs-feature-diagnostics.md) — the LSP analysis the agentic loop validates against
- [Optimiser](kcs-feature-optimiser.md) — the engine behind `/optimise`
