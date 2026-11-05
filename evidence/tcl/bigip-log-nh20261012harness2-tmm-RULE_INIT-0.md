# BIG-IP Tcl log: nh20261012harness2, tmm-RULE_INIT-0

## Question

The following cases investigate Tcl behavior relevant to name resolution and diagnostics in the LSP, compiler and VM. Each observation is limited to its own case and recorded context.

- **real-harness-explicit-source-child:** Which namespace subcommands, interp forms and dynamic definition operations exist in each context? Are unsupported literal commands rejected at load even in `catch`? Compare computed head/eval.

## Assumptions

- **real-harness-explicit-source-child:** Independent child-interpreter real-package capability. Does not substitute for native-parent package require or original suite.

Other assumptions were not separately recorded.

## Script

Fixture: [bigip-tmm-rule-init-1](fixtures/bigip-tmm-rule-init-1.md)

- run: `nh20261012harness2`
- case: `real-harness-explicit-source-child`

### nh_setup

```tcl-visible
set nh_child [interp create]; set nh_module {}; append nh_module [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module0::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module1::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module2::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module3::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module4::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module5::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module6::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module7::bytes] [call /Common/__tcl_lsp_probe_nh20261012harness2_0_module8::bytes]; list child-provider [interp eval $nh_child {list [info patchlevel] [info tclversion] [namespace current]}] module-bytes [string length $nh_module]⟦EOF⟧
```

### nh_operation

```tcl-visible
interp eval $nh_child $nh_module⟦EOF⟧
```

### nh_cleanup

```tcl-visible
interp delete $nh_child⟦EOF⟧
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

- real-harness-explicit-source-child, setup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be sure to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.

### tmm-RULE_INIT, unit 0:3, invocation once

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel Not recorded.
- Recorded input channel: tmm-RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are log lines selected by the recorded run, case, context, unit and invocation; they are not process stdout. Multiple log snapshots may contain the same line, so duplicates do not by themselves establish repeated executions. The following entries decode each retained NHO payload chunk separately.

- real-harness-explicit-source-child, setup, completion 0, result:

```text-visible
child-provider {8.4.6 8.4 ::} module-bytes 98763⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, result:

```text-visible
invalid command name "pwd"⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, metadata:

```text-visible
errorcode-present 1 errorinfo-present 1 return-options unavailable⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorcode:

```text-visible
NONE⟦EOF⟧
```

- real-harness-explicit-source-child, lookup, completion 1, errorinfo:

```text-visible
invalid command name "pwd"
    while executing
"pwd"
    (in namespace eval "::tcltest" script line 245)
    invoked from within
"namespace eval tcltest {

    # When the version number changes, be sure to update the pkgIndex.tcl file,
    # and the install directory in the Makef..."
    invoked from within
"interp eval $nh_child $nh_module"
    ("eval" body line 1)
    invoked from within
"eval $nh_operation"⟦EOF⟧
```

- real-harness-explicit-source-child, runtime-cleanup, completion 0, result:

```text-visible
⟦EOF⟧
```

- real-harness-explicit-source-child: intended operation reached.
