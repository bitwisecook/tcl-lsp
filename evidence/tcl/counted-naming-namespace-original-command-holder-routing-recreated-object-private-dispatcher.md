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
set r [namespace eval N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {
                            oo::class create C {
                                method m {} {return NEW}
                                method n {} {my m}
                            }
                            C create o
                        }
                        o m
                    }
                    p
                }]
                list $r [info commands ::N::o::my] [::N::o n]⟦EOF⟧
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "oo::class"⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4f4c44207b7d204e4557
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD {} NEW⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4f4c44207b7d204e4557
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD {} NEW⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4f4c44207b7d204e4557
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD {} NEW⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "oo::class"⟦EOF⟧
```
