# original level object conversion and selection

ID: `naming.variable.original-level-object-conversion-and-selection`

## Problem statement

The same uplevel level spelling can be an original String, WideInt or Double, and an embedded zero can alter a CString conversion without changing the counted object. Release-specific conversion/cache order determines whether the word selects a caller frame or becomes script text; a source numeric grammar alone cannot determine those object effects.

## Question

At current procedure level 2, what original object-cache change, completion code and result bytes occur for the 16 C and independently specified 21 Jim original level operands?

## Answers

### tcl8.4 — observed

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

Version: 8.4.20. Build: Original archive SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Input channel: Original native objects through Tcl_EvalObjv.

### tcl8.5 — observed

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

Version: 8.5.19. Build: Original archive SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Input channel: Original native objects through Tcl_EvalObjv.

### tcl8.6 — observed

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

Version: 8.6.18. Build: Original archive SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Input channel: Original native objects through Tcl_EvalObjv.

### tcl9.0 — observed

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

Version: 9.0.4. Build: Original archive SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Input channel: Original native objects through Tcl_EvalObjv.

### tcl9.1 — observed

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

Version: 9.1.0. Build: Original archive SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Input channel: Original native objects through Tcl_EvalObjv.

### jim — observed

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

Version: 0.84 label; exact revision not recorded in this manifest. Build: Original library SHA 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; probe SHA d5eaecb0fb20b6e26d3e1469e26cb51fafdf47cbee9615e97ddaada9bec03651. Input channel: Original native Jim objects through retained public argv/evaluation API.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.


## Conclusion

The retained 80 C and 21 Jim rows distinguish original String, embedded-zero, native integer and Double selector routes. C8.4 preserves the original cache in these cases; newer C releases and Jim have independently measured cache/validation order. The complete per-provider rows are the answer: they do not certify another activation, arbitrary cached objects, variable read completion or a native frame token.

## Scope

Exact retained probe.c/jim-probe.c object constructors and two procedure activations, C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-labelled capture. C inputs include 13 counted String spellings plus native WideInt/Double/NaN; Jim has 18 String spellings plus native Int/Double/NaN. Public object argv evaluation, no character-channel translation; BIG-IP not queried.

## Retained evidence

- `rust/tcl-syntax/tests/data/native_frame_levels/manifest.json` SHA256 `16f09e74eae120f9e1f131612052ebf3736ad8aadc0cd5bb6cff062dffd097ef`: Five original C library and complete output hashes; 16 windows per engine.
- `rust/tcl-syntax/tests/data/native_frame_levels/jim-manifest.json` SHA256 `49fb08a00bd9832ac668506753d4eee30332a1535539ccc0c09c75c88d9e0b42`: Original Jim library/binary/output hashes, 21 windows and process exit.
- `rust/tcl-syntax/tests/data/native_frame_levels/probe.c` SHA256 `cadd230a808b1d4932c457633110ae3cbd9cf98b38791d0e5b53002a8954d502`: Exact original C object constructors/argv/procedure activation probe.
- `rust/tcl-syntax/tests/data/native_frame_levels/jim-probe.c` SHA256 `88735df6f48baeda381918f6cfd32cfa6a4bd0e8a45a336d17ff68ce1de2aa20`: Exact independent Jim object constructors and activation probe.
- `rust/tcl-syntax/tests/data/native_frame_levels/8.4.20.jsonl` SHA256 `0549ad40b61857df057f28ce4d7af585e284e7ad1857eaf7af82f40d3c6cba2d`: All 16 original before/after cache, completion and result-hex windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/8.5.19.jsonl` SHA256 `88edaf514ebfa5266e4d3aca2f7615fcbc106a5444b32f4fe661b5ed59f9dd0d`: All 16 original before/after cache, completion and result-hex windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/8.6.18.jsonl` SHA256 `c5244d86a60462c341631fa4d4f6eafc5455ea984c8432b66251f80c180404ad`: All 16 original before/after cache, completion and result-hex windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/9.0.4.jsonl` SHA256 `b9fa336b681e6e8eea48358038eb0917c1ef7391a2c97e03a6cd5109ea39eab5`: All 16 original before/after cache, completion and result-hex windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/9.1.0.jsonl` SHA256 `b9fa336b681e6e8eea48358038eb0917c1ef7391a2c97e03a6cd5109ea39eab5`: All 16 original before/after cache, completion and result-hex windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/jim0.84.jsonl` SHA256 `a80131c7003b8b881a42f876efef2daff6ed088f49128f7ef86045eaae0c79ba`: All 21 independent original Jim cache/completion/result windows.
- `rust/tcl-syntax/tests/data/native_frame_levels/README.md` SHA256 `7df70505dd35afcbb5e6618138ff6e1368724ca5e2034ab421ec9490c0e0df82`: Separate C and Jim compile/replay protocols and bounded measurement explanation.

## Replay

```sh
cc -DSTDC_HEADERS=1 -DHAVE_UNISTD_H=1 -I/path/to/recorded/generic -I/path/to/recorded/unix rust/tcl-syntax/tests/data/native_frame_levels/probe.c /path/to/recorded/libtcl.a -lm -ldl -lpthread -lz -o /tmp/native-variable-proof
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses public object/evaluation entry points; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred. For Jim separately compile jim-probe.c against the recorded configured Jim library and its actual dependencies, then compare every jim0.84.jsonl row as documented in the retained README.

## Rust comparisons

- `command::tests::original_frame_selectors_match_native_cache_and_failure_order` in `rust/tcl-vm/src/command.rs`: Compares original level-object cache and failure order to the retained release-specific rows. No execution result is recorded here.
