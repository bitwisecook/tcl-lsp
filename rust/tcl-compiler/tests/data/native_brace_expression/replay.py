#!/usr/bin/env python3
"""Replay v4 with exact pinned inputs; earlier partial attempts remain immutable."""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


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
    for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim']:
        receipt = json.loads((fixture / 'v4/capture' / version / 'receipt.json').read_text())
        base = args.jim_root if version == 'jim' else args.c_root / ('tcl' + version)
        protocol = fixture / 'v4/protocol'
        header = base / 'jim.h' if version == 'jim' else base / 'generic/tcl.h'
        library = base / 'libjim.a' if version == 'jim' else base / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        makefile = base / 'Makefile' if version == 'jim' else base / 'unix/Makefile'
        owner = base / 'jim.c' if version == 'jim' else base / 'generic/tclInterp.c'
        for path, key in [(protocol / 'probe.c', 'probe_sha256'),
                          (protocol / 'capture.py', 'runner_sha256'),
                          (protocol / 'cases.h', 'case_header_sha256'),
                          (protocol / 'inputs.json', 'inputs_manifest_sha256'),
                          (header, 'header_sha256'), (library, 'library_sha256'),
                          (makefile, 'makefile_sha256'), (owner, 'source_owner_sha256'),
                          (fixture / 'v4/capture' / version / 'stdout.tsv', 'stdout_sha256'),
                          (fixture / 'v4/capture' / version / 'stderr', 'stderr_sha256')]:
            if sha(path) != receipt[key]:
                raise ValueError('changed exact input: ' + str(path))
        for name, digest in receipt['additional_source_sha256'].items():
            if sha(base / 'generic' / name) != digest:
                raise ValueError('changed additional source: ' + name)
        for relative, digest in receipt['input_file_sha256'].items():
            if sha(protocol / relative) != digest:
                raise ValueError('changed source/collector bytes: ' + relative)
        if args.verify_only:
            checks.append(dict(provider=version, executed=False, exact_inputs_verified=True))
            continue
        destination = args.output / version
        destination.mkdir()
        executable = destination / 'probe'
        flags = ['-DJIM_PROBE=1', '-I' + str(base)] if version == 'jim' else ['-I' + str(header.parent), '-I' + str(base / 'unix')]
        libraries = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl'] if version == 'jim' else ['-lm', '-ldl', '-lpthread', '-lz']
        command = ['cc', '-std=c99', *flags, str(protocol / 'probe.c'), str(library), *libraries, '-o', str(executable)]
        compiled = subprocess.run(command, capture_output=True, timeout=60)
        (destination / 'compile.stdout').write_bytes(compiled.stdout)
        (destination / 'compile.stderr').write_bytes(compiled.stderr)
        if compiled.returncode:
            raise RuntimeError('native harness compile failed: ' + version)
        environment = os.environ.copy()
        if version != 'jim':
            environment['TCL_LIBRARY'] = str(base / 'library')
        result = subprocess.run([str(executable), str(protocol / 'inputs')], capture_output=True, env=environment, timeout=30)
        (destination / 'stdout.tsv').write_bytes(result.stdout)
        (destination / 'stderr').write_bytes(result.stderr)
        equal = result.returncode == receipt['process_exit'] and sha(destination / 'stdout.tsv') == receipt['stdout_sha256'] and sha(destination / 'stderr') == receipt['stderr_sha256']
        checks.append(dict(provider=version, executed=True, equal=equal,
                           compile_command=command, executable_sha256=sha(executable),
                           process_exit=result.returncode))
        (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')
        if not equal:
            raise RuntimeError('changed original output: ' + version)
    (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')


if __name__ == '__main__':
    main()
