# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-20

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **brace-newline-command:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **display-name-roundtrip:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **name-script-prefix-near-miss:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

## Assumptions

- **brace-newline-command:** Actual newline in runtime script; keep literal-load variant separate.
- **display-name-roundtrip:** Display-FQN roundtrip after duplicate-slot setup; returned spelling need not identify the original command slot.
- **name-script-prefix-near-miss:** String script reparses spaced head.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-9](fixtures/bigip-tmm-traffic-event-9.md)

- case: `name-script-prefix-near-miss`
- case.2: `display-name-roundtrip`
- case.3: `brace-newline-command`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {proc {two words} x {return $x}; set head {two words}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*] [info commands ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*] [info vars ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command {}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command {list a b
{c}}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_brace_newline_command* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*] [info commands ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*] [info vars ::__tcl_lsp_probe_nh20261012broad27_brace_newline_command*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :p {} {return SECOND_SLOT}}}
```

### nh_operation.3

```tcl-visible
set ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip::displayed [namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip::a: {namespace which p}]; eval [list [set ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip::displayed]]
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_display_name_roundtrip* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*] [info commands ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*] [info vars ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip*]⟦EOF⟧
```

## Results

### tmm-traffic-event, unit 0:0, invocation 0; tmm-traffic-event, unit 0:0, invocation 1; tmm-traffic-event, unit 0:0, invocation 10; tmm-traffic-event, unit 0:0, invocation 11; tmm-traffic-event, unit 0:0, invocation 12; tmm-traffic-event, unit 0:0, invocation 13; tmm-traffic-event, unit 0:0, invocation 14; tmm-traffic-event, unit 0:0, invocation 15; tmm-traffic-event, unit 0:0, invocation 2; tmm-traffic-event, unit 0:0, invocation 3; tmm-traffic-event, unit 0:0, invocation 4; tmm-traffic-event, unit 0:0, invocation 5; tmm-traffic-event, unit 0:0, invocation 6; tmm-traffic-event, unit 0:0, invocation 7; tmm-traffic-event, unit 0:0, invocation 8

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-script-prefix-near-miss, setup, completion 0, result:

```text-visible
two words⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, result:

```text-visible
invalid command name "two"⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- name-script-prefix-near-miss, lookup, completion 1, errorinfo:

```text-visible
invalid command name "two"
    while executing
"two words {a b}"
    ("eval" body line 1)
    invoked from within
"eval "$head {a b}""
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_script_prefix_near_miss {eval "$head {a b}"}"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- name-script-prefix-near-miss, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- brace-newline-command, lookup, completion 0, result:

```text-visible
a b c⟦EOF⟧
```

- brace-newline-command, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, result:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- display-name-roundtrip, setup, completion 1, errorinfo:

```text-visible
can't create procedure ":p" in non-global namespace with name starting with ":"
    while executing
"proc :p {} {return SECOND_SLOT}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip::a" script line 1)
    invoked from within
"namespace eval a {proc :p {} {return SECOND_SLOT}}"
    (in namespace eval "::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip" script line 1)
    invoked from within
"namespace eval ::__tcl_lsp_probe_nh20261012broad27_display_name_roundtrip {namespace eval a: {proc p {} {return FIRST_SLOT}}; namespace eval a {proc :..."
    ("eval" body line 2)
    invoked from within
"eval $nh_setup"⟦EOF⟧
```

- display-name-roundtrip, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- brace-newline-command: intended operation reached.

- display-name-roundtrip: intended operation not reached.

- name-script-prefix-near-miss: intended operation reached.
