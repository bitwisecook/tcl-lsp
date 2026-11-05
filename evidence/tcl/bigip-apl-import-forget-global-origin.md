# BIG-IP APL callback: import-forget-global-origin

## Question

Import/export/force/forget/origin support and lifecycle; global-origin imports, source rename/redefinition/deletion, chains and conflicts.

This callback addresses only the `import-forget-global-origin` case in that broader question. Its result can inform name resolution and diagnostics in the LSP, compiler and VM, but does not establish the behavior of other cases or environments.

## Assumptions

Recorded case scope: NH16 adapted names; fresh interpreter only, global export state changes; clean ::__tcl_lsp_probe_nh20261012aplbroad1_import_forget_global_origin_source. Child-interpreter adaptation.

Other assumptions were not separately recorded. The APL callback was selected through the browser; this is not an iApp implementation, an iRule, or a stock Tcl measurement. A successful caught cleanup does not prove that every cleanup operation succeeded.

## Script

Fixture: [bigip-iapp-apl-1](fixtures/bigip-iapp-apl-1.md)

- run: `nh20261012aplbroad1`
- case: `import-forget-global-origin`

### nh_setup

```tcl-visible
set nh_fresh [interp create]⟦EOF⟧
```

### nh_operation

```tcl-visible
interp eval $nh_fresh [binary format H* 6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b7d0a70726f63203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f75726365207b7d207b72657475726e20474c4f42414c5f494d504f52547d0a6e616d657370616365206576616c203a3a207b6e616d657370616365206578706f7274205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263657d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b6e616d65737061636520696d706f7274203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263653b20736574206265666f7265205b5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d7d0a6e616d657370616365206576616c203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e207b6e616d65737061636520666f72676574203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263653b206c69737420246265666f7265205b6e616d657370616365207768696368205f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d205b696e666f20636f6d6d616e6473203a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e3a3a5f5f74636c5f6c73705f70726f62655f6e68323032363130313261706c62726f6164315f696d706f72745f666f726765745f676c6f62616c5f6f726967696e5f736f757263655d7d0a]⟦EOF⟧
```

### nh_cleanup

```tcl-visible
interp delete $nh_fresh⟦EOF⟧
```

## Results

### BIG-IP iApp APL

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.
- Recorded input channel: browser Application Services template choice callback.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result: decoded below.

These are the retained DOM option values projected one row per value, not process stdout. The fields below decode their hexadecimal payloads; completion codes are the callback's caught Tcl codes.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
interp0⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
GLOBAL_IMPORT ::__tcl_lsp_probe_nh20261012aplbroad1_import_forget_global_origin_source {}⟦EOF⟧
```

- runtime-cleanup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
