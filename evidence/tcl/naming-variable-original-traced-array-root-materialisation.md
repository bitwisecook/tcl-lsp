# Tcl execution evidence

## Question

> A write trace can be registered before an array has a value, and a callback can write a later element while array set processes its original pairs. The complete original public result is needed without inferring private shell, binding or receiver identity.

> What completion and final element values does the whole original program return after registering a write trace on an unset root and executing array set whose callback writes a later pair?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged original ASCII LF source runs independently in six fresh fully initialised provider interpreters under the unchanged counted-source CAPI driver, without an added source catch/eval/puts envelope. Six compile receipts/ELFs and six whole process streams/receipts retain source SHA4ffc50f2014c35f4bf451199b9916153f104e79fdf153b28c7280db8ea75177b and required archive/header/build/source pins. The initial request and expanded capture request remain separate original inputs. No private shell-materialisation observer, alias/foreign receiver, opaque/NUL/Unicode variable name, BIG-IP execution or executed software control is supplied. The linked Runtime public comparison uses all six untouched originals and whole completion/final values, preserving Jim trace failure. Separate software CellContents undefined-root markers retain the actual producer-issued VarHome and binding identity. VariableReceiver and retained NativeScalarAliasEntry may materialise only that selected original undefined root; an ordinary scalar, retired owner or same-spelled foreign link cannot issue or borrow the marker. These software ownership controls neither turn FIRST LAST into a provider private-shell/cell/header observation nor claim an executed Rust pass.

## Script

```tcl-visible
proc mutate {n e op} {if {$e eq {k}} {set ::heldArray(j) CALLBACK}}; trace add variable ::heldArray write mutate; set name ::heldArray; array set $name {k FIRST j LAST}; list $::heldArray(k) $::heldArray(j)
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|4649525354204c415354
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4649525354204c415354
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4649525354204c415354
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4649525354204c415354
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4649525354204c415354
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.
