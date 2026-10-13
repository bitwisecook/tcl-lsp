# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from pathlib import Path
import difflib, hashlib, json
root=Path(__file__).resolve().parents[5]
changes=[]
preimages={}
def transform(path, edits):
    old=(root/path).read_text(); new=old
    preimages[path]={'sha256':hashlib.sha256(old.encode()).hexdigest(),'bytes':len(old.encode())}
    for before,after in edits:
        assert new.count(before)==1,(path,before[:90],new.count(before))
        new=new.replace(before,after)
    changes.extend(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
transform('rust/tcl-compiler/src/native_lowering/ir.rs',[(
'''        /// The procedure's fully qualified name.
        qualified_name: String,
        /// The formal-parameter list exactly as written.
        params_raw: String,
        /// The body's exact source text.
        body_source: String,
''',
'''        /// Internal compiled-body selection key, never a publication operand.
        entry_key: String,
        /// Already evaluated original arguments, including the command head.
        argv: Vec<NativeValueId>,
''')])
transform('rust/tcl-compiler/src/native_lowering/lower.rs',[
('self.lower_definition(invoke)', 'self.lower_definition(invoke, &argv)'),
('fn lower_definition(&mut self, invoke: &GenericInvoke) -> Option<StatementOutcome> {','fn lower_definition(\n        &mut self,\n        invoke: &GenericInvoke,\n        argv: &[NativeValueId],\n    ) -> Option<StatementOutcome> {'),
('''        self.emit(NativeOp::DefineProc {
            qualified_name: procedure.qualified_name.clone(),
            params_raw: procedure.params_raw.clone(),
            body_source,
        });''',
'''        if argv.len() != 4 {
            return None;
        }
        self.max_argc = self.max_argc.max(argv.len());
        self.emit(NativeOp::DefineProc {
            entry_key: procedure.qualified_name.clone(),
            argv: argv.to_vec(),
        });''')])
transform('rust/tcl-compiler/src/codegen/wasm/native_emit.rs',[
('    proc_define_native: u32,','    proc_define_original_argv: u32,'),
('proc_define_native: add(wasm, CodegenAbiImportId::ProcDefineNative),','proc_define_original_argv: add(wasm, CodegenAbiImportId::ProcDefineOriginalArgv),'),
('NativeOp::Invoke { argv } | NativeOp::NestedInvoke { argv, .. } => argv.len(),','NativeOp::Invoke { argv }\n                | NativeOp::NestedInvoke { argv, .. }\n                | NativeOp::DefineProc { argv, .. } => argv.len(),'),
('''            NativeOp::DefineProc {
                qualified_name,
                params_raw,
                body_source,
            } => {
                self.text_pair(qualified_name);
                self.text_pair(params_raw);
                self.text_pair(body_source);
                self.push_entry_index(qualified_name);
                self.call(self.imports.proc_define_native);
                self.fail_on_code(completion);
            }''',
'''            NativeOp::DefineProc { entry_key, argv } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.push_entry_index(entry_key);
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.proc_define_original_argv);
                self.push(WasmOp::Drop);
                self.adopt_frame_completion(completion);
                self.release_scratch();
            }''')])
backend=(root/'rust/tcl-compiler/src/codegen/wasm/backend.rs').read_text()
start=backend.index('            SemanticOperationId::StructuredLowering(LoweringHookId::Proc) => {',backend.index('fn try_emit_direct_operation'))
end=backend.index('            // There is no `puts` fast path',start)
transform('rust/tcl-compiler/src/codegen/wasm/backend.rs',[
(backend[start:end],'''            SemanticOperationId::StructuredLowering(LoweringHookId::Proc) => false,
'''),
('''                NativeOp::DefineProc { qualified_name, .. } => {
                    out.insert(qualified_name.clone());
                }''','''                NativeOp::DefineProc { entry_key, .. } => {
                    out.insert(entry_key.clone());
                }''')])
transform('rust/tcl-runtime-api/src/codegen_abi.rs',[
('''    ProcDefineNative,
    /// Log one''','''    ProcDefineNative,
    /// Define a procedure from borrowed, already evaluated original argv.
    /// Parameters are argv, argc, optional compiled entry, and completion out.
    ProcDefineOriginalArgv,
    /// Log one'''),
('''        Self::ProcDefineNative,
        Self::LogCommand,''','''        Self::ProcDefineNative,
        Self::ProcDefineOriginalArgv,
        Self::LogCommand,'''),
('''            Self::LogCommand => tcl_import("tcl_codegen_log_command", I32_I32_I32, NONE),''','''            Self::ProcDefineOriginalArgv => tcl_import(
                "tcl_codegen_proc_define_original_argv",
                I32_I32_I32_I32,
                I32,
            ),
            Self::LogCommand => tcl_import("tcl_codegen_log_command", I32_I32_I32, NONE),''')])
transform('runtime/rust/src/cmd_proc.rs',[
('''fn proc_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments''','''fn proc_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    proc_cmd_with_native_entry(interp, argv, None)
}

pub(crate) fn proc_cmd_with_native_entry(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    native: Option<crate::interp::NativeProcEntry>,
) -> Code {
    let arguments'''),
('proc_cmd_original(interp, argv, &arguments)', 'proc_cmd_original(interp, argv, &arguments, native)'),
('''    arguments: &OriginalArguments,
) -> Code {
    use tcl_registry::native_procedure''','''    arguments: &OriginalArguments,
    native: Option<crate::interp::NativeProcEntry>,
) -> Code {
    use tcl_registry::native_procedure'''),
('''        (original_body, chosen_body),
        None,
        statics,''','''        (original_body, chosen_body),
        native,
        statics,''')])
abi=(root/'runtime/rust/src/codegen_abi.rs').read_text()
fn_start=abi.index('pub unsafe extern "C" fn tcl_invoke_argv(')
fn_end=abi.index('\n}\n',fn_start)+3
addition=abi[fn_start:fn_end].replace('tcl_invoke_argv','tcl_codegen_proc_define_original_argv').replace('''    argc: i32,
    out:''','''    argc: i32,
    entry: Option<NativeProcEntry>,
    out:''').replace('''let code = operation.interpreter.dispatch(words);''','''let code = crate::cmd_proc::proc_cmd_with_native_entry(
        &mut operation.interpreter,
        words,
        entry,
    );''')
addition='''/// Define from borrowed evaluated argv through the original procedure handler.
/// The caller retains every argument and supplies a checked compiled entry.
///
/// # Safety
/// `argv` must hold `argc` live caller-owned objects and `out` must be writable
/// completion storage. A present entry must satisfy `NativeProcEntry`.
#[no_mangle]
'''+addition+'\n'
transform('runtime/rust/src/codegen_abi.rs',[(
'''/// Release both owned object references in a [`TclCompletionAbi`] and reset its''',
addition+'''/// Release both owned object references in a [`TclCompletionAbi`] and reset its''')])
Path('/tmp/codegen-original-argv-repair.patch').write_text(''.join(changes))
Path('/tmp/codegen-original-argv-repair-preimages.json').write_text(json.dumps(preimages,indent=2)+'\n')
print('Prepared scratch implementation diff for seven source files; no repository files modified.')
