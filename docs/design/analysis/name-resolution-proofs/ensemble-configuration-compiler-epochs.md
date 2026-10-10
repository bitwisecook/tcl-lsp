# naming.ensemble.configuration-compiler-epochs

Kind: `native-observation`

## Problem statement

Repeated configuration, unknown-handler updates and rejected options can all look like an ensemble update but need not perform the same compiler-affecting setter transaction.

## Question

Which completed ensemble configuration updates advance the actual compiler epoch or change the original info compiler attachment?

## Conclusion

Successful configuration runs the native setter transaction even for repeated values; unknown/prefix setters and failed option parsing have separately recorded effects. Tcl 8.4 records the missing ensemble door. Counters remain relative to the same interpreter baseline.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t0\t0"
  ],
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t0\t0\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t3\t1",
    "prefix_false\t0\t5\t1",
    "same_prefix\t0\t7\t1",
    "bad_option\t1\t7\t1",
    "prefix_true\t0\t9\t1"
  ],
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t3\t1\nprefix_false\t0\t5\t1\nsame_prefix\t0\t7\t1\nbad_option\t1\t7\t1\nprefix_true\t0\t9\t1\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t17\t1",
    "prefix_false\t0\t20\t1",
    "same_prefix\t0\t23\t1",
    "bad_option\t1\t23\t1",
    "prefix_true\t0\t26\t1"
  ],
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t17\t1\nprefix_false\t0\t20\t1\nsame_prefix\t0\t23\t1\nbad_option\t1\t23\t1\nprefix_true\t0\t26\t1\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t20\t1",
    "prefix_false\t0\t23\t1",
    "same_prefix\t0\t26\t1",
    "bad_option\t1\t26\t1",
    "prefix_true\t0\t29\t1"
  ],
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t20\t1\nprefix_false\t0\t23\t1\nsame_prefix\t0\t26\t1\nbad_option\t1\t26\t1\nprefix_true\t0\t29\t1\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "observations": [
    "event\tcode\tcompiler\tinfo_hook",
    "initial\t0\t24\t1",
    "prefix_false\t0\t27\t1",
    "same_prefix\t0\t30\t1",
    "bad_option\t1\t30\t1",
    "prefix_true\t0\t33\t1"
  ],
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "event\tcode\tcompiler\tinfo_hook\ninitial\t0\t24\t1\nprefix_false\t0\t27\t1\nsame_prefix\t0\t30\t1\nbad_option\t1\t30\t1\nprefix_true\t0\t33\t1\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [runtime/rust/tests/data/native_ensemble_epochs/8.4.20.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.4.20.tsv). SHA-256 `de2bcc38107c5b28ba42dfe6738261270eaefe32f1a91660359254a3eb5cd611`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [runtime/rust/tests/data/native_ensemble_epochs/8.5.19.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.5.19.tsv). SHA-256 `c9e95133b383079d6a62159480bb14286b1ac49e8ec833f6db6e551e9c72ea28`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [runtime/rust/tests/data/native_ensemble_epochs/8.6.18.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/8.6.18.tsv). SHA-256 `69fa4b41b1287e3150e2d5b6e6e044713c9557edfbaddeccf1c60812ce4ffb7a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [runtime/rust/tests/data/native_ensemble_epochs/9.0.4.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/9.0.4.tsv). SHA-256 `620c632dc31e546717c29a06bcb39c062287315be72f9f9aa3f360cab167855e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [runtime/rust/tests/data/native_ensemble_epochs/9.1.0.tsv](../../../../runtime/rust/tests/data/native_ensemble_epochs/9.1.0.tsv). SHA-256 `b71cce70ec02711dddc9dfc79fece4e6a2a2e10fa966462381026f9ec2688ed8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. JSON pointer `/runs/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. JSON pointer `/runs/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. JSON pointer `/runs/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. JSON pointer `/runs/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [runtime/rust/tests/data/native_ensemble_epochs/manifest.json](../../../../runtime/rust/tests/data/native_ensemble_epochs/manifest.json). SHA-256 `e40ba518c2c395f476bbbc7149f0f47028e45a3b0af376879137d0df3ee66f30`. JSON pointer `/runs/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [runtime/rust/tests/data/native_ensemble_epochs/probe.c](../../../../runtime/rust/tests/data/native_ensemble_epochs/probe.c). SHA-256 `8ad537b1d5f3656d99bb7c69dcbc36d27227bde24ac80d82dadbd4a16a42ae37`. Exact retained input/program bytes; purpose is limited to this question.
- `file-12` (input): [runtime/rust/tests/data/native_ensemble_epochs/run.py](../../../../runtime/rust/tests/data/native_ensemble_epochs/run.py). SHA-256 `9f4d060455a7daf63f46e0e37269a24f993fa8a903ea17e8bd30005baf8775d2`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
