# Tcl counted-source execution evidence

## Question

> Equal displayed namespace paths do not establish current command routing or survival after namespace deletion/recreation. Procedure definitions, synchronous delete callbacks, TclOO calls and C alias publication have distinct original entry purposes; a combined eval/catch envelope can change the source channel.

> What exact public completion and separate query results do the five original namespace deletion/recreation, callback-created/replaced procedure, TclOO replacement and relative/rooted C alias controls produce under independent external API evaluations?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Five exact original ASCII files, two exact independent recorder preludes and two exact independent queries. A fresh externally compiled API driver per case uses full Tcl_Init and Tcl_EvalEx(flags0), or Jim core/static-extension initialization and counted Jim_EvalObj, then destroys that interpreter. The driver reads original files directly, without surrounding catch/eval/puts or joining prelude/body/query evaluations. Existing catch constructs inside the original alias/retained source remain original input. C-version and language availability differences remain separate; C8.4/C8.5 deferred failure gives no completed result tuple, and unavailable C TclOO or Jim namespace/trace/TclOO surfaces are not replaced with other APIs. The finite log order is preserved byte-exact. Source/library/build associations do not prove private implementation state or complete linked-source identity. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass. Both fixtures use default Interp::new helpers, not a six-provider native replay. Their public strings do not expose native command pointers, private dispatcher existence, reference counts or a complete current table. Original independently evaluated C8.6/C9.0/C9.1 and unavailable/deferred provider windows remain unchanged. Source linkage attaches no assertion outcome or native holder admission.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
namespace eval AliasHolder {
    namespace eval q {}
    proc inspect {} {
        namespace delete ::AliasHolder
        set relative_code [catch {interp alias {} q::relative {} list RELATIVE} relative_result]
        set rooted_code [catch {interp alias {} ::AliasHolder::q::rooted {} list ROOTED} rooted_result]
        set local_code [catch {q::relative} local_result]
        set root_code [catch {::AliasHolder::q::rooted} root_result]
        list $relative_code $relative_result $rooted_code $rooted_result $local_code $local_result $root_code $root_result [namespace exists ::AliasHolder]
    }
}
set first [AliasHolder::inspect]
list $first [namespace exists ::AliasHolder] [info commands ::AliasHolder::q::*]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b3020713a3a72656c61746976652030203a3a416c696173486f6c6465723a3a713a3a726f6f74656420302052454c4154495645203020524f4f54454420317d2031203a3a416c696173486f6c6465723a3a713a3a726f6f746564
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{0 q::relative 0 ::AliasHolder::q::rooted 0 RELATIVE 0 ROOTED 1} 1 ::AliasHolder::q::rooted⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b3020713a3a72656c61746976652030203a3a416c696173486f6c6465723a3a713a3a726f6f74656420302052454c4154495645203020524f4f54454420317d2031203a3a416c696173486f6c6465723a3a713a3a726f6f746564
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{0 q::relative 0 ::AliasHolder::q::rooted 0 RELATIVE 0 ROOTED 1} 1 ::AliasHolder::q::rooted⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b3020713a3a72656c61746976652030203a3a416c696173486f6c6465723a3a713a3a726f6f74656420302052454c4154495645203020524f4f54454420317d2031203a3a416c696173486f6c6465723a3a713a3a726f6f746564
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{0 q::relative 0 ::AliasHolder::q::rooted 0 RELATIVE 0 ROOTED 1} 1 ::AliasHolder::q::rooted⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b3020713a3a72656c61746976652030203a3a416c696173486f6c6465723a3a713a3a726f6f74656420302052454c4154495645203020524f4f54454420317d2031203a3a416c696173486f6c6465723a3a713a3a726f6f746564
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{0 q::relative 0 ::AliasHolder::q::rooted 0 RELATIVE 0 ROOTED 1} 1 ::AliasHolder::q::rooted⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b3020713a3a72656c61746976652030203a3a416c696173486f6c6465723a3a713a3a726f6f74656420302052454c4154495645203020524f4f54454420317d2031203a3a416c696173486f6c6465723a3a713a3a726f6f746564
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{0 q::relative 0 ::AliasHolder::q::rooted 0 RELATIVE 0 ROOTED 1} 1 ::AliasHolder::q::rooted⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|6e616d6573706163652c20756e6b6e6f776e20636f6d6d616e642022657869737473223a2073686f756c642062652063616e6f6e6963616c2c20636f64652c2063757272656e742c2064656c6574652c20656e73656d626c652c206576616c2c206578706f72742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c2075707661722c207768696368
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
namespace, unknown command "exists": should be canonical, code, current, delete, ensemble, eval, export, import, inscope, origin, parent, qualifiers, tail, upvar, which⟦EOF⟧
```
