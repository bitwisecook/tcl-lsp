# BIG-IP Tcl log: nh20261012broad2, tmm-traffic-event-19

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **name-list-prefix:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **name-quoted-head:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

- **name-substituted-head:** Do lexical accepted names and runtime evaluated names differ? Compare braced/quoted/substituted/list command prefixes, brace-newline rules and unaddressable-name output round trips.

## Assumptions

- **name-list-prefix:** List prefix retains spaced head and data.
- **name-quoted-head:** Quoted spaced command spelling.
- **name-substituted-head:** Substitution forms a single command word.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-traffic-event-2](fixtures/bigip-tmm-traffic-event-2.md)

- run: `nh20261012broad2`
- case: `name-substituted-head`
- case.2: `name-quoted-head`
- case.3: `name-list-prefix`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head {proc {two words} x {return $x}}
```

### nh_operation

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head {"two words" {a b}}
```

### nh_cleanup

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_name_quoted_head* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*] [info commands ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*] [info vars ::__tcl_lsp_probe_nh20261012broad27_name_quoted_head*]⟦EOF⟧
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head {proc {two words} x {return $x}; set head {two words}}
```

### nh_operation.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head {$head {a b}}
```

### nh_cleanup.2

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_name_substituted_head* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*] [info commands ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*] [info vars ::__tcl_lsp_probe_nh20261012broad27_name_substituted_head*]⟦EOF⟧
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix {}
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix {proc {two words} x {return $x}; set prefix [list {two words} {a b}]}
```

### nh_operation.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix {eval $prefix}
```

### nh_cleanup.3

```tcl-visible
foreach nh_child [interp slaves] {if {[string match __tcl_lsp_probe_nh20261012broad27_name_list_prefix* $nh_child]} {interp delete $nh_child}}
foreach nh_ns [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*] {namespace delete $nh_ns}
foreach nh_cmd [info commands ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*] {rename $nh_cmd {}}
foreach nh_var [info vars ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*] {unset -nocomplain $nh_var}
list [namespace children :: ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*] [info commands ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*] [info vars ::__tcl_lsp_probe_nh20261012broad27_name_list_prefix*]⟦EOF⟧
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

- name-quoted-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- name-quoted-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-quoted-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-substituted-head, setup, completion 0, result:

```text-visible
two words⟦EOF⟧
```

- name-substituted-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-substituted-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-list-prefix, setup, completion 0, result:

```text-visible
{two words} {a b}⟦EOF⟧
```

- name-list-prefix, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-list-prefix, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-list-prefix: intended operation reached.

- name-quoted-head: intended operation reached.

- name-substituted-head: intended operation reached.

### tmm-traffic-event, unit 0:0, invocation 9

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-traffic-event.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- name-quoted-head, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- name-quoted-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-quoted-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-substituted-head, setup, completion 0, result:

```text-visible
two words⟦EOF⟧
```

- name-substituted-head, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-substituted-head, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-list-prefix, setup, completion 0, result:

```text-visible
{two words} {a b}⟦EOF⟧
```

- name-list-prefix, lookup, completion 0, result:

```text-visible
a b⟦EOF⟧
```

- name-list-prefix, runtime-cleanup, completion 0, result:

```text-visible
{} {} {}⟦EOF⟧
```

- name-list-prefix: intended operation reached.

- name-quoted-head: intended operation reached.

- name-substituted-head: intended operation reached.
