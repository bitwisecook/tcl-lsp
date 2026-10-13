# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from pathlib import Path
import difflib, hashlib, json
root=Path(__file__).resolve().parents[5]
changes=[]
preimages={}
def change(path, fn):
 old=(root/path).read_text();new=fn(old)
 preimages[path]={'sha256':hashlib.sha256(old.encode()).hexdigest(),'bytes':len(old.encode())}
 assert old!=new,path
 changes.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
def native_tests(s):
 before='''    assert_eq!(
        defines,
        vec![&NativeOp::DefineProc {
            qualified_name: "::greet".into(),
            params_raw: "name".into(),
            body_source: " return hi ".into(),
        }],
        "the definition carries the front end's own name, params and body text"
    );'''
 after='''    let [NativeOp::DefineProc { entry_key, argv }] = defines.as_slice() else {
        panic!("the exact definition retains one original argv: {defines:?}");
    };
    assert_eq!(entry_key, "::greet");
    assert_eq!(argv.len(), 4);
    assert_eq!(original_constant(&function, argv[0]), "proc");
    assert_eq!(original_constant(&function, argv[1]), "greet");
    assert_eq!(original_constant(&function, argv[2]), "name");
    assert_eq!(original_constant(&function, argv[3]), " return hi ");'''
 assert s.count(before)==1
 return s.replace(before,after)+'\n'+Path(__file__).with_name('compiler-controls.rs').read_text()
change('rust/tcl-compiler/src/native_lowering/tests.rs',native_tests)
def pipeline(s):
 old='''        assert_eq!(
            calls_to(&output.module, CodegenAbiImportId::ProcDefineNative),
            0
        );'''
 assert s.count(old)==1
 s=s.replace(old,old+'''
        assert_eq!(
            calls_to(&output.module, CodegenAbiImportId::ProcDefineOriginalArgv),
            0
        );''')
 old='''        assert_eq!(calls_to(&output.module, CodegenAbiImportId::EvalCode), 0);
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::VarSet), 0);
        assert!(
            output.module.functions[0]'''
 assert s.count(old)==1
 s=s.replace(old,'''        assert_eq!(calls_to(&output.module, CodegenAbiImportId::EvalCode), 0);
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::VarSet), 0);
        assert_eq!(
            calls_to(&output.module, CodegenAbiImportId::ProcDefineOriginalArgv),
            0
        );
        assert!(
            output.module.functions[0]''')
 old='''        // ProcDefineNative with no exported table entry registers a source-only
        // body, as does ProcRegister. Its host entry performs pre-formal admission.'''
 assert s.count(old)==1
 s=s.replace(old,'''        // A definition without a compiled table entry keeps the original source
        // body. Its host entry retains independent pre-formal admission.''')
 old='''                + calls_to(&output.module, CodegenAbiImportId::ProcDefineNative)
                > 0'''
 assert s.count(old)==1
 return s.replace(old,'''                + calls_to(&output.module, CodegenAbiImportId::ProcDefineNative)
                + calls_to(&output.module, CodegenAbiImportId::ProcDefineOriginalArgv)
                > 0''')
change('rust/tcl-compiler/src/codegen/wasm/pipeline.rs',pipeline)
def wasm(s):
 s=s.replace('CodegenAbiImportId::ProcDefineNative','CodegenAbiImportId::ProcDefineOriginalArgv')
 old='''    assert!(wat.contains(r#""tcl_codegen_proc_register""#), "{wat}");'''
 assert s.count(old)==1
 s=s.replace(old,'''    assert_eq!(import_calls(&wat, "tcl_codegen_proc_register"), 0, "{wat}");''')
 old='''    // The procedure and both assignments have proven direct forms. `puts`
    // under the analysis tier always dispatches through prebuilt argv — no
    // fast path re-parses compatibility text — and the direct `[add …]` call
    // is retired along with it, so the last statement is two generic
    // prebuilt-argv invocations — the nested `add` and the `puts`; the native
    // tier lowers the latter to its `Puts` intrinsic and native lowering gives the former
    // its table-installed direct call.
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 2, "{wat}");'''
 assert s.count(old)==1
 return s.replace(old,'''    // Both assignments retain their proven direct forms. The procedure
    // declaration, nested add call and puts dispatch with original evaluated argv.
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 3, "{wat}");''')
change('rust/tcl-compiler/tests/wasm_codegen.rs',wasm)
change('rust/tcl-compiler/tests/wasm_real_link.rs',lambda s:s.replace('CodegenAbiImportId::ProcDefineNative','CodegenAbiImportId::ProcDefineOriginalArgv').replace('`tcl_codegen_proc_define_native`','`tcl_codegen_proc_define_original_argv`'))
def api(s):
 before='''    #[test]
    fn native_proc_dispatch_imports_carry_the_entry_index_and_a_test_counter()'''
 assert s.count(before)==1
 after='''    #[test]
    fn original_proc_definition_borrows_argv_and_writes_completion() {
        use CodegenAbiValueType::I32;
        let import = CodegenAbiImportId::ProcDefineOriginalArgv.descriptor();
        assert_eq!(import.module, "tcl");
        assert_eq!(import.name, "tcl_codegen_proc_define_original_argv");
        assert_eq!(import.parameters, &[I32; 4][..]);
        assert_eq!(import.results, &[I32][..]);
        assert!(CodegenAbiImportId::ALL.contains(&CodegenAbiImportId::ProcDefineOriginalArgv));
        assert!(CodegenAbiImportId::ProcDefineOriginalArgv.requires_host_refusal_check());
        let removed: Vec<_> = DESCRIPTORS.iter().copied().filter(|descriptor| {
            descriptor.name != import.name
        }).collect();
        assert_ne!(fingerprint(&removed, &LAYOUT), CODEGEN_ABI_VERSION);
    }

'''+before
 return s.replace(before,after)
change('rust/tcl-runtime-api/src/codegen_abi.rs',api)
Path('/tmp/codegen-original-argv-test-updates.patch').write_text(''.join(changes))
Path('/tmp/codegen-original-argv-test-preimages.json').write_text(json.dumps(preimages,indent=2)+'\n')
print('Prepared scratch test diff: existing definition assertion, both admission-refusal controls, lazy body import allowance, native counters/real link, descriptor/refusal/fingerprint.')
