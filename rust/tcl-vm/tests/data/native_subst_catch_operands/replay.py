#!/usr/bin/env python3
"""Reconfirm both exact original Subst-option/catch-output probe variants."""
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
    checks = []
    for variant in ['v1', 'v2']:
        for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim']:
            original = fixture / variant / version
            receipt = json.loads((original / 'receipt.json').read_text())
            probe = fixture / variant / 'probe.c'
            base = args.jim_root if version == 'jim' else args.c_root / ('tcl' + version)
            header = base / 'jim.h' if version == 'jim' else base / 'generic/tcl.h'
            library = base / 'libjim.a' if version == 'jim' else base / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
            makefile = base / 'Makefile' if version == 'jim' else base / 'unix/Makefile'
            owner = base / 'jim.c' if version == 'jim' else base / 'generic/tclCmdMZ.c'
            for path, key in [(probe, 'probe_sha256'), (header, 'header_sha256'), (library, 'library_sha256'), (makefile, 'makefile_sha256'), (owner, 'source_owner_sha256'), (original / 'stdout.tsv', 'stdout_sha256'), (original / 'stderr', 'stderr_sha256')]:
                if sha(path) != receipt[key]:
                    raise ValueError('changed exact input: ' + str(path))
            if args.verify_only:
                checks.append(dict(variant=variant, provider=version, executed=False, exact_inputs_verified=True))
                continue
            destination = args.output / variant / version
            destination.mkdir(parents=True)
            executable = destination / 'probe'
            flags = ['-DJIM_PROBE=1', '-I' + str(base)] if version == 'jim' else ['-I' + str(header.parent), '-I' + str(base / 'unix')]
            libraries = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl'] if version == 'jim' else ['-lm', '-ldl', '-lpthread', '-lz']
            command = ['cc', '-std=c99', *flags, str(probe), str(library), *libraries, '-o', str(executable)]
            compiled = subprocess.run(command, capture_output=True, timeout=60)
            (destination / 'compile.stdout').write_bytes(compiled.stdout)
            (destination / 'compile.stderr').write_bytes(compiled.stderr)
            if compiled.returncode:
                raise RuntimeError('harness compile failed: ' + variant + '/' + version)
            environment = os.environ.copy()
            if version != 'jim':
                environment['TCL_LIBRARY'] = str(base / 'library')
            result = subprocess.run([str(executable)], capture_output=True, timeout=30, env=environment)
            (destination / 'stdout.tsv').write_bytes(result.stdout)
            (destination / 'stderr').write_bytes(result.stderr)
            equal = result.returncode == receipt['process_exit'] and sha(destination / 'stdout.tsv') == receipt['stdout_sha256'] and sha(destination / 'stderr') == receipt['stderr_sha256']
            checks.append(dict(variant=variant, provider=version, executed=True, equal=equal, compile_command=command, executable_sha256=sha(executable), process_exit=result.returncode))
            (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')
            if not equal:
                raise RuntimeError('changed native result: ' + variant + '/' + version)
    (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')


if __name__ == '__main__':
    main()
