# naming.variable.namespace-upvar-target-order-and-incarnation-execution

Kind: `native-observation`

## Problem statement

A namespace can be destroyed/recreated between original alias operands, or lookup can fail after a target substitution already ran. Compiler acceptance alone cannot determine whether a local alias selects the new namespace incarnation, rejects a defined destination or preserves an earlier side effect.

## Question

What caught result bytes and visited side effect occur in the eight original C8.5 namespace-upvar execution controls?

## Conclusion

The eight C8.5 controls retain normal X/X-Y/empty-local results, NEW from the recreated namespace, defined-local and array-shaped-local errors, and visited1 before missing-namespace failure. This is execution order/incarnation evidence for those exact scripts, independent of compiler table identity, current arbitrary alias currency or unknown caller frames.

## Scope

Eight original ASCII C8.5.19 procedure body executions described in provenance.json and execution_cases.rs, including caught completion/result hex and visited status. Native shell binary SHA retained; original presenter and complete source wrapper are not. Other C versions, Jim and BIG-IP not queried for this separate execution subset.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Shell SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Channel: ASCII original procedures through native caught-result presenter. Dialect: Tcl.

All eight presenter processes exit0; guest completion codes remain individually caught.

```json
[
  {
    "name": "simple-result",
    "body": "namespace upvar ::N x local; set local",
    "exit": 0,
    "output": "0 58\nvisited 0\n",
    "source_sha256": "1135000d53b42389333a6e1cff643aa9a734e9b77ae23d84dd5f56774d65771b"
  },
  {
    "name": "original-namespace",
    "body": "namespace upvar [set ns ::N] [set name x] local; set local",
    "exit": 0,
    "output": "0 58\nvisited 0\n",
    "source_sha256": "0ebd24416c5f6c65d5c0ce807be14113757789d0672c5aac3a6b88a6d94534d3"
  },
  {
    "name": "multiple-pairs",
    "body": "namespace upvar ::N x first y second; list $first $second",
    "exit": 0,
    "output": "0 582059\nvisited 0\n",
    "source_sha256": "90ab4b00b82942c824844c982ea773c7ac2059dd145392c1f5e532daed7c340f"
  },
  {
    "name": "empty-local",
    "body": "namespace upvar ::N x {}; set {}",
    "exit": 0,
    "output": "0 58\nvisited 0\n",
    "source_sha256": "c1d746ce01a7ff6c46ad6b43327aff53ce33dd77785278461aa80094278d32d4"
  },
  {
    "name": "recreated-namespace",
    "body": "namespace upvar ::N x first [namespace delete ::N; namespace eval ::N {variable y NEW}; set name y] second; set second",
    "exit": 0,
    "output": "0 4e4557\nvisited 0\n",
    "source_sha256": "e79968bbc89ba9a2cc610d51c5cc56457fa9f3748e4b84345e45c40996bcc760"
  },
  {
    "name": "defined-local",
    "body": "set local BEFORE; namespace upvar ::N x local",
    "exit": 0,
    "output": "1 7661726961626c6520226c6f63616c2220616c726561647920657869737473\nvisited 0\n",
    "source_sha256": "9f82a98b453f272229cb31bfca4eea3dd97cb580188997324260d46a03b64121"
  },
  {
    "name": "array-local-generic",
    "body": "namespace upvar ::N x a(k)",
    "exit": 0,
    "output": "1 626164207661726961626c65206e616d65202261286b29223a2063616e2774206372656174652061207363616c6172207661726961626c652074686174206c6f6f6b73206c696b6520616e20617272617920656c656d656e74\nvisited 0\n",
    "source_sha256": "defddaff9cc81e50629efadb08e0c8744dbe2a98fc6024998eebda2393f3da5a"
  },
  {
    "name": "missing-namespace-after-target",
    "body": "namespace upvar ::MISSING [set ::visited YES; set name x] local",
    "exit": 0,
    "output": "1 6e616d65737061636520223a3a4d495353494e4722206e6f7420666f756e64\nvisited 1\n",
    "source_sha256": "0c25c16b15d1a70bbe3aa58967e27ed178cb6483f418fa202b541d395f40072a"
  }
]
```

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/provenance.json). SHA-256 `fc63dd3a41039235ef22369e3c229029e0cafc64a621536ee62340ab17923b6f`. Original C8.5 caught execution body/result/visited and wrapper-source hashes.
- `e1` (input): [rust/tcl-registry/tests/data/native_namespace_upvar_compilation/execution_cases.rs](../../../../rust/tcl-registry/tests/data/native_namespace_upvar_compilation/execution_cases.rs). SHA-256 `a6e9f2dd5461cc1e40cddb327c89e54e6e0269b6aafcc7933f374f1d16bddae0`. Exact current eight body/code/result/visited tuples copied from the original execution capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `original_c85_namespace_upvar_artifact_matches_original_namespace_lifetimes`: Independent current Rust comparison at the stated semantic boundary.
- [runtime/rust/src/interp/native_body_artifact/namespace_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/namespace_tests.rs), `interp::native_body_artifact::namespace_tests::original_c85_namespace_upvar_artifact_matches_original_namespace_lifetimes` (linked): Checks alias/error/order against original C8.5 execution tuples without borrowing compiler presence as a success certificate.

A named test is a coverage binding, not a claim that it executed.

## Replay

The original setup/catch/presenter wrappers are not retained; execution_cases.rs contains exact procedure body/result tuples, not an executable Tcl script. Fresh replay must recreate the explicitly documented namespace N with x X/y Y, retain the complete source wrapper and original p invocation, and independently catch/result-hex/visited each body. Compare all eight native tuples and record actual build/channel/status.
