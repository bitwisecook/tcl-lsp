# naming.package.ordinary-files-forget

Kind: `native-observation`

## Problem statement

Forgetting a package can clear its current file list while an earlier captured list still contains paths. A current query cannot replace the captured value lineage.

## Question

What original old/current file lists remain after loading parent, capturing files and forgetting parent?

## Conclusion

C9 retains the captured old path list and returns an empty current list after forget. Older C and Jim fail before reaching this file-list lifetime program.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20.

The exact original script and recorded process/guest output are:

```json
{
  "case": "files-forget",
  "script": "source /workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl; package require parent; set old [package files parent]; package forget parent; list $old [package files parent]",
  "exit": 0,
  "stdout": "1 {wrong # args: should be \"source fileName\"}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

The exact original script and recorded process/guest output are:

```json
{
  "case": "files-forget",
  "script": "source /workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl; package require parent; set old [package files parent]; package forget parent; list $old [package files parent]",
  "exit": 0,
  "stdout": "1 {wrong # args: should be \"source ?-encoding name? fileName\"}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "files-forget",
  "script": "source /workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl; package require parent; set old [package files parent]; package forget parent; list $old [package files parent]",
  "exit": 0,
  "stdout": "0 {{/workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl /workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl /workspace/.proofs/2286-packages-dialects/package-native-completion/leaf.tcl} {}}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "files-forget",
  "script": "source /workspace/.proofs/2286-packages-dialects/package-native-completion/pkgIndex.tcl; package require parent; set old [package files parent]; package forget parent; list $old [package files parent]",
  "exit": 0,
  "stdout": "1 {package, unknown command \"ifneeded\": should be forget, names, provide, require}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_package_ordinary](../../../../rust/tcl-registry/tests/data/native_package_ordinary)
