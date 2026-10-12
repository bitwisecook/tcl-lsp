#!/usr/bin/env python3
"""Run the retained ASCII empty-array-root controls against exact shell inputs."""
import argparse, hashlib, json, os
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--c-root', type=Path, required=True)
    parser.add_argument('--jim-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    args.output.mkdir(parents=True, exist_ok=False)
    comparisons = []
    for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
        receipt = json.loads((fixture/version/'receipt.json').read_text())
        base = args.jim_root if version=='jim' else args.c_root/('tcl'+version)
        executable = base/'jimsh' if version=='jim' else base/'unix/tclsh'
        header = base/'jim.h' if version=='jim' else base/'generic/tcl.h'
        owner = base/'jim.c' if version=='jim' else base/'generic/tclParse.c'
        makefile = base/'Makefile' if version=='jim' else base/'unix/Makefile'
        for path,key in [(fixture/'cases.tcl','source_sha256'),(executable,'executable_sha256'),(header,'header_sha256'),(owner,'source_owner_sha256'),(makefile,'makefile_sha256'),(fixture/version/'stdout','stdout_sha256'),(fixture/version/'stderr','stderr_sha256')]:
            if digest(path)!=receipt[key]:
                raise ValueError('changed exact input: '+str(path))
        if args.verify_only:
            comparisons.append({'provider':version,'inputs_verified':True,'executed':False})
            continue
        output = args.output/version
        output.mkdir()
        environment = os.environ.copy()
        if version!='jim':environment['TCL_LIBRARY']=str(base/'library')
        process = subprocess.run([str(executable)],input=(fixture/'cases.tcl').read_bytes(),capture_output=True,timeout=30,env=environment,check=False)
        (output/'stdout').write_bytes(process.stdout)
        (output/'stderr').write_bytes(process.stderr)
        equal = process.returncode==receipt['process_exit'] and digest(output/'stdout')==receipt['stdout_sha256'] and digest(output/'stderr')==receipt['stderr_sha256']
        comparisons.append({'provider':version,'executed':True,'process_exit':process.returncode,'stdout_sha256':digest(output/'stdout'),'stderr_sha256':digest(output/'stderr'),'equal':equal})
        if not equal:raise RuntimeError('changed exact guest result: '+version)
    (args.output/'comparisons.json').write_text(json.dumps(comparisons,indent=2)+'\n')


if __name__=='__main__':main()
