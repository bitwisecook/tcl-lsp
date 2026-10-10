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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.4.20.tsv). SHA-256 `7bf5f25bb475c439c01147ba14a0ea15f5e552738927d5b00b6927e3f85f814e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.5.19.tsv). SHA-256 `7bf5f25bb475c439c01147ba14a0ea15f5e552738927d5b00b6927e3f85f814e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/8.6.18.tsv). SHA-256 `7bf5f25bb475c439c01147ba14a0ea15f5e552738927d5b00b6927e3f85f814e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.0.4.tsv). SHA-256 `a5a6633a8027effaa6d2db7f82958d5433c10de276da973f5c2117ba6541e819`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/9.1.0.tsv). SHA-256 `a5a6633a8027effaa6d2db7f82958d5433c10de276da973f5c2117ba6541e819`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/manifest.json). SHA-256 `7d7414fe59ebf02c98ac477cf3b6e6f1878a9adcea3adf818a5a66beca235dbd`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [rust/tcl-registry/tests/data/native_stock_index_flags/probe.c](../../../../rust/tcl-registry/tests/data/native_stock_index_flags/probe.c). SHA-256 `9d8b08236627bcd7fbe6615d54d3002b4258531d14c6d5d4ddbda3ab9a77c992`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
