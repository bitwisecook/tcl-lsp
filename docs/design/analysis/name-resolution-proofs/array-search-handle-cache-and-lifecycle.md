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

Status: `observed`. Version: 8.4; 8.4.20. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.4.

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

Status: `observed`. Version: 8.5; 8.5.19. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.5.

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

Status: `observed`. Version: 8.6; 8.6.18. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl8.6.

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

Status: `observed`. Version: 9.0; 9.0.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.0.

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

Status: `observed`. Version: 9.1; 9.1.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Tcl_Eval. Exact lengths, flags and input constructors remain in the source.. Dialect: tcl9.1.

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

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: jim.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-syntax/tests/data/native_array_search/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_array_search/8.4.20.tsv). SHA-256 `6dc66203c4f324bb220eb0306950eb7b78a7ff2f48da92199868368b964e9b80`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (observation): [rust/tcl-syntax/tests/data/native_array_search/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_array_search/8.5.19.tsv). SHA-256 `b9006b978760b94be24968eb3cdaa77e58e9fdbb717b58e8e7e0babf90e0a757`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-syntax/tests/data/native_array_search/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_array_search/8.6.18.tsv). SHA-256 `44ab60622b7fb587777144a69607c6942276835fe0c67538402c642d4d442c97`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-3` (observation): [rust/tcl-syntax/tests/data/native_array_search/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_array_search/9.0.4.tsv). SHA-256 `109cbbcd94e42d6121f143ebb62fab513a5fe9c9705233fac920a33300d6278f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-4` (observation): [rust/tcl-syntax/tests/data/native_array_search/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_array_search/9.1.0.tsv). SHA-256 `109cbbcd94e42d6121f143ebb62fab513a5fe9c9705233fac920a33300d6278f`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5` (observation): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-5-row-6` (provider): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-5-row-7` (provider): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-5-row-8` (provider): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-5-row-9` (provider): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-5-row-10` (provider): [rust/tcl-syntax/tests/data/native_array_search/manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/manifest.json). SHA-256 `1d48c3bb478145c9d3c6269bd59b972b605ff0681d7a7f80279953fd9cc754b4`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-11` (input): [rust/tcl-syntax/tests/data/native_array_search/probe.c](../../../../rust/tcl-syntax/tests/data/native_array_search/probe.c). SHA-256 `e469d94eb250edf87f3075841695011c5010f395531ed48647937700f0eb37e5`. Exact retained input/program bytes; purpose is limited to this question.
- `file-12` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.4.txt](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.4.txt). SHA-256 `d26b494bc20e5dd3f7efd78004e84fa070cc125aad168759161a9c8faa547a96`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-13` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.5.txt](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.5.txt). SHA-256 `d26b494bc20e5dd3f7efd78004e84fa070cc125aad168759161a9c8faa547a96`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-14` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.6.txt](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.6.txt). SHA-256 `d26b494bc20e5dd3f7efd78004e84fa070cc125aad168759161a9c8faa547a96`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-15` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.0.txt](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.0.txt). SHA-256 `50c88cbc3d458bec4c8001bfbc643e6d0fd3f2ef3008521c49a45609ae5c4c34`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-16` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.1.txt](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.1.txt). SHA-256 `50c88cbc3d458bec4c8001bfbc643e6d0fd3f2ef3008521c49a45609ae5c4c34`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-17` (observation): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-17-row-18` (provider): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. JSON pointer `/observations/0`. Original provider/capture association at its exact selected row.
- `file-17-row-19` (provider): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. JSON pointer `/observations/1`. Original provider/capture association at its exact selected row.
- `file-17-row-20` (provider): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. JSON pointer `/observations/2`. Original provider/capture association at its exact selected row.
- `file-17-row-21` (provider): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. JSON pointer `/observations/3`. Original provider/capture association at its exact selected row.
- `file-17-row-22` (provider): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-manifest.json). SHA-256 `a1cb9e7800bac5eb9e6f47f2aa86f1797bf8a7311f79c423b2a4ba6c5a8aa2d8`. JSON pointer `/observations/4`. Original provider/capture association at its exact selected row.
- `file-23` (input): [rust/tcl-syntax/tests/data/native_array_search/search-lifecycle.tcl](../../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle.tcl). SHA-256 `8c734c315610bf1f6535d57ee9c00d992466a18c0147e0371e6c47dfc91caa2e`. Exact retained input/program bytes; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
