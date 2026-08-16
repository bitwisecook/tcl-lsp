# KCS: W103 — Does open with a pipeline prefix execute shell commands?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn when `open` is called with a `|` pipeline prefix?

## Why

A pipe prefix in the path executes shell commands under attacker control, enabling arbitrary command execution on the host.

## Symptoms

- A yellow squiggle appears under the first argument of the `open` call, with a message such as: "open with a pipeline containing variable/command substitution risks command injection. Validate and sanitize the command before passing to open."
- For a literal pipeline (no substitution), the severity is a hint: "open with a pipeline (&quot;|&quot;) executes an external command. Ensure the command is not influenced by untrusted input."
- For a bare variable argument: "open with a variable argument: if the value starts with &quot;|&quot;, it will execute a command pipeline. Validate input or use explicit I/O commands."

## Example that triggers it

```tcl
open "|$cmd"
```

The analyser reports **`W103`** on the `|$cmd` argument token.

## Fix

```tcl
open $validated_path
```

Validate that the path does not begin with `|` before passing it to `open`.

## How to suppress

Add `# noqa: W103` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W300`, `W313`
