# naming.dictionary.direct-search-mutation-close-versus-next

Kind: `native-observation`

## Problem statement

An active search lock does not necessarily force copy-on-write. Explicit close after mutation can be clean while advancing the invalidated search terminates the process.

## Question

How do close and next differ after replacing, adding or removing data in the original active-search backing?

## Conclusion

The sixteen original controls keep eight clean closes and eight process aborts separate. The active lock alone does not establish COW; advancing a changed backing triggers the actual native guard. Partial abort output is not an exhausted-search result.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-1`):

```json
{
  "version": "8.5.19",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-2`):

```json
{
  "version": "8.5.19",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-3`):

```json
{
  "version": "8.5.19",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-4`):

```json
{
  "version": "8.5.19",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-5`):

```json
{
  "version": "8.6.18",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-6`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-7`):

```json
{
  "version": "8.6.18",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-8`):

```json
{
  "version": "8.6.18",
  "exit": -6,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-9`):

```json
{
  "version": "9.0.4",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-10`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-11`):

```json
{
  "version": "9.0.4",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-12`):

```json
{
  "version": "9.0.4",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: direct original-object/private-header calls. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0-row-13`):

```json
{
  "version": "9.1.0",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-14`):

```json
{
  "version": "9.1.0",
  "exit": -4,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t1\n",
  "stderr": "concurrent dictionary modification and search\n"
}
```

`manifest.json` (`file-0-row-15`):

```json
{
  "version": "9.1.0",
  "exit": 0,
  "stdout": "before\t1\t2\t2\t1\nafter\t1\t2\t2\nclosed\n",
  "stderr": ""
}
```

`manifest.json` (`file-0-row-16`):

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

- `file-0` (observation): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/3`. Original provider/capture association at its exact selected row.
- `file-0-row-5` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/4`. Original provider/capture association at its exact selected row.
- `file-0-row-6` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/5`. Original provider/capture association at its exact selected row.
- `file-0-row-7` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/6`. Original provider/capture association at its exact selected row.
- `file-0-row-8` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/7`. Original provider/capture association at its exact selected row.
- `file-0-row-9` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/8`. Original provider/capture association at its exact selected row.
- `file-0-row-10` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/9`. Original provider/capture association at its exact selected row.
- `file-0-row-11` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/10`. Original provider/capture association at its exact selected row.
- `file-0-row-12` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/11`. Original provider/capture association at its exact selected row.
- `file-0-row-13` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/12`. Original provider/capture association at its exact selected row.
- `file-0-row-14` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/13`. Original provider/capture association at its exact selected row.
- `file-0-row-15` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/14`. Original provider/capture association at its exact selected row.
- `file-0-row-16` (provider): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/manifest.json). SHA-256 `ff6a97a63129070dc5746f2a88e7995f38b6f319a1e3c0493a36565de3c62b7c`. JSON pointer `/rows/15`. Original provider/capture association at its exact selected row.
- `file-17` (input): [rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/direct/probe.c). SHA-256 `9ac8f87c98e6fece548c4852ae0728bd9944b6917f8cd1e97d82341edea2cc15`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
