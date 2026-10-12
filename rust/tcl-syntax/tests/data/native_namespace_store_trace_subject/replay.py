#!/usr/bin/env python3
"""Reconfirm exact namespace-store and trace-subject inputs on six pinned providers."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--c-root',type=Path,required=True);p.add_argument('--jim-root',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--verify-only',action='store_true');a=p.parse_args();fixture=Path(__file__).resolve().parent;a.output.mkdir(parents=True,exist_ok=False);checks=[]
 for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
  receipt=json.loads((fixture/version/'receipt.json').read_text());base=a.jim_root if version=='jim' else a.c_root/('tcl'+version);header=base/'jim.h' if version=='jim' else base/'generic/tcl.h';library=base/'libjim.a' if version=='jim' else base/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a');makefile=base/'Makefile' if version=='jim' else base/'unix/Makefile';owner=base/'jim.c' if version=='jim' else base/'generic/tclVar.c';shell=base/'jimsh' if version=='jim' else base/'unix/tclsh';ns=receipt['namespace'];trace=receipt['trace']
  required=[(header,receipt['header_sha256']),(library,receipt['library_sha256']),(makefile,receipt['makefile_sha256']),(owner,receipt['source_owner_sha256']),(shell,ns['executable_sha256']),(fixture/'namespace-cases.tcl',ns['source_sha256']),(fixture/'trace-subject-probe.c',trace['probe_sha256'])]
  for name,part in [('namespace',ns),('trace',trace)]:required.extend([(fixture/version/(name+'.stdout'),part['stdout_sha256']),(fixture/version/(name+'.stderr'),part['stderr_sha256'])])
  for path,expected in required:
   if sha(path)!=expected:raise ValueError('changed exact input: '+str(path))
  if a.verify_only:checks.append({'provider':version,'executed':False,'inputs_verified':True});continue
  output=a.output/version;output.mkdir();env=os.environ.copy()
  if version!='jim':env['TCL_LIBRARY']=str(base/'library')
  n=subprocess.run([str(shell)],input=(fixture/'namespace-cases.tcl').read_bytes(),capture_output=True,timeout=30,env=env);(output/'namespace.stdout').write_bytes(n.stdout);(output/'namespace.stderr').write_bytes(n.stderr)
  flags=['-DJIM_PROBE=1','-I'+str(base)] if version=='jim' else ['-I'+str(header.parent)];libs=['-lm','-lssl','-lcrypto','-lz','-ldl'] if version=='jim' else ['-lm','-ldl','-lpthread','-lz'];exe=output/'probe';command=['cc','-std=c99',*flags,str(fixture/'trace-subject-probe.c'),str(library),*libs,'-o',str(exe)];compiled=subprocess.run(command,capture_output=True,timeout=60);(output/'compile.stdout').write_bytes(compiled.stdout);(output/'compile.stderr').write_bytes(compiled.stderr)
  if compiled.returncode:raise RuntimeError('probe compile failed: '+version)
  t=subprocess.run([str(exe)],capture_output=True,timeout=30,env=env);(output/'trace.stdout').write_bytes(t.stdout);(output/'trace.stderr').write_bytes(t.stderr)
  equal=n.returncode==ns['process_exit'] and t.returncode==trace['process_exit'] and all(sha(output/(name+'.'+stream))==part[stream+'_sha256'] for name,part in [('namespace',ns),('trace',trace)] for stream in ['stdout','stderr']);checks.append({'provider':version,'executed':True,'equal':equal,'namespace_exit':n.returncode,'trace_exit':t.returncode,'executable_sha256':sha(exe),'compile_command':command})
  if not equal:raise RuntimeError('changed exact native outcome: '+version)
 (a.output/'comparisons.json').write_text(json.dumps(checks,indent=2)+'\n')
if __name__=='__main__':main()
