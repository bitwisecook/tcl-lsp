# naming.package.version-original-header-reuse

Kind: `native-observation`

## Problem statement

Providing a version, converting its original object and querying it again can return equal text with either retained or newly constructed headers. The difference affects typed package value ownership.

## Question

Which original version object type/reference/identity windows survive provide, conversion, query, require, present and forget?

## Conclusion

The five C probes retain the original six-stage windows. The old C query/require/present rows and Tcl 8.6/C9 rows have different original-header behavior. No Jim/BIG-IP object ABI is measured and equivalent version spelling alone proves no header reuse.

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
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\tnone\t2\nconverted\t0\t12\tint\nqueried\t0\t0\tnone\t1\nrequired\t0\t0\tnone\t1\npresent\t0\t0\tnone\t1\nforgotten\t0\t1\tint\n",
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "created\t0\tnone\t2\nconverted\t0\t12\tint\nqueried\t0\t0\tnone\t1\nrequired\t0\t0\tnone\t1\npresent\t0\t0\tnone\t1\nforgotten\t0\t1\tint\n"
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
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\tnone\t2\nconverted\t0\t12\tint\nqueried\t0\t0\tnone\t1\nrequired\t0\t0\tnone\t1\npresent\t0\t0\tnone\t1\nforgotten\t0\t1\tint\n",
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "created\t0\tnone\t2\nconverted\t0\t12\tint\nqueried\t0\t0\tnone\t1\nrequired\t0\t0\tnone\t1\npresent\t0\t0\tnone\t1\nforgotten\t0\t1\tint\n"
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
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n",
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n"
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
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n",
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n"
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
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n",
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "created\t0\tnone\t3\nconverted\t0\t12\tint\nqueried\t0\t1\tint\t3\nrequired\t0\t1\tint\t3\npresent\t0\t1\tint\t3\nforgotten\t0\t1\tint\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_package_versions/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.4.20.tsv). SHA-256 `e85e7d70f5128ed496dcd1c13986dcf7c7fd2be4118df392342e5418c6c85fb2`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_package_versions/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.5.19.tsv). SHA-256 `e85e7d70f5128ed496dcd1c13986dcf7c7fd2be4118df392342e5418c6c85fb2`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_package_versions/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.6.18.tsv). SHA-256 `bcb1d734379c780a6fa33de51ad34b640cc318b5508b5b8a5154527aba9357a7`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_package_versions/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/9.0.4.tsv). SHA-256 `bcb1d734379c780a6fa33de51ad34b640cc318b5508b5b8a5154527aba9357a7`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_package_versions/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/9.1.0.tsv). SHA-256 `bcb1d734379c780a6fa33de51ad34b640cc318b5508b5b8a5154527aba9357a7`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json). SHA-256 `62912f4daeb5aa76c0e2017c3de09eebed6abbf31faba153617278a5b28cc003`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [rust/tcl-registry/tests/data/native_package_versions/probe.c](../../../../rust/tcl-registry/tests/data/native_package_versions/probe.c). SHA-256 `7c1718324758e5285792d0b3c75130ac4a627e7af1b4737b727f31bb9f39ffb6`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
