# naming.formal.original-string-zero-storage-and-lookup

Kind: `native-observation`

## Problem statement

A raw-zero formal String and a modified-zero String can enter the same List declaration yet reach different short-name cells. Compiled full reads and dynamic original-name reads must be compared independently rather than assuming an introspection spelling.

## Question

What compiled-full, compiled-short and dynamic-full results follow original raw-zero versus modified-zero String formals?

## Conclusion

Every capture installs all three bodies and returns ARG through the full compiled name. C8.4/8.5 raw-zero String formals also return1 ARG through short k and1 1 ARG through dynamic-full existence/read, while their modified-zero formals fail short k and return0 1 ARG dynamically. C8.6–9.1 and Jim keep both original pairs distinct from short k: short reads fail and dynamic-full returns0 1 ARG. Exact error/options bytes are retained. No info args/introspection command or private actual formal-slot pointer is sampled here.

## Scope

Selected raw-string rows from the retained observer-safe formal probe:24 full C rows per release and12 Jim String rows,132 total. Original names are counted3-byte k00z and4-byte kC080z; pure Bytearray is an independent C constructor path. Original parameter List and List-form body enter public object-vector proc; other bodies use ASCII strings and fullname returns the actual retained original name. Saved result/return options precede any error observation; C84 reads physical globals without callbacks, C85+ saved options, Jim uses its no-trace root scalar getter. The older pre-observer source exists only as a receipt digest and is not claimed recovered. No native source parser excerpt, info args or private formal allocation/pointer proof. Captured Jim git pin is recorded; launch patchlevel/UTF configuration/compiler version remain unknown. No Rust execution result.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.4 8.4.20.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-invocation","code":0,"result":"3120415247","result_type":"list","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"31203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
```

### tcl8.5

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.5 8.5.19.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-invocation","code":0,"result":"3120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"31203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### tcl9.1

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl9.1 9.1.0.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c61722043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c61722043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### jim

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: jim Jim capture.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: 5bac7c99ad65864c87da513e22e2f01703fa4e03.

```jsonl
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-definition","code":0,"result":"70","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"raw-string","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":null}
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `formal_storage_name_input`: Selects formal-name storage extent independently from enumeration/display purposes.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `parse_params_object`: Requires the selected real formal List conversion and default/member producers.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `cmd_proc::tests::native_formals_keep_storage_keys_and_original_default_objects` (linked): Asserts old-C rawzero formal truncation versus retained counted storage keys with authentic parse_params_object. It does not reproduce every attached dynamic body/error-options row.

## Evidence

- [rust/tcl-syntax/tests/data/native_formal_storage/probe.c](../../../../rust/tcl-syntax/tests/data/native_formal_storage/probe.c)
- [rust/tcl-syntax/tests/data/native_formal_storage/manifest.json](../../../../rust/tcl-syntax/tests/data/native_formal_storage/manifest.json)
