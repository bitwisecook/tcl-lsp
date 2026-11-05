# naming.numeric.octal-double-frontier

Kind: `native-observation`

## Problem statement

Invalid-octal wording depends on the reached primitive numeric state, not a generic string prefix. Raw-zero, sign, radix and trailing text controls must remain separate from expression parsing.

## Question

For twelve original sign/radix/invalid-octal spellings, what double primitive/evaluated diagnostic and cache/errorCode frontier is reached?

## Conclusion

The twelve actual double rows per release retain the exact invalid-octal/invalid-value/result differences, including source forms that fail before an implicit-octal interpretation and the counted raw-zero suffix. Only this selected primitive grammar and its propagated error are measured; no complete numeric parser equivalence is inferred.

## Scope

C public original-object double getter is reached by one native callback from ASCII Tcl_Eval. Exact source bytes and selected storage/priming axes are retained; primitive result is duplicated before the callback returns, and final result/cache/errorCode/original input bytes are then sampled. Twelve fresh String cases, twelve rows per selected getter. Capture source/library/executable/row count/run0 are recorded; original C output checksums, headers/compiler status/configure/runtime patch query were not recorded by these receipts. Retained streams have current permanent evidence hashes. BIG-IP is not tested; Jim is only measured for seeded fresh Strings.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
getter=1 storage=0 case=0 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230387822 errorCode=4e4f4e45 input=303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230387822
getter=1 storage=0 case=1 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822 errorCode=4e4f4e45 input=30382e3078 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822
getter=1 storage=0 case=2 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822 errorCode=4e4f4e45 input=306f38 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822
getter=1 storage=0 case=3 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822 errorCode=4e4f4e45 input=306f3738 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822
getter=1 storage=0 case=4 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722 errorCode=4e4f4e45 input=307847 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722
getter=1 storage=0 case=5 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222 errorCode=4e4f4e45 input=306232 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222
getter=1 storage=0 case=6 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222d20303822 errorCode=4e4f4e45 input=2d203038 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222d20303822
getter=1 storage=0 case=7 code=0 cache=double message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=1 storage=0 case=8 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022303030387822 errorCode=4e4f4e45 input=3030303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022303030387822
getter=1 storage=0 case=9 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822 errorCode=4e4f4e45 input=30386558 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822
getter=1 storage=0 case=10 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822 errorCode=4e4f4e45 input=3038652b3178 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822
getter=1 storage=0 case=11 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022092d303922 errorCode=4e4f4e45 input=092d30390078 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022092d303922
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
getter=1 storage=0 case=0 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=1 storage=0 case=1 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822
getter=1 storage=0 case=2 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822
getter=1 storage=0 case=3 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822
getter=1 storage=0 case=4 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722
getter=1 storage=0 case=5 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222
getter=1 storage=0 case=6 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222d20303822 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222d20303822
getter=1 storage=0 case=7 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=2b303920 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=1 storage=0 case=8 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=1 storage=0 case=9 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822
getter=1 storage=0 case=10 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822
getter=1 storage=0 case=11 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=092d30390078 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
getter=1 storage=0 case=0 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230387822
getter=1 storage=0 case=1 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230382e307822
getter=1 storage=0 case=2 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f3822
getter=1 storage=0 case=3 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022306f373822
getter=1 storage=0 case=4 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230784722
getter=1 storage=0 case=5 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f74202230623222
getter=1 storage=0 case=6 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742061206c697374 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742061206c697374
getter=1 storage=0 case=7 code=0 cache=int message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=1 storage=0 case=8 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f742022303030387822
getter=1 storage=0 case=9 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038655822
getter=1 storage=0 case=10 code=1 cache=string message=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420666c6f6174696e672d706f696e74206e756d6265722062757420676f7420223038652b317822
getter=1 storage=0 case=11 code=0 cache=int message= errorCode=4e4f4e45 input=092d30390078 primitive=
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter/errors.rs](../../../../rust/tcl-syntax/src/scalar_getter/errors.rs), `NativeScalarGetterProtocol::failure_presentation`: Retains primitive versus evaluated diagnostic and selected error-code update independently from representation conversion.
- [rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs), `scalar_getter::tests::error_tests::invalid_octal_and_native_character_units_match_actual_releases` (linked): Compares only the exact selected primitive/propgated/error-code fields across retained rows; no native callback dispatch authority is supplied.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt)
