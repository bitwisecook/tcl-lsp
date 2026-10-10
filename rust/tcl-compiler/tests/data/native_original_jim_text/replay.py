#!/usr/bin/env python3
"""Verify retained original source Text controls; optionally capture exact pinned providers anew."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify():
    aggregate = json.loads((ROOT / 'receipt.json').read_text())
    cases = {row['id']: row['source'].encode('ascii') for row in json.loads((ROOT / 'inputs.json').read_text())['cases']}
    for receipt in aggregate:
        directory = ROOT / receipt['provider']
        assert json.loads((directory / 'receipt.json').read_text()) == receipt
        for name, field in [('probe.c', 'probe_sha256'), ('inputs.json', 'inputs_sha256'), ('queue.json', 'queue_sha256'), ('capture.py', 'runner_sha256')]:
            assert sha(ROOT / name) == receipt[field], name
        for name, field in [('stdout.tsv', 'stdout_sha256'), ('stderr', 'stderr_sha256'), ('compile.stdout', 'compile_stdout_sha256'), ('compile.stderr', 'compile_stderr_sha256')]:
            assert sha(directory / name) == receipt[field], name
        assert receipt['compile_exit'] == receipt['process_exit'] == 0
        assert not (directory / 'stderr').read_bytes()
        lines = (directory / 'stdout.tsv').read_text().splitlines()
        assert lines == receipt['rows']
        rows = {parts[0]: (int(parts[1]), bytes.fromhex(parts[2])) for line in lines for parts in [line.split('|')]}
        assert len(rows) == len(lines) == (14 if receipt['provider'] == 'jim' else 25)
        modes = ['JIM_SOURCE'] if receipt['provider'] == 'jim' else ['SCRIPT_CODE_FLAGS_ZERO', 'DIRECT_FLAG']
        if receipt['provider'] == 'jim':
            assert rows['DIRECT_MODE_UNAVAILABLE'] == (0, b'Jim has no TCL_EVAL_DIRECT recipe')
        for mode in modes:
            for case, source in cases.items():
                assert rows['INPUT_' + mode + '_' + case] == (0, source)
                code, result = rows[mode + '_' + case + '_RESULT']
                assert code == (1 if case.startswith('UNKNOWN_') else 0)
                if case in ('UNQUOTED_ESCAPED', 'QUOTED_ESCAPED'):
                    assert result == b'p\xed\xa0\x80'
                elif case == 'EMPTY_QUOTED':
                    assert result == b''
                elif case == 'OPAQUE_PROC_HEAD':
                    assert result == b'VALUE'
        print(receipt['provider'], 'retained input/hash/mode/handler/result verification PASS')
    return aggregate


def recapture(output, original):
    queue = json.loads((ROOT / 'queue.json').read_text())
    for provider in queue['providers']:
        for name, digest in provider['required_sha256'].items():
            if sha(Path(name)) != digest:
                raise RuntimeError('provider input drift: ' + name)
    output.mkdir()
    receipts = []
    for provider, retained in zip(queue['providers'], original, strict=True):
        directory = output / provider['id']
        directory.mkdir()
        executable = directory / 'probe'
        command = ['cc', '-std=c99', *provider['flags'], str(ROOT / 'probe.c'), provider['library'], *provider['link_flags'], '-o', str(executable)]
        built = subprocess.run(command, capture_output=True, timeout=60)
        (directory / 'compile.stdout').write_bytes(built.stdout)
        (directory / 'compile.stderr').write_bytes(built.stderr)
        receipt = {'provider': provider['id'], 'required_sha256': provider['required_sha256'], 'compile_command': command, 'compile_exit': built.returncode, 'probe_sha256': sha(ROOT / 'probe.c'), 'inputs_sha256': sha(ROOT / 'inputs.json'), 'runner_sha256': sha(Path(__file__)), 'environment': provider['environment']}
        if built.returncode == 0:
            environment = os.environ.copy()
            environment.update(provider['environment'])
            ran = subprocess.run([str(executable)], env=environment, capture_output=True, timeout=60)
            (directory / 'stdout.tsv').write_bytes(ran.stdout)
            (directory / 'stderr').write_bytes(ran.stderr)
            receipt.update(process_exit=ran.returncode, executable_sha256=sha(executable), stdout_sha256=sha(directory / 'stdout.tsv'), stderr_sha256=sha(directory / 'stderr'))
        (directory / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        receipts.append(receipt)
        (output / 'receipt.json').write_text(json.dumps(receipts, indent=2) + '\n')
        if built.returncode or receipt.get('process_exit') != 0 or (directory / 'stdout.tsv').read_bytes() != (ROOT / retained['provider'] / 'stdout.tsv').read_bytes() or (directory / 'stderr').read_bytes():
            raise RuntimeError('fresh provider differs; attempted receipts retained at ' + str(directory))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument('--verify-only', action='store_true')
    operation.add_argument('--output', type=Path)
    arguments = parser.parse_args()
    original = verify()
    if arguments.output is not None:
        recapture(arguments.output, original)
