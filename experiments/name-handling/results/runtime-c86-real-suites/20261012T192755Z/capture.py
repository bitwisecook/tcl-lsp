#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Capture bounded fresh actual C8.6 and Native full-file naming questions."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
TREE = ROOT / 'tmp/tcl8.6.18'
RUNNER = Path('/tmp/name-native-c86-mathfunc-bootstrap-runner')
STEMS = ('var', 'proc', 'rename', 'upvar', 'namespace', 'interp', 'oo', 'ooNext2')
SUMMARY = re.compile(r'Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)')
FAILED = re.compile(r'^====\s+(\S+).*\sFAILED\s*$', re.MULTILINE)
STARTED = re.compile(r'^----\s+(\S+)\s+start\s*$', re.MULTILINE)

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def dump(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')

def limits():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))

def main():
    out = HERE / time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())
    out.mkdir(exist_ok=False)
    originals = [TREE / 'library/init.tcl', TREE / 'library/tcltest/tcltest.tcl',
                 TREE / 'library/tcltest/pkgIndex.tcl', TREE / 'license.terms']
    originals += [TREE / 'tests' / (stem + '.test') for stem in STEMS]
    pinned = {str(p): sha(p) for p in originals}
    for p in originals:
        dest = out / 'originals' / p.relative_to(TREE)
        dest.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(str(dest) + '.gz', 'wb') as stream:
            stream.write(p.read_bytes())
    owners = ['runtime/rust/src/cmd_mathfunc.rs', 'rust/tcl-registry/src/special_vars.rs']
    before = {p: sha(ROOT / p) for p in owners}
    assert before[owners[0]] == '03e6cfce6b828ca4ff9f6b58ddb1ac7ae58cd62251f495e771fbfcce99cad24d'
    assert before[owners[1]] == '548e2bfcdf94fde1e7811c18e8910dfe2450fdc0a6ca772bd35d7a895dca44a5'
    assert sha(RUNNER) == 'fb1ae94b86854a4f56476011dd50ead92ec7c94fac5aa721d678e72bb15885c6'
    engines = {'c': [str(TREE / 'unix/tclsh')],
               'runtime': [str(RUNNER), '--quiet', '--tcl-version', '8.6', '--init']}
    env = dict(os.environ, TCL_LIBRARY=str(TREE / 'library'))
    for key in ('TCL_BACKEND_CONSTRAINTS', 'TCLLIBPATH', 'TCL_TEST_VERBOSE', 'TCL_TEST_CONSTRAINTS'):
        env.pop(key, None)
    records = []
    provenance = dict(source_root=str(TREE), source_sha256=pinned, owner_before=before,
        binary_sha256={k: sha(Path(v[0])) for k, v in engines.items()},
        engines=engines, bootstrap='Native --init; C tclsh automatic matching init',
        harness='direct source of unchanged real library/tcltest/tcltest.tcl; package require 2.5',
        overlay=False, timeout_seconds=60, address_space_limit_bytes=1024**3,
        stream_limit_bytes=8*1024**2, capture_sha256=sha(Path(__file__)))
    dump(out / 'provenance.json', provenance)
    (out / 'capture.py').write_bytes(Path(__file__).read_bytes())
    (out / 'native-runner-source.rs').write_bytes((ROOT / 'runtime/rust/examples/run_script.rs').read_bytes())

    def retain(engine, stem, script):
        dest = out / engine / stem
        dest.mkdir(parents=True)
        wrapper = dest / 'input.tcl'
        wrapper.write_text(script)
        command = ['timeout', '--kill-after=5', '60', *engines[engine], str(wrapper)]
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix='name-c86-real-suite-') as cwd:
            stdout_path, stderr_path = dest / 'stdout.txt', dest / 'stderr.txt'
            with stdout_path.open('wb') as stdout, stderr_path.open('wb') as stderr:
                result = subprocess.run(command, env=env, cwd=cwd, stdout=stdout,
                                        stderr=stderr, preexec_fn=limits(), timeout=67)
        stdout = stdout_path.read_text(errors='replace')
        stderr = stderr_path.read_text(errors='replace')
        summaries = SUMMARY.findall(stdout)
        record = dict(engine=engine, stem=stem, argv=command, input_sha256=sha(wrapper),
            binary_sha256=provenance['binary_sha256'][engine], exit_code=result.returncode,
            elapsed_seconds=round(time.monotonic()-start, 3),
            init_reached='NH_INIT' in stdout, harness_reached='NH_HARNESS' in stdout,
            suite_entered='NH_SUITE_ENTER' in stdout, suite_returned='NH_SUITE_RETURN' in stdout,
            failed_ids=sorted(set(FAILED.findall(stdout))), started_count=len(STARTED.findall(stdout)),
            last_started_id=(STARTED.findall(stdout) or [None])[-1],
            stdout_sha256=sha(stdout_path), stderr_sha256=sha(stderr_path),
            TCL_LIBRARY=env['TCL_LIBRARY'], TCL_BACKEND_CONSTRAINTS=None)
        if stem in STEMS:
            record['test_sha256'] = pinned[str(TREE / 'tests' / (stem + '.test'))]
        if result.returncode in (124, 137):
            status = 'timeout-or-killed'
        elif result.returncode < 0 or result.returncode in (134, 139):
            status = 'crash'
        elif 'execution refused:' in stderr or 'initialisation refused:' in stderr:
            status = 'host-refusal'
        elif result.returncode != 0:
            status = 'process-error'
        elif summaries:
            status = 'completed-suite'
            record.update(zip(('total', 'passed', 'skipped', 'failed'), map(int, summaries[-1])))
        elif stem == 'startup' and stdout == 'NH_INIT\n' and not stderr:
            status = 'startup-marker-reached'
        else:
            status = 'no-suite-summary'
        if summaries and status != 'completed-suite':
            record['partial_summary'] = list(map(int, summaries[-1]))
        record['status'] = status
        dump(dest / 'record.json', record)
        records.append(record)
        dump(out / 'records.json', records)
        print(json.dumps({k: record[k] for k in ('engine', 'stem', 'exit_code', 'status', 'harness_reached', 'last_started_id')}), flush=True)
        if status in ('crash', 'timeout-or-killed') or re.search(r'out of memory|memory allocation.*failed', stderr, re.I):
            raise RuntimeError('STOP for review: ' + engine + '/' + stem + '/' + status)
        if stdout_path.stat().st_size + stderr_path.stat().st_size > 8*1024**2:
            raise RuntimeError('STOP for review: large stream')

    for engine in engines:
        retain(engine, 'startup', 'puts NH_INIT\n')
    for stem in STEMS:
        script = ('puts NH_INIT\n'
            'if {[catch {interp debug {} -frame 1} frame_debug]} {puts [list NH_FRAME_DEBUG unavailable $frame_debug]} else {puts [list NH_FRAME_DEBUG enabled]}\n'
            f'source {{{TREE / "library/tcltest/tcltest.tcl"}}}\n'
            'package require tcltest 2.5\nnamespace import -force ::tcltest::*\n'
            'puts [list NH_HARNESS library [info library] tcl [package present Tcl] tcltest [package present tcltest]]\n'
            f'if {{[file normalize [info library]] ne [file normalize {{{TREE / "library"}}}]}} {{error {{wrong upstream library}}}}\n'
            '::tcltest::configure -verbose {body error start skip}\n'
            'puts NH_SUITE_ENTER\n'
            f'source {{{TREE / "tests" / (stem + ".test")}}}\n'
            'puts NH_SUITE_RETURN\n')
        for engine in engines:
            retain(engine, stem, script)
    after = {p: sha(ROOT / p) for p in owners}
    dump(out / 'validation.json', dict(owner_before=before, owner_after=after,
        frozen_owners_unchanged=before == after,
        original_sources_unchanged=all(sha(Path(p)) == v for p, v in pinned.items()),
        runner_unchanged=sha(RUNNER) == provenance['binary_sha256']['runtime'],
        records=len(records)))
    print('OUTPUT ' + str(out), flush=True)

if __name__ == '__main__':
    main()
