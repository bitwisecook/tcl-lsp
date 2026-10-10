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

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_stock_index/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.4.20.tsv). SHA-256 `67c434321b43550083e0234d9af9c0a2849070fec255e133e37e4e341397ac9b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-registry/tests/data/native_stock_index/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.5.19.tsv). SHA-256 `67c434321b43550083e0234d9af9c0a2849070fec255e133e37e4e341397ac9b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-registry/tests/data/native_stock_index/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/8.6.18.tsv). SHA-256 `67c434321b43550083e0234d9af9c0a2849070fec255e133e37e4e341397ac9b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-registry/tests/data/native_stock_index/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/9.0.4.tsv). SHA-256 `67c434321b43550083e0234d9af9c0a2849070fec255e133e37e4e341397ac9b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-registry/tests/data/native_stock_index/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_stock_index/9.1.0.tsv). SHA-256 `67c434321b43550083e0234d9af9c0a2849070fec255e133e37e4e341397ac9b`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [rust/tcl-registry/tests/data/native_stock_index/manifest.json](../../../../rust/tcl-registry/tests/data/native_stock_index/manifest.json). SHA-256 `b9cd16ec4e751e9b1fd3e77dfb10018041205e12c51111f4d7ad6cb3e16c0a0f`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [rust/tcl-registry/tests/data/native_stock_index/probe.c](../../../../rust/tcl-registry/tests/data/native_stock_index/probe.c). SHA-256 `6e7ee72ac083a7ea316a372e2bbfd61e0b96e7076deba17feead89815c33e9cf`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
