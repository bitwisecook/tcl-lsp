# naming.command.compiler-and-namespace-epochs

Kind: `native-observation`

## Problem statement

A command replacement, rename or namespace recreation can leave the same printed name while changing compiler/resolver currency. Comparing counters from different interpreters would falsely attest a cached lookup.

## Question

How do the actual interpreter compiler, namespace resolver and command-reference counters change around the recorded operations?

## Conclusion

The private-header probe records counters after each completed operation. Compare each counter with its own interpreter baseline: bootstrap counts differ by release, and recreated namespaces have a new identity and restarted counters. These rows are native header observations, not a source-only compiler receipt.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler",
    "initial\t0\t0\t0\t-1\t-1\t1",
    "namespace_create\t0\t0\t0\t0\t0\t1",
    "plain_create\t0\t0\t0\t0\t0\t1",
    "plain_rename\t0\t0\t0\t0\t0\t1",
    "plain_delete\t0\t0\t0\t0\t0\t1",
    "compiled_shadow\t0\t0\t0\t1\t1\t1",
    "shadow_delete\t0\t0\t0\t1\t1\t1",
    "compiled_rename\t0\t1\t0\t1\t1\t-1",
    "replacement_plain\t0\t1\t0\t1\t1\t0",
    "replacement_delete\t0\t1\t0\t1\t1\t-1",
    "compiled_restore\t0\t2\t0\t1\t1\t1",
    "compiled_hide\t0\t3\t0\t1\t1\t-1",
    "compiled_expose\t0\t4\t0\t1\t1\t1",
    "namespace_delete\t0\t4\t0\t-1\t-1\t1",
    "namespace_recreate\t0\t4\t0\t0\t0\t1"
  ],
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler\ninitial\t0\t0\t0\t-1\t-1\t1\nnamespace_create\t0\t0\t0\t0\t0\t1\nplain_create\t0\t0\t0\t0\t0\t1\nplain_rename\t0\t0\t0\t0\t0\t1\nplain_delete\t0\t0\t0\t0\t0\t1\ncompiled_shadow\t0\t0\t0\t1\t1\t1\nshadow_delete\t0\t0\t0\t1\t1\t1\ncompiled_rename\t0\t1\t0\t1\t1\t-1\nreplacement_plain\t0\t1\t0\t1\t1\t0\nreplacement_delete\t0\t1\t0\t1\t1\t-1\ncompiled_restore\t0\t2\t0\t1\t1\t1\ncompiled_hide\t0\t3\t0\t1\t1\t-1\ncompiled_expose\t0\t4\t0\t1\t1\t1\nnamespace_delete\t0\t4\t0\t-1\t-1\t1\nnamespace_recreate\t0\t4\t0\t0\t0\t1\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler",
    "initial\t0\t3\t0\t-1\t-1\t1",
    "namespace_create\t0\t3\t0\t0\t0\t1",
    "plain_create\t0\t3\t0\t0\t0\t1",
    "plain_rename\t0\t3\t0\t0\t0\t1",
    "plain_delete\t0\t3\t0\t0\t0\t1",
    "compiled_shadow\t0\t3\t0\t1\t1\t1",
    "shadow_delete\t0\t3\t0\t1\t1\t1",
    "compiled_rename\t0\t4\t0\t1\t1\t-1",
    "replacement_plain\t0\t4\t0\t1\t1\t0",
    "replacement_delete\t0\t4\t0\t1\t1\t-1",
    "compiled_restore\t0\t5\t0\t1\t1\t1",
    "compiled_hide\t0\t6\t0\t1\t1\t-1",
    "compiled_expose\t0\t7\t0\t1\t1\t1",
    "namespace_path\t0\t7\t0\t2\t2\t1",
    "same_path\t0\t7\t0\t3\t3\t1",
    "clear_path\t0\t7\t0\t4\t4\t1",
    "namespace_delete\t0\t7\t0\t-1\t-1\t1",
    "namespace_recreate\t0\t7\t0\t0\t0\t1"
  ],
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler\ninitial\t0\t3\t0\t-1\t-1\t1\nnamespace_create\t0\t3\t0\t0\t0\t1\nplain_create\t0\t3\t0\t0\t0\t1\nplain_rename\t0\t3\t0\t0\t0\t1\nplain_delete\t0\t3\t0\t0\t0\t1\ncompiled_shadow\t0\t3\t0\t1\t1\t1\nshadow_delete\t0\t3\t0\t1\t1\t1\ncompiled_rename\t0\t4\t0\t1\t1\t-1\nreplacement_plain\t0\t4\t0\t1\t1\t0\nreplacement_delete\t0\t4\t0\t1\t1\t-1\ncompiled_restore\t0\t5\t0\t1\t1\t1\ncompiled_hide\t0\t6\t0\t1\t1\t-1\ncompiled_expose\t0\t7\t0\t1\t1\t1\nnamespace_path\t0\t7\t0\t2\t2\t1\nsame_path\t0\t7\t0\t3\t3\t1\nclear_path\t0\t7\t0\t4\t4\t1\nnamespace_delete\t0\t7\t0\t-1\t-1\t1\nnamespace_recreate\t0\t7\t0\t0\t0\t1\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler",
    "initial\t0\t17\t0\t-1\t-1\t1",
    "namespace_create\t0\t17\t0\t0\t0\t1",
    "plain_create\t0\t17\t0\t0\t0\t1",
    "plain_rename\t0\t17\t0\t0\t0\t1",
    "plain_delete\t0\t17\t0\t0\t0\t1",
    "compiled_shadow\t0\t17\t0\t1\t1\t1",
    "shadow_delete\t0\t17\t0\t1\t1\t1",
    "compiled_rename\t0\t18\t0\t1\t1\t-1",
    "replacement_plain\t0\t18\t0\t1\t1\t0",
    "replacement_delete\t0\t18\t0\t1\t1\t-1",
    "compiled_restore\t0\t19\t0\t1\t1\t1",
    "compiled_hide\t0\t20\t0\t1\t1\t-1",
    "compiled_expose\t0\t21\t0\t1\t1\t1",
    "namespace_path\t0\t21\t0\t2\t2\t1",
    "same_path\t0\t21\t0\t3\t3\t1",
    "clear_path\t0\t21\t0\t4\t4\t1",
    "namespace_delete\t0\t21\t0\t-1\t-1\t1",
    "namespace_recreate\t0\t21\t0\t0\t0\t1"
  ],
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler\ninitial\t0\t17\t0\t-1\t-1\t1\nnamespace_create\t0\t17\t0\t0\t0\t1\nplain_create\t0\t17\t0\t0\t0\t1\nplain_rename\t0\t17\t0\t0\t0\t1\nplain_delete\t0\t17\t0\t0\t0\t1\ncompiled_shadow\t0\t17\t0\t1\t1\t1\nshadow_delete\t0\t17\t0\t1\t1\t1\ncompiled_rename\t0\t18\t0\t1\t1\t-1\nreplacement_plain\t0\t18\t0\t1\t1\t0\nreplacement_delete\t0\t18\t0\t1\t1\t-1\ncompiled_restore\t0\t19\t0\t1\t1\t1\ncompiled_hide\t0\t20\t0\t1\t1\t-1\ncompiled_expose\t0\t21\t0\t1\t1\t1\nnamespace_path\t0\t21\t0\t2\t2\t1\nsame_path\t0\t21\t0\t3\t3\t1\nclear_path\t0\t21\t0\t4\t4\t1\nnamespace_delete\t0\t21\t0\t-1\t-1\t1\nnamespace_recreate\t0\t21\t0\t0\t0\t1\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler",
    "initial\t0\t20\t0\t-1\t-1\t1",
    "namespace_create\t0\t20\t0\t0\t0\t1",
    "plain_create\t0\t20\t0\t0\t0\t1",
    "plain_rename\t0\t20\t0\t0\t0\t1",
    "plain_delete\t0\t20\t0\t0\t0\t1",
    "compiled_shadow\t0\t20\t0\t1\t1\t1",
    "shadow_delete\t0\t20\t0\t1\t1\t1",
    "compiled_rename\t0\t21\t0\t1\t1\t-1",
    "replacement_plain\t0\t21\t0\t1\t1\t0",
    "replacement_delete\t0\t21\t0\t1\t1\t-1",
    "compiled_restore\t0\t22\t0\t1\t1\t1",
    "compiled_hide\t0\t23\t0\t1\t1\t-1",
    "compiled_expose\t0\t24\t0\t1\t1\t1",
    "namespace_path\t0\t24\t0\t2\t2\t1",
    "same_path\t0\t24\t0\t3\t3\t1",
    "clear_path\t0\t24\t0\t4\t4\t1",
    "namespace_delete\t0\t24\t0\t-1\t-1\t1",
    "namespace_recreate\t0\t24\t0\t0\t0\t1"
  ],
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler\ninitial\t0\t20\t0\t-1\t-1\t1\nnamespace_create\t0\t20\t0\t0\t0\t1\nplain_create\t0\t20\t0\t0\t0\t1\nplain_rename\t0\t20\t0\t0\t0\t1\nplain_delete\t0\t20\t0\t0\t0\t1\ncompiled_shadow\t0\t20\t0\t1\t1\t1\nshadow_delete\t0\t20\t0\t1\t1\t1\ncompiled_rename\t0\t21\t0\t1\t1\t-1\nreplacement_plain\t0\t21\t0\t1\t1\t0\nreplacement_delete\t0\t21\t0\t1\t1\t-1\ncompiled_restore\t0\t22\t0\t1\t1\t1\ncompiled_hide\t0\t23\t0\t1\t1\t-1\ncompiled_expose\t0\t24\t0\t1\t1\t1\nnamespace_path\t0\t24\t0\t2\t2\t1\nsame_path\t0\t24\t0\t3\t3\t1\nclear_path\t0\t24\t0\t4\t4\t1\nnamespace_delete\t0\t24\t0\t-1\t-1\t1\nnamespace_recreate\t0\t24\t0\t0\t0\t1\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler",
    "initial\t0\t24\t0\t-1\t-1\t1",
    "namespace_create\t0\t24\t0\t0\t0\t1",
    "plain_create\t0\t24\t0\t0\t0\t1",
    "plain_rename\t0\t24\t0\t0\t0\t1",
    "plain_delete\t0\t24\t0\t0\t0\t1",
    "compiled_shadow\t0\t24\t0\t1\t1\t1",
    "shadow_delete\t0\t24\t0\t1\t1\t1",
    "compiled_rename\t0\t25\t0\t1\t1\t-1",
    "replacement_plain\t0\t25\t0\t1\t1\t0",
    "replacement_delete\t0\t25\t0\t1\t1\t-1",
    "compiled_restore\t0\t26\t0\t1\t1\t1",
    "compiled_hide\t0\t27\t0\t1\t1\t-1",
    "compiled_expose\t0\t28\t0\t1\t1\t1",
    "namespace_path\t0\t28\t0\t2\t2\t1",
    "same_path\t0\t28\t0\t3\t3\t1",
    "clear_path\t0\t28\t0\t4\t4\t1",
    "namespace_delete\t0\t28\t0\t-1\t-1\t1",
    "namespace_recreate\t0\t28\t0\t0\t0\t1"
  ],
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "event\tcode\tcompiler\tglobal_resolver\tN_resolver\tN_command_reference\tset_compiler\ninitial\t0\t24\t0\t-1\t-1\t1\nnamespace_create\t0\t24\t0\t0\t0\t1\nplain_create\t0\t24\t0\t0\t0\t1\nplain_rename\t0\t24\t0\t0\t0\t1\nplain_delete\t0\t24\t0\t0\t0\t1\ncompiled_shadow\t0\t24\t0\t1\t1\t1\nshadow_delete\t0\t24\t0\t1\t1\t1\ncompiled_rename\t0\t25\t0\t1\t1\t-1\nreplacement_plain\t0\t25\t0\t1\t1\t0\nreplacement_delete\t0\t25\t0\t1\t1\t-1\ncompiled_restore\t0\t26\t0\t1\t1\t1\ncompiled_hide\t0\t27\t0\t1\t1\t-1\ncompiled_expose\t0\t28\t0\t1\t1\t1\nnamespace_path\t0\t28\t0\t2\t2\t1\nsame_path\t0\t28\t0\t3\t3\t1\nclear_path\t0\t28\t0\t4\t4\t1\nnamespace_delete\t0\t28\t0\t-1\t-1\t1\nnamespace_recreate\t0\t28\t0\t0\t0\t1\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv)
- [runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv)
- [runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv)
- [runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv)
- [runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv)
- [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json)
- [runtime/rust/tests/data/native_compiler_epochs/probe.c](../../../../runtime/rust/tests/data/native_compiler_epochs/probe.c)
