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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_EvalEx. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_package_versions/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_package_versions/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_package_versions/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_package_versions/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_package_versions/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_package_versions/9.1.0.tsv)
- [rust/tcl-registry/tests/data/native_package_versions/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_versions/manifest.json)
- [rust/tcl-registry/tests/data/native_package_versions/probe.c](../../../../rust/tcl-registry/tests/data/native_package_versions/probe.c)
