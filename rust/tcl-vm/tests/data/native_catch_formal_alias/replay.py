#!/usr/bin/env python3
"""Compare the exact caught-error alias controls with six retained shell runs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha(path):
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
    results = []
    for receipt in json.loads((fixture / 'receipt.json').read_text()):
        provider = receipt['provider']
        base = args.jim_root if provider == 'jim' else args.c_root / ('tcl' + provider)
        shell = base / 'jimsh' if provider == 'jim' else base / 'unix/tclsh'
        header = base / 'jim.h' if provider == 'jim' else base / 'generic/tcl.h'
        makefile = base / 'Makefile' if provider == 'jim' else base / 'unix/Makefile'
        owner = base / 'jim.c' if provider == 'jim' else base / 'generic/tclParse.c'
        required = [(fixture / 'cases.tcl', receipt['source_sha256']),
                    (shell, receipt['executable_sha256']),
                    (header, receipt['header_sha256']),
                    (makefile, receipt['makefile_sha256']),
                    (owner, receipt['source_owner_sha256']),
                    (fixture / provider / 'stdout', receipt['stdout_sha256']),
                    (fixture / provider / 'stderr', receipt['stderr_sha256'])]
        for path, expected in required:
            if sha(path) != expected:
                raise ValueError('changed retained input: ' + str(path))
        if args.verify_only:
            results.append({'provider': provider, 'inputs_verified': True, 'executed': False})
            continue
        env = os.environ.copy()
        if provider != 'jim':
            env['TCL_LIBRARY'] = str(base / 'library')
        run = subprocess.run([str(shell)], input=(fixture / 'cases.tcl').read_bytes(),
                             capture_output=True, timeout=30, env=env, check=False)
        destination = args.output / provider
        destination.mkdir()
        (destination / 'stdout').write_bytes(run.stdout)
        (destination / 'stderr').write_bytes(run.stderr)
        equal = (run.returncode == receipt['process_exit']
                 and sha(destination / 'stdout') == receipt['stdout_sha256']
                 and sha(destination / 'stderr') == receipt['stderr_sha256'])
        results.append({'provider': provider, 'executed': True, 'process_exit': run.returncode,
                        'equal': equal, 'command': [str(shell)]})
        if not equal:
            raise RuntimeError('changed native result: ' + provider)
    (args.output / 'comparisons.json').write_text(json.dumps(results, indent=2) + '\n')


if __name__ == '__main__':
    main()
