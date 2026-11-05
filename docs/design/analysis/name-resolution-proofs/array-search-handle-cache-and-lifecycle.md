# naming.array.search-handle-cache-and-lifecycle

Kind: `native-observation`

## Problem statement

An array-search handle can cache an ID/name offset, expire after mutation, or collide with a new active handle spelling. Lookup text alone does not establish which search is current.

## Question

What original handle fields, result bytes and separate script search-lifecycle outcomes occur in the captured C controls?

## Conclusion

The physical and script tables remain separate. C8 records decimal ID/name-offset cache behavior; C9 records original handle identity plus CString spelling. The captured ABI uses 64-bit unsigned long; independent 32-bit recipe expectations are not native observations.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4; 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-6`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "stderr": ""
}
```

`search-lifecycle-manifest.json` (`file-17-row-18`):

```json
{
  "version": "8.4",
  "exit": 0,
  "stderr": ""
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "0\t0\tarray search\t1\t4\t31\t\n1\t0\tarray search\t1\t5\t31\t\n2\t0\tarray search\t1\t5\t31\t\n3\t0\tarray search\t1\t5\t31\t\n4\t1\tarray search\t-1\t5\t636f756c646e27742066696e64207365617263682022732d2d312d6122\t\n5\t0\tarray search\t1\t13\t31\t\n6\t1\tarray search\t-1\t23\t636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\t\n7\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\t\n8\t0\tarray search\t1\t4\t31\t\n9\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e746966696572202242414422\t\n10\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d3122\t\n11\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d2d6122\t\n12\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d61ff222069736e277420666f72207661726961626c6520226122\t\n"
}
```

`search-lifecycle-8.4.txt` (`file-12`):

```json
{
  "original_table": "initial-id 0 732d312d61\nundefined-shell-removal-anymore 0 31\nundefined-shell-removal-next 0 6b3035\nunsigned-leading-zero 0 31\nsigned-leading-plus 0 31\nwrong-variable 1 736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\ndefine-existing-keeps-search 0 31\nnew-key-invalidates 1 636f756c646e27742066696e64207365617263682022732d312d6122\nrestarted-id 0 732d312d61\nconcurrent-id 0 732d322d61\ndeleted-top-id-reused 0 732d322d61\nlong-overflow-identifier 1 636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\ndone-search-miss 1 636f756c646e27742066696e64207365617263682022732d312d6122\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5; 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-7`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "stderr": ""
}
```

`search-lifecycle-manifest.json` (`file-17-row-19`):

```json
{
  "version": "8.5",
  "exit": 0,
  "stderr": ""
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "0\t0\tarray search\t1\t4\t31\t534545444544204152524159\n1\t0\tarray search\t1\t5\t31\t534545444544204152524159\n2\t0\tarray search\t1\t5\t31\t534545444544204152524159\n3\t0\tarray search\t1\t5\t31\t534545444544204152524159\n4\t1\tarray search\t-1\t5\t636f756c646e27742066696e64207365617263682022732d2d312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d312d61\n5\t0\tarray search\t1\t13\t31\t534545444544204152524159\n6\t1\tarray search\t-1\t23\t636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\t54434c204c4f4f4b555020415252415953454152434820732d31383434363734343037333730393535313631372d61\n7\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d6f74686572\n8\t0\tarray search\t1\t4\t31\t534545444544204152524159\n9\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e746966696572202242414422\t534545444544204152524159\n10\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d3122\t534545444544204152524159\n11\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d2d6122\t534545444544204152524159\n12\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d61ff222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d61ff\n"
}
```

`search-lifecycle-8.5.txt` (`file-13`):

```json
{
  "original_table": "initial-id 0 732d312d61\nundefined-shell-removal-anymore 0 31\nundefined-shell-removal-next 0 6b3035\nunsigned-leading-zero 0 31\nsigned-leading-plus 0 31\nwrong-variable 1 736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\ndefine-existing-keeps-search 0 31\nnew-key-invalidates 1 636f756c646e27742066696e64207365617263682022732d312d6122\nrestarted-id 0 732d312d61\nconcurrent-id 0 732d322d61\ndeleted-top-id-reused 0 732d322d61\nlong-overflow-identifier 1 636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\ndone-search-miss 1 636f756c646e27742066696e64207365617263682022732d312d6122\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6; 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-8`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "stderr": ""
}
```

`search-lifecycle-manifest.json` (`file-17-row-20`):

```json
{
  "version": "8.6",
  "exit": 0,
  "stderr": ""
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "0\t0\tarray search\t1\t4\t31\t\n1\t0\tarray search\t1\t5\t31\t\n2\t0\tarray search\t1\t5\t31\t\n3\t0\tarray search\t1\t5\t31\t\n4\t1\tarray search\t-1\t5\t636f756c646e27742066696e64207365617263682022732d2d312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d312d61\n5\t0\tarray search\t1\t13\t31\t\n6\t1\tarray search\t-1\t23\t636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\t54434c204c4f4f4b555020415252415953454152434820732d31383434363734343037333730393535313631372d61\n7\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d6f74686572\n8\t0\tarray search\t1\t4\t31\t\n9\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e746966696572202242414422\t54434c204c4f4f4b555020415252415953454152434820424144\n10\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d3122\t54434c204c4f4f4b555020415252415953454152434820732d31\n11\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d2d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d61\n12\t1\tarray search\t1\t4\t736561726368206964656e7469666965722022732d312d61ff222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d61ff\n"
}
```

`search-lifecycle-8.6.txt` (`file-14`):

```json
{
  "original_table": "initial-id 0 732d312d61\nundefined-shell-removal-anymore 0 31\nundefined-shell-removal-next 0 6b3035\nunsigned-leading-zero 0 31\nsigned-leading-plus 0 31\nwrong-variable 1 736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\ndefine-existing-keeps-search 0 31\nnew-key-invalidates 1 636f756c646e27742066696e64207365617263682022732d312d6122\nrestarted-id 0 732d312d61\nconcurrent-id 0 732d322d61\ndeleted-top-id-reused 0 732d322d61\nlong-overflow-identifier 1 636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\ndone-search-miss 1 636f756c646e27742066696e64207365617263682022732d312d6122\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0; 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-9`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "stderr": ""
}
```

`search-lifecycle-manifest.json` (`file-17-row-21`):

```json
{
  "version": "9.0",
  "exit": 0,
  "stderr": ""
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "0\t0\tnone\t-\t-\t31\t\n1\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d30312d6122\t54434c204c4f4f4b555020415252415953454152434820732d30312d61\n2\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d2b312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2b312d61\n3\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d20312d6122\t54434c204c4f4f4b5550204152524159534541524348207b732d20312d617d\n4\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d2d312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d312d61\n5\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d343239343936373239372d6122\t54434c204c4f4f4b555020415252415953454152434820732d343239343936373239372d61\n6\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\t54434c204c4f4f4b555020415252415953454152434820732d31383434363734343037333730393535313631372d61\n7\t1\tnone\t-\t-\t736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d6f74686572\n8\t0\tnone\t-\t-\t31\t\n9\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e746966696572202242414422\t54434c204c4f4f4b555020415252415953454152434820424144\n10\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d3122\t54434c204c4f4f4b555020415252415953454152434820732d31\n11\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d2d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d61\n12\t1\tnone\t-\t-\t736561726368206964656e7469666965722022732d312d61ff222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d61ff\n"
}
```

`search-lifecycle-9.0.txt` (`file-15`):

```json
{
  "original_table": "initial-id 0 732d312d61\nundefined-shell-removal-anymore 0 31\nundefined-shell-removal-next 0 6b3035\nunsigned-leading-zero 1 636f756c646e27742066696e64207365617263682022732d30312d6122\nsigned-leading-plus 1 636f756c646e27742066696e64207365617263682022732d2b312d6122\nwrong-variable 1 736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\ndefine-existing-keeps-search 0 31\nnew-key-invalidates 1 636f756c646e27742066696e64207365617263682022732d312d6122\nrestarted-id 0 732d312d61\nconcurrent-id 0 732d322d61\ndeleted-top-id-reused 0 732d322d61\nlong-overflow-identifier 1 636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\ndone-search-miss 1 636f756c646e27742066696e64207365617263682022732d312d6122\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1; 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-5-row-10`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "stderr": ""
}
```

`search-lifecycle-manifest.json` (`file-17-row-22`):

```json
{
  "version": "9.1",
  "exit": 0,
  "stderr": ""
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "0\t0\tnone\t-\t-\t31\t\n1\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d30312d6122\t54434c204c4f4f4b555020415252415953454152434820732d30312d61\n2\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d2b312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2b312d61\n3\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d20312d6122\t54434c204c4f4f4b5550204152524159534541524348207b732d20312d617d\n4\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d2d312d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d312d61\n5\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d343239343936373239372d6122\t54434c204c4f4f4b555020415252415953454152434820732d343239343936373239372d61\n6\t1\tnone\t-\t-\t636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\t54434c204c4f4f4b555020415252415953454152434820732d31383434363734343037333730393535313631372d61\n7\t1\tnone\t-\t-\t736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d6f74686572\n8\t0\tnone\t-\t-\t31\t\n9\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e746966696572202242414422\t54434c204c4f4f4b555020415252415953454152434820424144\n10\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d3122\t54434c204c4f4f4b555020415252415953454152434820732d31\n11\t1\tnone\t-\t-\t696c6c6567616c20736561726368206964656e7469666965722022732d2d6122\t54434c204c4f4f4b555020415252415953454152434820732d2d61\n12\t1\tnone\t-\t-\t736561726368206964656e7469666965722022732d312d61ff222069736e277420666f72207661726961626c6520226122\t54434c204c4f4f4b555020415252415953454152434820732d312d61ff\n"
}
```

`search-lifecycle-9.1.txt` (`file-16`):

```json
{
  "original_table": "initial-id 0 732d312d61\nundefined-shell-removal-anymore 0 31\nundefined-shell-removal-next 0 6b3035\nunsigned-leading-zero 1 636f756c646e27742066696e64207365617263682022732d30312d6122\nsigned-leading-plus 1 636f756c646e27742066696e64207365617263682022732d2b312d6122\nwrong-variable 1 736561726368206964656e7469666965722022732d312d6f74686572222069736e277420666f72207661726961626c6520226122\ndefine-existing-keeps-search 0 31\nnew-key-invalidates 1 636f756c646e27742066696e64207365617263682022732d312d6122\nrestarted-id 0 732d312d61\nconcurrent-id 0 732d322d61\ndeleted-top-id-reused 0 732d322d61\nlong-overflow-identifier 1 636f756c646e27742066696e64207365617263682022732d31383434363734343037333730393535313631372d6122\ndone-search-miss 1 636f756c646e27742066696e64207365617263682022732d312d6122\n"
}
```

### jim, bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_array_search](../../../../rust/tcl-syntax/tests/data/native_array_search)
