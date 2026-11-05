# naming.bigip.variable-static-event-currency

Kind: `native-observation`

## Problem statement

A static initialization seen on all TMMs can be mistaken for a broadcast of later event mutations. The consumer needs actual responding-unit evidence and separate missing/read/recreate results.

## Question

What do the mapped per-unit lifecycle and static-cell controls observe after target-unit write and unset/recreation?

## Conclusion

RULE_INIT and CLIENT_ACCEPTED reach the recorded active scalar control. The static seed is seen on all four actual units; write and unset/recreation on0:2 leave the other three seeds intact. The missing read before recreation is preserved. This proves only this rule/build and observed units.

## Scope

Exact mapped source tuples and per-unit response/log evidence; no chassis-wide or unobserved-TMM claim. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Exact mapped source tuples and per-unit response/log evidence; no chassis-wide or unobserved-TMM claim. Version: bigip 21.1.0.1.

RULE_INIT and CLIENT_ACCEPTED reach the recorded active scalar control. The static seed is seen on all four actual units; write and unset/recreation on0:2 leave the other three seeds intact. The missing read before recreation is preserved. This proves only this rule/build and observed units.

The measured report rows and their limits are:

The representative active scalar control completed with identical result
bytes and `read unset` order in RULE_INIT once on each of `0:0`–`0:3`, then in
CLIENT_ACCEPTED on one mapped connection per unit. The complete direct markers
are in
`vl-context-events.log`.

The `static::` rule logged a `SEED` initialization from every actual unit. The
following traffic then used the same four stable source tuples:

| Measured configuration | `0:0` | `0:1` | `0:2` | `0:3` |
| --- | --- | --- | --- | --- |
| initial read | `SEED` | `SEED` | `SEED` | `SEED` |
| after write on `0:2` | `SEED` | `SEED` | `MARK_UNIT` | `SEED` |
| after unset/recreate on `0:2` | `SEED` | `SEED` | `RECREATED` | `SEED` |

The unset request on `0:2` observed catch 1 with exact message
`can't read "static::__tcl_lsp_2286_vl1_cell": no such variable` before
recreation. Thus the measured `static::` storage is per TMM for this rule and
version: neither mutation nor unset/recreation broadcast to the other three
units. The raw sequence is the `vl-static-*.body` set in
[`client-server`](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex/client-server), and the four
initialization markers are in the continuous LTM capture.

## Consumer bindings

- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::install_irules_static_simulation`: Install the explicit authored RuleInitPublication policy independently of physical Native object provider and source grammar.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::publish_authored_static`: Publish only the selected authored initialization context across live enrolled worker namespaces; ordinary executing-worker event mutations remain in their actual worker.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `Vm::apply_static_publication`: Apply explicit authored initialization publication through existing real worker namespace cells and preserve the policy's existing recipient operation semantics. The retained appliance responses constrain only the named software state regressions.
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs), `interp::authored_tmm_static::tests::authored_static_policy_matches_mapped_unit_missing_read_before_recreation` (linked): Explicit authored static publication software policy reads twelve stable-tuple per-unit response bodies and the retained target-unit unset response. Four genuine enrolled VM interpreters compare SEED/MARK_UNIT/RECREATED locality and the exact caught missing-read result before recreation. Coverage is limited to these static-cell state and missing-result observations; the recorded RULE_INIT/CLIENT_ACCEPTED scalar trace chronology is separate.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md)
- [rust/tcl-vm/src/interp/authored_tmm_static.rs](../../../../rust/tcl-vm/src/interp/authored_tmm_static.rs)
