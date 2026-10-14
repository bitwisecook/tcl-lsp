from pathlib import Path
import difflib,hashlib,json
root=Path('/workspace/tcl-lsp'); out=Path('/tmp/name-compiler19-corrections-20261014')
paths=['rust/tcl-compiler/src/cfg_builder/global_write_info.rs','rust/tcl-compiler/src/cfg_builder/upvar_info.rs','rust/tcl-compiler/src/analyser/diagnostics/tests.rs','rust/tcl-compiler/src/cfg_builder/mod.rs','rust/tcl-compiler/src/compilation_unit.rs','rust/tcl-compiler/src/command_binding.rs']
for p in paths:
 text=(root/p).read_text(); before=out/'before'/p; before.parent.mkdir(parents=True,exist_ok=True);before.write_text(text)
 if p.endswith('global_write_info.rs'):
  text=text.replace('mod tests {\n    use super::*;','mod tests {\n    use super::*;\n    use tcl_registry::{ArgRole, CommandRegistry, CommandSpec, StateTransitionDescriptor};',1)
 elif p.endswith('upvar_info.rs'):
  marker='        let selected = crate::cfg_builder::detect_upvar_procs_with_registry(&module, registry);\n'
  assert text.count(marker)==1
  text=text.replace(marker,'        let body_id = crate::test_body_identity::fixture_id(&module.procedures, "::helper");\n'+marker)
  for name in ['selected','unavailable','missing']:text=text.replace(name+'["::helper"]',name+'[&body_id]')
 elif p.endswith('diagnostics/tests.rs'):
  import re
  text,n=re.subn(r'(\s+req.command_bindings.clone\(\),\n)(\s+)(\),)',lambda m:m[1]+m[2]+'    req.procedure_summary_owners.clone(),\n'+m[2]+m[3],text)
  assert n==2,n
 elif p.endswith('cfg_builder/mod.rs'):
  start=text.index('    fn opaque_call_barrier(');end=text.index('\n    fn ',start+5)
  part=text[start:end];assert part.count('            canonical_command,\n')==1
  text=text[:start]+part.replace('            canonical_command,\n','')+text[end:]
 elif p.endswith('compilation_unit.rs'):
  text=text.replace('(ir_module, cfg_module, prepared): (&\'a IrModule, &\'a CfgModule, &\'a PreparedCfgContext)', '(ir_module, cfg_module, _prepared): (&\'a IrModule, &\'a CfgModule, &\'a PreparedCfgContext)',1)
 else:
  text=text.replace('            insert_rebound_candidates(name, namespace, &mut self.proc_rebound_names);','            self.proc_rebound_names.extend(\n                tcl_syntax::naming::bareword_resolution_candidates(namespace, name)\n                    .into_iter()\n                    .map(|candidate| nqn(&candidate)),\n            );',1)
  start=text.index('/// Record every name a bare `rename` / `interp alias` argument could')
  end=text.index('/// Whether `script` declares anything',start)
  text=text[:start]+text[end:]
  marker='    #[test]\n    fn trusts_proc_binding_false_for_namespace_relative_rename()'
  test='''    #[test]
    fn rebound_candidates_include_relative_qualified_names() {
        let namespace = crate::ir::ExecutionNamespace::exact("::ns");
        for (written, expected) in [
            ("inner::p", vec!["inner::p", "ns::inner::p"]),
            ("p", vec!["ns::p", "p"]),
            ("::inner::p", vec!["inner::p"]),
        ] {
            let mut bindings = ModuleCommandBindings::default();
            bindings.record_proc_rebound_candidates(written, &namespace);
            assert_eq!(
                bindings.proc_rebound_names,
                expected.into_iter().map(String::from).collect()
            );
            assert!(bindings.bindings.is_empty());
        }
    }

'''
  assert marker in text;text=text.replace(marker,test+marker,1)
 after=out/'after'/p;after.parent.mkdir(parents=True,exist_ok=True);after.write_text(text)
