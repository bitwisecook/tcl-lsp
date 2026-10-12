#!/usr/bin/env python3
"""Fresh bounded C8.6/private-worker and real all.tcl observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
TREE = ROOT / 'tmp/tcl8.6.18'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def limits():
    resource.setrlimit(resource.RLIMIT_AS, (1024**3, 1024**3))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--runner', type=Path, required=True)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / 'object-control-source.c').write_bytes((HERE / 'object-control.c').read_bytes())
    (out / 'capture-source.py').write_bytes(Path(__file__).read_bytes())
    runner = args.runner.resolve()
    env = dict(os.environ, TCL_LIBRARY=str(TREE / 'library'))
    for key in ('TCL_BACKEND_CONSTRAINTS', 'TCLLIBPATH', 'TCL_TEST_VERBOSE', 'TCL_TEST_CONSTRAINTS'):
        env.pop(key, None)
    sources = ['runtime/rust/src/cmd_clock.rs', 'runtime/rust/src/interp.rs',
        'rust/tcl-cmd-core/src/clock.rs', 'rust/tcl-registry/src/commands/tcl/clock_.rs',
        'rust/tcl-registry/src/commands/tcl/mod.rs', 'runtime/rust/examples/run_script.rs']
    paths = [ROOT / source for source in sources]
    paths += [HERE / 'object-control.c', HERE / 'capture.py', TREE / 'generic/tclClock.c',
        TREE / 'library/init.tcl', TREE / 'library/clock.tcl', TREE / 'library/tcltest/tcltest.tcl',
        TREE / 'tests/all.tcl', TREE / 'tests/var.test', TREE / 'unix/libtcl8.6.a',
        runner, TREE / 'unix/tclsh']
    before = {str(p): sha(p) for p in paths}
    records = []

    def execute(name, argv, cwd=None):
        dest = out / name
        dest.mkdir(parents=True)
        start = time.monotonic()
        with (dest / 'stdout.txt').open('wb') as stdout, (dest / 'stderr.txt').open('wb') as stderr:
            result = subprocess.run(['timeout', '--kill-after=5', '60', *argv], env=env,
                cwd=cwd, stdout=stdout, stderr=stderr, preexec_fn=limits(), timeout=67)
        record = dict(name=name, argv=argv, exit_code=result.returncode,
            elapsed_seconds=round(time.monotonic()-start, 3),
            stdout_sha256=sha(dest / 'stdout.txt'), stderr_sha256=sha(dest / 'stderr.txt'))
        records.append(record)
        return result.returncode

    c_binary = out / 'object-control'
    compile_argv = ['cc', '-DHAVE_UNISTD_H=1', '-I' + str(TREE / 'unix'),
        '-I' + str(TREE / 'generic'), str(HERE / 'object-control.c'),
        str(TREE / 'unix/libtcl8.6.a'), '-lm', '-ldl', '-lpthread', '-lz', '-o', str(c_binary)]
    if execute('compile-object-control', compile_argv) != 0:
        raise RuntimeError('object control compilation failed; retained streams')
    execute('c-object-control', [str(c_binary)])
    before[str(c_binary)] = sha(c_binary)
    engines = {'c': [str(TREE / 'unix/tclsh')],
        'runtime': [str(runner), '--quiet', '--tcl-version', '8.6', '--init']}
    eleven = (HERE.parent / 'clock-parse-format-args/input.tcl').read_bytes()
    (out / 'eleven.tcl').write_bytes(eleven)
    all_script = '\n'.join([
        'puts NH_INIT',
        f'source {{{TREE / "library/tcltest/tcltest.tcl"}}}',
        'puts NH_HARNESS',
        'set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]',
        'puts NH_ORCHESTRATION_ENTER',
        f'source {{{TREE / "tests/all.tcl"}}}',
        'puts NH_ORCHESTRATION_RETURN', ''])
    (out / 'all-tcl-var.tcl').write_text(all_script)
    for engine, command in engines.items():
        for name in ('eleven', 'all-tcl-var'):
            with tempfile.TemporaryDirectory(prefix='clock-parse-format-args-') as cwd:
                execute(f'{engine}/{name}', [*command, str(out / f'{name}.tcl')], cwd)
    after = {str(p): sha(p) for p in paths}
    (out / 'provenance.json').write_text(json.dumps(dict(
        source_and_binary_before=before, source_and_binary_after=after,
        inputs={name: sha(out / name) for name in ('eleven.tcl', 'all-tcl-var.tcl')},
        overlay=False, bootstrap='matching real Tcl_Init for C; Native selected C86 --init',
        scope='Private C86 worker only; F5 unmeasured; original all.tcl selects var.test',
        timeout_seconds=60, address_space_limit_bytes=1024**3,
        records=records), indent=2) + '\n')
    print(out)

if __name__ == '__main__':
    main()
