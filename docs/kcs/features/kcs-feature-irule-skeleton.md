# KCS: feature — iRule Event Skeleton

> **Audience:** User
> **Type:** Functionality

## Summary

Pick iRules events from a list and generate a skeleton iRule with those event handlers.

## Applies to

VS Code, Copilot Chat, Claude skill

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Insert iRule Event Skeleton` |
| VS Code chat | `@irule /scaffold` |
| Claude Code | `/irule-scaffold` |

## How to use

- **VS Code**: Run `Tcl: Insert iRule Event Skeleton` from the command palette. A multi-select picker shows all available events grouped by category. The generated skeleton includes `when` blocks with comments describing when each event fires.
- **VS Code chat**: `@irule /scaffold` generates a skeleton with AI-guided event selection.
- **Claude Code**: `/irule-scaffold` generates a skeleton interactively.

## Operational context

The skeleton generator uses the event registry to know which events exist, their firing order, and which commands are valid in each event. This helps users start new iRules with the correct structure.

## Failure modes

- Missing events after registry updates.

## Screenshots

- `23-irule-skeleton` — event picker for skeleton generation

![event picker for skeleton generation](../screenshots/23-irule-skeleton.png)

## Discoverability

- [KCS feature index](README.md)
- [Command registry event model](../../../docs/design/contracts/command-registry-event-model.md)
