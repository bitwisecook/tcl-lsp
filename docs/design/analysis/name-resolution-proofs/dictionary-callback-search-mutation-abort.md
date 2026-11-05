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

### tcl8.4, jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

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

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/manifest.json)
- [rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_search/mutation/callbacks/probe.c)
