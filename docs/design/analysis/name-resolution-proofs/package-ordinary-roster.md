# naming.package.ordinary-roster

Kind: `native-observation`

## Problem statement

An invalid package member exposes the actual dispatcher roster. Assuming the same roster across releases can select a nonexistent worker.

## Question

What member roster is printed by the original caught package bad invocation?

## Conclusion

The C roster adds prefer after Tcl 8.4 and files in C9; the captured Jim roster is forget/names/provide/require. This measures the original diagnostic surface, not independently callable native methods.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The current capture.py supplies a catch/list stdout wrapper, but its whole-file digest differs from the original manifest source_sha256; it is not claimed as the original capture helper. The original helper bytes are unavailable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.4.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {bad option \"bad\": must be forget, ifneeded, names, present, provide, require, unknown, vcompare, versions, or vsatisfies}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.5.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {bad option \"bad\": must be forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.6.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {bad option \"bad\": must be forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {bad option \"bad\": must be files, forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.1.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {bad option \"bad\": must be files, forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Version: Jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "roster",
  "script": "catch {package bad} m; set m",
  "exit": 0,
  "stdout": "0 {package, unknown command \"bad\": should be forget, names, provide, require}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Historical references

- `file-0` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/Jim.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/Jim.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (input): [rust/tcl-registry/tests/data/native_package_ordinary/capture.py](../../../../rust/tcl-registry/tests/data/native_package_ordinary/capture.py). Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (input): [rust/tcl-registry/tests/data/native_package_ordinary/leaf.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/leaf.tcl). Exact retained input/program bytes; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-15` (input): [rust/tcl-registry/tests/data/native_package_ordinary/nested.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/nested.tcl). Exact retained input/program bytes; purpose is limited to this question.
- `file-16` (input): [rust/tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl). Exact retained input/program bytes; purpose is limited to this question.
- `file-17` (input): [rust/tcl-registry/tests/data/native_package_ordinary/skip.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/skip.tcl). Exact retained input/program bytes; purpose is limited to this question.
- `selected-input-and-outcome-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/0/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/1/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/2/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/3/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/4/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-jim` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). JSON pointer `/engines/5/rows/0`. Exact original script plus caught output/status for this question and provider.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted. The manifest retains each inner source string and result. The current capture.py supplies a catch/list stdout wrapper, but its whole-file digest differs from the original manifest source_sha256; it is not claimed as the original capture helper. The original helper bytes are unavailable.
