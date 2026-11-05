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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim provider profile recorded.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_jim_unset/provenance.json](../../../../rust/tcl-syntax/tests/data/native_jim_unset/provenance.json)
- [rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv](../../../../rust/tcl-syntax/tests/data/native_jim_unset/windows.tsv)
