# naming.jim.dictionary-unset-target-and-copy-frontiers

Kind: `native-observation`

## Problem statement

A Jim indexed variable can have a missing or malformed scalar dictionary root, a shared root, or an upvar link. Treating all as an ordinary C array would produce wrong errors and copy behavior.

## Question

What original completion/result is observed for the nine Jim unset receiver and quiet/link scenarios?

## Conclusion

The retained scalar-dictionary and link programs keep root-kind failure, missing member, sharing and quiet behavior separate. The result bytes establish those programs only; physical root ownership and C array protocols are not inferred.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The complete original catch/binary-scan input bytes are independently recovered in the attached wire-input artifact by exact agreement with each original source digest; this is an offline byte check and adds no native observation.

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

Status: `observed`. Version: provider profile recorded; full version query not recorded. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-1`):

```json
{
  "case": "none",
  "stdout": "0\t",
  "exit": 0
}
```

`provenance.json` (`file-0-row-2`):

```json
{
  "case": "missing-parent",
  "stdout": "1\t63616e277420756e736574202261286b29223a207661726961626c652069736e2774206172726179",
  "exit": 0
}
```

`provenance.json` (`file-0-row-3`):

```json
{
  "case": "malformed-parent",
  "stdout": "1\t63616e277420756e736574202261286b29223a206e6f207375636820656c656d656e7420696e206172726179",
  "exit": 0
}
```

`provenance.json` (`file-0-row-4`):

```json
{
  "case": "missing-member",
  "stdout": "1\t63616e277420756e736574202261286b29223a206e6f207375636820656c656d656e7420696e206172726179",
  "exit": 0
}
```

`provenance.json` (`file-0-row-5`):

```json
{
  "case": "shared-root",
  "stdout": "0\t7b6f74686572204f7d207b6b204b206f74686572204f7d",
  "exit": 0
}
```

`provenance.json` (`file-0-row-6`):

```json
{
  "case": "quiet-parent",
  "stdout": "0\t30",
  "exit": 0
}
```

`provenance.json` (`file-0-row-7`):

```json
{
  "case": "quiet-malformed",
  "stdout": "0\t4d414c464f524d4544",
  "exit": 0
}
```

`provenance.json` (`file-0-row-8`):

```json
{
  "case": "indexed-link",
  "stdout": "1\t63616e277420756e736574202276286b29223a207661726961626c652069736e2774206172726179",
  "exit": 0
}
```

`provenance.json` (`file-0-row-9`):

```json
{
  "case": "simple-link",
  "stdout": "1\t63616e277420756e736574202276223a206e6f2073756368207661726961626c65",
  "exit": 0
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/3`. Original provider/capture association at its exact selected row.
- `file-0-row-5` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/4`. Original provider/capture association at its exact selected row.
- `file-0-row-6` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/5`. Original provider/capture association at its exact selected row.
- `file-0-row-7` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/6`. Original provider/capture association at its exact selected row.
- `file-0-row-8` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/7`. Original provider/capture association at its exact selected row.
- `file-0-row-9` (provider): [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json). SHA-256 `04b8217436481af3e3f2a62a5d32a68b597ee8b64c3203a042df8ce6ed711544`. JSON pointer `/cases/8`. Original provider/capture association at its exact selected row.
- `file-10` (observation): [rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv). SHA-256 `599ec015d7b86c99e1eea2d9a2ca62f53aa007aed98cec50fb6cbf461bf29eb2`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `reconstructed-wire-inputs` (input): [docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json](../../../../docs/design/analysis/name-resolution-proofs/reconstructed-capture-inputs.json). SHA-256 `8fc1af984ce6fd8d66b87a0b24a0d7cf770f53e751ebe33e8be652b6799e4012`. JSON pointer `/groups/jim-unset`. Exact original complete capture inputs recovered offline from retained body hex and catch/binary-scan wrapper, with every original whole-input digest checked. This supplies no new guest execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
