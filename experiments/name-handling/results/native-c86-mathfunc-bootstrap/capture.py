#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Replay this bounded question into a REQUIRED fresh destination."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
TREE = ROOT / 'tmp/tcl8.6.18'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', required=True, type=Path)
    parser.add_argument('--runner', required=True, type=Path)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    runner = args.runner.resolve()
    env = dict(os.environ, TCL_LIBRARY=str(TREE / 'library'))
    probe = out / 'c86-probe'
    shutil.copyfile(HERE / 'probe.c', out / 'probe.c')
    shutil.copyfile(Path(__file__), out / 'capture.py')
    sources = [TREE / 'generic/tclBasic.c', TREE / 'generic/tclProc.c',
               TREE / 'generic/tclInt.h', TREE / 'library/init.tcl',
               TREE / 'unix/libtcl8.6.a', TREE / 'unix/tclConfig.sh']
    compile_argv = ['cc', '-DHAVE_UNISTD_H=1', '-I' + str(TREE / 'unix'),
                    '-I' + str(TREE / 'generic'), str(out / 'probe.c'),
                    str(TREE / 'unix/libtcl8.6.a'), '-lm', '-ldl', '-lpthread',
                    '-lz', '-o', str(probe)]
    built = subprocess.run(compile_argv, capture_output=True, timeout=30)
    (out / 'compile.stdout.txt').write_bytes(built.stdout)
    (out / 'compile.stderr.txt').write_bytes(built.stderr)
    if built.returncode:
        raise SystemExit(built.returncode)
    runs = []

    def retain(label, argv, path=None):
        try:
            result = subprocess.run(argv, env=env, capture_output=True, timeout=30)
            code, timeout, stdout, stderr = result.returncode, False, result.stdout, result.stderr
        except subprocess.TimeoutExpired as result:
            code, timeout, stdout, stderr = None, True, result.stdout or b'', result.stderr or b''
        (out / (label + '.stdout.txt')).write_bytes(stdout)
        (out / (label + '.stderr.txt')).write_bytes(stderr)
        runs.append(dict(case=label, argv=argv, exit_code=code, timed_out=timeout,
                         input_path=str(path) if path else None,
                         input_sha256=sha(path) if path else None,
                         binary_sha256=sha(Path(argv[0])),
                         stdout_sha256=sha(out / (label + '.stdout.txt')),
                         stderr_sha256=sha(out / (label + '.stderr.txt')),
                         TCL_LIBRARY=env['TCL_LIBRARY']))

    for name in ('constructor', 'empty-birth', 'init-max-shape', 'present-replacement'):
        path = HERE / (name + '-input.tcl')
        shutil.copyfile(path, out / path.name)
        retain('c86-' + name, [str(probe), str(path)], path)
        retain('runtime-' + name, [str(runner), '--quiet', '--tcl-version', '8.6', str(path)], path)
    retain('c86-real-init', [str(probe), 'init'])
    startup = ROOT / 'experiments/name-handling/clock-bootstrap-epoch/startup-input.tcl'
    shutil.copyfile(startup, out / 'startup-input.tcl')
    retain('runtime-real-init', [str(runner), '--quiet', '--tcl-version', '8.6', '--init', str(startup)], startup)
    library = {str(p.relative_to(TREE / 'library')): sha(p)
               for p in sorted((TREE / 'library').rglob('*')) if p.is_file()}
    with gzip.open(out / 'library-source-hashes.json.gz', 'wt') as stream:
        json.dump(library, stream, indent=2)
    (out / 'records.json').write_text(json.dumps(dict(
        question='Does the measured C8.6 constructor roster permit genuine mathfunc procedure births while preserving independent compiler currency, and how far does real init reach?',
        context='Fresh constructor per process. Native inputs use core-only except the single real init. C API uses Tcl_CreateInterp before any library init. No tcltest harness verdict or other-profile roster inference.',
        compile_argv=compile_argv, compile_exit_code=built.returncode,
        source_sha256={str(p): sha(p) for p in sources}, runs=runs), indent=2) + '\n')
    for row in runs:
        print(row['case'], row['exit_code'], row['timed_out'])


if __name__ == '__main__':
    main()
