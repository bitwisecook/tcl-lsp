# naming.bigip.variable-read-destroy-recreate

Kind: `native-observation`

## Problem statement

A read callback can destroy and recreate its receiver before a command writes its result. Retaining the prior cell or trace registration would yield a different result and callback order.

## Question

Which cell and registration does the exact scalar/array lifecycle discriminator use after read-triggered unset/recreation?

## Conclusion

Modern and legacy runtime trace APIs install and complete on each recorded TMM. The main result is {NEW EXTRA} {NEW EXTRA}, with read then unset, exact name arguments, and no retained registration after recreation. The later ordinary-global controls are distinct and do not prove arbitrary cross-TMM sharing.

## Scope

The exact main-supported dynamic program on0:0 through0:3; literal trace/proc loader rejections are independent source inputs. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: The exact main-supported dynamic program on0:0 through0:3; literal trace/proc loader rejections are independent source inputs. Version: bigip 21.1.0.1.

Modern and legacy runtime trace APIs install and complete on each recorded TMM. The main result is {NEW EXTRA} {NEW EXTRA}, with read then unset, exact name arguments, and no retained registration after recreation. The later ordinary-global controls are distinct and do not prove arbitrary cross-TMM sharing.

The measured report rows and their limits are:

Modern `trace add variable` and legacy `trace variable` each installed at
runtime with catch code 0 on every TMM. The operation also completed with code
0 and returned byte-identical results on `0:0` through `0:3`:

```text
{NEW EXTRA} {NEW EXTRA}
```

For both APIs, callback order was `read`, then `unset`. `name1` was exactly
hex `52323238365f766c315f76616c7565` (`R2286_vl1_value`), `name2` was empty,
and `trace info variable` was empty after recreation. The final read did not
invoke the destroyed registration. The raw per-unit responses are
`vl-main-u0.body`
through
`vl-main-u3.body`.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/VARIABLE_LIFECYCLE_RESULTS.md)
