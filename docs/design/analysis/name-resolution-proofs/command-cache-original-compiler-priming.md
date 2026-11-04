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

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.4.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","1","1","1"],["1","cmdName","1","cmdName","0","1","1"],["2","cmdName","1","cmdName","0","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","1","1","1"]].

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.5.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","1","1","1"],["1","cmdName","1","cmdName","0","1","1"],["2","cmdName","1","cmdName","0","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","1","1","1"]].

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl8.6.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","cmdName","1","cmdName","2","1","1"],["2","cmdName","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl9.0.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","none","1","cmdName","2","1","1"],["2","int","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C private original-object TclSetCmdNameObj ABI. Dialect: tcl9.1.

Observations [case,before type,before resident,after type,target,after resident,same original pointer]: [["0","cmdName","1","cmdName","2","1","1"],["1","none","1","cmdName","2","1","1"],["2","int","1","cmdName","2","1","1"],["3","cmdName","1","cmdName","1","1","1"],["4","cmdName","1","cmdName","2","1","1"]].

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Historical references

- `probe` (input): [runtime/rust/tests/data/native_command_cache_priming/probe.c](../../../../runtime/rust/tests/data/native_command_cache_priming/probe.c). Exact original object constructors, Tcl_GetCommandFromObj calls, private TclSetCmdNameObj priming and cache target sampler.
- `receipt` (provider): [runtime/rust/tests/data/native_command_cache_priming/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_priming/manifest.json). Original release/build/process metadata and output digests.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.4.20.tsv). Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.5.19.tsv). Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_command_cache_priming/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/8.6.18.tsv). Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_command_cache_priming/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/9.0.4.tsv). Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_command_cache_priming/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_priming/9.1.0.tsv). Cases 0–4 in retained order; target 1=A, 2=B, 0=null. Original type/residency is sampled before the second priming call.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/obj.rs](../../../../runtime/rust/src/obj.rs), `prime_native_command_name_cache`: Applies the separately selected compiler priming recipe to an original object; it is not an independently issued live lookup-world receipt.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_priming_matches_every_native_original_object_row` (linked): Compares all twenty-five original before/after primary, target and resident-pointer windows under the selected release.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Exact retained probe/output hashes match original metadata. Private cache layout and TclSetCmdNameObj must be compiled against each selected release; original capture paths alone are not a current runnable protocol.
