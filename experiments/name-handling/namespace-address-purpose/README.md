# Empty namespace operands by command purpose

Question: does an explicit empty namespace operand denote the current namespace
inside a non-global namespace, and do lookup and creation report the same
outcome?

[input.tcl](input.tcl) runs `namespace exists` before and after `namespace eval`
in global and `::__nh_outer` frames. An absolute `::` operand is the control.
Each operation has a stable context/variant/stage ID in the raw Tcl-list output.
The helper uses `uplevel 1` so the operation retains the invoking namespace
frame. Inputs are executed in fresh disposable real C interpreters. This is
command-level evidence; it does not measure a C API with supplied flags.

| Operand and context | Exists before | Eval completion/result | Exists after |
| --- | --- | --- | --- |
| Empty, global | 1 | 0 / `::` | 1 |
| Empty, `::__nh_outer` | 0 | 1 / `can't create namespace "": only global namespace can have empty name` | 0 |
| Absolute `::`, either context | 1 | 0 / `::` | 1 |
| Tcl-constructed Unicode NUL plus `suffix`, global | 0 | 0 / a distinct namespace with that suffix | 1 |
| Tcl-constructed Unicode NUL plus `suffix`, `::__nh_outer` | 0 | 0 / a distinct child with that suffix | 1 |

All five C releases produce those bounded command outcomes. For the failed
creation, `errorCode` is `NONE` on 8.4.20/8.5.19 and
`TCL OPERATION NAMESPACE CREATEGLOBAL` on 8.6.18/9.0.4/9.1.0. Each release
retains the complete errorInfo, completion/result and literal NUL-containing
output in its `stdout.bin`; stderr is empty and every process exits zero.
That process status includes the intentionally caught creation error.

| Reference | Provenance | Raw result |
| --- | --- | --- |
| C 8.4.20 | [record](8.4.20/provenance.json) | [stdout](8.4.20/stdout.bin) |
| C 8.5.19 | [record](8.5.19/provenance.json) | [stdout](8.5.19/stdout.bin) |
| C 8.6.18 | [record](8.6.18/provenance.json) | [stdout](8.6.18/stdout.bin) |
| C 9.0.4 | [record](9.0.4/provenance.json) | [stdout](9.0.4/stdout.bin) |
| C 9.1.0 | [record](9.1.0/provenance.json) | [stdout](9.1.0/stdout.bin) |

Each record identifies exact input/binary/init hashes, actual invocation,
measurement time, manifest-selected source tag and immutable upstream commit.
The original full library/tests inventory was independently validated against
the retained upstream Git blob receipt. A separate identity control verifies
the same hashed binary's reported release, patchlevel and matching library
after measurement; its original input and raw channels are retained. Tcltest
is not used. No Jim, F5 or VM result is claimed; these observations are not
promoted to consumer conformance tests.

The NUL control is a Tcl STRING operand constructed by `format %c 0`, not a
byte buffer passed directly to a C namespace API. Native string materialisation
can encode that character as modified UTF-8. Its successful namespace creation
must not supply a raw-C-NUL expectation.

Reproduce from the repository root with each manifest-selected source-built
interpreter and its matching library, into separate new capture files:

```sh
TCL_LIBRARY=/absolute/tcl8.6.18/library \
LD_LIBRARY_PATH=/absolute/tcl8.6.18/unix \
/absolute/tcl8.6.18/unix/tclsh \
  experiments/name-handling/namespace-address-purpose/input.tcl \
  > /absolute/new-stdout.bin 2> /absolute/new-stderr.bin
```

## Consumer obligations

VM `namespace_address_from_native_bytes` repeats the shared C address geometry
and currently selects its context path for empty relative input.
`NativeNameProtocol::namespace_address_path` instead refuses that non-global
geometry. Checked lookup must preserve a proved missing original operand as
`Ok(None)`, separately from unsupported context/protocol. Namespace creation
needs its measured operation-specific diagnostic and release error code.

Unchecked `Namespaces::find_namespace_bytes` also contains an explicit
empty-to-current shortcut. Its callers need purpose review: an omitted optional
operand is already represented independently as `None` in the shared
parent/children resolver, while an explicit operand is a lookup. Import and
namespace-variable qualifier helpers retain their own original input purpose
and absolute-root prefix. Do not change a sentinel contract by assuming it is
an original operand, or introduce an empty-to-current fallback for actual
lookup. Direct VM original-object `exists` uses the checked namespace getter;
cache validity and namespace incarnation must stay independent of display text.
