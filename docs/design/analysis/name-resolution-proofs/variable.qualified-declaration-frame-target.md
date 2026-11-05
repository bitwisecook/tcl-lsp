# Does top-level variable ::a::x 5 create an unqualified root alias, and what does the procedure-qualified declaration write?

Proof ID: `naming.variable.qualified-declaration-frame-target`

## Problem statement

A qualified variable declaration at root and the same declaration in a procedure can have different alias effects. Reusing the unchanged expectation that root x must exist would hide an engine-dependent target. The selected frame and each operation in the same interpreter determine which observation applies.

## Question

Does top-level variable ::a::x 5 create an unqualified root alias, and what does the procedure-qualified declaration write?

## Answers

| Provider | Status | Version/build | Answer |

|---|---|---|---|

| tcl8.4 | observed | not recorded beyond engine label tcl8.4; binary SHA 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4 | Root ::a::x returns5, root x read fails, qualified unset succeeds; procedure returns7. |

| tcl8.5 | observed | not recorded beyond engine label tcl8.5; binary SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935 | Root ::a::x returns5, root x read fails, qualified unset succeeds; procedure returns7. |

| tcl8.6 | observed | not recorded beyond engine label tcl8.6; binary SHA 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0 | Root ::a::x returns5, root x read fails, qualified unset succeeds; procedure returns7. |

| tcl9.0 | observed | not recorded beyond engine label tcl9.0; binary SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18 | Root ::a::x returns5, root x read fails, qualified unset succeeds; procedure returns7. |

| tcl9.1 | observed | not recorded beyond engine label tcl9.1; binary SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c | Root ::a::x returns5, root x read fails, qualified unset succeeds; procedure returns7. |

| jim | observed | not recorded beyond engine label jim; binary SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0 | Root ::a::x read fails, root x returns5, unsetting ::a::x fails; procedure returns7. |

| bigip | not-tested | not recorded; not recorded | No appliance observation for this question. |

## Conclusion

The five recorded C engines store root-level ::a::x without defining root x; Jim stores x and leaves ::a::x absent. A procedure local x follows the declared link and yields 7 in all six recorded runs. These exact scripts establish only this declaration/frame distinction.

## Scope

Exact retained ASCII script, root interpreter followed by procedure call; six recorded engine labels and binary hashes. Patch levels and Jim revision are not recorded by this manifest. No counted-NUL, dynamic callback, Native object/header or BIG-IP semantics are claimed.

## Exact retained evidence

- `manifest`: [runtime/rust/tests/data/native_variable_qualified_target/manifest.json](../../../../runtime/rust/tests/data/native_variable_qualified_target/manifest.json). Original operation list and per-engine complete stdout/status/hash.

- `source`: [runtime/rust/tests/data/native_variable_qualified_target/source.tcl](../../../../runtime/rust/tests/data/native_variable_qualified_target/source.tcl). Exact declared source operations in one interpreter.

## Rust contracts

Rust assertions and native observations are separate. No Rust execution result is inferred from this record.

## Exact native answers

### tcl8.4

```text
0 {}
0 {}
0 5
1 {can't read "x": no such variable}
0 {}
procedure 0 7
```

### tcl8.5

```text
0 {}
0 {}
0 5
1 {can't read "x": no such variable}
0 {}
procedure 0 7
```

### tcl8.6

```text
0 {}
0 {}
0 5
1 {can't read "x": no such variable}
0 {}
procedure 0 7
```

### tcl9.0

```text
0 {}
0 {}
0 5
1 {can't read "x": no such variable}
0 {}
procedure 0 7
```

### tcl9.1

```text
0 {}
0 {}
0 5
1 {can't read "x": no such variable}
0 {}
procedure 0 7
```

### jim

```text
0 {}
0 {}
1 {can't read "::a::x": no such variable}
0 5
1 {can't unset "::a::x": no such variable}
procedure 0 7
```

## Linked Rust assertion

- `cmd_var::tests::variable_qualified_target` in `runtime/rust/src/cmd_var.rs`: Exact native script root/procedure distinction is compared through authentic engine constructors.
