# BIG-IP Tcl log: nh20261012pat1, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **alias-prefix-lifetime:** After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?

- **computed-binding-mutation:** When a command name is assembled into a variable and the command at that name is replaced, does executing the unchanged variable use the original or the replacement binding?

- **computed-qualified:** Can a qualified command name assembled from a namespace variable be invoked as a computed command head, and which command does that spelling reach?

## Assumptions

- **alias-prefix-lifetime:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **computed-binding-mutation:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.
- **computed-qualified:** Adapted wrapper execution namespace/frame; not exact portable input. Setup includes all original commands before the final one.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-2](fixtures/bigip-tmm-rule-init-2.md)

- run: `nh20261012pat1`
- case: `computed-binding-mutation`
- case.2: `alias-prefix-lifetime`
- case.3: `computed-qualified`

### nh_setup

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime {}
namespace eval ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime {proc emit {fixed value} {list $fixed $value}}
interp alias {} ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::local {} ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::emit {a b}
rename ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::local ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::renamed
set description [interp alias {} ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::local]
```

### nh_operation

```tcl-visible
list $description [::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::renamed {c d}]
```

### nh_cleanup

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime]} {namespace delete ::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime}
```

### nh_setup.2

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation {}
namespace eval ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation {proc emit {} {return first}}
set ns ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation
set head ${ns}::emit
rename ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation::emit ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation::old
proc ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation::emit {} {return replacement}
```

### nh_operation.2

```tcl-visible
$head
```

### nh_cleanup.2

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation]} {namespace delete ::__tcl_lsp_probe_nh20261012pat15_computed_binding_mutation}
```

### nh_setup.3

```tcl-visible
namespace eval ::__tcl_lsp_probe_nh20261012pat15_computed_qualified {}
namespace eval ::__tcl_lsp_probe_nh20261012pat15_computed_qualified {proc emit {} {return first}}
set ns ::__tcl_lsp_probe_nh20261012pat15_computed_qualified
set head ${ns}::emit
```

### nh_operation.3

```tcl-visible
$head
```

### nh_cleanup.3

```tcl-visible
if {[namespace exists ::__tcl_lsp_probe_nh20261012pat15_computed_qualified]} {namespace delete ::__tcl_lsp_probe_nh20261012pat15_computed_qualified}
```

## Results

### tmm-RULE_INIT, unit 0:0, invocation once; tmm-RULE_INIT, unit 0:1, invocation once; tmm-RULE_INIT, unit 0:2, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- alias-prefix-lifetime, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::emit {a b}⟦EOF⟧
```

- alias-prefix-lifetime, lookup, completion 0, result:

```text-visible
{::__tcl_lsp_probe_nh20261012pat15_alias_prefix_lifetime::emit {a b}} {{a b} {c d}}⟦EOF⟧
```

- alias-prefix-lifetime, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, setup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-binding-mutation, lookup, completion 0, result:

```text-visible
replacement⟦EOF⟧
```

- computed-binding-mutation, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- computed-qualified, setup, completion 0, result:

```text-visible
::__tcl_lsp_probe_nh20261012pat15_computed_qualified::emit⟦EOF⟧
```

- computed-qualified, lookup, completion 0, result:

```text-visible
first⟦EOF⟧
```

- computed-qualified, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- alias-prefix-lifetime: intended operation reached.

- computed-binding-mutation: intended operation reached.

- computed-qualified: intended operation reached.
