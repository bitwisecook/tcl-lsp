# naming.numeric.wasi-libc-environment

Kind: `environment-observation`.

## Problem statement

A numeric environment supplies actual C widths, libc conversions and one errno cell. Host LP64 observations or Rust binding constants do not determine a wasm32 SDK guest. The conversion entry state, reached errno and a source adapter's pre-call rejection must also remain separate, so a consumer cannot borrow an unsupported Tcl Long32 or Runtime libc recipe from a matching width.

## Question

For the actual pinned SDK34 wasm32-wasip1 C libc module run under Wasmtime49, what are C target sizes/limits, shared errno-cell identity, and reached integer/double conversion results, end offsets and independently seeded errno effects?

## Conclusion

The exact SDK34.0 C guest compiled by clang23.1.0-wasi-sdk and executed by Wasmtime49.0.2 reports CHAR_BIT8, sizeof(int/long/pointer)4, sizeof(long long/double)8, EDOM18 and ERANGE68. Three witnesses join errno to __errno_location. The 315 independent raw libc calls retain their exact C values, end offsets, floating bytes and seeded effects: call_errno is the call-entry state; after is reached errno copied before formatting. Empty/bad conversion reaches the separately pinned SDK EINVAL28. The 12 adapter controls independently execute at offsets0/1 of counted1NULX and reject offsets2/3 before libc. Original439 execution fails before guest output on a read-only Wasmtime cache; original440 disables that optional cache and captures all rows using the unchanged C probe and identical module bytes. These are measured SDK environment results; no Tcl/Jim/BIG-IP execution, Cargo Runtime library identity or Tcl Long32/getter authority follows.

Queried output layout, retained primary cache width and a reached fresh C84 parsing recipe remain separate eligibility purposes. Descriptive Long32 layout cannot manufacture Long64 storage or a C84 build recipe.

## Scope

Exactly the original439 and440 requests, launchers, fixed C probe/case bytes, compiled module bytes, eight process/version steps, complete original stdout/stderr/receipts, selected SDK/compiler/linker/runtime/header/library input bytes and unchanged before/after pins. SDK VERSION, compiler and runner version streams identify this measured tool context. The guest has 21 inputs, five libc workers and three independent seeds (315 rows), three errno-cell witnesses and 12 separately tagged counted-prefix adapter controls. SDK EDOM/ERANGE/EINVAL macros are actual header/guest facts, not Rust binding values. Selected inputs do not close unrecorded dynamic tool dependencies or the whole sysroot. This observation supplies no Cargo Runtime final libc identity, Tcl release/Long32/scalar getter/cache/header/issuer/application/command/frame/source-provider/expression-truth/browser capability or general counted-NUL admission. Independent Rust-sysroot measurement, original native scalar/expression questions and software purpose contracts retain separate evidence. These two added Syntax controls describe software ABI/cache equations only. The independent SDK and Rust guest observations retain their exact environments; none certifies a Tcl32 getter, actual Runtime reactor/cdylib libc, engine/header issuer, browser or hosted provider. Security projection: the retained Wasmtime executable and its embedded public trust-anchor keys are replaced by a redaction notice. Original tool hashes remain historical identity; executable replay requires independently obtained and verified local tool bytes.

## Measured environment answers

| Capture | Actual version/target/channel | Status | Answer and strict limit |
| --- | --- | --- | --- |
| `wasi-sdk34-wasmtime49-run439` | wasi-sdk34.0; clang23.1.0-wasi-sdk; Wasmtime49.0.2 (3c8a3e79a); wasm32-wasip1 C SDK libc guest; Fixed C byte arrays; independent errno seeds; Wasmtime command run | `failed` | Compiler and runtime version queries and compilation exit0; execution exits1 before any guest row because the Wasmtime cache directory is read-only. Complete failure stdout/stderr, module and unchanged selected pins remain retained independently. |
| `wasi-sdk34-wasmtime49-run440` | wasi-sdk34.0; clang23.1.0-wasi-sdk; Wasmtime49.0.2 (3c8a3e79a); wasm32-wasip1 C SDK libc guest; Fixed C byte arrays; independent errno seeds; Wasmtime command run -C cache=n | `observed` | Compiler and runtime version queries, compilation and execution exit0, with empty stderr. The unchanged probe captures315 raw libc rows,3 errno-cell witnesses and12 independent adapter controls. The actual target is CHAR_BIT8/int4/long4/longlong8/double8/pointer4; EDOM18/ERANGE68 and reached EINVAL28 are SDK facts. No Cargo Runtime libc, Tcl Long32 or native-provider result is issued. |

## Original provider answers

| Provider | Status | Answer |
| --- | --- | --- |
| `tcl8.4` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `tcl8.5` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `tcl8.6` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `tcl9.0` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `tcl9.1` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `jim` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |
| `bigip` | `not-tested` | This SDK libc guest does not invoke or establish this original provider. No Tcl getter, runtime library, name, frame or execution answer is inferred. |

## Exact producer and original rows

`probe.c` lines78–81 seed/read errno, optionally reset worker4 and copy `call_errno` before the switch. Lines83–87 call the selected libc function. Line90 copies `after`; the first printf starts at91. No later formatter can change that already copied integer. SDK `__errno_values.h` line31 defines EINVAL28, separately from EDOM18/ERANGE68. The original field names remain unchanged.

The rows below are selected verbatim from the complete original440 stdout; they are an index into that capture, not independent measurements. All315 unique case/worker/seed triples,21 original CASE byte vectors,3 cell witnesses and12 adapter rows are retained.

```text
ROW	case=0	worker=0	seed_kind=0	seed=0	before=0	call_errno=0	after=28	end=0	input_len=0	prefix_len=0	value=0
ROW	case=1	worker=2	seed_kind=1	seed=18	before=18	call_errno=18	after=28	end=0	input_len=3	prefix_len=3	value=0
ROW	case=2	worker=0	seed_kind=0	seed=0	before=0	call_errno=0	after=0	end=4	input_len=4	prefix_len=4	value=16
ROW	case=2	worker=0	seed_kind=2	seed=68	before=68	call_errno=68	after=68	end=4	input_len=4	prefix_len=4	value=16
ROW	case=3	worker=0	seed_kind=0	seed=0	before=0	call_errno=0	after=0	end=1	input_len=2	prefix_len=2	value=0
ROW	case=9	worker=0	seed_kind=0	seed=0	before=0	call_errno=0	after=68	end=10	input_len=10	prefix_len=10	value=2147483647
ROW	case=11	worker=0	seed_kind=0	seed=0	before=0	call_errno=0	after=68	end=11	input_len=11	prefix_len=11	value=-2147483648
ROW	case=12	worker=1	seed_kind=0	seed=0	before=0	call_errno=0	after=68	end=20	input_len=20	prefix_len=20	value=4294967295
ROW	case=12	worker=2	seed_kind=0	seed=0	before=0	call_errno=0	after=0	end=20	input_len=20	prefix_len=20	value=18446744073709551615
ROW	case=13	worker=2	seed_kind=0	seed=0	before=0	call_errno=0	after=68	end=20	input_len=20	prefix_len=20	value=18446744073709551615
ROW	case=14	worker=3	seed_kind=1	seed=18	before=18	call_errno=18	after=18	end=3	input_len=3	prefix_len=3	value=nan	double_bytes=000000000000f87f	is_nan=1	is_infinite=0	signbit=0
ROW	case=17	worker=3	seed_kind=0	seed=0	before=0	call_errno=0	after=68	end=6	input_len=6	prefix_len=6	value=inf	double_bytes=000000000000f07f	is_nan=0	is_infinite=1	signbit=0
ROW	case=18	worker=4	seed_kind=2	seed=68	before=68	call_errno=0	after=68	end=7	input_len=7	prefix_len=7	value=0x0p+0	double_bytes=0000000000000000	is_nan=0	is_infinite=0	signbit=0
ROW	case=19	worker=2	seed_kind=1	seed=18	before=18	call_errno=18	after=18	end=1	input_len=3	prefix_len=1	value=1
ROW	case=20	worker=2	seed_kind=0	seed=0	before=0	call_errno=0	after=28	end=0	input_len=2	prefix_len=0	value=0
```

## Restoration and replay

The Wasmtime input is now a redaction notice because its executable embedded
public vendor trust-anchor keys. Historical receipts retain its original
identity, but the executable cannot be restored from this checkout. Follow
[local tool verification](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasmtime-49.0.2/REDACTION.md)
before a new measurement. The restoration instructions below apply only to
unchanged retained inputs.

Verify each unchanged retained original request/launcher/process/module/stream and input-storage hash, decompress selected gzip inputs to recover their complete original bytes, and read all raw rows with the exact C producer. Original launchers require their original recorded absolute input paths and a new capture directory; Root alone may launch a new separately retained measurement. Unrecorded dynamic compiler/runner dependencies and the full SDK sysroot are not reconstructed by selected inputs. No Cargo build, Tcl/Jim process, Rust assertion or SDK-to-Runtime libc join is supplied by this question.

[selected-input-storage.json](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-input-storage.json) records each complete original size/hash/mode, compressed size/hash and original absolute path. Restore to a chosen directory by relative stored path and verify both stored and uncompressed hashes; do not overwrite SDK/current runtime files. Original launch paths must be supplied separately for a new Root-owned run.

```python
from pathlib import Path
import gzip, hashlib, json
fixture = Path("rust/tcl-registry/tests/data/native_wasi_libc_environment_original")
repository = Path.cwd()
output = Path("/tmp/restored-wasi-sdk-selected-inputs")
for item in json.loads((fixture / "selected-input-storage.json").read_text())["inputs"]:
    source = repository / item["retained_path"]
    stored = source.read_bytes()
    assert len(stored) == item["retained_bytes"]
    assert hashlib.sha256(stored).hexdigest() == item["retained_sha256"]
    original = gzip.decompress(stored) if item["compression"] == "gzip" else stored
    assert len(original) == item["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == item["original_sha256"]
    relative = Path(item["original_path"]).relative_to("/workspace/.tools")
    destination = output / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("xb") as stream:
        stream.write(original)
    destination.chmod(item["original_mode"])
```

The [independent native scalar ABI/getter captures](numeric-original-capi-scalar-publication-width.md), [primitive versus expression routes](numeric-original-primitive-boolean-vs-expression-truth.md) and [logical operand sites](numeric-original-logical-operand-truth-sites.md) answer their own original provider questions. No software assertion is attached to this SDK environment record.

## Evidence

- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original)
- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/VERSION](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/VERSION)
- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/bin](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/bin)
- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/lib/clang/23/include/stddef.h](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/lib/clang/23/include/stddef.h)
- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/share/wasi-sysroot/include/wasm32-wasip1](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/share/wasi-sysroot/include/wasm32-wasip1)
- [rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/share/wasi-sysroot/include/wasm32-wasip1/bits/alltypes.h](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasi-sdk-34.0/share/wasi-sysroot/include/wasm32-wasip1/bits/alltypes.h)
- [rust/tcl-syntax/src/scalar_getter/abi_tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/abi_tests.rs)
