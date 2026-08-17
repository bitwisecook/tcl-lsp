# KCS: W126 — Why does the analyser flag a non-channel value in a channel argument?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a non-channel value is used where a channel is expected?

## Why

Commands like `puts`, `gets`, and `close` expect a channel identifier (e.g. `stdout`, `$fd`). Passing an ordinary string instead will cause a runtime error such as "can not find channel named ...".

## Symptoms

- A yellow squiggle appears under the argument, with the message "non-channel value in channel argument position".

## Example that triggers it

```tcl
puts "output.txt" "hello"
```

The analyser reports **`W126`** on the `"output.txt"` argument.

## Fix

```tcl
set fd [open "output.txt" w]
puts $fd "hello"
close $fd
```

Open the file first and pass the resulting channel handle.

## How to suppress

Add `# noqa: W126` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W200`, `E002`
