# naming.numeric.rust-wasi-libc-environment

Kind: `environment-observation`.

## Problem statement

The SDK C guest and a Rust WASIp1 command can report the same widths and conversion rows while selecting different libc archives. A numeric consumer needs the actual Rust FFI layout, reached errno state and original link inputs without borrowing a Runtime reactor, Tcl Long32 or host LP64 recipe. A failed compiler setup also cannot supply guest conversion observations.

## Question

For this actual Rust-sysroot wasm32-wasip1 command module, what are the compiled Rust C-FFI alias widths/limits, reached libc conversions and immediate errno effects over the unchanged21 original vectors? Which exact Rust sysroot libraries and rust-lld inputs produced the guest module?

## Conclusion

The original Rust1.99.0 standalone wasm32-wasip1 command guest executed by Wasmtime49.0.2 reports CHAR_BIT8, char1/int4/long4/longlong8/double8/pointer4. Its EDOM18/ERANGE68/EINVAL28 labels are pinned libc0.2.190 Rust bindings. Original442 stops at the unstable explicit wasm-lld flavor before linking or guest output. Original443 removes that override and retains eight successful version/build/execute steps,315 raw libc rows,3 errno-cell witnesses and12 independent prefix controls. The reached errno is captured immediately after the call and before formatting; empty/bad conversions reach EINVAL28. The actual link trace selects Rust sysroot libc.a SHA5d8ba34d8c6fd0ac59e0efe37241143887f8f232864bbeacc4181ae739f63371, the command CRT,16 explicit rlib arguments and two captured temporary Rust objects. All315 common conversion fields equal the independently retained SDK440 rows only after the explicit call_errno→entry_errno and after→reached_errno label mapping. This bounded row equality supplies no library equivalence, Runtime reactor/cdylib libc identity, Tcl getter/Long32 recipe or native-provider answer.

## Scope

Exactly the original442 and443 requests, identical fixed Rust probe and transparent linker wrapper,21 byte inputs/five workers/three independent seeds,15 closed process/version steps, complete original streams/receipts, actual443 module/link trace/command CRT/temporary objects,58 selected Rust WASIp1 target-library bodies and their unchanged before/after hashes. Complete retained source/configuration/FFI files preserve the original requested context, including the frozen original Runtime Cargo.toml rather than changed live bytes. Large host compiler/linker libraries are exact hash/size references requiring matching external tool bytes; the already retained matching Wasmtime tool is reused without duplicating it. Unrecorded dynamic dependencies and a complete host toolchain are not reconstructed. The actual Rust FFI widths and binding values remain separate from SDK C header macros. No Runtime Cargo reactor/cdylib/libtommath build, Tcl/Jim/BIG-IP process, scalar getter/cache/header/application issuer/expression truth/name/frame/browser capability, general counted-NUL admission or Tcl32 recipe is established. Security projection: the retained Wasmtime executable and its embedded public trust-anchor keys are replaced by a redaction notice. Original tool hashes remain historical identity; executable replay requires independently obtained and verified local tool bytes.

## Measured environment answers

| Capture | Version, target and channel | Status | Answer and strict limit |
| --- | --- | --- | --- |
| `rust-wasip1-original442` | rustc1.99.0 (b940084d7eb6a299eb4bfeb8e34901bc051e7ac4,2026-09-28); LLVM/LLD23.1.1; Wasmtime49.0.2 (3c8a3e79a); Standalone Rust wasm32-wasip1 command guest; Fixed Rust byte vectors, original libc FFI calls and independent errno seeds; Wasmtime run -C cache=n | `failed` | Six version/sysroot/target/linker/runner queries exit0. Compilation exits1 because the explicit wasm-lld linker flavor requires unstable options. No link receipt, compiled module, guest execution or conversion rows are present; the complete failed compile stderr and unchanged81 selected input pins remain retained. |
| `rust-wasip1-original443` | rustc1.99.0 (b940084d7eb6a299eb4bfeb8e34901bc051e7ac4,2026-09-28); LLVM/LLD23.1.1; Wasmtime49.0.2 (3c8a3e79a); Standalone Rust wasm32-wasip1 command guest; Fixed Rust byte vectors, original libc FFI calls and independent errno seeds; Wasmtime run -C cache=n | `observed` | All eight version/build/execute steps exit0. The actual command guest captures315 conversion rows,3 errno-cell witnesses and12 prefix controls, with empty guest stderr. Successful compile stderr retains19629 bytes of link-trace warnings; it is not empty. The actual Rust FFI target is char8/int4/long4/longlong8/double8/pointer4, with pinned Rust EDOM18/ERANGE68/EINVAL28 bindings and immediate reached errno fields. The selected Rust sysroot libc archive differs from the SDK archive. No Runtime reactor/cdylib or native-provider result follows. |

## Original provider answers

| Provider | Status | Answer |
| --- | --- | --- |
| `tcl8.4` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `tcl8.5` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `tcl8.6` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `tcl9.0` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `tcl9.1` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `jim` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |
| `bigip` | `not-tested` | This standalone Rust libc guest does not invoke or establish this original provider. No Tcl getter, Runtime library, name, frame or execution answer is inferred. |

## Exact producer and linkage

The unchanged `probe.rs` reads `before` at line177 after the independent seed, optionally resets worker4, and reads `entry_errno` at181 immediately before the selected FFI call. Line197 copies `reached_errno`; the first ROW printf begins206. This source order preserves immediate call effects independently of formatting. The five workers are strtol, strtoul, strtoull, strtod without reset, and strtod with reset. The pinned libc0.2.190 WASI source supplies the Rust error labels; the guest observes actual aliases/limits and conversions.

The transparent original link receipt retains the exact default target selection, command CRT,16 explicit rlib arguments, two temporary Rust compiler objects and selected Rust libc archive. Link stdout joins numeric archive members to that actual archive. The original442 flavor rejection precedes any link/guest and supplies no conversion answer. No source/header-to-library equivalence or Runtime final-module join is inferred.

## Selected original rows

These35 rows are copied verbatim from the complete original443 stdout. They index the original315 observations and are not additional runs.

```text
ROW	case=0	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=0	prefix_len=0	value=0
ROW	case=0	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=0	prefix_len=0	value=0
ROW	case=0	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=0	prefix_len=0	value=0
ROW	case=2	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=4	input_len=4	prefix_len=4	value=16
ROW	case=2	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=4	input_len=4	prefix_len=4	value=16
ROW	case=2	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=4	input_len=4	prefix_len=4	value=16
ROW	case=3	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=2	prefix_len=2	value=0
ROW	case=3	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=2	prefix_len=2	value=0
ROW	case=3	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=2	prefix_len=2	value=0
ROW	case=4	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=5	input_len=9	prefix_len=9	value=-17
ROW	case=4	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=5	input_len=9	prefix_len=9	value=4294967279
ROW	case=4	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=5	input_len=9	prefix_len=9	value=18446744073709551599
ROW	case=9	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=10	input_len=10	prefix_len=10	value=2147483647
ROW	case=9	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=10	input_len=10	prefix_len=10	value=2147483648
ROW	case=9	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=10	input_len=10	prefix_len=10	value=2147483648
ROW	case=11	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=11	input_len=11	prefix_len=11	value=-2147483648
ROW	case=11	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=11	input_len=11	prefix_len=11	value=2147483647
ROW	case=11	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=11	input_len=11	prefix_len=11	value=18446744071562067967
ROW	case=12	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=20	input_len=20	prefix_len=20	value=2147483647
ROW	case=12	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=20	input_len=20	prefix_len=20	value=4294967295
ROW	case=12	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=20	input_len=20	prefix_len=20	value=18446744073709551615
ROW	case=13	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=20	input_len=20	prefix_len=20	value=2147483647
ROW	case=13	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=20	input_len=20	prefix_len=20	value=4294967295
ROW	case=13	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=20	input_len=20	prefix_len=20	value=18446744073709551615
ROW	case=14	worker=3	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=3	input_len=3	prefix_len=3	value=nan	double_bytes=000000000000f87f	is_nan=1	is_infinite=0	signbit=0
ROW	case=15	worker=3	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=3	input_len=3	prefix_len=3	value=inf	double_bytes=000000000000f07f	is_nan=0	is_infinite=1	signbit=0
ROW	case=16	worker=3	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=4	input_len=4	prefix_len=4	value=-inf	double_bytes=000000000000f0ff	is_nan=0	is_infinite=1	signbit=1
ROW	case=17	worker=3	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=6	input_len=6	prefix_len=6	value=inf	double_bytes=000000000000f07f	is_nan=0	is_infinite=1	signbit=0
ROW	case=18	worker=3	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=68	end=7	input_len=7	prefix_len=7	value=0x0p+0	double_bytes=0000000000000000	is_nan=0	is_infinite=0	signbit=0
ROW	case=19	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=3	prefix_len=1	value=1
ROW	case=19	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=3	prefix_len=1	value=1
ROW	case=19	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=0	end=1	input_len=3	prefix_len=1	value=1
ROW	case=20	worker=0	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=2	prefix_len=0	value=0
ROW	case=20	worker=1	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=2	prefix_len=0	value=0
ROW	case=20	worker=2	seed_kind=0	seed=0	before=0	entry_errno=0	reached_errno=28	end=0	input_len=2	prefix_len=0	value=0
```

## Restoration and replay

The Wasmtime input is now a redaction notice because its executable embedded
public vendor trust-anchor keys. Historical receipts retain its original
identity, but the executable cannot be restored from this checkout. Follow
[local tool verification](../../../../rust/tcl-registry/tests/data/native_wasi_libc_environment_original/selected-inputs/wasmtime-49.0.2/REDACTION.md)
before a new measurement. The restoration instructions below apply only to
unchanged retained inputs.

Verify the whole original requests, receipts, module, link/process streams and source/storage mappings. Restore each retained target/provenance input into a new directory and verify compressed and original size/hash. A fresh run requires exact matching external host-tool bytes and the original recorded absolute input paths or a newly sealed request with explicit replacements; reading the retained evidence does not execute it. The original transparent wrapper captures link arguments and temporary objects without changing the selected compiler/default target or library recipe. No Runtime reactor/cdylib or Tcl process/assertion is supplied by this standalone command guest.

The storage mapping distinguishes whole retained target/provenance bodies from exact host-tool references and the existing retained Wasmtime bytes. Restore into a new directory; external references must be supplied and verified separately.

```python
from pathlib import Path
import gzip, hashlib, json
repository = Path.cwd()
fixture = repository / "rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original"
output = Path("/tmp/restored-rust-wasip1-selected-inputs")
for item in json.loads((fixture / "selected-input-storage.json").read_text())["inputs"]:
    stored = (repository / item["retained_path"]).read_bytes()
    assert len(stored) == item["retained_bytes"]
    assert hashlib.sha256(stored).hexdigest() == item["retained_sha256"]
    original = gzip.decompress(stored) if item["compression"] == "gzip" else stored
    assert len(original) == item["original_bytes"]
    assert hashlib.sha256(original).hexdigest() == item["original_sha256"]
    relative = Path(item["restore_relative_path"])
    assert not relative.is_absolute() and ".." not in relative.parts
    destination = output / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("xb") as stream:
        stream.write(original)
    destination.chmod(item["original_mode"])
```

The [independent SDK C environment](numeric-wasi-libc-environment.md), [original host scalar getters](numeric-original-capi-scalar-publication-width.md), [primitive versus expression truth](numeric-original-primitive-boolean-vs-expression-truth.md) and [logical operand sites](numeric-original-logical-operand-truth-sites.md) answer their own purposes. All seven original provider answers remain not-tested here. A matching Runtime reactor/cdylib final link requires its own capture.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/capture443/linker](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/capture443/linker)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/libc-binding](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/libc-binding)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/rust-toolchain/lib](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/rust-toolchain/lib)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/.cargo](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/.cargo)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/runtime/rust](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/runtime/rust)
- [rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/rust/tcl-compiler/tests/common](../../../../rust/tcl-registry/tests/data/native_rust_wasi_libc_environment_original/selected-provenance/repo-inputs/rust/tcl-compiler/tests/common)
