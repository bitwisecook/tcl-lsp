#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Execute the exact private-worker question into a required fresh directory."""
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
TIMEOUT = 30


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def inventory(directory):
    return {str(p.relative_to(directory)): sha(p)
            for p in sorted(directory.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    if out.exists():
        parser.error('--output-dir must name a nonexistent fresh destination')
    pin_path = ROOT / 'rust/tcl-test-support/jim-reference.txt'
    pin = dict(line.split('=', 1) for line in pin_path.read_text().splitlines() if line and not line.startswith('#'))
    jim = ROOT / 'tmp/jimtcl-oracle'
    if (jim / '.git/HEAD').read_text().strip() != pin['revision']:
        parser.error('Jim checkout differs from the maintained oracle pin')
    out.mkdir(parents=True)
    shutil.copyfile(HERE / 'input.tcl', out / 'input.tcl')
    shutil.copyfile(HERE / 'source-only-review.md', out / 'source-only-review.md')
    shutil.copyfile(Path(__file__), out / 'capture.py')
    input_path = HERE / 'input.tcl'
    input_before = sha(input_path)
    source_paths = [pin_path, HERE / 'source-only-review.md', HERE / 'input.tcl', Path(__file__)]
    runs = []

    def execute(label, shell, env, **context):
        before = sha(shell)
        argv = [str(shell), str(input_path)]
        try:
            result = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, timeout=TIMEOUT)
            code, timeout, stdout, stderr = result.returncode, False, result.stdout, result.stderr
        except subprocess.TimeoutExpired as result:
            code, timeout, stdout, stderr = None, True, result.stdout or b'', result.stderr or b''
        for kind, data in [('stdout', stdout), ('stderr', stderr)]:
            (out / (label + '.' + kind + '.txt')).write_bytes(data)
        row = dict(case=label, argv=argv, cwd=str(ROOT), exit_code=code,
                   timed_out=timeout, timeout_seconds=TIMEOUT,
                   binary_sha256_before=before, binary_sha256_after=sha(shell),
                   input_path=str(input_path), input_sha256=input_before,
                   environment={k: env.get(k) for k in ['TCL_LIBRARY', 'TCLLIBPATH', 'JIMLIB']},
                   stdout=label + '.stdout.txt', stdout_sha256=sha(out / (label + '.stdout.txt')),
                   stderr=label + '.stderr.txt', stderr_sha256=sha(out / (label + '.stderr.txt')),
                   **context)
        runs.append(row)

    for version in ['8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0']:
        tree = ROOT / ('tmp/tcl' + version)
        label = 'c' + version
        source_paths.extend(tree / p for p in ['generic/tclClock.c', 'unix/tclAppInit.c', 'unix/tclConfig.sh'])
        static = tree / ('unix/libtcl' + '.'.join(version.split('.')[:2]) + '.a')
        source_paths.append(static)
        library_before = inventory(tree / 'library')
        with gzip.open(out / (label + '.library-hashes.json.gz'), 'wt') as stream:
            json.dump(library_before, stream, indent=2)
        env = dict(os.environ, TCL_LIBRARY=str(tree / 'library'))
        for key in ['TCLLIBPATH', 'JIMLIB', 'TCL_BACKEND_CONSTRAINTS']:
            env.pop(key, None)
        execute(label, tree / 'unix/tclsh', env, release=version,
                startup='Stock tclsh Tcl_AppInit/Tcl_Init with real matching TCL_LIBRARY before unchanged script.',
                library_directory=str(tree / 'library'), library_inventory=label + '.library-hashes.json.gz',
                library_inventory_sha256=sha(out / (label + '.library-hashes.json.gz')),
                library_file_count=len(library_before), static_library_sha256=sha(static))
        runs[-1]['library_unchanged_during_run'] = library_before == inventory(tree / 'library')
    jim_sources = [jim / p for p in ['jimsh.c', 'initjimsh.tcl', 'jim-clock.c', 'jim-config.h', 'jimautoconf.h', 'Makefile']]
    source_paths.extend(jim_sources)
    startup_before = {str(p): sha(p) for p in jim_sources}
    write_json(out / 'jim-startup-hashes.json', startup_before)
    env = dict(os.environ)
    for key in ['TCL_LIBRARY', 'TCLLIBPATH', 'JIMLIB', 'TCL_BACKEND_CONSTRAINTS']:
        env.pop(key, None)
    execute('jim-' + pin['revision'][:7], jim / 'jimsh', env, release=pin['patchlevel'],
            revision=pin['revision'], maintained_pin=str(pin_path), maintained_pin_sha256=sha(pin_path),
            startup='Stock Jim shell; configured static extensions and embedded initjimsh; no C Tcl init substitution.',
            startup_hashes='jim-startup-hashes.json', startup_hashes_sha256=sha(out / 'jim-startup-hashes.json'))
    runs[-1]['startup_sources_unchanged_during_run'] = startup_before == {str(p): sha(p) for p in jim_sources}
    source_hashes = {str(p): sha(p) for p in source_paths}
    excerpts = []
    ranges = {'8.4.20': [(39, 90)], '8.5.19': [(195, 227)],
              '8.6.18': [(56, 88), (209, 223), (242, 305), (1877, 1994), (2113, 2128)],
              '9.0.4': [(134, 154)], '9.1.0': [(142, 156)]}
    for version, selected in ranges.items():
        source = ROOT / ('tmp/tcl' + version + '/generic/tclClock.c')
        lines = source.read_bytes().splitlines(keepends=True)
        for first, last in selected:
            file = 'c' + version + '-clock-' + str(first) + '-' + str(last) + '.c'
            (out / file).write_bytes(b''.join(lines[first - 1:last]))
            excerpts.append(dict(source=str(source), source_sha256=sha(source), first_line=first, last_line=last,
                                 excerpt=file, excerpt_sha256=sha(out / file)))
    source = ROOT / 'tmp/tcl8.6.18/library/clock.tcl'
    file = 'c8.6.18-clock-format-664-674.tcl'
    (out / file).write_bytes(b''.join(source.read_bytes().splitlines(keepends=True)[663:674]))
    excerpts.append(dict(source=str(source), source_sha256=sha(source), first_line=664, last_line=674,
                         excerpt=file, excerpt_sha256=sha(out / file)))
    write_json(out / 'source-excerpts.json', excerpts)
    write_json(out / 'records.json', dict(
        question='What does the named private ::tcl::clock::ParseFormatArgs worker answer for these exact eleven inputs after real matching startup, and which selected releases lack that worker?',
        source_basis='source-only-review.md is the preceding source review, explicitly not a runtime result.',
        process_policy='One fresh stock process per selected release, unchanged eleven-case input; caught errors are emitted explicitly; no native runner/build or tcltest harness.',
        context_limits='These are post-startup named-private-worker observations. Absence does not answer public clock format. Source/configuration hashes do not independently authenticate embedded binary bytes.',
        F5='Unmeasured; no answer inferred.', input_sha256_before=input_before, input_sha256_after=sha(input_path),
        capture_sha256=sha(Path(__file__)), source_hashes=source_hashes, runs=runs))
    for row in runs:
        print(row['case'], row['exit_code'], row['timed_out'])


if __name__ == '__main__':
    main()
