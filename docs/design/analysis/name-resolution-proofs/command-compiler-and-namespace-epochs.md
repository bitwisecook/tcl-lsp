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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv). SHA-256 `7c6d63b8babcc2e8e2e6bb712aa74d000b09d883115520c64597109e076ebc73`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv). SHA-256 `73786165e9d52500b7a3f384fcc441d794c11bb2c781504a1d5aba5890ddff02`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv). SHA-256 `88fcd9b25c48f201ef9693f4d9c38a7ec2466e4c9df92e7b6da5fb85e8e9c05f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv). SHA-256 `650b88902df64c238b39d47d5d2657690407ca5b840783b61be43175012bc054`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv](../../../../runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv). SHA-256 `5ea64739f5af30d6c33577a8803a5205fedfb5849b00d2fcf15b14a4b3bbe037`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. JSON pointer `/runs/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. JSON pointer `/runs/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. JSON pointer `/runs/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. JSON pointer `/runs/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [runtime/rust/tests/data/native_compiler_epochs/manifest.json](../../../../runtime/rust/tests/data/native_compiler_epochs/manifest.json). SHA-256 `0592e891f9e68e6658760747dec80a5ce50e6c51b71ae7a7757b71c1c75942a8`. JSON pointer `/runs/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [runtime/rust/tests/data/native_compiler_epochs/probe.c](../../../../runtime/rust/tests/data/native_compiler_epochs/probe.c). SHA-256 `47b4a6f5a0088bd53a61994c9a14b0ad2d424d93a7bd9edb4fac3739caacdcf0`. Exact retained input/program bytes; purpose is limited to this question.
- `file-12` (input): [runtime/rust/tests/data/native_compiler_epochs/run.py](../../../../runtime/rust/tests/data/native_compiler_epochs/run.py). SHA-256 `9f4d060455a7daf63f46e0e37269a24f993fa8a903ea17e8bd30005baf8775d2`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
