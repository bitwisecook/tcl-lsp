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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_Eval, Jim_EvalObjVector. Exact lengths, flags and input constructors remain in the source. Version: jim pinned Jim 0.84.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_jim_local/behaviour.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_local/behaviour.tsv)
- [rust/tcl-syntax/tests/data/native_jim_local/manifest.json](../../../../rust/tcl-syntax/tests/data/native_jim_local/manifest.json)
- [rust/tcl-syntax/tests/data/native_jim_local/observations.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_local/observations.tsv)
- [rust/tcl-syntax/tests/data/native_jim_local/probe.c](../../../../rust/tcl-syntax/tests/data/native_jim_local/probe.c)
