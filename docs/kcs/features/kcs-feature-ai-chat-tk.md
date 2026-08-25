# KCS: feature — @tk Chat Participant

> **Audience:** User
> **Type:** Functionality

## Summary

VS Code Copilot Chat participant for creating, explaining, and previewing Tk
GUI applications, with static-analysis uncertainty called out explicitly.

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

Uses the Tcl analysis engine with Tk-specific system prompts. Created code is
validated through the agentic loop and can be previewed in the static Tk
Preview pane. Before making a layout claim, the assistant should request the
structured `tk_layout` context and distinguish confirmed model facts from
uncertainties. The preview never executes generated Tcl.

## Failure modes

- AI features disabled (`tclLsp.ai.enabled` is false).
- Copilot extension not installed.
- `tk_layout` or the static preview model is unavailable; in that case the
  assistant must say that it has not verified the widget tree.

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

Running `@tk /preview` then opens the static Tk Preview pane with the
rendered model. It does not run the script or simulate the button callback.

## Callback and event guidance

Treat “callback” as a broad Tcl/Tk pattern, not only a widget `-command`
option. The assistant should look for and explain:

- timers and idle work with [`after`](https://www.tcl-lang.org/man/tcl8.6/TclCmd/after.htm),
  including the returned ID and `after cancel` relationship;
- channel and file readiness with `fileevent` and `chan event`;
- variable and command tracing with [`trace`](https://www.tcl-lang.org/man/tcl8.6/TclCmd/trace.htm);
- widget/window events with [`bind`](https://www.tcl-lang.org/man/tcl8.6/TkCmd/bind.htm),
  `bindtags`, virtual events, and additive `+script` bindings;
- window-manager callbacks such as `wm protocol`, which are delivered later
  by the window manager, not synchronously by the registration command;
- namespace-safe callback construction with
  [`namespace code`](https://www.tcl-lang.org/man/tcl8.5/TclCmd/namespace.htm)
  and list-built command prefixes such as `[list command $value]`;
- widget command prefixes including scrollbar/view synchronization,
  validation, menu post/command hooks, and other registry-described callback
  options.

For event bindings, explain substitutions such as `%W`, `%x`, `%y`, `%K`, and
the event-specific substitutions documented by the official `bind` manual.
Do not imply that every callback accepts the same arguments: the event,
widget option, or command-prefix descriptor determines the appended arguments
and return contract. Prefer braced scripts and `[list ...]` construction when
that preserves the intended values and namespace.

## Layout, resources, and uncertainty

The assistant should request `tk_layout` before asserting a widget hierarchy,
geometry-manager assignment, or preview result. Report the model's source
spans and uncertainty reasons. Do not infer a final layout from a partial
snippet when the widget path, command, option, or value is computed.

Resource references such as images, fonts, menus, `-textvariable`,
`-listvariable`, and `-variable` are useful code relationships, but a complete
resource/lifetime graph is planned rather than a shipped preview guarantee.
Likewise, event/callback graphs are planned; recognized callback descriptors
must not be presented as proof that the callback will run.

Mention platform uncertainty for fonts, themes, DPI, native window-manager
behavior, keyboard conventions, and accessibility. A static webview preview
is a structural aid, not a native Tk rendering oracle.

## Discoverability

- [KCS feature index](README.md)
- [Tk Preview](kcs-feature-tk-preview.md)
- [Static Tk UI model](../../design/tk-static-ui-model.md)
