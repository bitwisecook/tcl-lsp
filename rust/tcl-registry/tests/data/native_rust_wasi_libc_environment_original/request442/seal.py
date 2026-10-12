from pathlib import Path
import ast,hashlib,json,re
root=Path(__file__).resolve().parent
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
def pin(path):
    p=Path(path)
    return {'path':str(p),'resolved_path':str(p.resolve()),'bytes':p.stat().st_size,'sha256':sha(p)}
for name in ['launch.py','link-wrapper.py','prepare-source.py']:
    ast.parse((root/name).read_text())
(root/'link-wrapper.py').chmod(0o755)
sysroot=Path('/workspace/.rustup/toolchains/stable-x86_64-unknown-linux-gnu')
libdir=sysroot/'lib/rustlib/wasm32-wasip1/lib'
assert libdir.is_dir()
target_pins=[pin(p) for p in sorted(libdir.rglob('*')) if p.is_file()]
assert any(Path(p['path']).name=='libc.a' for p in target_pins)
assert any(Path(p['path']).name=='crt1-command.o' for p in target_pins)
assert any(Path(p['path']).name.startswith('libstd-') and Path(p['path']).suffix=='.rlib' for p in target_pins)
cases=json.loads((root/'cases.json').read_text())
source=(root/'probe.rs').read_text()
actual=[]
for m in re.finditer(r'InputCase\s*\{\s*name:\s*c"([^"]+)",\s*bytes:\s*&\[([0-9,\s]+)\],\s*length:\s*(\d+)\s*,?\s*\}',source,re.S):
    actual.append((m[1],bytes(int(v.strip()) for v in m[2].split(',') if v.strip()),int(m[3])))
assert len(actual)==21
for original,selected in zip(cases,actual):
    assert selected==(original['name'],bytes.fromhex(original['original_hex'])+b'\0',original['counted_length'])
source_inputs=[root/'probe.rs',root/'cases.json',root/'link-wrapper.py',root/'prepare-source.py']
compiler=sysroot/'bin/rustc'
linker=sysroot/'lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld'
runner=Path('/workspace/.tools/wasmtime-49.0.2/wasmtime')
tool_inputs=[compiler,linker,runner]+sorted(p for p in (sysroot/'lib').iterdir() if p.is_file())
meta_inputs=[sysroot/'lib/rustlib/components',sysroot/'lib/rustlib/manifest-rust-std-wasm32-wasip1',sysroot/'lib/rustlib/multirust-channel-manifest.toml',sysroot/'lib/rustlib/multirust-config.toml']
recipe_inputs=[Path('/workspace/tcl-lsp/rust/tcl-compiler/tests/common/wasm_link.rs'),Path('/workspace/tcl-lsp/runtime/rust/build.rs'),Path('/workspace/tcl-lsp/.cargo/config.toml'),Path('/workspace/tcl-lsp/runtime/rust/Cargo.toml'),Path('/workspace/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libc-0.2.190/src/wasi/mod.rs')]
prior_inputs=[Path('/tmp/grammar-wasi-numeric-request439/request.json'),Path('/tmp/native-wasi-numeric-environment439-capture/receipt.json'),Path('/tmp/grammar-wasi-numeric-request440/request.json'),Path('/tmp/native-wasi-numeric-environment440-capture/receipt.json')]
assert sha(prior_inputs[-1])=='778623026c9c51732b594d4b2b1254f917c141141dca0afd71fcd13fc37f6518'
input_pins=[pin(p) for p in source_inputs+tool_inputs+meta_inputs+recipe_inputs+prior_inputs]+target_pins
assert len({item['path'] for item in input_pins})==len(input_pins)
request={
    'question_id':'naming.numeric.wasi-libc-environment',
    'question':'For this actual Rust-sysroot wasm32-wasip1 command module, what are the compiled Rust C-FFI alias widths/limits, reached libc conversions and immediate errno effects over the unchanged21 original vectors? Which exact Rust sysroot libraries and rust-lld inputs produced the guest module?',
    'scope':'Independent Rust-sysroot guest libc/environment observation. FFI type widths and limits are queried in this actual compiled Rust guest; errno labels/prototypes are from pinned libc0.2.190 WASI bindings, not SDK macros. No Tcl getter, Long32 recipe, expression truth, object/header/cache, NativeScalarGetterTarget, installed Runtime/engine/Source provider, browser or hostLP64 capability is established. Standalone command CRT and std allocation/printing remain separate from Runtime cdylib/reactor. SDK439failure/440success remain independent observations; matching widths cannot certify library equivalence.',
    'target':'wasm32-wasip1', 'sysroot':str(sysroot), 'target_libdir':str(libdir),
    'compiler':str(compiler), 'linker':str(linker), 'runner':str(runner),
    'probe_path':str(root/'probe.rs'), 'probe_sha256':sha(root/'probe.rs'),
    'link_wrapper_sha256':sha(root/'link-wrapper.py'), 'launcher_sha256':sha(root/'launch.py'),
    'input_pins':input_pins, 'target_library_pins':target_pins,
    'compile_flags':['--edition=2021','--crate-type=bin','--crate-name=wasi_numeric_environment_original','--target=wasm32-wasip1','--sysroot='+str(sysroot),'-C','opt-level=0','-C','debuginfo=0','-C','panic=abort','-C','linker='+str(root/'link-wrapper.py'),'-C','linker-flavor=wasm-lld','-C','link-arg=--trace','-C','link-arg=--global-base=2097152','-D','warnings'],
    'environment':{'RUSTUP_HOME':'/workspace/.rustup','PATH':'/workspace/.cargo/bin:/workspace/.tools/wasmtime-49.0.2:/workspace/.tools/wasi-sdk-34.0/bin:/usr/local/bin:/usr/bin:/bin'},
    'runner_flags':['run','-C','cache=n'], 'expected_runner_version_fragment':'49.0.2',
    'capture_root':'/tmp/native-rust-wasi-numeric-environment442-capture',
    'expected_line_counts':{'ABI':1,'ERRNO_CELL':3,'CASE':21,'ROW':315,'ADAPTER':12,'SUMMARY':1},
    'case_map':cases,
    'workers':{'0':'actual linked strtol radix0 no reset','1':'actual linked strtoul radix0 no reset','2':'actual linked strtoull radix0 no reset','3':'actual linked strtod no reset','4':'actual linked strtod explicit reset after pre-reset snapshot'},
    'seed_kinds':{'0':'0','1':'EDOM from pinned Rust libc WASI binding','2':'ERANGE from pinned Rust libc WASI binding'},
    'field_boundaries':{'before':'Actual current guest errno after independent seeding, before optional worker4 reset','entry_errno':'Actual errno snapshot immediately before libc call, after any explicit reset','reached_errno':'Actual errno snapshot immediately after libc call, before any formatting/classification','errno_cells':'Two successive actual __errno_location calls and read/write/reset through same actual returned cell; no C macro identity claimed','double_bytes':'Complete actual c_double/f64 native-endian bytes captured after reached_errno snapshot','adapter':'Same counted1NULX/prefix1 guard; offsets0/1 real guest strtoull, offsets2/3 rejected before call. Not Tcl source/value admission.'},
    'runtime_recipe_correspondence':{'source':'rust/tcl-compiler/tests/common/wasm_link.rs::build_reserved_runtime','same':'actual target wasm32-wasip1, actual installed rustc/rust-lld/target sysroot and --global-base=2097152','different':'This is direct rustc bin with command CRT, explicit transparent trace wrapper and no SDK-built libtommath/Runtime/engine crates; actual Runtime Cargo cdylib link inputs must still be independently witnessed. No foreign Runtime ABI issuer inferred.'},
    'prior439_440':[pin(p) for p in prior_inputs],
    'capture_requirements':['Root only exact rustc -vV/sysroot/target-libdir/cfg/linker-version/runner-version argv/output/exit receipts','Full current target library inventory hashes before/after, exact rust-lld argv/response-file bytes and temporary object copies, raw --trace link output','Unchanged source/cases/prototypes/binding/tool/relevant recipe pins before/after; exact resulting module bytes/hash','Complete unfiltered guest stdout/stderr/exit and row counts; no zero-row or skipped positive; all failures retained independently'],
    'root_only_execution':'python3 /tmp/grammar-rust-wasi-numeric-request442/launch.py [new capture directory]; agent did not invoke rustc, linker, Cargo or guest runner.',
    'source_validation':{'rustfmt2021_exit':0,'all21original_counted_vectors_exact':True,'python_launch_and_wrapper_ast_parse':True,'all_existing_pins_exact_at_seal':True,'actual_wasip1_libraries_installed_by_root':True,'cargo_native_or_guest_run_by_agent':False}
}
(root/'request.json').write_text(json.dumps(request,indent=2)+'\n')
manifest={'name':'GrammarRustWasiNumericEnvironmentRequest442','status':'SEALED REQUEST ONLY; Root sole compile/run','request_path':str(root/'request.json'),'request_sha256':sha(root/'request.json'),'probe_path':str(root/'probe.rs'),'probe_sha256':sha(root/'probe.rs'),'launcher_path':str(root/'launch.py'),'launcher_sha256':sha(root/'launch.py'),'link_wrapper_path':str(root/'link-wrapper.py'),'link_wrapper_sha256':sha(root/'link-wrapper.py'),'target_library_count':len(target_pins),'all_input_pin_count':len(input_pins),'rows':315,'errno_cells':3,'adapter_controls':12,'validation':request['source_validation']}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
