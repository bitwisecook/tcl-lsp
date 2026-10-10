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

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=b0f14f8000fa4d9901e3fb61926ea76f7fa7eccf005fbdc2da2303339f49cec2; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=13b66883b7d7b7eb01e982b57c303ac69edaad8d076502617c09db09c553c442; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector.. Dialect: C Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=efdc88a254984ca377fd6098ce6ef4b2ce1c1f98fe2919c6c104ec642079ec9c; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector.. Dialect: C Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=e1fb9749b9ef0c62f96dcc4ba7ad71ba07c46e18a2baff2b1d237b9d053f2189; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=f7b76f574e8264dfe42c866b8bf069f397c8f8e34f718ff978e6b29b8cb339b1; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted String or pure Bytearray formal name inside real List argv; public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector.. Dialect: C Tcl.

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

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original observation for this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_formal_storage/probe.c](../../../../rust/tcl-syntax/tests/data/native_formal_storage/probe.c). SHA-256 `507bb48a6b43c9c7f0d678ea0b034737c88a5991716e6510a3fdec1f8d5dc9c2`. Exact retained observer-safe source with original String/Bytearray/List producers and independent compiled/dynamic bodies.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_formal_storage/manifest.json](../../../../rust/tcl-syntax/tests/data/native_formal_storage/manifest.json). SHA-256 `56dfde3d1e423a32a9593e05078deaf85a22ae40011e3dd8e57510273490ac15`. Six source/header/lib/executable/status/raw stream associations and exact Jim revision. Older unretained source hash is metadata only.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_formal_storage/8.4.20.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/8.4.20.jsonl). SHA-256 `31f30f6854db77de2981b373e67eaf7cc2112ee9baa123a279f0391cf078bda5`. Full original JSONL; selected path pure-bytearray. Error/result/return-option hex captured before observer effects.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_formal_storage/8.5.19.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/8.5.19.jsonl). SHA-256 `a80de9f57c83cb739369e495890f08dde27fd843740e5ad57394b7f265586ce7`. Full original JSONL; selected path pure-bytearray. Error/result/return-option hex captured before observer effects.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_formal_storage/8.6.18.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/8.6.18.jsonl). SHA-256 `846314c8a2ddb4a024d9c530636aa9dab0418d89d9665bb1bce4fac0b66d8ee9`. Full original JSONL; selected path pure-bytearray. Error/result/return-option hex captured before observer effects.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_formal_storage/9.0.4.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/9.0.4.jsonl). SHA-256 `846314c8a2ddb4a024d9c530636aa9dab0418d89d9665bb1bce4fac0b66d8ee9`. Full original JSONL; selected path pure-bytearray. Error/result/return-option hex captured before observer effects.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_formal_storage/9.1.0.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/9.1.0.jsonl). SHA-256 `0a2410eba0ce88c83f64cc2f5e89844cd8b659810b2267a832f5abfacbd53d00`. Full original JSONL; selected path pure-bytearray. Error/result/return-option hex captured before observer effects.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `formal_storage_name_input`: Selects formal-name storage extent independently from enumeration/display purposes.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `parse_params_object`: Requires the selected real formal List conversion and default/member producers.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
