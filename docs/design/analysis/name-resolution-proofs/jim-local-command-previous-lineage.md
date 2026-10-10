# naming.jim.local-command-previous-lineage

Kind: `native-observation`

## Problem statement

Replacing or deleting a local command can leave an original lookup object while changing its live previous node and procedure epoch. A retired negative cache must not become a previous-command proof.

## Question

What original live-node, epoch, upcall and cleanup-name windows occur in the six independent Jim local-command scenarios?

## Conclusion

The original lookup name and live previous-node lineage are separately observed for replacement, nested upcalls, native previous workers, rename refusal, creation and deletion. Retired negative links supply no live lineage. Public error globals and return options are outside this probe.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.0.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.1.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Version: pinned Jim 0.84. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_Eval, Jim_EvalObjVector. Exact lengths, flags and input constructors remain in the source.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-1-row-2`):

```json
{
  "engine": "pinned Jim 0.84",
  "compile_exit": 0,
  "run_exit": 0,
  "rows": 63,
  "captures": [
    "original command/cache identity",
    "procedure epoch",
    "live previous node lineage",
    "upcall counters",
    "original local cleanup name ownership"
  ]
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-syntax/tests/data/native_jim_local/behaviour.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_local/behaviour.tsv). SHA-256 `25b705ad35d11ebcb3402538bbb7155519caa0b9db1595d5bf106c0155f308e6`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-syntax/tests/data/native_jim_local/manifest.json](../../../../rust/tcl-syntax/tests/data/native_jim_local/manifest.json). SHA-256 `efae1058812ba8675530d41653c2ffd72574a4554af9c7e08291e5554f382b83`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1-row-2` (provider): [rust/tcl-syntax/tests/data/native_jim_local/manifest.json](../../../../rust/tcl-syntax/tests/data/native_jim_local/manifest.json). SHA-256 `efae1058812ba8675530d41653c2ffd72574a4554af9c7e08291e5554f382b83`. JSON pointer ``. Original provider/capture association at its exact selected row.
- `file-3` (observation): [rust/tcl-syntax/tests/data/native_jim_local/observations.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_local/observations.tsv). SHA-256 `eed98085bfe690e8fdb3262a0bfd4a599ee5d63c9f6060d6b85b902445545db4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (input): [rust/tcl-syntax/tests/data/native_jim_local/probe.c](../../../../rust/tcl-syntax/tests/data/native_jim_local/probe.c). SHA-256 `2e35a6697e2f6d7075c1475814a75c614231f4fe3dd1807e4d7ffcd0fcbc574c`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
