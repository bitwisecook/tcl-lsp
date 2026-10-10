#!/usr/bin/env python3
"""Compare each exact literal/alias source against its original native capture."""
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--c-root', type=Path, required=True)
    parser.add_argument('--jim-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    args.output.mkdir(parents=True, exist_ok=False)
    checks = []
    for variant in ['v1', 'v2']:
        source = fixture / variant / 'cases.tcl'
        for provider, version in [('tcl8.4', '8.4.20'), ('tcl8.5', '8.5.19'),
                                  ('tcl8.6', '8.6.18'), ('tcl9.0', '9.0.4'),
                                  ('tcl9.1', '9.1.0'), ('jim', 'jim')]:
            original = fixture / variant / provider
            receipt = json.loads((original / 'receipt.json').read_text())
            base = args.jim_root if provider == 'jim' else args.c_root / ('tcl' + version)
            executable = base / 'jimsh' if provider == 'jim' else base / 'unix/tclsh'
            header = base / 'jim.h' if provider == 'jim' else base / 'generic/tcl.h'
            library = base / 'libjim.a' if provider == 'jim' else base / 'unix' / ('libtcl' + '.'.join(version.split('.')[:2]) + '.a')
            makefile = base / 'Makefile' if provider == 'jim' else base / 'unix/Makefile'
            owner = base / 'jim.c' if provider == 'jim' else base / 'generic/tclCompCmds.c'
            pairs = [(source, 'source_sha256'), (executable, 'executable_sha256'),
                     (header, 'header_sha256'), (library, 'library_sha256'),
                     (makefile, 'makefile_sha256'), (owner, 'source_owner_sha256')]
            pairs += [(original / name, name.replace('.', '_') + '_sha256')
                      for name in ['stdout', 'stderr', 'startup.input', 'startup.stdout', 'startup.stderr']]
            for path, key in pairs:
                if sha(path) != receipt[key]:
                    raise ValueError('changed exact input or original stream: ' + str(path))
            if args.verify_only:
                checks.append(dict(variant=variant, provider=provider, executed=False, exact_inputs_verified=True))
                continue
            destination = args.output / variant / provider
            destination.mkdir(parents=True)
            environment = os.environ.copy()
            if provider != 'jim':
                environment['TCL_LIBRARY'] = str(base / 'library')
            command = [str(executable), str(source)]
            # Tcl/Jim error headings include the original source-file argument.
            # Reuse that exact recorded path for v1 Jim, which aborts before cases.
            if variant == 'v1' and provider == 'jim':
                original_path = Path(receipt['source'])
                if not original_path.is_file() or sha(original_path) != receipt['source_sha256']:
                    raise ValueError('original v1 Jim error source path is required: ' + str(original_path))
                command = [str(executable), str(original_path)]
            result = subprocess.run(command, capture_output=True, env=environment, timeout=30)
            (destination / 'stdout').write_bytes(result.stdout)
            (destination / 'stderr').write_bytes(result.stderr)
            equal = (result.returncode == receipt['process_exit']
                     and sha(destination / 'stdout') == receipt['stdout_sha256']
                     and sha(destination / 'stderr') == receipt['stderr_sha256'])
            checks.append(dict(variant=variant, provider=provider, executed=True, equal=equal,
                               command=command, process_exit=result.returncode,
                               executable_sha256=sha(executable)))
            (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')
            if not equal:
                raise RuntimeError('changed native outcome: ' + variant + '/' + provider)
    (args.output / 'comparisons.json').write_text(json.dumps(checks, indent=2) + '\n')


if __name__ == '__main__':
    main()
