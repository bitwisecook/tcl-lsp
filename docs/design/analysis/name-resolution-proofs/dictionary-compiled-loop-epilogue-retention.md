# naming.dictionary.compiled-loop-epilogue-retention

Kind: `native-observation`

## Problem statement

Compiled dictionary loops release their iterator/root on completion, error, break, continue or return. A generic loop with equal output cannot establish the compiled iterator lifetime.

## Question

What original root references and complete completion/options values are observed before, during and after compiled for/map epilogues?

## Conclusion

Thirty-five complete captured C executions retain independent epilogue and root windows. Map availability begins later than for. Tcl 8.4 absence supplies no loop result; compiled coverage additionally requires the native compiled entry, not a generic output match.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 0
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": ""
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 5
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "for-normal\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-error\t0\t3120424f445920312030207b2d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 10
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "for-normal\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-normal\t0\t30207b6b204f4b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 10
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "for-normal\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-normal\t0\t30207b6b204f4b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "compile_stderr": "",
  "exit": 0,
  "stderr": "",
  "rows": 10
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "for-normal\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nfor-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-normal\t0\t30207b6b204f4b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-error\t0\t3120424f445920312030207b2d6572726f72737461636b207b494e4e4552207b72657475726e496d6d20424f4459207b7d7d2043414c4c20707d202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b424f44590a202020207768696c6520657865637574696e670a226572726f7220424f4459227d202d6572726f726c696e652031202d636f64652031202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-break\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-continue\t0\t30207b7d20302030207b2d636f64652030202d6c6576656c20307d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\nmap-return\t0\t3220424f445920302031207b2d636f64652030202d6c6576656c20317d\t7b6265666f726520347d207b647572696e6720357d207b616674657220347d\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/9.1.0.tsv)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/manifest.json](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/manifest.json)
- [rust/tcl-vm/tests/data/native_dictionary_search/epilogues/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_search/epilogues/probe.c)
