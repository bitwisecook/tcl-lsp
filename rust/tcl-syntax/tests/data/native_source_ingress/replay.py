#!/usr/bin/env python3
"""Rebuild and compare the public C source-ingress probe under selected releases."""
import argparse
import hashlib
import json
import pathlib
import subprocess

VERSIONS = ('8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tcl-source-root', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--version', choices=VERSIONS, action='append')
    parser.add_argument('--cc', default='cc')
    args = parser.parse_args()
    corpus = pathlib.Path(__file__).resolve().parent
    args.output.mkdir(parents=True, exist_ok=True)
    records = []
    for version in args.version or VERSIONS:
        source = args.tcl_source_root.resolve() / ('tcl' + version)
        library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        header = source / 'generic' / 'tcl.h'
        if not library.is_file() or not header.is_file():
            raise SystemExit('required native library/header absent: ' + str(source))
        out = args.output.resolve() / version
        out.mkdir(parents=True, exist_ok=True)
        executable = out / 'probe'
        build = [args.cc, '-std=c99', '-I' + str(header.parent), str(corpus / 'probe.c'), str(library), '-lm', '-ldl', '-lpthread', '-lz', '-o', str(executable)]
        compiled = subprocess.run(build, capture_output=True, timeout=60)
        (out / 'compile.stdout').write_bytes(compiled.stdout)
        (out / 'compile.stderr').write_bytes(compiled.stderr)
        if compiled.returncode:
            raise SystemExit('native compile failed: ' + version)
        command = [str(executable), str(corpus / 'source.tcl')]
        run = subprocess.run(command, capture_output=True, timeout=60)
        (out / 'stdout.jsonl').write_bytes(run.stdout)
        (out / 'stderr').write_bytes(run.stderr)
        if run.returncode or run.stderr:
            raise SystemExit('native harness failed: ' + version)
        actual = [json.loads(row) for row in run.stdout.splitlines()]
        expected = [json.loads(row) for row in (corpus / version / 'stdout.jsonl').read_bytes().splitlines()]
        reported = next(row for row in actual if row['path'] == 'startup')['bytes']
        if bytes.fromhex(reported).decode('ascii') != version:
            raise SystemExit('native reported version mismatch: ' + version)
        record = {'version': version, 'compile_command': build, 'compile_exit': compiled.returncode,
                  'command': command, 'exit': run.returncode, 'source_sha256': sha(corpus / 'source.tcl'),
                  'probe_sha256': sha(corpus / 'probe.c'), 'library_sha256': sha(library),
                  'tcl_header_sha256': sha(header), 'executable_sha256': sha(executable),
                  'stdout_sha256': sha(out / 'stdout.jsonl'), 'stderr_sha256': sha(out / 'stderr'),
                  'rows': actual, 'matches_captured_rows': actual == expected,
                  'implementation_sources': [{'path': str(source / 'generic' / name), 'sha256': sha(source / 'generic' / name)} for name in ('tclIO.c', 'tclEncoding.c', 'tclUtf.c')]}
        records.append(record)
        (args.output / 'receipt.json').write_text(json.dumps(records, indent=2) + '\n')
        if actual != expected:
            raise SystemExit('native observation mismatch (preserved in output): ' + version)
    print(json.dumps({'providers': len(records), 'rows': sum(len(r['rows']) for r in records), 'matches': True}))


if __name__ == '__main__':
    main()
