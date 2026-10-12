# absolute level cache reselects current frame

ID: `naming.variable.absolute-level-cache-reselects-current-frame`

## Problem statement

A levelReference cache can outlive a selected frame, encounter a replacement at the same level, or be reused in a different interpreter. Treating its cached type or original numeric level as a persistent frame identity would follow a retired/foreign activation.

## Question

For one retained #1 object, which frame/result/cache and type-hook fields appear before a frame exists, after push/pop/replacement, in another interpreter and after duplication plus original release?

## Answers

### tcl8.4 — observed

Final compile/run both exit 0 with empty runtime stderr. First attempt also completed with exit 0.

```tsv
missing	-1	-1	none	1	1	0	0	0	0
first-frame	1	1	none	1	1	0	0	0	0
retired-frame	-1	-1	none	1	1	0	0	0	0
replacement-frame	1	1	none	1	1	0	0	0	0
other-interpreter	1	1	none	1	1	0	0	0	0
duplicate-after-original-release	1	1	none	1	1	0	0	0	0
```

Version: 8.4.20. Build: Archive SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; tclInt.h SHA 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; probe SHA bd0fa65d6d8203327a08ac4a7bfcad0f46b61169f118a0d4a46c21b10b5ab257. Input channel: Original Tcl_NewStringObj(#1,2), private frame API.

### tcl8.5 — observed

Final compile/run both exit 0 with empty runtime stderr. First attempt also completed with exit 0.

```tsv
missing	-1	-1	levelReference	1	1	0	0	0	0
first-frame	1	1	levelReference	1	1	0	0	0	0
retired-frame	-1	-1	levelReference	1	1	0	0	0	0
replacement-frame	1	1	levelReference	1	1	0	0	0	0
other-interpreter	1	1	levelReference	1	1	0	0	0	0
duplicate-after-original-release	1	1	levelReference	1	1	0	0	0	0
```

Version: 8.5.19. Build: Archive SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; tclInt.h SHA c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; probe SHA 9e61f6642791564d6ce8ac6c9c116b0a67aa96047897c6f2a147a357d3564f87. Input channel: Original Tcl_NewStringObj(#1,2), private frame API.

### tcl8.6 — observed

Final compile/run both exit 0 with empty runtime stderr. First attempt failed linking with unresolved zlib references; no runtime observation from that attempt.

```tsv
missing	-1	-1	levelReference	1	1	0	0	0	0
first-frame	1	1	levelReference	1	1	0	0	0	0
retired-frame	-1	-1	levelReference	1	1	0	0	0	0
replacement-frame	1	1	levelReference	1	1	0	0	0	0
other-interpreter	1	1	levelReference	1	1	0	0	0	0
duplicate-after-original-release	1	1	levelReference	1	1	0	0	0	0
```

Version: 8.6.18. Build: Archive SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; tclInt.h SHA aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; probe SHA 99da56f6ad2e3c6b12ef18c9f4ef23faa65728f050415154659f99b2b37303b9. Input channel: Original Tcl_NewStringObj(#1,2), private frame API.

### tcl9.0 — observed

Final compile/run both exit 0 with empty runtime stderr. First attempt failed linking with unresolved zlib references; no runtime observation from that attempt.

```tsv
missing	-1	-1	levelReference	1	1	0	0	0	0
first-frame	1	1	levelReference	1	1	0	0	0	0
retired-frame	-1	-1	levelReference	1	1	0	0	0	0
replacement-frame	1	1	levelReference	1	1	0	0	0	0
other-interpreter	1	1	levelReference	1	1	0	0	0	0
duplicate-after-original-release	1	1	levelReference	1	1	0	0	0	0
```

Version: 9.0.4. Build: Archive SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; tclInt.h SHA eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; probe SHA 7a7cb57adc705adba367d662e15622596f519e08a02562b9f04204518e46837b. Input channel: Original Tcl_NewStringObj(#1,2), private frame API.

### tcl9.1 — observed

Final compile/run both exit 0 with empty runtime stderr. First attempt failed linking with unresolved zlib references; no runtime observation from that attempt.

```tsv
missing	-1	-1	levelReference	1	1	0	0	0	0
first-frame	1	1	levelReference	1	1	0	0	0	0
retired-frame	-1	-1	levelReference	1	1	0	0	0	0
replacement-frame	1	1	levelReference	1	1	0	0	0	0
other-interpreter	1	1	levelReference	1	1	0	0	0	0
duplicate-after-original-release	1	1	levelReference	1	1	0	0	0	0
```

Version: 9.1.0. Build: Archive SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; tclInt.h SHA 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; probe SHA ffc9f8a32d92a4eaf77e5c800b8f2cc6227fe0568e326070907b3945cc790ef2. Input channel: Original Tcl_NewStringObj(#1,2), private frame API.

### jim — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.


## Conclusion

All five successful final probes report missing and retired frame selection as -1, and select level 1 after first push, replacement, other-interpreter push and duplication. C8.4 retains no levelReference cache; C8.5+ retains levelReference with resident spelling and no four type hooks in these six windows. This proves re-selection behaviour for this object sequence, not enduring frame identity or variable-slot ownership.

## Scope

Thirty completed final private TclGetFrame/TclObjGetFrame windows, C8.4.20–9.1.0, original counted #1 object. The first attempt separately completed C8.4/C8.5 but failed linking C8.6/C9 because of unresolved zlib dependencies; those failed attempts have no runtime observations. Jim and BIG-IP not queried.

## Retained evidence

- `rust/tcl-syntax/tests/data/native_frame_reference/manifest.json` SHA256 `47807f08736e5e0108ed9f1e09278d1d4d264248ae4607b869aeb2c4be0d5edd`: Complete final compile/run argv, headers/archive/binary hashes and stdout/stderr hex.
- `rust/tcl-syntax/tests/data/native_frame_reference/attempt1-manifest.json` SHA256 `6a8b3a4a1beab074e6c3c2fcc0a2c98e12935e75b06d4ca710a2c3eb0e58843b`: First attempt: two completed providers and three actual failed-link stderr records, without runtime rows for failed links.
- `rust/tcl-syntax/tests/data/native_frame_reference/probe.c` SHA256 `68d3b6f998be2fc11e441346ea40593edf5e48e64fabe4748d6d5b85b084e2bb`: Exact retained object/push/pop/replacement/foreign-interpreter/duplicate sequence.
- `rust/tcl-syntax/tests/data/native_frame_reference/8.4.20.tsv` SHA256 `55dfa04de710719ffd1adfc87b0fc9452a414f852f185c5d83b67d737b31e124`: Six original frame selection, cache residency/refcount and four type-hook presence windows.
- `rust/tcl-syntax/tests/data/native_frame_reference/8.5.19.tsv` SHA256 `0db9c43dbd534632e8d1f0a29f3dda631e8fe33f62c808f398642f644fe4552a`: Six original frame selection, cache residency/refcount and four type-hook presence windows.
- `rust/tcl-syntax/tests/data/native_frame_reference/8.6.18.tsv` SHA256 `0db9c43dbd534632e8d1f0a29f3dda631e8fe33f62c808f398642f644fe4552a`: Six original frame selection, cache residency/refcount and four type-hook presence windows.
- `rust/tcl-syntax/tests/data/native_frame_reference/9.0.4.tsv` SHA256 `0db9c43dbd534632e8d1f0a29f3dda631e8fe33f62c808f398642f644fe4552a`: Six original frame selection, cache residency/refcount and four type-hook presence windows.
- `rust/tcl-syntax/tests/data/native_frame_reference/9.1.0.tsv` SHA256 `0db9c43dbd534632e8d1f0a29f3dda631e8fe33f62c808f398642f644fe4552a`: Six original frame selection, cache residency/refcount and four type-hook presence windows.

## Replay

```sh
cc -DSTDC_HEADERS=1 -DHAVE_UNISTD_H=1 -I/path/to/recorded/generic -I/path/to/recorded/unix rust/tcl-syntax/tests/data/native_frame_reference/probe.c /path/to/recorded/libtcl.a -lm -ldl -lpthread -lz -o /tmp/native-variable-proof
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses private interpreter/frame/compiler headers; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred.

## Rust comparisons

- `cmd_eval::native_frame_reference_tests::level_reference_reselects_current_frames_in_thirty_native_windows` in `runtime/rust/src/cmd_eval/native_frame_reference_tests.rs`: Checks re-selection rather than persistence across the thirty original windows. No execution result is recorded here.
- `exec::native_uplevel_tests::level_reference_reselects_current_frames_in_thirty_native_windows` in `rust/tcl-vm/src/exec/native_uplevel_tests.rs`: Checks the independently scoped VM frame-cache path. No execution result is recorded here.
