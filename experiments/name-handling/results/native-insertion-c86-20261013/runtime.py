from pathlib import Path
import subprocess,os,difflib,hashlib,json
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-native-insert-20261013');name='runtime/rust/src/interp/native_body_artifact/native_list_operations.rs';a=(root/name).read_text();s=a
s=s.replace('''    // Insert and Set require the concrete replacement and nested path-COW owners.
''','''    Insert { steps: Vec<InsertionStep> },
''')
mark='''pub(super) struct ListOperationsOperation {''';add='''enum InsertionStep {
    Operand(NamespaceOperand),
    List(usize),
    Over(usize),
    Reverse(usize),
    Range(NativeCompiledListRange),
    Concat,
}
''';assert mark in s;s=s.replace(mark,add+mark)
mark='''            _ => return Err((unavailable("original list replacement storage transaction")).into()),''';add='''            Plan::Insert { list, index, elements } => {
                use tcl_registry::native_list_operations_compilation::{
                    c86_list_insertion_steps, NativeListInsertionStep as Step,
                };
                let steps = c86_list_insertion_steps(list, index, elements)
                    .ok_or_else(|| unavailable("original list insertion instruction"))?;
                let mut prepared = Vec::with_capacity(steps.len());
                for step in steps {
                    prepared.push(match step {
                        Step::Operand(operand) => InsertionStep::Operand(
                            self.namespace_operand(words, &operand, &mut prepared_words, depth)?,
                        ),
                        Step::List(count) => InsertionStep::List(count),
                        Step::Over(depth) => InsertionStep::Over(depth),
                        Step::Reverse(count) => InsertionStep::Reverse(count),
                        Step::Range(range) => InsertionStep::Range(range),
                        Step::Concat => InsertionStep::Concat,
                    });
                }
                ListOperation::Insert { steps: prepared }
            }
''';assert mark in s;s=s.replace(mark,add+mark)
mark='''                ListOperation::Assign { list, targets } => {''';add='''                ListOperation::Insert { steps } => {
                    let mut stack = Vec::<obj::Owned>::new();
                    for step in steps {
                        current().map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                        match step {
                            InsertionStep::Operand(operand) => {
                                stack.push(self.body_namespace_operand(artifact, command, operand, execution)?);
                                current().map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                                if execution.done { return Ok(None); }
                            }
                            InsertionStep::List(count) => {
                                let values = stack.split_off(stack.len().checked_sub(*count).expect("native insertion List stack"));
                                let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                                stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(&pointers, protocol)));
                            }
                            InsertionStep::Over(depth) => {
                                let index = stack.len().checked_sub(depth + 1).expect("native insertion OVER stack");
                                stack.push(obj::Owned::retain(stack[index].as_ptr()));
                            }
                            InsertionStep::Reverse(count) => {
                                let start = stack.len().checked_sub(*count).expect("native insertion REVERSE stack");
                                stack[start..].reverse();
                            }
                            InsertionStep::Range(range) => {
                                let original = stack.pop().expect("native insertion range stack");
                                let headers = crate::interp::native_arguments::OriginalArguments::enter(self, &[original.as_ptr()])
                                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                                let reached = crate::list::native_list_range_using(
                                    original.as_ptr(), *range, protocol,
                                    &|| { current()?; headers.check_for(&actual) },
                                    &|error| variable_original.refuse(error),
                                );
                                let reached = reached.map_err(|error| self.report_cmd_error_using(error, Some(&current)));
                                drop(headers);
                                drop(original);
                                current().map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                                stack.push(reached?);
                            }
                            InsertionStep::Concat => {
                                let source = stack.pop().expect("native insertion List source");
                                let target = stack.pop().expect("native insertion List target");
                                let headers = crate::interp::native_arguments::OriginalArguments::enter(self, &[target.as_ptr(), source.as_ptr()])
                                    .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                                let reached = crate::list::concatenate_native_lists_using(
                                    target.as_ptr(), source.as_ptr(), protocol,
                                    &|| { current()?; headers.check_for(&actual) },
                                    &|error| variable_original.refuse(error),
                                );
                                let reached = reached.map_err(|error| self.report_cmd_error_using(error, Some(&current)));
                                drop(headers);
                                drop(source);
                                drop(target);
                                current().map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
                                stack.push(reached?);
                            }
                        }
                    }
                    debug_assert_eq!(stack.len(), 1);
                    stack.pop().expect("native insertion result")
                }
''';assert mark in s;s=s.replace(mark,add+mark)
p=out/'after'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(s);q=out/'before'/name;q.parent.mkdir(parents=True,exist_ok=True);q.write_text(a);env=os.environ.copy();env.update(RUSTUP_HOME='/workspace/.rustup',CARGO_HOME='/workspace/.cargo');subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(p)],env=env,check=True)
