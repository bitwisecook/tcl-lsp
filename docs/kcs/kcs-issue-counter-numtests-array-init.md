# KCS: tcltest `numTests(Failed)` reads as empty string

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI

## Question

Why does a tcltest summary line print
`Total 9 Passed 9 Skipped 0 Failed` — with no digit after `Failed` —
instead of `Failed 0` when every test passed?

## Symptoms

- `::tcltest::numTests(Failed)` reads back as an empty string rather than
  `0` when the counter was never incremented.
- The other slots (`Total`, `Passed`, `Skipped`) print correctly, because
  each was incremented at least once during the run.

## Answer

This is a symptom of the retired 1.x Python engine, not of the native
toolchain. Two code paths wrote to the same logical variable but disagreed
about its name: tcltest's own `ArrayDefault` helper seeded the array under
the bare name `numTests`, while the compiled procs read and wrote a
fully-qualified `::tcltest::numTests`. The slot the compiled procs read
was therefore never the slot the initial `array set` filled, so an
un-incremented counter came back as the null value it was created with,
whose string form is empty.

If you see this, you are on a 1.x Python build. Move to a current native
build. The native engine resolves a bare name inside a `namespace eval`
body to the namespace-qualified variable, so the initialising `array set`
and the compiled `incr` land in the same slot and an untouched counter
reads back as `0`.

## How to check which one you are on

Run the initialisation and the read through the same namespace, the way
tcltest does:

```tcl
namespace eval ::tt {
    proc ArrayDefault {varName value} {
        variable $varName
        array set $varName $value
    }
    ArrayDefault numTests [list Total 0 Passed 0 Skipped 0 Failed 0]
    proc bump {} {
        variable numTests
        incr numTests(Passed)
    }
}
::tt::bump
puts "Passed $::tt::numTests(Passed) Failed $::tt::numTests(Failed)"
```

A correct engine prints `Passed 1 Failed 0`. An engine with the split
prints an empty value after `Failed`.

## Related

- [KCS index](README.md)
