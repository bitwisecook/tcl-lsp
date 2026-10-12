# Native stringification and cleanup callbacks

These retained native fixtures distinguish top-level custom scalar conversion
from ordinary List/Dict length, including generic and compiled execution, on
all five pinned C releases and Jim 0.84. `probe.c`/`jim-probe.c` install a
custom object string updater which changes interpreter-visible state.
`free-probe.c` also discriminates a mutating cleanup callback.
`empty-foreach-probe.c` and its Jim counterpart return an empty list string
while changing state before the zero-trip body decision.

The adjacent original native observation JSON and exact text outputs retain
every measured case, including releases without Dict support. The separate
fixture-hash manifest pins the published source/output bytes. These sources
can be rebuilt with the same version-selected static-library commands as the
parent `native_list_methods` README. Jim sources link the configured pinned
`libjim.a` with `-ldl -lm`.

An ordinary outer List/Dict cannot serve as proof of stock children or of a
custom scalar's conversion. Zero iterations do not establish an unchanged
world: native conversion can invoke the updater before the body is skipped.
Interpreter-world effects, cache/representation changes and completion/result
facts remain independent obligations in registry and source projections.
