# naming.dictionary.callback-search-mutation-abort

Kind: `native-observation`

## Problem statement

A catch/try around a search callback might appear able to catch concurrent modification, but the native backing epoch guard can terminate the process outside guest handlers.

## Question

What process status and original reference/output windows follow same-backing mutation and callback search advancement?

## Conclusion

All fourteen captured processes abort with their original partial streams and native concurrency diagnostic. They are completed process observations, not successful guest comparisons; catch/try does not turn them into a guest completion.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-1`):

```json
{
  "version": "8.5.19",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-2`):

```json
{
  "version": "8.5.19",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-3`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-4`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-5`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-6`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-7`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-8`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-9`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-10`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-11`):

```json
{
  "version": "9.1.0",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-12`):

```json
{
  "version": "9.1.0",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-13`):

```json
{
  "version": "9.1.0",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-14`):

```json
{
  "version": "9.1.0",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/3`. Original provider/capture association at its exact selected row.
- `file-0-row-5` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/4`. Original provider/capture association at its exact selected row.
- `file-0-row-6` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/5`. Original provider/capture association at its exact selected row.
- `file-0-row-7` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/6`. Original provider/capture association at its exact selected row.
- `file-0-row-8` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/7`. Original provider/capture association at its exact selected row.
- `file-0-row-9` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/8`. Original provider/capture association at its exact selected row.
- `file-0-row-10` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/9`. Original provider/capture association at its exact selected row.
- `file-0-row-11` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/10`. Original provider/capture association at its exact selected row.
- `file-0-row-12` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/11`. Original provider/capture association at its exact selected row.
- `file-0-row-13` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/12`. Original provider/capture association at its exact selected row.
- `file-0-row-14` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json). SHA-256 `c565ebfa40e52bd75bdd8079d7429ed9a89808f6a3750a383671c56a12b411fa`. JSON pointer `/rows/13`. Original provider/capture association at its exact selected row.
- `file-15` (input): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/probe.c). SHA-256 `41380709c2c816c59340764d135c25e075d9694b2a6f71c3d87d11648ed7ee6e`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
