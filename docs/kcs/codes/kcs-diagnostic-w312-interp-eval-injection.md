# KCS: W312 — Why does the analyser warn about `interp eval` with an unbraced script?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag `interp eval` (or `interp invokehidden`) when
the script argument is not braced, or when there is more than one script
word?

## Why

`interp eval` runs a script in a child interpreter. The script text is
substituted in the *parent* first, so a `$variable` or `[command]` in an
unbraced script word is expanded before the child ever sees it — anything
that value contains becomes code in the child. And with several script
words, `interp eval` concatenates them into one script exactly as `eval`
does, which makes the join points injection sites too.

W312 is the cross-interpreter twin of `W301`, which reports the same
shape for `uplevel`.

## Symptoms

- A yellow squiggle on the first script word, with the message "interp
  eval with an unbraced script argument may cause code injection. Use
  braces: interp eval $child {...}".
- For the multi-word form, the message is "interp eval with multiple
  arguments concatenates them into a script (like eval). Use a single
  braced body to avoid injection."

## Example that triggers it

```tcl
set child [interp create]
set name [gets stdin]
interp eval $child "set user $name"
```

The analyser reports **`W312`** on the script word: `$name` is
substituted in the parent, so a value containing `[exec …]` or a `;`
becomes code in the child.

## Fix

```tcl
set child [interp create]
set name [gets stdin]
interp eval $child [list set user $name]
```

`list` builds a script whose words are quoted properly, so the value can
never be read as code. A fully literal braced body works too when nothing
has to be passed in:

```tcl
interp eval $child {set user anonymous}
```

The check covers `interp eval` and `interp invokehidden`, plus Tk's
`console eval` and `consoleinterp eval` / `consoleinterp record`. A
legal abbreviation such as `interp ev` resolves to the same subcommand
and is checked the same way. A braced script word, or one the analyser
can prove is a canonical list, does not fire.

## How to suppress

`W312` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=W312`
directive at the top of the file, or for a whole project with
`disabled = W312` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W101`, `W301`, `W309`, `T105`
