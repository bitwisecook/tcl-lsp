# naming.variable.original-level-object-conversion-and-selection

Kind: `native-observation`

## Problem statement

The same uplevel level spelling can be an original String, WideInt or Double, and an embedded zero can alter a CString conversion without changing the counted object. Release-specific conversion/cache order determines whether the word selects a caller frame or becomes script text; a source numeric grammar alone cannot determine those object effects.

## Question

At current procedure level 2, what original object-cache change, completion code and result bytes occur for the 16 C and independently specified 21 Jim original level operands?

## Conclusion

The retained 80 C and 21 Jim rows distinguish original String, embedded-zero, native integer and Double selector routes. C8.4 preserves the original cache in these cases; newer C releases and Jim have independently measured cache/validation order. The complete per-provider rows are the answer: they do not certify another activation, arbitrary cached objects, variable read completion or a native frame token.

## Scope

Exact retained probe.c/jim-probe.c object constructors and two procedure activations, C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-labelled capture. C inputs include 13 counted String spellings plus native WideInt/Double/NaN; Jim has 18 String spellings plus native Int/Double/NaN. Public object argv evaluation, no character-channel translation; BIG-IP not queried.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original native objects through Tcl_EvalObjv. Version: tcl8.4 8.4.20.

Exactly 16 completed observations. Fields case/before/after/code/result retain the original object and native result bytes.

```jsonl
{"case":0,"before":"string","after":"string","code":0,"result":"31"}
{"case":1,"before":"string","after":"string","code":0,"result":"31"}
{"case":2,"before":"string","after":"string","code":0,"result":"31"}
{"case":3,"before":"string","after":"string","code":0,"result":"31"}
{"case":4,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222d3122"}
{"case":5,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222b3122"}
{"case":6,"before":"string","after":"string","code":0,"result":"31"}
{"case":7,"before":"string","after":"string","code":1,"result":"657870656374656420696e74656765722062757420676f742022312e3022"}
{"case":8,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
{"case":9,"before":"string","after":"string","code":1,"result":"626164206c6576656c20223231343734383336343822"}
{"case":10,"before":"string","after":"string","code":1,"result":"626164206c6576656c20223432393439363732393522"}
{"case":11,"before":"string","after":"string","code":0,"result":"32"}
{"case":12,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d65202262616422"}
{"case":13,"before":"wideInt","after":"wideInt","code":0,"result":"31"}
{"case":14,"before":"double","after":"double","code":1,"result":"657870656374656420696e74656765722062757420676f742022312e3022"}
{"case":15,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520226e616e22"}
```

### tcl8.5

Status: `observed`. Channel: Original native objects through Tcl_EvalObjv. Version: tcl8.5 8.5.19.

Exactly 16 completed observations. Fields case/before/after/code/result retain the original object and native result bytes.

```jsonl
{"case":0,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":1,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":2,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":3,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":4,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222d3122"}
{"case":5,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222b3122"}
{"case":6,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":7,"before":"string","after":"string","code":1,"result":"657870656374656420696e74656765722062757420676f742022312e3022"}
{"case":8,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
{"case":9,"before":"string","after":"levelReference","code":1,"result":"626164206c6576656c20223231343734383336343822"}
{"case":10,"before":"string","after":"levelReference","code":1,"result":"626164206c6576656c20223432393439363732393522"}
{"case":11,"before":"string","after":"levelReference","code":0,"result":"32"}
{"case":12,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d65202262616422"}
{"case":13,"before":"int","after":"int","code":0,"result":"31"}
{"case":14,"before":"double","after":"double","code":1,"result":"657870656374656420696e74656765722062757420676f742022312e3022"}
{"case":15,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
```

### tcl8.6

Status: `observed`. Channel: Original native objects through Tcl_EvalObjv. Version: tcl8.6 8.6.18.

Exactly 16 completed observations. Fields case/before/after/code/result retain the original object and native result bytes.

```jsonl
{"case":0,"before":"string","after":"int","code":0,"result":"31"}
{"case":1,"before":"string","after":"int","code":0,"result":"31"}
{"case":2,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":3,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":4,"before":"string","after":"int","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222d3122"}
{"case":5,"before":"string","after":"int","code":0,"result":"31"}
{"case":6,"before":"string","after":"int","code":0,"result":"31"}
{"case":7,"before":"string","after":"double","code":1,"result":"626164206c6576656c2022312e3022"}
{"case":8,"before":"string","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
{"case":9,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223231343734383336343822"}
{"case":10,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223432393439363732393522"}
{"case":11,"before":"string","after":"int","code":0,"result":"32"}
{"case":12,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d65202262616422"}
{"case":13,"before":"int","after":"int","code":0,"result":"31"}
{"case":14,"before":"double","after":"double","code":1,"result":"626164206c6576656c2022312e3022"}
{"case":15,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original native objects through Tcl_EvalObjv. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exactly 16 completed observations. Fields case/before/after/code/result retain the original object and native result bytes.

```jsonl
{"case":0,"before":"string","after":"int","code":0,"result":"31"}
{"case":1,"before":"string","after":"int","code":0,"result":"31"}
{"case":2,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":3,"before":"string","after":"levelReference","code":0,"result":"31"}
{"case":4,"before":"string","after":"int","code":1,"result":"626164206c6576656c20222d3122"}
{"case":5,"before":"string","after":"int","code":0,"result":"31"}
{"case":6,"before":"string","after":"int","code":0,"result":"31"}
{"case":7,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d652022312e3022"}
{"case":8,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
{"case":9,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223231343734383336343822"}
{"case":10,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223432393439363732393522"}
{"case":11,"before":"string","after":"int","code":0,"result":"32"}
{"case":12,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d65202262616422"}
{"case":13,"before":"int","after":"int","code":0,"result":"31"}
{"case":14,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d652022312e3022"}
{"case":15,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
```

### jim

Status: `observed`. Channel: Original native Jim objects through retained public argv/evaluation API. Version: jim 0.84 label.

Exactly 21 observations; process exit 0. The Jim roster differs explicitly from the C roster.

```jsonl
{"case":0,"before":"string","after":"int","code":0,"result":"31"}
{"case":1,"before":"string","after":"int","code":0,"result":"31"}
{"case":2,"before":"string","after":"string","code":0,"result":"31"}
{"case":3,"before":"string","after":"string","code":0,"result":"31"}
{"case":4,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222d3122"}
{"case":5,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520222b3122"}
{"case":6,"before":"string","after":"int","code":0,"result":"31"}
{"case":7,"before":"string","after":"string","code":1,"result":"626164206c6576656c2022312e3022"}
{"case":8,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
{"case":9,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223231343734383336343822"}
{"case":10,"before":"string","after":"int","code":1,"result":"626164206c6576656c20223432393439363732393522"}
{"case":11,"before":"string","after":"int","code":0,"result":"32"}
{"case":12,"before":"string","after":"string","code":1,"result":"696e76616c696420636f6d6d616e64206e616d65202262616422"}
{"case":13,"before":"string","after":"string","code":1,"result":"626164206c6576656c202223312022"}
{"case":14,"before":"string","after":"string","code":1,"result":"626164206c6576656c2022233932323333373230333638353437373538303822"}
{"case":15,"before":"string","after":"string","code":1,"result":"626164206c6576656c2022232d30783830303030303030303030303030303022"}
{"case":16,"before":"string","after":"string","code":0,"result":"30"}
{"case":17,"before":"string","after":"string","code":0,"result":"31"}
{"case":18,"before":"int","after":"int","code":0,"result":"31"}
{"case":19,"before":"double","after":"double","code":1,"result":"626164206c6576656c2022312e3022"}
{"case":20,"before":"double","after":"double","code":1,"result":"696e76616c696420636f6d6d616e64206e616d6520224e614e22"}
```

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `original_frame_selectors_match_native_cache_and_failure_order`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::original_frame_selectors_match_native_cache_and_failure_order` (linked): Compares original level-object cache and failure order to the retained release-specific rows.
- [runtime/rust/src/cmd_eval/native_frame_reference_tests.rs](../../../../runtime/rust/src/cmd_eval/native_frame_reference_tests.rs), `cmd_eval::native_frame_reference_tests::original_frame_level_uses_entered_environment_and_preserves_native_counts` (linked): Finite source-defined control: original frame level uses entered environment and preserves native counts. Software operation-owner control; existing provider recordings remain independent. An independently bound C-API scalar issuer does not bind a portable levelReference to that issuer's interpreter or lifetime. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Evidence

- [rust/tcl-syntax/tests/data/native_frame_levels](../../../../rust/tcl-syntax/tests/data/native_frame_levels)
