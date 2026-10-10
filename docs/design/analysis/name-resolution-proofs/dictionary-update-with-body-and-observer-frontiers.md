# naming.dictionary.update-with-body-and-observer-frontiers

Kind: `native-observation`

## Problem statement

Compiled and generic dict update/with can complete a body, remove or replace its root, or call read/write observers. A final list result can hide an aborted native process or a body-to-writeback frontier.

## Question

What original body/callback/key/result completions occur in the fixed compiled and generic update/with programs?

## Conclusion

The complete native process tables retain 329 code/result references. Seven signal-terminated write-error controls remain separately captured, never counted as positive comparisons. Tcl 8.4 dict absence and Jim trace absence do not establish entered body/writeback behavior. No private return state is inferred.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-8`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 56
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "compiled-update-plain\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-remove\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-root-unset\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-root-invalid\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-root-replace\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-guest-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-guest-return\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-plain\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-remove\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-root-unset\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-root-invalid\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-root-replace\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-guest-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-guest-return\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-with-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ncompiled-update-raw-nul\t1\t63616e2774207265616420226b223a206e6f2073756368207661726961626c65\ncompiled-with-raw-nul\t1\t63616e2774207265616420226b223a206e6f2073756368207661726961626c65\ncompiled-update-raw-ff\t1\t63616e2774207265616420226bff7461696c223a206e6f2073756368207661726961626c65\ncompiled-with-raw-ff\t1\t63616e2774207265616420226bff7461696c223a206e6f2073756368207661726961626c65\ncompiled-update-modified-nul\t1\t63616e2774207265616420226bc0807461696c223a206e6f2073756368207661726961626c65\ncompiled-with-modified-nul\t1\t63616e2774207265616420226bc0807461696c223a206e6f2073756368207661726961626c65\ncompiled-update-array-name\t1\t63616e27742072656164202261287129223a206e6f2073756368207661726961626c65\ncompiled-with-array-name\t1\t63616e27742072656164202261287129223a206e6f2073756368207661726961626c65\ngeneric-update-plain\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-remove\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-root-unset\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-root-invalid\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-root-replace\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-guest-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-guest-return\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-plain\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-remove\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-root-unset\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-root-invalid\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-root-replace\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-guest-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-guest-return\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-with-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d65202264696374227d2030207b6b20424153457d\ngeneric-update-raw-nul\t1\t63616e2774207265616420226b223a206e6f2073756368207661726961626c65\ngeneric-with-raw-nul\t1\t63616e2774207265616420226b223a206e6f2073756368207661726961626c65\ngeneric-update-raw-ff\t1\t63616e2774207265616420226bff7461696c223a206e6f2073756368207661726961626c65\ngeneric-with-raw-ff\t1\t63616e2774207265616420226bff7461696c223a206e6f2073756368207661726961626c65\ngeneric-update-modified-nul\t1\t63616e2774207265616420226bc0807461696c223a206e6f2073756368207661726961626c65\ngeneric-with-modified-nul\t1\t63616e2774207265616420226bc0807461696c223a206e6f2073756368207661726961626c65\ngeneric-update-array-name\t1\t63616e27742072656164202261287129223a206e6f2073756368207661726961626c65\ngeneric-with-array-name\t1\t63616e27742072656164202261287129223a206e6f2073756368207661726961626c65\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-9`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 56
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "compiled-update-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-update-remove\t0\t30207b7d2030207b7d\ncompiled-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-update-local-read-error\t0\t30207b7d2030207b7d\ncompiled-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b2d636f64652030202d6c6576656c20307d\ncompiled-with-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-with-remove\t0\t30207b7d2030207b7d\ncompiled-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-with-local-read-error\t0\t30207b7d2030207b7d\ncompiled-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-with-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ncompiled-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ncompiled-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-update-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-update-remove\t0\t30207b7d2030207b7d\ngeneric-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-update-local-read-error\t0\t30207b7d2030207b7d\ngeneric-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ngeneric-with-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-with-remove\t0\t30207b7d2030207b7d\ngeneric-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-with-local-read-error\t0\t30207b7d2030207b7d\ngeneric-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-with-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ngeneric-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-10`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": -11,
      "rows": 1,
      "stdout_hex": "7b2263617365223a2267656e657269632d776974682d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a"
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 55
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "compiled-update-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-update-remove\t0\t30207b7d2030207b7d\ncompiled-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-update-local-read-error\t0\t30207b7d2030207b7d\ncompiled-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d203020776f726b\ncompiled-with-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-with-remove\t0\t30207b7d2030207b7d\ncompiled-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-with-local-read-error\t0\t30207b7d2030207b7d\ncompiled-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-with-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d203020776f726b\ncompiled-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ncompiled-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-update-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-update-remove\t0\t30207b7d2030207b7d\ngeneric-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-update-local-read-error\t0\t30207b7d2030207b7d\ngeneric-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ngeneric-with-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-with-remove\t0\t30207b7d2030207b7d\ngeneric-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-with-local-read-error\t0\t30207b7d2030207b7d\ngeneric-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-11`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": -4,
      "rows": 1,
      "stdout_hex": "7b2263617365223a22636f6d70696c65642d7570646174652d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a223331323037623633363136653237373432303733363537343230323236343232336132303537353234393534343537643230333032303737366637323662222c22726573756c745f74797065223a226c697374222c226572726f725f636f6465223a6e756c6c2c226572726f725f696e666f223a6e756c6c2c2272657475726e5f6f7074696f6e73223a223264363336663634363532303330323032643663363537363635366332303330227d0a"
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": -4,
      "rows": 1,
      "stdout_hex": "7b2263617365223a22636f6d70696c65642d776974682d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a223331323037623633363136653237373432303733363537343230323236343232336132303537353234393534343537643230333032303737366637323662222c22726573756c745f74797065223a226c697374222c226572726f725f636f6465223a6e756c6c2c226572726f725f696e666f223a6e756c6c2c2272657475726e5f6f7074696f6e73223a223264363336663634363532303330323032643663363537363635366332303330227d0a"
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": -11,
      "rows": 1,
      "stdout_hex": "7b2263617365223a2267656e657269632d776974682d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a"
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 55
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "compiled-update-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-update-remove\t0\t30207b7d2030207b7d\ncompiled-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-update-local-read-error\t0\t30207b7d2030207b7d\ncompiled-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-with-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-with-remove\t0\t30207b7d2030207b7d\ncompiled-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-with-local-read-error\t0\t30207b7d2030207b7d\ncompiled-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ncompiled-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-update-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-update-remove\t0\t30207b7d2030207b7d\ngeneric-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-update-local-read-error\t0\t30207b7d2030207b7d\ngeneric-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ngeneric-with-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-with-remove\t0\t30207b7d2030207b7d\ngeneric-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-with-local-read-error\t0\t30207b7d2030207b7d\ngeneric-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-12`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": -4,
      "rows": 1,
      "stdout_hex": "7b2263617365223a22636f6d70696c65642d7570646174652d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a223331323037623633363136653237373432303733363537343230323236343232336132303537353234393534343537643230333032303737366637323662222c22726573756c745f74797065223a226c697374222c226572726f725f636f6465223a6e756c6c2c226572726f725f696e666f223a6e756c6c2c2272657475726e5f6f7074696f6e73223a223264363336663634363532303330323032643663363537363635366332303330227d0a"
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": -4,
      "rows": 1,
      "stdout_hex": "7b2263617365223a22636f6d70696c65642d776974682d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a223331323037623633363136653237373432303733363537343230323236343232336132303537353234393534343537643230333032303737366637323662222c22726573756c745f74797065223a226c697374222c226572726f725f636f6465223a6e756c6c2c226572726f725f696e666f223a6e756c6c2c2272657475726e5f6f7074696f6e73223a223264363336663634363532303330323032643663363537363635366332303330227d0a"
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": -11,
      "rows": 1,
      "stdout_hex": "7b2263617365223a2267656e657269632d776974682d646963742d77726974652d6572726f72222c2270617468223a2266697865642d6772616d6d6172222c226f70223a226e61746976652d726573756c74222c22636f6465223a302c22726573756c74223a"
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 55
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "compiled-update-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-update-remove\t0\t30207b7d2030207b7d\ncompiled-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-update-local-read-error\t0\t30207b7d2030207b7d\ncompiled-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-with-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-with-remove\t0\t30207b7d2030207b7d\ncompiled-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ncompiled-with-local-read-error\t0\t30207b7d2030207b7d\ncompiled-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ncompiled-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ncompiled-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-update-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-update-remove\t0\t30207b7d2030207b7d\ngeneric-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-update-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-update-local-read-error\t0\t30207b7d2030207b7d\ngeneric-update-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b63616e277420736574202264223a2057524954457d2030207b6b20424153457d\ngeneric-with-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-with-remove\t0\t30207b7d2030207b7d\ngeneric-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-with-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-with-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-with-guest-return\t0\t3220424f44592030207b6b204e4558547d\ngeneric-with-local-read-error\t0\t30207b7d2030207b7d\ngeneric-with-dict-read-error\t0\t30207b7d2030207b6b20424153457d\ngeneric-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-update-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\n"
}
```

### jim

Status: `observed`. Version: Jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObj, Jim_EvalObjVector, Tcl_EvalObjEx, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-7-row-13`):

```json
{
  "version": "Jim",
  "compile_exit": 0,
  "exit_statuses": [
    {
      "case": "compiled-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "compiled-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-plain",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-remove",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-unset",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-invalid",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-root-replace",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-guest-return",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-local-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-read-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-dict-write-error",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-raw-ff",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-modified-nul",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-update-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    },
    {
      "case": "generic-with-array-name",
      "exit": 0,
      "rows": 1,
      "stdout_hex": null
    }
  ],
  "rows": 56
}
```

`Jim.tsv` (`file-5`):

```json
{
  "original_table": "compiled-update-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-update-remove\t0\t30207b7d2030207b7d\ncompiled-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ncompiled-update-guest-return\t0\t3020424f44592030207b6b204e4558547d\ncompiled-update-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-update-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-update-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-with-plain\t0\t30204e4558542030207b6b204e4558547d\ncompiled-with-remove\t0\t30207b7d2030207b7d\ncompiled-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ncompiled-with-root-invalid\t0\t30207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ncompiled-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ncompiled-with-guest-error\t0\t3120424f44592030207b6b20424153457d\ncompiled-with-guest-return\t0\t3220424f44592030207b6b20424153457d\ncompiled-with-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-with-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-with-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ncompiled-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ncompiled-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ncompiled-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ncompiled-update-array-name\t1\t63616e27742072656164202261223a206e6f2073756368207661726961626c65\ncompiled-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\ngeneric-update-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-update-remove\t0\t30207b7d2030207b7d\ngeneric-update-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-update-root-invalid\t0\t31207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-update-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-update-guest-error\t0\t3120424f44592030207b6b204e4558547d\ngeneric-update-guest-return\t0\t3020424f44592030207b6b204e4558547d\ngeneric-update-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-update-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-update-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-with-plain\t0\t30204e4558542030207b6b204e4558547d\ngeneric-with-remove\t0\t30207b7d2030207b7d\ngeneric-with-root-unset\t0\t30207b7d2031207b63616e27742072656164202264223a206e6f2073756368207661726961626c657d\ngeneric-with-root-invalid\t0\t30207b6d697373696e672076616c756520746f20676f2077697468206b65797d2030204f4444\ngeneric-with-root-replace\t0\t30204e4558542030207b71204f54484552206b204e4558547d\ngeneric-with-guest-error\t0\t3120424f44592030207b6b20424153457d\ngeneric-with-guest-return\t0\t3220424f44592030207b6b20424153457d\ngeneric-with-local-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-with-dict-read-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-with-dict-write-error\t0\t31207b696e76616c696420636f6d6d616e64206e616d6520227472616365227d2030207b6b20424153457d\ngeneric-update-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-with-raw-nul\t0\t30204e455854207b6b007461696c204e4558547d204e455854\ngeneric-update-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-with-raw-ff\t0\t30204e455854207b6bff7461696c204e4558547d204e455854\ngeneric-update-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-with-modified-nul\t0\t30204e455854207b6bc0807461696c204e4558547d204e455854\ngeneric-update-array-name\t1\t63616e27742072656164202261223a206e6f2073756368207661726961626c65\ngeneric-with-array-name\t0\t30204e455854207b61287129204e4558547d204e455854\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/8.4.20.tsv). SHA-256 `3512a4f1961cfa17e579ce92eb7ce2be933f35ff3929835a0e9e2f8f7deb9973`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/8.5.19.tsv). SHA-256 `a0033268917b943996f7de5b5775af158e0963728c909d69709e37a03852dcaf`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/8.6.18.tsv). SHA-256 `3ff4d58e8ade025822f8af6038d1756be2b0463021beff931fe310087e2ab25d`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/9.0.4.tsv). SHA-256 `a1094a066714055a5393060cb61a14a9732465064b8fe3c7e193b58fa4597c5f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/9.1.0.tsv). SHA-256 `a1094a066714055a5393060cb61a14a9732465064b8fe3c7e193b58fa4597c5f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/Jim.tsv). SHA-256 `c298afcfa08a51117f18ff6fa44098e3bbb30c7c044af534a8590d67ffe085bc`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv). SHA-256 `a457008448a336a389ab7c049c44f787c8361714358c6dd802f312464f9340f0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-7-row-8` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-7-row-9` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-7-row-10` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-7-row-11` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-7-row-12` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-7-row-13` (provider): [rust/tcl-vm/tests/data/native_dictionary_body/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_body/manifest.json). SHA-256 `fa787d8358bd24fe60efbcd838a2825f3b6a69c51e0045f2d84fa641bcabbcbf`. JSON pointer `/observations/5`. Original provider/capture association at its exact selected row.
- `file-14` (input): [rust/tcl-vm/tests/data/native_dictionary_body/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_body/probe.c). SHA-256 `e95451728352a915dfbc033719e60aabcab9ae4f53566088b583bbd4f12c6f3d`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
