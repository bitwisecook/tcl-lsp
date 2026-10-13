from pathlib import Path
import hashlib,json,difflib,subprocess,os
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-native-insert-20261013');records=[]
def edit(name,fn):
 a=(root/name).read_text();b=fn(a);assert a!=b,name
 p=out/'after'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(b);q=out/'before'/name;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(a)
 env=os.environ.copy();env.update(RUSTUP_HOME='/workspace/.rustup',CARGO_HOME='/workspace/.cargo');subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(p)],env=env,check=True);b=p.read_text();records.append((name,a,b))
def geometry(s):
 mark='/// Complete native list stack recipe, independent of its registration issuer.\n'
 add='''/// Original C8.6 insertion burst, preserving its List conversion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListInsertionStep {
    Operand(NativeCompilerWordOperand),
    List(usize),
    Over(usize),
    Reverse(usize),
    Range(tcl_syntax::native_compiled_index::NativeCompiledListRange),
    Concat,
}

/// Project only the retained C8.6 immediate insertion recipe into physical steps.
/// No command registration, operand value or execution permission is inferred.
#[must_use]
pub fn c86_list_insertion_steps(
    list: NativeCompilerWordOperand,
    index: NativeListInsertionIndex,
    elements: Vec<NativeCompilerWordOperand>,
) -> Option<Vec<NativeListInsertionStep>> {
    use NativeListInsertionStep as Step;
    let NativeListInsertionIndex::Immediate(index) = index else { return None; };
    let index = index.encoded();
    let count = elements.len();
    let mut steps = vec![Step::Operand(list)];
    steps.extend(elements.into_iter().map(Step::Operand));
    steps.push(Step::List(count));
    match index {
        0 => steps.push(Step::Reverse(2)),
        -2 => {},
        index => {
            let (last, first) = if index > 0 {
                (index.checked_sub(1)?, index)
            } else {
                (index, index.checked_add(1)?)
            };
            steps.extend([
                Step::Over(1),
                Step::Range(tcl_syntax::native_compiled_index::NativeCompiledListRange {
                    first: NativeCompiledListIndex::from_encoded(0),
                    last: NativeCompiledListIndex::from_encoded(last),
                }),
                Step::Reverse(3),
                Step::Range(tcl_syntax::native_compiled_index::NativeCompiledListRange {
                    first: NativeCompiledListIndex::from_encoded(first),
                    last: NativeCompiledListIndex::from_encoded(-2),
                }),
                Step::Concat,
            ]);
        }
    }
    steps.push(Step::Concat);
    Some(steps)
}

'''
 assert mark in s;return s.replace(mark,add+mark)
edit('rust/tcl-registry/src/native_list_operations_compilation.rs',geometry)
def routing(s):
 mark='''        NativeCompilationGrammar::ListIndex => {''';add='''        NativeCompilationGrammar::ListInsertion if version == TclVersion::V8_6 => {
            crate::native_list_operations_compilation::compile_native_list_operation(
                words,
                operand_from,
                version,
                crate::native_list_operations_compilation::NativeListOperationKind::Insert,
            )
            .map(NativeInstructionPlan::ListOperations)
            .map_err(|_| Unavailable::OperandGeometry)
        }
''';assert mark in s;return s.replace(mark,add+mark,1)
edit('rust/tcl-registry/src/native_instruction_plan.rs',routing)
def emitter(s):
 old='''            Plan::Insert { .. } | Plan::Set { .. } => return false,''';new='''            Plan::Insert { list, index, elements } => {
                use tcl_registry::native_list_operations_compilation::{
                    c86_list_insertion_steps, NativeListInsertionStep as Step,
                };
                let Some(steps) = c86_list_insertion_steps(list, index, elements) else {
                    return false;
                };
                for step in steps {
                    match step {
                        Step::Operand(operand) => {
                            if !Self::list_operand(command, operand, operations) {
                                return false;
                            }
                        }
                        Step::List(count) => operations.push(operation(Op::LIST, &[super::super::bytecode_imm(count)])),
                        Step::Over(depth) => operations.push(operation(Op::OVER, &[super::super::bytecode_imm(depth)])),
                        Step::Reverse(count) => operations.push(operation(Op::REVERSE, &[super::super::bytecode_imm(count)])),
                        Step::Range(range) => operations.push(Task::NativeListRange(range)),
                        Step::Concat => operations.push(operation(Op::LIST_CONCAT, &[])),
                    }
                }
            }
            Plan::Set { .. } => return false,''';assert old in s;return s.replace(old,new)
edit('rust/tcl-compiler/src/codegen/native_list_operations.rs',emitter)
patch=[]
for name,a,b in records:
 patch.extend(difflib.unified_diff(a.splitlines(True),b.splitlines(True),fromfile='a/'+name,tofile='b/'+name))
(out/'implementation.patch').write_text(''.join(patch));(out/'manifest.json').write_text(json.dumps({'files':[{'path':name,'before_sha256':hashlib.sha256(a.encode()).hexdigest(),'after_sha256':hashlib.sha256(b.encode()).hexdigest()} for name,a,b in records],'status':'Parsed/format only; uncompiled. Native C8.6 routing and physical burst. C9 refusal unchanged. Runtime native-body Insert producer still pending.'},indent=2)+'\n')
