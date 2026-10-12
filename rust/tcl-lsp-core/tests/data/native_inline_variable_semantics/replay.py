#!/usr/bin/env python3
"""Verify retained variable inlining controls, or replay exact selected builds."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
PROVIDERS = {'c84', 'c85', 'c86', 'c90', 'c91', 'jim'}

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def original_rows(variant):
    directory = ROOT / variant
    receipt = json.loads((directory / 'receipts.json').read_text())
    for row in receipt['records']:
        source = directory / row['source']
        if source.read_bytes().hex() != row['source_hex'] or digest(source) != row['source_sha256']:
            raise ValueError('source association changed: ' + str(source))
        for stream in ['stdout', 'stderr']:
            if (directory / row[stream]).read_bytes().hex() != row[stream + '_hex']:
                raise ValueError('original stream association changed: ' + row[stream])
        if row['version_exit'] != 0:
            raise ValueError('original version query did not complete')
        yield directory, row

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--provider', action='append', default=[], metavar='CODE=EXE')
    parser.add_argument('--variant', choices=['stdin', 'file', 'both'], default='file')
    parser.add_argument('--receipt', type=Path)
    args = parser.parse_args()
    variants = ['stdin', 'file'] if args.variant == 'both' else [args.variant]
    rows = [(variant, directory, row) for variant in variants for directory, row in original_rows(variant)]
    if args.verify_only:
        print(json.dumps({'verified_original_associations': len(rows), 'variants': variants, 'native_launches': 0}))
        return
    try:
        providers = dict(value.split('=', 1) for value in args.provider)
    except ValueError:
        parser.error('--provider requires CODE=EXE')
    if set(providers) != PROVIDERS or len(args.provider) != len(PROVIDERS):
        parser.error('supply each of c84/c85/c86/c90/c91/jim exactly once')
    if args.receipt is None:
        parser.error('--receipt is required for actual replays')
    for code, executable in providers.items():
        selected = [row for _, _, row in rows if row['provider'] == code]
        expected_hashes = {row['executable_sha256'] for row in selected}
        path = Path(executable).resolve()
        if expected_hashes != {digest(path)}:
            parser.error('selected build differs from original executable: ' + code)
        version = subprocess.run([str(path)], input=b'puts [info patchlevel]\n', capture_output=True, timeout=10)
        if version.returncode != 0 or any(version.stdout.hex() != row['version_stdout_hex'] for row in selected):
            parser.error('selected executed version differs from original: ' + code)
    comparisons = []
    for variant, directory, row in rows:
        executable = str(Path(providers[row['provider']]).resolve())
        source = directory / row['source']
        argv = [executable, str(source)] if variant == 'file' else [executable]
        completed = subprocess.run(argv, input=None if variant == 'file' else source.read_bytes(), capture_output=True, timeout=10)
        expected_stderr = bytes.fromhex(row['stderr_hex'])
        if variant == 'file':
            # File-error diagnostics contain the argv source filename. Only
            # that exact recorded filename is replaced; all other bytes stay.
            expected_stderr = expected_stderr.replace(row['argv'][1].encode(), str(source).encode())
        matches = completed.returncode == row['exit'] and completed.stdout.hex() == row['stdout_hex'] and completed.stderr == expected_stderr
        comparisons.append({'variant': variant, 'provider': row['provider'], 'source': row['source'],
                            'source_sha256': digest(source), 'argv': argv, 'exit': completed.returncode,
                            'stdout_hex': completed.stdout.hex(), 'stderr_hex': completed.stderr.hex(),
                            'expected_filename_adjusted_stderr_hex': expected_stderr.hex(), 'matches_original': matches})
    result = {'scope': 'Exact fixed source controls; no Rust refactoring or ambient interpreter result.',
              'comparisons': comparisons, 'passed': all(row['matches_original'] for row in comparisons)}
    args.receipt.write_text(json.dumps(result, indent=2) + '\n')
    if not result['passed']:
        raise SystemExit(1)

if __name__ == '__main__':
    main()
