# BIG-IP Tcl log: nh20261012broad1, tmm-procedure-2

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **duplicate-display-slots:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **empty-command-head:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

- **lone-colon-command:** Colon runs, empty command/tail, lone colon names, namespace segment colons and duplicate display FQNs: which slots are actually reached?

## Assumptions

- **duplicate-display-slots:** NH10 adapted names; invocation identity and displayed name are distinct observations.
- **empty-command-head:** Computed empty head; distinguish it from namespace-qualified empty tail.
- **lone-colon-command:** Lone-colon command in owned namespace.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-procedure-5](fixtures/bigip-tmm-procedure-5.md)

- case: `duplicate-display-slots`
- case.2: `lone-colon-command`
- case.3: `empty-command-head`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {}
namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval a {proc :p {} {return SECOND_SLOT}}
}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    list [namespace eval a: {p}] [namespace eval a {:p}]         [namespace eval a: {namespace which p}]         [namespace eval a {namespace which :p}]
}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_duplicate_display_slots* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*] [info commands ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*] [info vars ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {}
namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {:}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_lone_colon_command* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*] [info commands ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*] [info vars ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_empty_command_head {}
namespace eval ::__tcl_lsp_probe_nh20261012broad16_empty_command_head {proc {} {} {return EMPTY_HEAD}; set head {}}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad16_empty_command_head {$head}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad16_empty_command_head* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*] [info commands ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*] [info vars ::__tcl_lsp_probe_nh20261012broad16_empty_command_head*]⟦EOF⟧
```

## Results

### tmm-procedure, unit 0:0, invocation once; tmm-procedure, unit 0:1, invocation once; tmm-procedure, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" script line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colon_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.

### tmm-procedure, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-procedure.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- duplicate-display-slots, setup, completion 1, result:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- duplicate-display-slots, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots" script line 3)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_duplicate_display_slots {
    namespace eval a: {proc p {} {return FIRST_SLOT}}
    namespace eval ..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- duplicate-display-slots, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- lone-colon-command, setup, completion 1, result:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"⟦EOF⟧
```

- lone-colon-command, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- lone-colon-command, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":" in non-global namespace with name starting with ":"
    while executing
"proc : {} {return LONE_COLON}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad16_lone_colon_command" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad16_lone_colon_command {proc : {} {return LONE_COLON}}"
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- lone-colon-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- empty-command-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- empty-command-head, lookup, completion 0, result:

```text-visible
EMPTY_HEAD⟦EOF⟧
```

- empty-command-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- duplicate-display-slots: intended operation not reached.

- empty-command-head: intended operation reached.

- lone-colon-command: intended operation not reached.
