from pathlib import Path
import difflib,subprocess,hashlib,json
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-compiler20-warning-cleanup-20261014')
paths=['rust/tcl-compiler/src/optimiser/manager.rs','rust/tcl-compiler/src/interprocedural.rs','rust/tcl-compiler/src/cfg_builder/upvar_info.rs','rust/tcl-compiler/src/command_binding.rs']
for p in paths:
 before=(root/p).read_text();after=before
 if p.endswith('manager.rs'):after=after.replace('    use std::collections::HashSet;\n','',1)
 elif p.endswith('upvar_info.rs'):
  start=after.index('/// Compose a callee\'s caller-frame effects into `info`');end=after.index('#[cfg(test)]',start);after=after[:start]+after[end:]
 elif p.endswith('command_binding.rs'):
  start=after.index('    /// Every source spelling explicitly represented by the closed may-state.');end=after.index('    /// Current changed cells',start);after=after[:start]+after[end:]
 else:
  for line in ['    caller: &\'a str,\n','        caller: mqname,\n','        caller: qname,\n','            caller: "::top",\n','    mqname: &\'a str,\n','                    mqname: &mqname,\n','        mqname,\n','    qname: &\'a str,\n','                qname,\n','        qname,\n','        let qname = proc.qualified_name.as_str();\n']:
   assert after.count(line)==1,(line,after.count(line));after=after.replace(line,'',1)
  start=after.index('                // A lambda body runs in a fresh call frame')
  end=after.index('                scan_source_for_calls(body_text, lambda_ctx, facts, depth + 1);',start)+len('                scan_source_for_calls(body_text, lambda_ctx, facts, depth + 1);')
  after=after[:start]+'                scan_source_for_calls(body_text, ctx, facts, depth + 1);'+after[end:]
  old='''    // *written* head stays in use where the word is not a command binding at
    // all: `resolve_internal_call` matches a procedure qname, and the
    // instance-dispatch arm reads a `$receiver` variable out of it.'''
  new='''    // The written head retains a `$receiver` variable for instance dispatch;
    // procedure targets require their original source receipts.'''
  assert old in after;after=after.replace(old,new)
 for stage,text in [('before',before),('after',after)]:
  dest=out/stage/p;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(text)
 env={'RUSTUP_HOME':'/workspace/.rustup','CARGO_HOME':'/workspace/.cargo','PATH':'/usr/bin:/bin'}
 subprocess.run(['/workspace/.cargo/bin/rustfmt','--edition','2024','--config','skip_children=true',str(out/'after'/p)],env=env,check=True)
patch=''.join(''.join(difflib.unified_diff((out/'before'/p).read_text().splitlines(keepends=True),(out/'after'/p).read_text().splitlines(keepends=True),fromfile='a/'+p,tofile='b/'+p)) for p in paths)
(out/'implementation.patch').write_text(patch)
r=subprocess.run(['git','apply','--check',str(out/'implementation.patch')],cwd=root,capture_output=True,text=True)
manifest={'validation':'rustfmt parsed; no Cargo/test execution','apply_check':r.returncode,'before':{p:hashlib.sha256((out/'before'/p).read_bytes()).hexdigest() for p in paths},'after':{p:hashlib.sha256((out/'after'/p).read_bytes()).hexdigest() for p in paths},'patch_sha256':hashlib.sha256(patch.encode()).hexdigest()}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(len(patch),manifest['patch_sha256'],r.returncode,r.stderr)
