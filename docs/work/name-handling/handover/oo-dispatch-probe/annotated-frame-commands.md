# Annotated command records at frame call sites

The [item 93](../../plan.md) spike failed its coverage and answer criteria.
Each release returned 131 records for reached source positions across the 16
fixtures, including records at 12 of the 16 marked positions. In 8 cases the
record and an LSP consumer agreed with Tcl. In 3 cases an LSP consumer matched
Tcl and the record gave a different answer or left the target unresolved.
The remaining record, for `namespace inscope`, and the LSP both left the
target unresolved.

## Reproduce

From the repository root, with the pinned Tcl 8.6.18, 9.0.4 and 9.1.0 shells
available through the shared reference interpreter discovery:

```sh
cargo run --locked -p tcl-lsp-core --example annotated_frame_commands
```

The probe runs each unmodified [frame fixture](frames/) as a file in C Tcl,
then builds a compilation unit under the corresponding dialect. It enumerates
the original source positions where the retained compiler binding reports
reached dispatch and reads `invocation_at_source` at each position. It uses
the marked word for definition, hover and analyser diagnostics, and the
command head for binding. In f14 the editor queries `b` and the binding query
starts at `my`. In f06 and f13 the mark identifies a captured target operand,
whose dispatch takes place later through the alias or forwarder.

The Tcl fixtures print the selected procedure or a value unique to the
selected method. The negative control g01 catches the invalid command
completion and prints its message. All 48 runs exited successfully with empty
stderr, and each fixture gave the same stdout on the three releases.

The probe compares the record's proved procedure, selected receiver method or
definite absence with that answer. A unique definition at the expected
declaration line or an exact hover counts as a matching LSP answer. For g01
the matching observation is W123 and an empty definition. The comparison
uses [the archived LSP observations](frames.out) for 8.6 and 9.0 and fresh
observations for all three releases. Both sets identified the same matching
sites. The command reports the experimental verdict after measuring every
lane and exits with an error if it cannot complete a measurement.

## Marked sites

Positions are one-based line and column. `B` identifies `::B::m` and `C.b`
identifies `::C::b`.

| Fixture and position | Tcl stdout | Compiler record answer | Matching LSP answer |
|---|---|---|---|
| f01 namespace procedure, 4:19 | `::a::helper` | `::a::helper` | Yes |
| f02 namespace path, 5:19 | `::lib::helper` | `::lib::helper` | Yes |
| f03 TclOO defining namespace, 4:40 | `::helper` | `::ns::helper` | Yes |
| f04 TclOO `next`, 2:50 | `B` | Unresolved | Yes |
| f05 `apply` namespace, 3:19 | `::lib::helper` | `::lib::helper` | Yes |
| f06 alias target, 4:32 | `::helper` | **Missing record** | No |
| f07 global `uplevel`, 4:32 | `::helper` | `::helper` | Yes |
| f08 `namespace inscope`, 3:33 | `::lib::helper` | Unresolved | No |
| f09 `namespace eval`, 3:28 | `::a::helper` | `::a::helper` | Yes |
| f10 global `after` callback, 4:29 | `::helper` | **Missing record** | No |
| f11 import, 5:19 | `::lib::helper` | `::lib::helper` | Yes |
| f12 `namespace code` callback, 4:35 | `::lib::helper` | **Missing record** | No |
| f13 TclOO forward target, 4:36 | `::helper` | **Missing record** | No |
| f14 TclOO `my`, 1:36 | `C.b` | `::C::b` | Yes |
| g01 unrelated namespace, 3:19 | `1:invalid command name "helper"` | Absence unproved | Yes |
| g02 namespace path set later, 3:19 | `::lib::helper` | `::lib::helper` | Yes |
