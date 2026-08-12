# KCS: feature — @tk Chat Participant

> **Audience:** User
> **Type:** Functionality

## Summary

VS Code Copilot Chat participant for creating, explaining, and previewing Tk GUI applications.

## Applies to

Copilot Chat

## Availability

| Context | How |
|---------|-----|
| VS Code Copilot Chat | Type `@tk` then a slash command or question |

## How to use

Type `@tk` in the Copilot Chat panel followed by a slash command:

| Command | Description |
|---------|-------------|
| `/create` | Create a Tk GUI from a description |
| `/explain` | Explain a Tk GUI's widget hierarchy and layout |
| `/preview` | Open the Tk Preview pane for the current file |
| `/help` | Show available features and commands |

Or ask a free-form Tk question without a slash command.

## Operational context

Uses the Tcl analysis engine with Tk-specific system prompts. Created code is validated through the agentic loop and can be previewed immediately in the Tk Preview pane.

## Failure modes

- AI features disabled (`tclLsp.ai.enabled` is false).
- Copilot extension not installed.

## Test anchors

- `editors/vscode/src/test/chatUtilities.test.ts`

## Example

A prompt in the Copilot Chat panel:

> `@tk /create a window with a label and a button that changes the label text`

The participant generates a short Tk script:

```tcl
package require Tk
label .lbl -text "Hello"
button .btn -text "Change" -command {.lbl configure -text "Clicked"}
pack .lbl .btn
```

Running `@tk /preview` then opens the live Tk Preview pane with the
rendered window.

## Discoverability

- [KCS feature index](README.md)
- [Tk Preview](kcs-feature-tk-preview.md)
