#!/usr/bin/env python3
"""Fresh selected C86 getter/pool and unchanged original all.tcl observations."""
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
ROOT = HERE.parents[4]
TREE = ROOT / 'tmp/tcl8.6.18'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

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
    runner = args.runner.resolve()
    getter_source = HERE.parent / 'getenv-question/command-control.c'
    paths = [ROOT / p for p in ['rust/tcl-platform/src/lib.rs', 'rust/tcl-host-native/src/lib.rs',
        'rust/tcl-registry/src/commands/tcl/clock_.rs', 'rust/tcl-registry/src/commands/tcl/mod.rs',
        'runtime/rust/src/cmd_clock.rs', 'runtime/rust/src/host_wasm.rs',
        'runtime/rust/src/interp.rs', 'runtime/rust/examples/run_script.rs']]
    paths += [HERE / 'capture.py', HERE / 'two-worker-control.c', getter_source, runner,
        TREE / 'unix/tclsh', TREE / 'unix/libtcl8.6.a', TREE / 'generic/tclClock.c',
        TREE / 'library/init.tcl', TREE / 'library/clock.tcl', TREE / 'library/tcltest/tcltest.tcl',
        TREE / 'tests/all.tcl', TREE / 'tests/var.test']
    before = {str(p): sha(p) for p in paths}
    for p in [HERE / 'capture.py', HERE / 'two-worker-control.c', getter_source]:
        (out / p.name).write_bytes(p.read_bytes())
    records = []
    env = dict(os.environb)
    for key in [b'NH_CLOCK_GETENV_MISSING', b'TCL_BACKEND_CONSTRAINTS', b'TCLLIBPATH',
        b'TCL_TEST_VERBOSE', b'TCL_TEST_CONSTRAINTS']:
        env.pop(key, None)
    env.update({b'TCL_LIBRARY': str(TREE / 'library').encode(),
        b'NH_CLOCK_GETENV_TEST': b'host-value', b'NH_CLOCK_GETENV_\xff': b'raw-name-value',
        b'NH_CLOCK_GETENV_RAW_VALUE': b'\xff'})

    def execute(name, argv):
        dest = out / name
        dest.mkdir(parents=True)
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix='getenv-runtime-') as cwd:
            with (dest / 'stdout.txt').open('wb') as stdout, (dest / 'stderr.txt').open('wb') as stderr:
                result = subprocess.run(['timeout', '--kill-after=5', '60', *argv], env=env,
                    cwd=cwd, stdout=stdout, stderr=stderr, preexec_fn=limits(), timeout=67)
        records.append(dict(name=name, argv=argv, exit_code=result.returncode,
            elapsed_seconds=round(time.monotonic()-start, 3),
            stdout_sha256=sha(dest / 'stdout.txt'), stderr_sha256=sha(dest / 'stderr.txt')))
        return result.returncode

    binaries = {}
    for name, source in [('getter-control', getter_source), ('two-worker-control', HERE / 'two-worker-control.c')]:
        binary = out / name
        argv = ['cc', '-DHAVE_UNISTD_H=1', '-I' + str(TREE / 'unix'), '-I' + str(TREE / 'generic'),
            str(source), str(TREE / 'unix/libtcl8.6.a'), '-lm', '-ldl', '-lpthread', '-lz', '-o', str(binary)]
        if execute('compile-' + name, argv) != 0:
            raise RuntimeError('C control compilation failed; raw streams retained')
        binaries[str(binary)] = sha(binary)
    execute('c-getter-object', [str(out / 'getter-control')])
    for order in ['parser-first', 'getenv-first']:
        execute('c-two-owner-' + order, [str(out / 'two-worker-control'), order])
    (out / 'getter-script.tcl').write_bytes((HERE.parent / 'getenv-question/input.tcl').read_bytes())
    (out / 'eleven.tcl').write_bytes((ROOT / 'experiments/name-handling/results/clock-parse-format-args/input.tcl').read_bytes())
    all_input = '\n'.join(['puts NH_INIT',
        f'source {{{TREE / "library/tcltest/tcltest.tcl"}}}', 'puts NH_HARNESS',
        'set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]',
        'puts NH_ORCHESTRATION_ENTER', f'source {{{TREE / "tests/all.tcl"}}}',
        'puts NH_ORCHESTRATION_RETURN', ''])
    (out / 'all-tcl-var.tcl').write_text(all_input)
    engines = {'c': [str(TREE / 'unix/tclsh')],
        'runtime': [str(runner), '--quiet', '--tcl-version', '8.6', '--init']}
    for engine, command in engines.items():
        for name in ['getter-script', 'eleven', 'all-tcl-var']:
            execute(engine + '/' + name, [*command, str(out / (name + '.tcl'))])
    after = {str(p): sha(p) for p in paths}
    (out / 'provenance.json').write_text(json.dumps(dict(source_and_binary_before=before,
        source_and_binary_after=after, c_control_binaries=binaries, records=records,
        inputs={name: sha(out / name) for name in ['getter-script.tcl', 'eleven.tcl', 'all-tcl-var.tcl']},
        scope='Selected Native C86 on actual Unix NativeHost; real matching init/tcltest and unchanged all.tcl selecting var.test; control explicitly retires other five C workers before observing two surviving owners.',
        environment_control='Only explicitly named benign NH_CLOCK_GETENV_* controls are queried/output; no environment dump. Native normal env-array write/unset synchronisation remains separate.',
        binary_scope='Fresh standalone runner has no tcl-compiler/LSP dependencies. Separate registry unit build contains diagnostic-frozen dependency sources, recorded in validation-02.',
        timeout_seconds=60, address_space_limit_bytes=1024**3, overlay=False, F5='unmeasured'),
        indent=2) + '\n')
    print(out)

if __name__ == '__main__':
    main()
