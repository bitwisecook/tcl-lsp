# KCS: feature — @tk Chat Participant

> **Audience:** User
> **Type:** Functionality

## Summary

VS Code Copilot Chat participant for creating, explaining, and previewing Tk
GUI applications, with static-analysis uncertainty called out explicitly.

## Applies to

Copilot Chat

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

Uses the Tcl analysis engine with Tk-specific system prompts. Created code is
validated through the agentic loop and can be previewed in the static Tk
Preview pane. Layout claims are drawn from the structured `tk_layout` context,
which separates confirmed model facts from uncertainties. The preview never
executes generated Tcl.

## Failure modes

- AI features disabled (`tclLsp.ai.enabled` is false).
- Copilot extension not installed.
- `tk_layout` or the static preview model is unavailable; in that case the
  assistant must say that it has not verified the widget tree.

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

Running `@tk /preview` then opens the static Tk Preview pane with the
rendered model. It does not run the script or simulate the button callback.

## What it will not tell you

Callbacks are explained as code relationships, not as proof of behaviour. A
recognised `-command`, `bind`, `after`, `trace`, `fileevent`, or `wm protocol`
descriptor says the callback is registered, never that it will run or with
what arguments.

Layout answers come from the static model, so a computed widget path, command,
option, or value leaves the hierarchy uncertain and the participant says so.
Fonts, themes, DPI, window-manager behaviour, keyboard conventions, and
accessibility are platform-dependent and outside the model. The preview is a
structural aid, not a native Tk rendering oracle — see
[Static Tk UI model](../../design/tk-static-ui-model.md).

## Discoverability

- [KCS feature index](README.md)
- [Tk Preview](kcs-feature-tk-preview.md)
- [Static Tk UI model](../../design/tk-static-ui-model.md)
