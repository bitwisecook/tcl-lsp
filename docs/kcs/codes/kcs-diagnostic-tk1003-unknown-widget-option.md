# KCS: TK1003 — Why does the analyser say this widget option is unknown?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag a `-option` I passed when creating a widget?

## Why

Each Tk widget command declares the options it accepts. Passing one it
does not know raises "unknown option" when the script runs, and the
widget is never created. A misspelling is the common cause, so the
analyser offers the closest declared option as a suggestion.

## Symptoms

- A hint underline on the option word, with the message "Unknown option
  '-tex' for button. Did you mean '-text'?"
- A **Replace with '-text'** quick fix on the diagnostic when a close
  match exists.

## Example that triggers it

```tcl
package require Tk

button .ok -tex "OK" -comand submit
```

The analyser reports **`TK1003`** on `-tex` and again on `-comand`.

## Fix

```tcl
package require Tk

button .ok -text "OK" -command submit
```

Apply the suggested replacement, or correct the option by hand.

## When it does not fire

- **Outside a Tk document.** The check needs the `tk` dialect or a
  `package require Tk` the analyser can resolve.
- **On an unmodelled widget.** A widget command with no registry entry is
  never checked, so a custom widget cannot produce a false positive.
- **On an abbreviation.** Tk accepts any unique prefix of a declared
  option, and so does the check. An ambiguous prefix is left alone.
- **On a dynamic option word** (`-$style`, `-[pick]`) — the analyser
  cannot know what it resolves to.
- **After the first non-option word**, where Tk itself stops parsing
  options.

Option values are skipped, so a value that itself starts with `-` (the
`-2` in `-padx -2`) is read as data, not as an unknown option.

## How to suppress

`TK1003` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=TK1003`
directive at the top of the file, or for a whole project with
`disabled = TK1003` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `TK1001`, `TK1002`, `W004`, `W145`
