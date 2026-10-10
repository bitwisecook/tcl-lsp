#!/usr/bin/env python3
"""Replay both retained recursion/frame-release probe inputs, preserving guest errors."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

VERSIONS = ('8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0', 'jim')

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tcl-source-root', type=Path, required=True)
    parser.add_argument('--jim-source-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--version', action='append', choices=VERSIONS)
    parser.add_argument('--variant', action='append', choices=('v1', 'v2'))
    parser.add_argument('--cc', default='cc')
    args = parser.parse_args()
    corpus = Path(__file__).resolve().parent
    rows = []
    for variant in args.variant or ('v1', 'v2'):
        probe = corpus / variant / 'probe.c'
        for version in args.version or VERSIONS:
            previous = json.loads((corpus / variant / version / 'receipt.json').read_text())
            out = args.output.resolve() / variant / version
            out.mkdir(parents=True, exist_ok=False)
            executable = out / 'probe'
            env = os.environ.copy()
            if version == 'jim':
                source = args.jim_source_root.resolve()
                header, library = source / 'jim.h', source / 'libjim.a'
                flags = ['-DJIM_PROBE=1', '-I' + str(source)]
                links = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl']
            else:
                source = args.tcl_source_root.resolve() / ('tcl' + version)
                header = source / 'generic' / 'tcl.h'
                library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
                flags = ['-I' + str(header.parent), '-I' + str(source / 'unix')]
                links = ['-lm', '-ldl', '-lpthread', '-lz']
                env['TCL_LIBRARY'] = str(source / 'library')
            for path, key in [(probe, 'probe_sha256'), (header, 'header_sha256'), (library, 'library_sha256')]:
                if not path.is_file() or sha(path) != previous[key]:
                    raise SystemExit('Required captured input differs or is absent: ' + str(path))
            command = [args.cc, '-std=c99', *flags, str(probe), str(library), *links, '-o', str(executable)]
            compiled = subprocess.run(command, capture_output=True, timeout=60)
            (out / 'compile.stdout').write_bytes(compiled.stdout)
            (out / 'compile.stderr').write_bytes(compiled.stderr)
            if compiled.returncode:
                raise SystemExit('Native compile failure; streams retained: ' + str(out))
            actual = subprocess.run([str(executable)], env=env, capture_output=True, timeout=60)
            (out / 'stdout.tsv').write_bytes(actual.stdout)
            (out / 'stderr').write_bytes(actual.stderr)
            expected = (corpus / variant / version / 'stdout.tsv').read_bytes()
            receipt = dict(provider=version, variant=variant, compile_command=command, compile_exit=compiled.returncode,
                           probe_sha256=sha(probe), header_sha256=sha(header), library_sha256=sha(library),
                           executable_sha256=sha(executable), process_exit=actual.returncode,
                           stdout_sha256=sha(out / 'stdout.tsv'), stderr_sha256=sha(out / 'stderr'),
                           matches=actual.returncode == previous['process_exit'] and actual.stdout == expected
                           and actual.stderr == (corpus / variant / version / 'stderr').read_bytes())
            (out / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
            rows.append(receipt)
            if not receipt['matches']:
                raise SystemExit('Exact process/stream mismatch; all guest errors retained: ' + str(out))
    print(json.dumps(dict(processes=len(rows), guest_rows=len(rows) * 8, matches=True)))

if __name__ == '__main__':
    main()
