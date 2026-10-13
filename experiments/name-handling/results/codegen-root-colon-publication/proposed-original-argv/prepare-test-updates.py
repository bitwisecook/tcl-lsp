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
def backend_tests(s):
    start=s.index('    /// A definition whose body word is a substitution must not be registered')
    end=s.index('    #[test]\n    fn host_script_admission_boxes_the_complete_original_byte_image',start)
    s=s[:start]+s[end:]
    before='''        assert!(
            !proc_definition_is_written_out(&unit.ir_module, definition.0, definition.1),
            "a substituted body word must keep the generic invocation",
        );'''
    after='''        let facts = function_facts(
            enclosing,
            &unit.ir_module,
            &registry,
            &unit.command_mutations,
            false,
        );
        let plan = facts.leaf_invocations.get(&span_key(definition.0))
            .expect("the original variable word has a generic argv plan");
        let [_, _, _, body] = plan.root.words.as_slice() else {
            panic!("the original four-word vector survives: {:?}", plan.root.words);
        };
        assert!(matches!(body, WasmWordPlan::Scalar { name, .. } if name == "body"));
        assert!(!facts.leaf_declines.contains_key(&span_key(definition.0)));'''
    assert s.count(before)==1
    s=s.replace(before,after)
    start=s.index('    /// The same proof, on a body that *is* written out, still binds')
    end=s.index('    /// A leaf command whose words all compile selects a plan',start)
    replacement='''    #[test]
    fn a_written_out_definition_body_keeps_original_generic_argv() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        let unit = retained_source_unit(
            std::sync::Arc::clone(&context),
            "proc make {} { proc p {x} {return $x} }; make\\n",
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let procedure = unit.ir_module.procedures.get("::p").expect("procedure");
        let enclosing = unit.procedures.get("::make").expect("the enclosing body");
        let definition = proc_statement_tokens(enclosing)
            .into_iter()
            .find(|(span, _)| *span == procedure.span)
            .expect("the original definition keeps its own span");
        let facts = function_facts(
            enclosing,
            &unit.ir_module,
            registry,
            &unit.command_mutations,
            false,
        );
        let plan = facts.leaf_invocations.get(&span_key(definition.0))
            .expect("the original literal definition retains its generic argv");
        let [head, name, parameters, body] = plan.root.words.as_slice() else {
            panic!("one complete original vector: {:?}", plan.root.words);
        };
        assert!(matches!(head, WasmWordPlan::Literal { text, .. } if text == "proc"));
        assert!(matches!(name, WasmWordPlan::Literal { text, .. } if text == "p"));
        assert!(matches!(parameters, WasmWordPlan::Literal { text, .. } if text == "x"));
        assert!(matches!(body, WasmWordPlan::Literal { text, .. } if text == "return $x"));
        assert!(!facts.leaf_declines.contains_key(&span_key(definition.0)));
    }

    #[test]
    fn a_top_level_written_out_definition_retains_original_name_operand() {
        let facts = top_level_facts("proc :audit {x} {return $x}\\n");
        assert_eq!(facts.leaf_invocations.len(), 1);
        let plan = facts.leaf_invocations.values().next().expect("one original argv plan");
        let [_, name, _, body] = plan.root.words.as_slice() else {
            panic!("one complete original vector: {:?}", plan.root.words);
        };
        assert!(matches!(name, WasmWordPlan::Literal { text, .. } if text == ":audit"));
        assert!(matches!(body, WasmWordPlan::Literal { text, .. } if text == "return $x"));
        assert!(facts.leaf_declines.is_empty());
    }

'''
    return s[:start]+replacement+s[end:]
change('rust/tcl-compiler/src/codegen/wasm/backend.rs',backend_tests)

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
