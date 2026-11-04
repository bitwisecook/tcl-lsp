# naming.numeric.octal-boolean-frontier

Kind: `native-observation`

## Problem statement

Invalid-octal wording depends on the reached primitive numeric state, not a generic string prefix. Raw-zero, sign, radix and trailing text controls must remain separate from expression parsing.

## Question

For twelve original sign/radix/invalid-octal spellings, what boolean primitive/evaluated diagnostic and cache/errorCode frontier is reached?

## Conclusion

The twelve actual boolean rows per release retain the exact invalid-octal/invalid-value/result differences, including source forms that fail before an implicit-octal interpretation and the counted raw-zero suffix. Only this selected primitive grammar and its propagated error are measured; no complete numeric parser equivalence is inferred.

## Scope

C public original-object boolean getter is reached by one native callback from ASCII Tcl_Eval. Exact source bytes and selected storage/priming axes are retained; primitive result is duplicated before the callback returns, and final result/cache/errorCode/original input bytes are then sampled. Twelve fresh String cases, twelve rows per selected getter. Capture source/library/executable/row count/run0 are recorded; original C output checksums, headers/compiler status/configure/runtime patch query were not recorded by these receipts. Retained streams have current permanent evidence hashes. BIG-IP is not tested; Jim is only measured for seeded fresh Strings.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=2 storage=0 case=0 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822 errorCode=4e4f4e45 input=303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822
getter=2 storage=0 case=1 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822 errorCode=4e4f4e45 input=30382e3078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822
getter=2 storage=0 case=2 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822 errorCode=4e4f4e45 input=306f38 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822
getter=2 storage=0 case=3 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822 errorCode=4e4f4e45 input=306f3738 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822
getter=2 storage=0 case=4 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722 errorCode=4e4f4e45 input=307847 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722
getter=2 storage=0 case=5 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222 errorCode=4e4f4e45 input=306232 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222
getter=2 storage=0 case=6 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822 errorCode=4e4f4e45 input=2d203038 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822
getter=2 storage=0 case=7 code=0 cache=boolean message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=2 storage=0 case=8 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822 errorCode=4e4f4e45 input=3030303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822
getter=2 storage=0 case=9 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822 errorCode=4e4f4e45 input=30386558 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822
getter=2 storage=0 case=10 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822 errorCode=4e4f4e45 input=3038652b3178 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822
getter=2 storage=0 case=11 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d303922 errorCode=4e4f4e45 input=092d30390078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d303922
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=2 storage=0 case=0 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=1 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822
getter=2 storage=0 case=2 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822
getter=2 storage=0 case=3 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822
getter=2 storage=0 case=4 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722
getter=2 storage=0 case=5 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222
getter=2 storage=0 case=6 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822
getter=2 storage=0 case=7 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=2b303920 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=8 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=9 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822
getter=2 storage=0 case=10 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822
getter=2 storage=0 case=11 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=092d30390078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=2 storage=0 case=0 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=1 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822
getter=2 storage=0 case=2 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822
getter=2 storage=0 case=3 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822
getter=2 storage=0 case=4 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722
getter=2 storage=0 case=5 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222
getter=2 storage=0 case=6 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222d20303822
getter=2 storage=0 case=7 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=2b303920 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=8 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230303038782220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=2 storage=0 case=9 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822
getter=2 storage=0 case=10 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822
getter=2 storage=0 case=11 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=54434c2056414c5545204e554d424552 input=092d30390078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=2 storage=0 case=0 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822
getter=2 storage=0 case=1 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822
getter=2 storage=0 case=2 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822
getter=2 storage=0 case=3 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822
getter=2 storage=0 case=4 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722
getter=2 storage=0 case=5 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222
getter=2 storage=0 case=6 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742061206c697374 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742061206c697374
getter=2 storage=0 case=7 code=0 cache=int message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=2 storage=0 case=8 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822
getter=2 storage=0 case=9 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822
getter=2 storage=0 case=10 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822
getter=2 storage=0 case=11 code=0 cache=int message= errorCode=4e4f4e45 input=092d30390078 primitive=
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=2 storage=0 case=0 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230387822
getter=2 storage=0 case=1 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382e307822
getter=2 storage=0 case=2 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f3822
getter=2 storage=0 case=3 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022306f373822
getter=2 storage=0 case=4 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230784722
getter=2 storage=0 case=5 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230623222
getter=2 storage=0 case=6 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742061206c697374 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742061206c697374
getter=2 storage=0 case=7 code=0 cache=int message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=2 storage=0 case=8 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f742022303030387822
getter=2 storage=0 case=9 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038655822
getter=2 storage=0 case=10 code=1 cache=string message=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420626f6f6c65616e2076616c75652062757420676f7420223038652b317822
getter=2 storage=0 case=11 code=0 cache=int message= errorCode=4e4f4e45 input=092d30390078 primitive=
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Historical references

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c). Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json). Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt). Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt). Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt). Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt). Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt). Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter/errors.rs](../../../../rust/tcl-syntax/src/scalar_getter/errors.rs), `NativeScalarGetterProtocol::failure_presentation`: Retains primitive versus evaluated diagnostic and selected error-code update independently from representation conversion.
- [rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs), `scalar_getter::tests::error_tests::invalid_octal_and_native_character_units_match_actual_releases` (linked): Compares only the exact selected primitive/propgated/error-code fields across retained rows; no native callback dispatch authority is supplied.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
