# naming.native-replay.provider-build-and-startup-identity

Kind: `native-observation`

## Problem statement

A launcher filename or surviving transcript hash cannot identify the native executable that actually runs a proof. Modern compiler compatibility flags and Jim UTF-8 configuration can also change the build. A fresh build/version record must identify the actual provider separately from its naming observations.

## Question

Which exact native C Tcl and pinned Jim providers, configurations and original CLI version/patchlevel outputs are retained for reconfirming naming questions?

## Conclusion

All five C processes returned their recorded release patchlevels and the pinned Jim process returned 0.84 / 0.84-9-g5bac7c9 with UTF-8 enabled. The compile configuration, original probe bytes and separate status/stdout/stderr hashes are retained. This validates only the build/startup identity; it proves no naming behavior or parity with an older binary.

## Scope

These are actual current provider build/version captures under the recorded GCC14 configuration. Earlier failed configure/build attempts are not successful guest observations. C84 environment CFLAGS and prefix remain in its exact selected recipe. BIG-IP was not built or launched for this question. No Rust tests are executed by this record.

## Provider answers

### tcl8.4

Status: `observed`. Channel: CLI stdin. Version: tcl8.4 8.4.20.

Actual process exit 0; stdout '8.4\n8.4.20\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["env", "CFLAGS=-O2 -Wno-error=implicit-int -Wno-error=implicit-function-declaration -Wno-error=incompatible-pointer-types", "./configure", "--cache-file=/dev/null", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/gcc14-compatibility/8.4.20/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl8.5

Status: `observed`. Channel: CLI stdin. Version: tcl8.5 8.5.19.

Actual process exit 0; stdout '8.5\n8.5.19\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--cache-file=/dev/null", "--disable-shared", "--enable-threads", "CFLAGS=-O2 -Wno-error=implicit-int -Wno-error=implicit-function-declaration -Wno-error=incompatible-pointer-types", "--prefix=/workspace/.proofs/native-providers/gcc14-compatibility/8.5.19/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl8.6

Status: `observed`. Channel: CLI stdin. Version: tcl8.6 8.6.18.

Actual process exit 0; stdout '8.6\n8.6.18\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/8.6.18/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl9.0

Status: `observed`. Channel: CLI stdin. Version: tcl9.0 9.0.4.

Actual process exit 0; stdout '9.0\n9.0.4\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/9.0.4/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl9.1

Status: `observed`. Channel: CLI stdin. Version: tcl9.1 9.1.0.

Actual process exit 0; stdout '9.1\n9.1.0\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/9.1.0/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### jim

Status: `observed`. Channel: CLI stdin. Version: jim 0.84-9-g5bac7c9.

Actual process exit 0; stdout '0.84\n0.84-9-g5bac7c9\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--prefix=/workspace/.proofs/native-providers/jimtcl/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt. Jim revision 5bac7c99ad65864c87da513e22e2f01703fa4e03; UTF-8 enabled True; retained config hash d092046fe07c2ec14b77da74304135da24725d766e1c9fd74ee32eeb07702221. This startup identity is independent of any older Jim artifact hash.

### bigip

Status: `not-tested`.

This record did not build or launch the closed-source appliance. Its independent appliance reports are separate question evidence.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-test-support/tests/data/native_resolution_reference_builds](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds)
