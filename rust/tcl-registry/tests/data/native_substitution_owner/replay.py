#!/usr/bin/env python3
"""Rebuild the exact configured C substitution probe and compare retained streams."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, required=True,
                        help='Parent of configured tcl8.4.20 through tcl9.1.0 trees')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--variant', choices=['v1', 'v2', 'v3', 'v4', 'v5', 'v6'], default='v6')
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    corpus = Path(__file__).resolve().parent
    manifest = json.loads((corpus / 'manifest.json').read_text())
    variant = manifest['variants'][args.variant]
    probe = corpus / args.variant / 'probe.c'
    if digest(probe) != variant['probe_sha256']:
        raise ValueError('changed original probe')
    args.output.mkdir(parents=True, exist_ok=False)
    comparisons = []
    for version, row in variant['providers'].items():
        original = json.loads((corpus / row['receipt']).read_text())
        source = args.source_root / ('tcl' + version)
        header = source / 'generic' / 'tcl.h'
        library = source / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        makefile = source / 'unix' / 'Makefile'
        for path, expected in [(header, original['header_sha256']),
                               (library, original['library_sha256']),
                               (makefile, original['makefile_sha256'])]:
            if digest(path) != expected:
                raise ValueError(f'provider input changed: {path}')
        for name, expected in original['private_headers_sha256'].items():
            if digest(source / 'generic' / name) != expected:
                raise ValueError(f'private header changed: {version}/{name}')
        for name, expected in [('stdout.tsv', original['stdout_sha256']),
                               ('stderr', original['stderr_sha256'])]:
            if digest(corpus / args.variant / version / name) != expected:
                raise ValueError(f'captured stream changed: {version}/{name}')
        definition = next(line.partition('=')[2].strip() for line in makefile.read_text().splitlines()
                          if line.split('=', 1)[0].strip() == 'AC_FLAGS')
        flags = shlex.split(definition)
        if flags != original['configured_flags']:
            raise ValueError(f'configured flags changed: {version}')
        if args.verify_only:
            comparisons.append({'provider': version, 'inputs_verified': True, 'executed': False})
            continue
        directory = args.output / version
        directory.mkdir()
        executable = directory / 'probe'
        command = ['cc', '-std=c99', '-D_GNU_SOURCE', *flags,
                   '-I' + str(header.parent), '-I' + str(source / 'unix'),
                   str(probe), str(library), '-lm', '-ldl', '-lpthread', '-lz',
                   '-o', str(executable)]
        compiled = subprocess.run(command, capture_output=True, timeout=60, check=False)
        (directory / 'compile.stdout').write_bytes(compiled.stdout)
        (directory / 'compile.stderr').write_bytes(compiled.stderr)
        if compiled.returncode:
            raise RuntimeError(f'probe compilation failed: {version}')
        environment = os.environ.copy()
        environment['TCL_LIBRARY'] = str(source / 'library')
        result = subprocess.run([str(executable)], env=environment, capture_output=True,
                                timeout=60, check=False)
        (directory / 'stdout.tsv').write_bytes(result.stdout)
        (directory / 'stderr').write_bytes(result.stderr)
        match = (result.returncode == original['process_exit']
                 and digest(directory / 'stdout.tsv') == original['stdout_sha256']
                 and digest(directory / 'stderr') == original['stderr_sha256'])
        comparisons.append({'provider': version, 'compile_command': command,
                            'executable_sha256': digest(executable),
                            'process_exit': result.returncode, 'exact_stream_match': match})
        if not match:
            raise RuntimeError(f'original process/streams differ: {version}')
    (args.output / 'comparisons.json').write_text(json.dumps(comparisons, indent=2) + '\n')


if __name__ == '__main__':
    main()
