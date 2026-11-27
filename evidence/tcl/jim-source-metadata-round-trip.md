# Jim source metadata round trip

## Question

Does `info source` annotate a string and retrieve its filename and line, and
what does an unannotated list report?

## Assumptions

A fresh source-built Jim interpreter uses stock commands. This establishes
only the pinned Jim release, with no extensions or host embedding.

## Script

```tcl-visible
set value [info source {puts hi} f.tcl 12]
puts [list $value [info source $value] [info source [list a b]]]
```

## Results

### Jim 0.84-15-g2f8e155

Invocation: `/workspace/tcl-lsp/tmp/jimtcl-oracle/jimsh`, script on standard input.
The source revision is pinned by `rust/tcl-test-support/jim-reference.txt`.

- Exit status: 0.
- Standard output:

```text-visible
{puts hi} {f.tcl 12} {{} 1}
```

- Standard error: empty.
