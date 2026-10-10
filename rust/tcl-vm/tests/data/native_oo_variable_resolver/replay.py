#!/usr/bin/env python3
"""Replay the exact original counted TclOO variable resolver probe."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--c-root',type=Path,required=True)
    parser.add_argument('--jim-root',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--verify-only',action='store_true')
    args=parser.parse_args();fixture=Path(__file__).resolve().parent
    args.output.mkdir(parents=True,exist_ok=False);checks=[]
    for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
        receipt=json.loads((fixture/version/'receipt.json').read_text());base=args.jim_root if version=='jim' else args.c_root/('tcl'+version)
        probe=fixture/'probe.c';header=base/'jim.h' if version=='jim' else base/'generic/tcl.h'
        library=base/'libjim.a' if version=='jim' else base/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a')
        makefile=base/'Makefile' if version=='jim' else base/'unix/Makefile'
        owner=base/'jim.c' if version=='jim' else base/('generic/tclCmdMZ.c' if version in ['8.4.20','8.5.19'] else 'generic/tclOOMethod.c')
        for path,key in [(probe,'probe_sha256'),(header,'header_sha256'),(library,'library_sha256'),(makefile,'makefile_sha256'),(owner,'source_owner_sha256'),(fixture/version/'stdout.tsv','stdout_sha256'),(fixture/version/'stderr','stderr_sha256')]:
            if sha(path)!=receipt[key]:raise ValueError('changed exact input '+str(path))
        if args.verify_only:
            checks.append(dict(provider=version,executed=False,exact_inputs_verified=True));continue
        dest=args.output/version;dest.mkdir();exe=dest/'probe'
        flags=['-DJIM_PROBE=1','-I'+str(base)] if version=='jim' else ['-I'+str(header.parent),'-I'+str(base/'unix')]
        libs=['-lm','-lssl','-lcrypto','-lz','-ldl'] if version=='jim' else ['-lm','-ldl','-lpthread','-lz']
        command=['cc','-std=c99',*flags,str(probe),str(library),*libs,'-o',str(exe)]
        compiled=subprocess.run(command,capture_output=True,timeout=60)
        (dest/'compile.stdout').write_bytes(compiled.stdout);(dest/'compile.stderr').write_bytes(compiled.stderr)
        if compiled.returncode:raise RuntimeError('harness compile failed '+version)
        env=os.environ.copy()
        if version!='jim':env['TCL_LIBRARY']=str(base/'library')
        result=subprocess.run([str(exe)],capture_output=True,timeout=30,env=env)
        (dest/'stdout.tsv').write_bytes(result.stdout);(dest/'stderr').write_bytes(result.stderr)
        equal=result.returncode==receipt['process_exit'] and sha(dest/'stdout.tsv')==receipt['stdout_sha256'] and sha(dest/'stderr')==receipt['stderr_sha256']
        checks.append(dict(provider=version,executed=True,equal=equal,compile_command=command,executable_sha256=sha(exe),process_exit=result.returncode))
        (args.output/'comparisons.json').write_text(json.dumps(checks,indent=2)+'\n')
        if not equal:raise RuntimeError('changed native result '+version)
    (args.output/'comparisons.json').write_text(json.dumps(checks,indent=2)+'\n')

if __name__=='__main__':main()
