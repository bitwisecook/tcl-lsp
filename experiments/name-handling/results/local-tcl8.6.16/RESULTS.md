# Installed C Tcl 8.6.16 observations

Exploratory measurements from a fresh installed interpreter. This version is
outside the pinned 8.6.18 reference. Matching source/build provenance was not
validated. These results establish no complete C matrix, latest Jim result or
F5 answer. No conformance fixture is promoted from this run.

| Case | Completion code | Observed result |
| --- | --- | --- |
| NH01 | 0 | `CURRENT` |
| NH02 | 0 | `GLOBAL_TARGET` |
| NH03 | 0 | `PATH_TARGET` |
| NH04 | 1 | `invalid command name "parent_only"` |
| NH05 | 0 | `7 7` |
| NH06 | 0 | `GLOBAL_VALUE` |
| NH07 | 0 | `2 2` |
| NH08 | 0 | `EMPTY LITERAL_DOLLAR LITERAL_BRACES 1` |
| NH09 | 0 | `NORMAL EMPTY_TAIL` |
| NH10 | 0 | `FIRST_SLOT SECOND_SLOT ::__nh_10::a:::p ::__nh_10::a:::p` |
| NH11 | 0 | `ORIGINAL ::__nh_11_src::pp` |
| NH12 | 0 | `NEW` |
| NH13 | 0 | `OLD NEW` |
| NH14 | 0 | `2` |
| NH15 | 0 | `NAMESPACE_VALUE` |
| NH16 | 0 | `GLOBAL_IMPORT ::__nh_16_source {}` |

NH04 records the ordinary missing-command error; it is not a runner failure.
NH10 reaches two distinct slots whose display strings are identical. NH12
observes the new implementation through the imported name on this interpreter.
NH16 removes the local import: `info commands` for the local slot is empty,
while `namespace which` falls back to the still-existing global source command.

Exact script identity and invocation are in [provenance.json](provenance.json).
Raw channels are [stdout.txt](stdout.txt) and [stderr.txt](stderr.txt); byte-valued
decoded records are in [observations.json](observations.json).
