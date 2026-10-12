# naming.command-cache.original-compiler-priming

Kind: `native-observation`

## Problem statement

Compiler priming can preserve an already cached target in older releases or overwrite it in later releases; an unresolved lookup may also leave cmdName, no primary or an integer primary. Reconstructing a target from a name label would lose these distinct original object states.

## Question

When TclSetCmdNameObj primes five original lookup/priming states, does it replace the cached target and preserve resident string identity?

## Conclusion

C8.4/8.5 retains an existing A target when primed with B; failed-name/integer lookups already have unresolved cmdName and remain null after priming. C8.6 replaces all B-selected targets, including unresolved cmdName. C9.0/9.1 failed-name and integer lookup retains none or int before priming, then becomes cmdName with B. Priming A with A keeps A in all releases. All twenty-five original resident string pointers remain unchanged. These are actual private compiler-priming windows, not proof that a later command-world hit is live.

## Scope

Five fixed original object states per C release: successful A lookup then B; failed string lookup then B; failed integer lookup then B; prime A then A; prime A then B. The probe samples the private command-name cache target and original bytes. Jim/BIG-IP are not tested, and no general absent-command or allocation-lifetime conclusion is drawn.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=84644f2265bced1a1ac3d846f7a98fcd290ebe7bb0d38f5c5ae6c5c1c3f7e92b; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.4.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","1","1","1"],["1","cmdName","1","cmdName","0","1","1"],["2","cmdName","1","cmdName","0","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","1","1","1"]].

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=ba24964a8be2930a468cd3afbd2949c287313933faa31a1f08e199a32bc5ed40; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.5.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","1","1","1"],["1","cmdName","1","cmdName","0","1","1"],["2","cmdName","1","cmdName","0","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","1","1","1"]].

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=78a6c980295e929894cad539dfac81384e14f2121a8fce75ca17a05d6843c28a; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.6.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","cmdName","1","cmdName","2","1","1"],["2","cmdName","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=ecfb078e85c63a91198cb78883db3e606a1a9b467e22a8bebc406ab510e70257; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl9.0.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","none","1","cmdName","2","1","1"],["2","int","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=5840d7f0fcbf81fe56ced18b8f85f61da81aabc65facb61caa805f3b3daf149a; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl9.1.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","none","1","cmdName","2","1","1"],["2","int","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Exact evidence

- `probe` (input): [runtime/rust/tests/data/native_command_cache_priming/probe.c](../../../../runtime/rust/tests/data/native_command_cache_priming/probe.c). SHA-256 `dee2b7cae10b26030cd072e61abf89d8562f12b9657ad5b2b2b566d1ed43517e`. Exact original object constructors, Tcl_GetCommandFromObj calls, private TclSetCmdNameObj priming and cache target sampler.
- `receipt` (provider): [runtime/rust/tests/data/native_command_cache_priming/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_priming/manifest.json). SHA-256 `565b7b78b8747f3c044b1c4ee1679b0cfc4437bea170ba628204ee2fc20adeeb`. Original release/build/process metadata and output digests.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.4.20.tsv). SHA-256 `4d86bf947b295d1e1b1af21f164f72571879fb6ba6badce07a9297730d1c5079`. Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.5.19.tsv). SHA-256 `4d86bf947b295d1e1b1af21f164f72571879fb6ba6badce07a9297730d1c5079`. Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.6.18.tsv). SHA-256 `4154073ee08d3bed1f0f81a2daed1faf0927241f70ccf20d12219711a7df1687`. Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_command_cache_priming/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/9.0.4.tsv). SHA-256 `f70ef5ea755dd3bab2ea4031df9b786e092532f8cadd91c2c68d507ab4280791`. Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_command_cache_priming/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/9.1.0.tsv). SHA-256 `f70ef5ea755dd3bab2ea4031df9b786e092532f8cadd91c2c68d507ab4280791`. Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/obj.rs](../../../../runtime/rust/src/obj.rs), `prime_native_command_name_cache`: Applies the separately selected compiler priming recipe to an original object; it is not an independently issued live lookup-world receipt.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_priming_matches_every_native_original_object_row` (linked): Compares all twenty-five original before/after primary, target and resident-pointer windows under the selected release.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Exact retained probe/output hashes match original metadata. Private cache layout and TclSetCmdNameObj must be compiled against each selected release; original capture paths alone are not a current runnable protocol.
