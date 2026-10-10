#!/usr/bin/env python3
"""Verify exact retained inputs/streams or reproduce one provider in fresh output."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
VERSIONS = {'tcl8.4':'8.4.20','tcl8.5':'8.5.19','tcl8.6':'8.6.18','tcl9.0':'9.0.4','tcl9.1':'9.1.0','jim':'jim'}

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def verify():
    checked = 0
    for variant in ['v1', 'v2']:
        packet = ROOT / variant / 'protocol'
        inputs = json.loads((packet / 'inputs.json').read_text())
        for item in inputs:
            source = packet / 'inputs' / (item['id'] + '.tcl')
            assert source.read_bytes().hex() == item['source_hex']
            assert digest(source) == item['source_sha256']
        for version in VERSIONS.values():
            capture = ROOT / variant / 'capture' / version
            receipt = json.loads((capture / 'receipt.json').read_text())
            assert receipt['rows'] == (capture / 'stdout.tsv').read_text().splitlines()
            for leaf, key in [('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256'),('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256')]:
                assert digest(capture / leaf) == receipt[key]
            for leaf, key in [('probe.c','probe_sha256'),('cases.h','cases_sha256'),('inputs.json','inputs_sha256'),('capture.py','runner_sha256')]:
                assert digest(packet / leaf) == receipt[key]
            assert receipt['compile_exit'] == 0 and receipt['process_exit'] == 0
            checked += 1
    print(f'PASS: {checked} original provider/variant receipts; no native or Rust launch.')

parser = argparse.ArgumentParser()
parser.add_argument('--verify-only', action='store_true')
parser.add_argument('--variant', choices=['v1','v2'], default='v2')
parser.add_argument('--provider', choices=list(VERSIONS))
parser.add_argument('--source', type=Path)
parser.add_argument('--library', type=Path)
parser.add_argument('--output', type=Path)
args = parser.parse_args()
verify()
if not args.verify_only:
    if not all([args.provider, args.source, args.library, args.output]):
        parser.error('a new launch requires provider, source, library and fresh output directory')
    version = VERSIONS[args.provider]
    packet = ROOT / args.variant / 'protocol'
    original = json.loads((ROOT / args.variant / 'capture' / version / 'receipt.json').read_text())
    jim = args.provider == 'jim'
    header = args.source / ('jim.h' if jim else 'generic/tcl.h')
    owner = args.source / ('jim.c' if jim else 'generic/tclInterp.c')
    makefile = args.source / ('Makefile' if jim else 'unix/Makefile')
    for path, key in [(header,'header_sha256'),(owner,'source_owner_sha256'),(makefile,'makefile_sha256'),(args.library,'library_sha256')]:
        if digest(path) != original[key]:
            parser.error(f'original provider input mismatch: {key}')
    args.output.mkdir(exist_ok=False)
    flags = ['-DJIM_PROBE=1','-I'+str(args.source)] if jim else ['-I'+str(header.parent),'-I'+str(args.source/'unix')]
    libraries = ['-lm','-lssl','-lcrypto','-lz','-ldl'] if jim else ['-lm','-ldl','-lpthread','-lz']
    executable = args.output / 'probe'
    command = ['cc','-std=c99',*flags,str(packet/'probe.c'),str(args.library),*libraries,'-o',str(executable)]
    build = subprocess.run(command,capture_output=True,timeout=60)
    (args.output/'compile.stdout').write_bytes(build.stdout)
    (args.output/'compile.stderr').write_bytes(build.stderr)
    receipt = {'provider':args.provider,'variant':args.variant,'compile_command':command,'compile_exit':build.returncode,'header_sha256':digest(header),'library_sha256':digest(args.library),'makefile_sha256':digest(makefile),'source_owner_sha256':digest(owner),'probe_sha256':digest(packet/'probe.c'),'inputs_sha256':digest(packet/'inputs.json')}
    if build.returncode == 0:
        environment = os.environ.copy()
        if not jim:
            environment['TCL_LIBRARY'] = str(args.source/'library')
        result = subprocess.run([str(executable)],env=environment,capture_output=True,timeout=30)
        (args.output/'stdout.tsv').write_bytes(result.stdout)
        (args.output/'stderr').write_bytes(result.stderr)
        receipt.update(process_exit=result.returncode,stdout_sha256=digest(args.output/'stdout.tsv'),stderr_sha256=digest(args.output/'stderr'),executable_sha256=digest(executable))
    (args.output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps({k:v for k,v in receipt.items() if k.endswith('_exit')}))
    if build.returncode or receipt.get('process_exit'):
        raise SystemExit(1)
