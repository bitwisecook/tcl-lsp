#!/usr/bin/env python3
"""Compare original-object word/subst observations under the exact six providers."""
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
    parser.add_argument('--version', choices=VERSIONS, action='append')
    parser.add_argument('--cc', default='cc')
    args = parser.parse_args()
    corpus = Path(__file__).resolve().parent
    args.output.mkdir(parents=True, exist_ok=True)
    captures = []
    for version in args.version or VERSIONS:
        out = args.output.resolve() / version
        out.mkdir(parents=True, exist_ok=True)
        executable = out / 'probe'
        env = os.environ.copy()
        if version == 'jim':
            source = args.jim_source_root.resolve()
            header, library = source / 'jim.h', source / 'libjim.a'
            flags = ['-DUSE_JIM', '-I' + str(source)]
            libs = ['-lm', '-lssl', '-lcrypto', '-lz', '-ldl']
        else:
            source = args.tcl_source_root.resolve() / ('tcl' + version)
            header = source / 'generic' / 'tcl.h'
            library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
            flags = ['-I' + str(header.parent)]
            libs = ['-lm', '-ldl', '-lpthread', '-lz']
            env['TCL_LIBRARY'] = str(source / 'library')
        if not header.is_file() or not library.is_file():
            raise SystemExit('Required original library/header absent: ' + str(source))
        command = [args.cc, '-std=c99', *flags, str(corpus / 'probe.c'), str(library), *libs, '-o', str(executable)]
        compiled = subprocess.run(command, capture_output=True, timeout=60)
        (out / 'compile.stdout').write_bytes(compiled.stdout)
        (out / 'compile.stderr').write_bytes(compiled.stderr)
        if compiled.returncode:
            raise SystemExit('Native compile failure, streams retained: ' + version)
        run = subprocess.run([str(executable)], env=env, capture_output=True, timeout=60)
        (out / 'stdout.jsonl').write_bytes(run.stdout)
        (out / 'stderr').write_bytes(run.stderr)
        capture = dict(version=version, probe_sha256=sha(corpus / 'probe.c'),
                       input_sha256=sha(corpus / 'input.json'), library_sha256=sha(library),
                       header_sha256=sha(header), compile_command=command,
                       compile_exit=compiled.returncode, executable_sha256=sha(executable),
                       exit=run.returncode, stdout_sha256=sha(out / 'stdout.jsonl'),
                       stderr_sha256=sha(out / 'stderr'))
        captures.append(capture)
        (args.output / 'receipt.json').write_text(json.dumps(captures, indent=2) + '\n')
        if run.returncode or run.stderr:
            raise SystemExit('Native harness failure, streams retained: ' + version)
        actual = [json.loads(row) for row in run.stdout.splitlines()]
        expected = [json.loads(row) for row in (corpus / version / 'stdout.jsonl').read_bytes().splitlines()]
        capture['rows'] = actual
        capture['matches_captured_rows'] = actual == expected
        (args.output / 'receipt.json').write_text(json.dumps(captures, indent=2) + '\n')
        if actual != expected:
            raise SystemExit('Native row/startup mismatch, outcomes retained: ' + version)
    print(json.dumps(dict(providers=len(captures), rows=sum(len(row['rows']) for row in captures), matches=True)))


if __name__ == '__main__':
    main()
