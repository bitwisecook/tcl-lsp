# naming.formal.original-bytearray-materialized-name-storage

Kind: `native-observation`

## Problem statement

The same raw byte octets enter as a pure Bytearray rather than a resident String. Its native materialization can change declaration/parser behavior before full-name lookup, so raw-zero String observations cannot be reused.

## Question

Do pure Bytearray raw-zero and modified-zero formal names produce the same short/dynamic read results as the original String controls?

## Conclusion

All five C captures install both original Bytearray formals and return ARG through the compiled full name. Their short k reads fail and dynamic-full returns0 1 ARG, including C8.4/8.5 where the raw-zero String sibling collapses to short k. Jim's Bytearray constructor branch is compiled out and not tested; no guest unsupported error is inferred. The outputs measure actual materialization/consumer results, not original stored Bytearray formal identity.

## Scope

Selected pure-bytearray rows from the retained observer-safe formal probe:24 full C rows per release and12 Jim String rows,132 total. Original names are counted3-byte k00z and4-byte kC080z; pure Bytearray is an independent C constructor path. Original parameter List and List-form body enter public object-vector proc; other bodies use ASCII strings and fullname returns the actual retained original name. Saved result/return options precede any error observation; C84 reads physical globals without callbacks, C85+ saved options, Jim uses its no-trace root scalar getter. The older pre-observer source exists only as a receipt digest and is not claimed recovered. No native source parser excerpt, info args or private formal allocation/pointer proof. Captured Jim git pin is recorded; launch patchlevel/UTF configuration/compiler version remain unknown. No Rust execution result.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.4 8.4.20.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":null}
```

### tcl8.5

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.5 8.5.19.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"4e4f4e45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72636f6465204e4f4e45202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c6172312043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### tcl9.1

Status: `observed`. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. Version: tcl9.1 9.1.0.

Exact selected definition/invocation rows with result, error-code/info and return-options hex. Jim revision recorded in this receipt: None.

```jsonl
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c61722043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-raw-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-full-invocation","code":0,"result":"415247","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"compiled-short-invocation","code":1,"result":"63616e2774207265616420226b223a206e6f2073756368207661726961626c65","result_type":"string","error_code":"54434c2052454144205641524e414d45","error_info":"63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a22702041524722","return_options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b494e4e4552206c6f61645363616c61722043414c4c207b70204152477d7d202d6572726f72636f6465207b54434c2052454144205641524e414d457d202d6572726f72696e666f207b63616e2774207265616420226b223a206e6f2073756368207661726961626c650a202020207768696c6520657865637574696e670a22736574206b220a202020202870726f63656475726520227022206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227020415247227d202d6572726f726c696e652031"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-definition","code":0,"result":"","result_type":"string","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
{"case":"formal-modified-nul","path":"pure-bytearray","op":"dynamic-full-invocation","code":0,"result":"30203120415247","result_type":"list","error_code":null,"error_info":null,"return_options":"2d636f64652030202d6c6576656c2030"}
```

### jim, bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `formal_storage_name_input`: Selects formal-name storage extent independently from enumeration/display purposes.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `parse_params_object`: Requires the selected real formal List conversion and default/member producers.

## Evidence

- [rust/tcl-syntax/tests/data/native_formal_storage/probe.c](../../../../rust/tcl-syntax/tests/data/native_formal_storage/probe.c)
- [rust/tcl-syntax/tests/data/native_formal_storage/manifest.json](../../../../rust/tcl-syntax/tests/data/native_formal_storage/manifest.json)
