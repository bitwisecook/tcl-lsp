#!/usr/bin/env python3
"""Read-only fresh stock getter observations and frozen Native failure stack."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
VERSIONS = ('8.4.20', '8.5.19', '8.6.18', '9.0.4', '9.1.0')

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--runner', type=Path, required=True)
    args = parser.parse_args()
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    for name in ('input.tcl', 'command-control.c', 'capture.py'):
        (out / name).write_bytes((HERE / name).read_bytes())
    paths = [HERE / name for name in ('input.tcl', 'command-control.c', 'capture.py')]
    paths += [ROOT / name for name in ('rust/tcl-platform/src/lib.rs', 'rust/tcl-host-native/src/lib.rs',
        'rust/tcl-registry/src/commands/tcl/clock_.rs', 'rust/tcl-registry/src/special_vars.rs',
        'runtime/rust/src/cmd_clock.rs', 'runtime/rust/src/interp.rs')]
    runner = args.runner.resolve()
    paths.append(runner)
    for version in VERSIONS:
        tree = ROOT / ('tmp/tcl' + version)
        paths += [tree / name for name in ('generic/tclClock.c', 'generic/tclEnv.c', 'generic/tclInt.h',
            'library/init.tcl', 'library/clock.tcl', 'unix/tclsh') if (tree / name).is_file()]
        paths.append(tree / ('unix/libtcl' + '.'.join(version.split('.')[:2]) + '.a'))
    before = {str(p): sha(p) for p in paths}
    env = dict(os.environb)
    for key in (b'NH_CLOCK_GETENV_MISSING', b'TCLLIBPATH', b'TCL_BACKEND_CONSTRAINTS'):
        env.pop(key, None)
    env.update({b'NH_CLOCK_GETENV_TEST': b'host-value', b'NH_CLOCK_GETENV_\xff': b'raw-name-value',
        b'NH_CLOCK_GETENV_RAW_VALUE': b'\xff'})
    records = []

    def execute(name, argv, environment):
        dest = out / name
        dest.mkdir(parents=True)
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix='clock-getenv-question-') as cwd:
            with (dest / 'stdout.txt').open('wb') as stdout, (dest / 'stderr.txt').open('wb') as stderr:
                result = subprocess.run(['timeout', '--kill-after=5', '60', *argv], env=environment,
                    cwd=cwd, stdout=stdout, stderr=stderr, timeout=67)
        record = dict(name=name, argv=argv, exit_code=result.returncode,
            elapsed_seconds=round(time.monotonic()-start, 3),
            stdout_sha256=sha(dest / 'stdout.txt'), stderr_sha256=sha(dest / 'stderr.txt'))
        records.append(record)
        return result.returncode

    binary_hashes = {}
    for version in VERSIONS:
        tree = ROOT / ('tmp/tcl' + version)
        selected = dict(env)
        selected[b'TCL_LIBRARY'] = str(tree / 'library').encode()
        binary = out / ('command-control-' + version)
        argv = ['cc', '-DHAVE_UNISTD_H=1', '-I' + str(tree / 'unix'), '-I' + str(tree / 'generic'),
            str(HERE / 'command-control.c'), str(tree / ('unix/libtcl' + '.'.join(version.split('.')[:2]) + '.a')),
            '-lm', '-ldl', '-lpthread', '-lz', '-o', str(binary)]
        if execute(version + '/compile', argv, selected) != 0:
            continue
        binary_hashes[str(binary)] = sha(binary)
        execute(version + '/command-control', [str(binary)], selected)
        execute(version + '/script', [str(tree / 'unix/tclsh'), str(out / 'input.tcl')], selected)
    tree = ROOT / 'tmp/tcl8.6.18'
    stack = '\n'.join([
        f'source {{{tree / "library/tcltest/tcltest.tcl"}}}',
        'set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]',
        f'set code [catch {{source {{{tree / "tests/all.tcl"}}}}} value]',
        'puts [list outcome $code $value]',
        'if {$code} {puts [list errorcode $::errorCode]; puts [list errorinfo $::errorInfo]}', ''])
    (out / 'all-stack.tcl').write_text(stack)
    selected = dict(env)
    selected[b'TCL_LIBRARY'] = str(tree / 'library').encode()
    execute('runtime/all-stack', [str(runner), '--quiet', '--tcl-version', '8.6', '--init', str(out / 'all-stack.tcl')], selected)
    after = {str(p): sha(p) for p in paths}
    (out / 'provenance.json').write_text(json.dumps(dict(source_and_binary_before=before,
        source_and_binary_after=after, control_binary_hashes=binary_hashes, records=records,
        environment_control='Only explicitly named NH_CLOCK_GETENV_* keys are queried/output; synthetic values in fresh child processes, including raw Unix byte key/value.',
        scope='Read-only Rust diagnosis. Fresh stock C8.4–9.1; existing frozen Native runner only for caught original all.tcl stack. No Rust edits/builds; no F5 or Windows inference.',
        bootstrap='matching real Tcl_Init/library; Native existing --init',
        input_sha256=sha(out / 'input.tcl'), all_stack_sha256=sha(out / 'all-stack.tcl')),
        indent=2) + '\n')
    print(out)

if __name__ == '__main__':
    main()
