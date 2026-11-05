# naming.selector.stock-index-stride-and-flags

Kind: `native-observation`

## Problem statement

Changing table stride or using temporary/null flags changes the selected index recipe. Treating an index as only a word-to-number map loses native table and publication conditions.

## Question

What original cache/index windows do changed strides, temporary-table lookup and null/empty controls produce on each captured release?

## Conclusion

The probe keeps pointer stride, struct stride, changed-stride, temporary, exact and null controls independent. Release-specific compiled guards limit the reached cases. Missing flag/API cases are not inferred from other providers; the full retained outputs define the accepted scope.

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
  "run_exit": 0,
  "stdout": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n",
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n"
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
  "run_exit": 0,
  "stdout": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n",
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n"
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
  "run_exit": 0,
  "stdout": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n",
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\n"
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
  "run_exit": 0,
  "stdout": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\nordinary\t0\t0\tindex\t1\t1\ntemporary-exact-miss\t1\t-1\tindex\t1\t1\ncache-after-temporary-miss\t0\t0\tindex\t1\t1\ntemporary-fresh\t0\t0\tnone\t1\t1\nnull-empty\t0\t-1\tnone\t1\t1\nempty-exact-entry\t0\t2\tindex\t1\t1\ncached-null-absent\t0\t2\tindex\t0\t1\nnull-object\t0\t-1\tnone\t0\t1\n",
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\nordinary\t0\t0\tindex\t1\t1\ntemporary-exact-miss\t1\t-1\tindex\t1\t1\ncache-after-temporary-miss\t0\t0\tindex\t1\t1\ntemporary-fresh\t0\t0\tnone\t1\t1\nnull-empty\t0\t-1\tnone\t1\t1\nempty-exact-entry\t0\t2\tindex\t1\t1\ncached-null-absent\t0\t2\tindex\t0\t1\nnull-object\t0\t-1\tnone\t0\t1\n"
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
  "run_exit": 0,
  "stdout": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\nordinary\t0\t0\tindex\t1\t1\ntemporary-exact-miss\t1\t-1\tindex\t1\t1\ncache-after-temporary-miss\t0\t0\tindex\t1\t1\ntemporary-fresh\t0\t0\tnone\t1\t1\nnull-empty\t0\t-1\tnone\t1\t1\nempty-exact-entry\t0\t2\tindex\t1\t1\ncached-null-absent\t0\t2\tindex\t0\t1\nnull-object\t0\t-1\tnone\t0\t1\n",
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "pointer-stride\t0\t2\tindex\t1\t1\nstruct-stride\t0\t1\tindex\t1\t1\nchanged-stride-absent\t0\t2\tindex\t1\t0\nordinary\t0\t0\tindex\t1\t1\ntemporary-exact-miss\t1\t-1\tindex\t1\t1\ncache-after-temporary-miss\t0\t0\tindex\t1\t1\ntemporary-fresh\t0\t0\tnone\t1\t1\nnull-empty\t0\t-1\tnone\t1\t1\nempty-exact-entry\t0\t2\tindex\t1\t1\ncached-null-absent\t0\t2\tindex\t0\t1\nnull-object\t0\t-1\tnone\t0\t1\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv)
- [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json)
- [rust/tcl-registry/tests/data/native_stock_index_flags/probe.c](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/probe.c)
