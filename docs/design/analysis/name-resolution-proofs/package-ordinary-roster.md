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

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.4.20.tsv). SHA-256 `23ac558f74be73ba0ea1b7d2b048c76e7da51c27ec2fac351b46478148e6532a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.5.19.tsv). SHA-256 `ef09b1c3c8fb4ed486ff4e60d31c2a7c44f01ca15e57661f6d20b39020be48b3`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/8.6.18.tsv). SHA-256 `8fe05bc71afc395dd1d7f20774da44c6b469abb312608aa9b95174bcfbb90b1a`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/9.0.4.tsv). SHA-256 `75ff9049df1eebd3ac11c2a817a406501178a5df494747cbcc9ef26f337e2cbe`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/9.1.0.tsv). SHA-256 `75ff9049df1eebd3ac11c2a817a406501178a5df494747cbcc9ef26f337e2cbe`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/Jim.tsv](../../../../rust/tcl-registry/tests/data/native_package_ordinary/Jim.tsv). SHA-256 `2643a2c8af33df394cbf98a788952598d6b3c45e02c8986aa6a9e012360f2ee1`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (input): [rust/tcl-registry/tests/data/native_package_ordinary/capture.py](../../../../rust/tcl-registry/tests/data/native_package_ordinary/capture.py). SHA-256 `fe93f81b3fc626e6583d65c329200309ea7bb72c6c614102001e29fe0524015e`. Exact retained input/program bytes; purpose is limited to this question.
- `file-7` (input): [rust/tcl-registry/tests/data/native_package_ordinary/leaf.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/leaf.tcl). SHA-256 `8e70a5fd49cc1b402e17295510b3b0f07d0fe03728ef200cee14681b29dec3a3`. Exact retained input/program bytes; purpose is limited to this question.
- `file-8` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-15` (input): [rust/tcl-registry/tests/data/native_package_ordinary/nested.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/nested.tcl). SHA-256 `4466b4f9bc69db5b9cf4fae98bab32fa268fa8d8b7a3e64cd19e091db2c5b474`. Exact retained input/program bytes; purpose is limited to this question.
- `file-16` (input): [rust/tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/pkgIndex.tcl). SHA-256 `68704c0f38f315d4b48cecc99bc85e87e9f7de8662407ce9815b5b86ace25371`. Exact retained input/program bytes; purpose is limited to this question.
- `file-17` (input): [rust/tcl-registry/tests/data/native_package_ordinary/skip.tcl](../../../../rust/tcl-registry/tests/data/native_package_ordinary/skip.tcl). SHA-256 `2703799fea55b157d76e31b8757ac25abb26d753c3629a8e8ac8a146f6038199`. Exact retained input/program bytes; purpose is limited to this question.
- `selected-input-and-outcome-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/0/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/1/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/2/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/3/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/4/rows/0`. Exact original script plus caught output/status for this question and provider.
- `selected-input-and-outcome-jim` (observation): [rust/tcl-registry/tests/data/native_package_ordinary/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_ordinary/manifest.json). SHA-256 `5877faca17503732c358c0a44cc87b3cb53d5e5c7e831bddb5b7cd9a78abc41c`. JSON pointer `/engines/5/rows/0`. Exact original script plus caught output/status for this question and provider.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted. The manifest retains each inner source string and result. The current capture.py supplies a catch/list stdout wrapper, but its whole-file digest differs from the original manifest source_sha256; it is not claimed as the original capture helper. The original helper bytes are unavailable.
