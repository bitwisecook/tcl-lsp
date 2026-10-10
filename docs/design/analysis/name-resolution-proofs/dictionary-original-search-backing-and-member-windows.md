# naming.dictionary.original-search-backing-and-member-windows

Kind: `native-observation`

## Problem statement

Shimmering a dictionary root to a List or mutating a duplicate can release backing/member references at a different point from search close or exhaustion. Holding only displayed key/value text loses those original windows.

## Question

Which original dictionary/member reference windows occur across the five native DictSearch modes?

## Conclusion

Search retains the original dictionary representation without cloning each member. Early close, natural exhaustion, shimmer and duplicate mutation retain distinct windows before String observers. C8.4/Jim absence of this C API supplies no entered search behavior.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-7`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 1
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "unsupported-native-C-search\n"
}
```

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-8`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 25
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "0\tbefore-search\tdict\t0\t1\t2\t2\t0\n0\tafter-search\tdict\t0\t1\t2\t2\t1\n0\tafter-action\tdict\t0\t1\t2\t2\t1\n0\tafter-done\tdict\t0\t1\t2\t2\t0\n0\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n1\tbefore-search\tdict\t0\t1\t2\t2\t0\n1\tafter-search\tdict\t0\t1\t2\t2\t1\n1\tafter-action\tlist\t0\t1\t3\t3\t1\n1\tafter-done\tlist\t0\t1\t2\t2\t0\n1\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n2\tbefore-search\tdict\t0\t1\t2\t2\t0\n2\tafter-search\tdict\t0\t1\t2\t2\t1\n2\tafter-action\tlist\t0\t1\t2\t2\t0\n2\tafter-done\tlist\t0\t1\t2\t2\t0\n2\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n3\tbefore-search\tdict\t0\t1\t2\t2\t0\n3\tafter-search\tdict\t0\t1\t2\t2\t1\n3\tafter-action\tdict\t0\t1\t2\t2\t1\n3\tafter-done\tdict\t0\t1\t2\t2\t0\n3\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n4\tbefore-search\tdict\t0\t1\t2\t2\t0\n4\tafter-search\tdict\t0\t1\t2\t2\t1\n4\tafter-action\tdict\t0\t1\t2\t2\t0\n4\tafter-done\tdict\t0\t1\t2\t2\t0\n4\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-9`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 25
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "0\tbefore-search\tdict\t0\t1\t2\t2\t0\n0\tafter-search\tdict\t0\t1\t2\t2\t1\n0\tafter-action\tdict\t0\t1\t2\t2\t1\n0\tafter-done\tdict\t0\t1\t2\t2\t0\n0\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n1\tbefore-search\tdict\t0\t1\t2\t2\t0\n1\tafter-search\tdict\t0\t1\t2\t2\t1\n1\tafter-action\tlist\t0\t1\t3\t3\t1\n1\tafter-done\tlist\t0\t1\t2\t2\t0\n1\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n2\tbefore-search\tdict\t0\t1\t2\t2\t0\n2\tafter-search\tdict\t0\t1\t2\t2\t1\n2\tafter-action\tlist\t0\t1\t2\t2\t0\n2\tafter-done\tlist\t0\t1\t2\t2\t0\n2\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n3\tbefore-search\tdict\t0\t1\t2\t2\t0\n3\tafter-search\tdict\t0\t1\t2\t2\t1\n3\tafter-action\tdict\t0\t1\t2\t2\t1\n3\tafter-done\tdict\t0\t1\t2\t2\t0\n3\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n4\tbefore-search\tdict\t0\t1\t2\t2\t0\n4\tafter-search\tdict\t0\t1\t2\t2\t1\n4\tafter-action\tdict\t0\t1\t2\t2\t0\n4\tafter-done\tdict\t0\t1\t2\t2\t0\n4\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-10`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 25
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "0\tbefore-search\tdict\t0\t1\t2\t2\t0\n0\tafter-search\tdict\t0\t1\t2\t2\t1\n0\tafter-action\tdict\t0\t1\t2\t2\t1\n0\tafter-done\tdict\t0\t1\t2\t2\t0\n0\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n1\tbefore-search\tdict\t0\t1\t2\t2\t0\n1\tafter-search\tdict\t0\t1\t2\t2\t1\n1\tafter-action\tlist\t0\t1\t3\t3\t1\n1\tafter-done\tlist\t0\t1\t2\t2\t0\n1\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n2\tbefore-search\tdict\t0\t1\t2\t2\t0\n2\tafter-search\tdict\t0\t1\t2\t2\t1\n2\tafter-action\tlist\t0\t1\t2\t2\t0\n2\tafter-done\tlist\t0\t1\t2\t2\t0\n2\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n3\tbefore-search\tdict\t0\t1\t2\t2\t0\n3\tafter-search\tdict\t0\t1\t2\t2\t1\n3\tafter-action\tdict\t0\t1\t2\t2\t1\n3\tafter-done\tdict\t0\t1\t2\t2\t0\n3\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n4\tbefore-search\tdict\t0\t1\t2\t2\t0\n4\tafter-search\tdict\t0\t1\t2\t2\t1\n4\tafter-action\tdict\t0\t1\t2\t2\t0\n4\tafter-done\tdict\t0\t1\t2\t2\t0\n4\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-11`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 25
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "0\tbefore-search\tdict\t0\t1\t2\t2\t0\n0\tafter-search\tdict\t0\t1\t2\t2\t1\n0\tafter-action\tdict\t0\t1\t2\t2\t1\n0\tafter-done\tdict\t0\t1\t2\t2\t0\n0\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n1\tbefore-search\tdict\t0\t1\t2\t2\t0\n1\tafter-search\tdict\t0\t1\t2\t2\t1\n1\tafter-action\tlist\t0\t1\t3\t3\t1\n1\tafter-done\tlist\t0\t1\t2\t2\t0\n1\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n2\tbefore-search\tdict\t0\t1\t2\t2\t0\n2\tafter-search\tdict\t0\t1\t2\t2\t1\n2\tafter-action\tlist\t0\t1\t2\t2\t0\n2\tafter-done\tlist\t0\t1\t2\t2\t0\n2\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n3\tbefore-search\tdict\t0\t1\t2\t2\t0\n3\tafter-search\tdict\t0\t1\t2\t2\t1\n3\tafter-action\tdict\t0\t1\t2\t2\t1\n3\tafter-done\tdict\t0\t1\t2\t2\t0\n3\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n4\tbefore-search\tdict\t0\t1\t2\t2\t0\n4\tafter-search\tdict\t0\t1\t2\t2\t1\n4\tafter-action\tdict\t0\t1\t2\t2\t0\n4\tafter-done\tdict\t0\t1\t2\t2\t0\n4\tafter-root-drop\tdropped\t0\t0\t1\t1\t0\n"
}
```

### jim

Status: `observed`. Version: Jim. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-12`):

```json
{
  "version": "Jim",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 1
}
```

`Jim.tsv` (`file-5`):

```json
{
  "original_table": "unsupported-native-C-search\n"
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/8.4.20.tsv). SHA-256 `1358e42e60fa8a7ad175a3235ec817d49df50adecf13e1a70fc5f6ec1cf9785e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/8.5.19.tsv). SHA-256 `ff1d6093e7abadcfb9c986a95d2578cd79113da85a3e56726b525c8a93c2a3b0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/8.6.18.tsv). SHA-256 `ff1d6093e7abadcfb9c986a95d2578cd79113da85a3e56726b525c8a93c2a3b0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/9.0.4.tsv). SHA-256 `ff1d6093e7abadcfb9c986a95d2578cd79113da85a3e56726b525c8a93c2a3b0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/9.1.0.tsv). SHA-256 `ff1d6093e7abadcfb9c986a95d2578cd79113da85a3e56726b525c8a93c2a3b0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/Jim.tsv). SHA-256 `1358e42e60fa8a7ad175a3235ec817d49df50adecf13e1a70fc5f6ec1cf9785e`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-6-row-7` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-6-row-8` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-6-row-9` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-6-row-10` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-6-row-11` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-6-row-12` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/manifest.json). SHA-256 `cf61f642805b85a2c4605e560055bee894c4dfd1f2304ce93dc127bd4229ac6c`. JSON pointer `/observations/5`. Original provider/capture association at its exact selected row.
- `file-13` (input): [rust/tcl-vm/tests/data/native_dictionary_search/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_search/probe.c). SHA-256 `4b4028d35fd4004a44e8faeb933e551774e3366fa111dc16935f9851acfa0ea9`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
