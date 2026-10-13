from pathlib import Path
import subprocess,os,hashlib,json,difflib
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-native-insert-20261013')
test='''
    #[test]
    fn original_c86_insertion_matches_five_native_result_and_original_header_pairs() {
        let rows = include_str!("__TABLE__");
        let mut windows = 0;
        for line in rows.lines().filter(|line| line.starts_with("R|")) {
            let result = line.split('|').collect::<Vec<_>>();
            let case = result[1].parse::<usize>().unwrap();
            if ![5, 6, 7, 8, 29].contains(&case) { continue; }
            let prefix = format!("O|{case}|");
            let original = rows.lines().find(|line| line.starts_with(&prefix)).unwrap()
                .split('|').collect::<Vec<_>>();
            run("tcl8.6", CASES[case], &result, &original);
            windows += 1;
        }
        assert_eq!(windows, 5);
    }
'''
paths=[('runtime/rust/src/interp/native_body_artifact/native_list_operations.rs','../../../../../rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt',True),('rust/tcl-vm/src/exec/native_list_operations_tests.rs','../../../tcl-registry/tests/data/native_list_operations/8.6.18.txt',False)]
for name,table,module in paths:
 before=out/'before'/name;after=out/'after'/name
 if not after.exists():
  after.parent.mkdir(parents=True,exist_ok=True);before.parent.mkdir(parents=True,exist_ok=True);before.write_text((root/name).read_text());after.write_text(before.read_text())
 s=after.read_text();new=test.replace('__TABLE__',table)
 if module:
  mark='''}\n\n#[cfg(test)]\n#[path = "native_list_operations/original_boundary_tests.rs"]''';assert mark in s;s=s.replace(mark,new+mark)
 else:s+=new
 after.write_text(s)
# Document every public shared instruction coordinate.
p=out/'after/rust/tcl-registry/src/native_list_operations_compilation.rs';s=p.read_text();s=s.replace('''pub enum NativeListInsertionStep {
    Operand''','''pub enum NativeListInsertionStep {
    /// Evaluate this actual original or parser-expanded operand.
    Operand''').replace('''    List(usize),
    Over(usize),
    Reverse(usize),
    Range(tcl_syntax::native_compiled_index::NativeCompiledListRange),
    Concat,''','''    /// Construct a List from the last counted original object owners.
    List(usize),
    /// Retain the original stack owner at this depth.
    Over(usize),
    /// Reverse these existing stack roles without creating values.
    Reverse(usize),
    /// Reach the selected immediate range getter and storage transaction.
    Range(tcl_syntax::native_compiled_index::NativeCompiledListRange),
    /// Reach the original target-copy and source-conversion concat worker.
    Concat,''');p.write_text(s)
# New regression keeps the existing C9 Operation refusal intact.
p=out/'after/rust/tcl-registry/src/native_instruction_plan.rs';s=p.read_text();mark='''    #[test]
    fn descriptor_without_portable_instruction_cannot_become_generic()''';new='''    #[test]
    fn original_c86_insertion_retains_its_immediate_list_recipe() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
        use crate::native_list_operations_compilation::{NativeListInsertionIndex, NativeListOperationInstruction};
        let plan = project(b"linsert $args 0 interp alias {} $name {}", NativeCompilationGrammar::ListInsertion, TclVersion::V8_6).unwrap();
        assert!(matches!(plan, NativeInstructionPlan::ListOperations(NativeListOperationInstruction::Insert {
            list: Operand::Original(1),
            index: NativeListInsertionIndex::Immediate(index),
            elements,
        }) if index.encoded() == 0 && elements == (3..8).map(Operand::Original).collect::<Vec<_>>()));
        assert_eq!(project(b"linsert $args $index X", NativeCompilationGrammar::ListInsertion, TclVersion::V8_6), Err(NativeInstructionPlanUnavailable::OperandGeometry));
    }

''';assert mark in s;s=s.replace(mark,new+mark);p.write_text(s)
env=os.environ.copy();env.update(RUSTUP_HOME='/workspace/.rustup',CARGO_HOME='/workspace/.cargo');patch=[];files=[]
for p in sorted((out/'after').rglob('*.rs')):
 name=str(p.relative_to(out/'after'));subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(p)],env=env,check=True);a=(out/'before'/name).read_text();b=p.read_text();patch.extend(difflib.unified_diff(a.splitlines(True),b.splitlines(True),fromfile='a/'+name,tofile='b/'+name));files.append({'path':name,'before_sha256':hashlib.sha256(a.encode()).hexdigest(),'after_sha256':hashlib.sha256(b.encode()).hexdigest()})
(out/'implementation.patch').write_text(''.join(patch));(out/'manifest.json').write_text(json.dumps({'files':files,'status':'Formatted/parsed, uncompiled. C8.6 Native original physical burst shared by Compiler and Runtime artifact. C9 Operation refusal unchanged. Five recorded same-original result/header windows added per backend.','oracle_inputs':['rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt','rust/tcl-registry/tests/data/native_list_operations/probe.c'],'additional_experiment':'probe.tcl + c86.stdout captured original C8.6.18 exact physical instructions'},indent=2)+'\n');print('files',len(files))
