# naming.selector.stock-index-original-cache-reuse

Kind: `native-observation`

## Problem statement

A previously abbreviated selector can hold an index that later exact lookup reuses, even after its string representation is invalidated or duplicated. Rechecking only current bytes would lose native cache behavior.

## Question

How do stock GetIndex calls handle abbreviated, cached-exact, absent-string, duplicate and counted-NUL original selectors?

## Conclusion

The original native rows retain code/index/type/string residency and pointer windows at each call. A cache hit can survive exact flags and an absent string; the original counted-NUL object follows the selected getter. The retained probe hash identifies the actual input despite other manifest provenance fields.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n",
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n",
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n",
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n",
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "run_exit": 0,
  "stdout": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n",
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "abbreviated\t0\t0\tindex\t1\t1\ncached-exact\t0\t0\tindex\t1\t1\ncached-absent\t0\t0\tindex\t0\t1\nduplicate-absent\t0\t0\tindex\t0\t1\ncounted-nul\t0\t0\tindex\t1\t1\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_stock_index/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_stock_index/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_stock_index/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_stock_index/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_stock_index/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/9.1.0.tsv)
- [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json)
- [rust/tcl-registry/tests/data/native_stock_index/probe.c](../../../../rust/tcl-registry/tests/data/native_stock_index/probe.c)
