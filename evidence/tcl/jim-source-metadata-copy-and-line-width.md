# Jim source metadata copy and line width

## Question

Does annotation copy the source string, how does a wide line narrow, and
what does an invalid line report?

## Assumptions

A fresh pinned Jim interpreter uses stock commands, with no extensions.

## Script

```tcl-visible
set original [list a b]
set tagged [info source $original f.tcl 4294967295]
puts [list [info source $original] [info source $tagged] $tagged]
puts [list [catch {info source $original f.tcl bad} message] $message]
```

## Results

### Jim 0.84-15-g2f8e155

Invocation: `/workspace/tcl-lsp/tmp/jimtcl-oracle/jimsh`, script on standard input.
The source revision is pinned by `rust/tcl-test-support/jim-reference.txt`.

- Exit status: 0.
- Standard output:

```text-visible
{{} 1} {f.tcl -1} {a b}
1 {expected integer but got "bad"}
```

- Standard error: empty.
