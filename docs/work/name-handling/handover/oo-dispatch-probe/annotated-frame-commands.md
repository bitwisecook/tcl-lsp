# Annotated command records at frame call sites

The [item 93](../../plan.md) probe passed on C Tcl 8.6.18, 9.0.4 and 9.1.0.
Each release yielded 136 records across the fixtures, including all 16 marked
positions. The records matched Tcl in all 33
comparisons where an archived or current LSP consumer gave the correct answer.
They also matched Tcl for the alias, `after` callback and TclOO forward.
`namespace inscope` and the deferred `namespace code` body retain records with
unresolved targets. The LSP abstains from an exact answer at both sites.

## Reproduce

From the repository root, with the pinned Tcl 8.6.18, 9.0.4 and 9.1.0 shells
available through the shared reference interpreter discovery:

```sh
cargo run --locked -p tcl-lsp-core --example annotated_frame_commands
```

The probe runs each unmodified [frame fixture](frames/) as a file in C Tcl,
then builds a compilation unit under the corresponding dialect. It enumerates
the original source positions where the retained compiler binding reports
reached dispatch and reads `invocation_at_source` at each position. It also
projects future-script bindings through their contiguous source mappings and
links reached alias and receiver invocations to the original captured prefix
words. A future entry carries its conditional frame and lookup state. In f06
and f13 the mark identifies a captured operand and the reached position
belongs to the alias or receiver invocation that selects it.

The probe uses the marked word for definition, hover and analyser diagnostics,
and the command head for binding. In f14 the editor queries `b` and the binding
query starts at `my`. The receiver annotation uses the original TclOO lookup
path. The plain `next` case uses the retained stock helper, entered method
and sole superclass to identify the provider conditional on helper selection.
The absence answer concerns the invoked name's slot before Tcl's unresolved
command handler.

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
sites. The command exits with an error for missing records, contradictory
answers, or failure to match Tcl wherever an existing consumer does.

## Marked sites

Positions are one-based line and column. `B` identifies `::B::m` and `C.b`
identifies `::C::b`.

| Fixture and position | Tcl stdout | Compiler record answer | Matching LSP answer |
|---|---|---|---|
| f01 namespace procedure, 4:19 | `::a::helper` | `::a::helper` | Yes |
| f02 namespace path, 5:19 | `::lib::helper` | `::lib::helper` | Yes |
| f03 TclOO defining namespace, 4:40 | `::helper` | `::helper` | Yes |
| f04 TclOO `next`, 2:50 | `B` | `::B::m` | Yes |
| f05 `apply` namespace, 3:19 | `::lib::helper` | `::lib::helper` | Yes |
| f06 alias target, 4:32 | `::helper` | `::helper` | No |
| f07 global `uplevel`, 4:32 | `::helper` | `::helper` | Yes |
| f08 `namespace inscope`, 3:33 | `::lib::helper` | Unresolved | No |
| f09 `namespace eval`, 3:28 | `::a::helper` | `::a::helper` | Yes |
| f10 global `after` callback, 4:29 | `::helper` | `::helper` | No |
| f11 import, 5:19 | `::lib::helper` | `::lib::helper` | Yes |
| f12 `namespace code` callback, 4:35 | `::lib::helper` | Unresolved | No |
| f13 TclOO forward target, 4:36 | `::helper` | `::helper` | No |
| f14 TclOO `my`, 1:36 | `C.b` | `::C::b` | Yes |
| g01 unrelated namespace, 3:19 | `1:invalid command name "helper"` | Absent | Yes |
| g02 namespace path set later, 3:19 | `::lib::helper` | `::lib::helper` | Yes |
