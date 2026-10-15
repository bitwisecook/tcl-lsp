from pathlib import Path
import argparse
import datetime
import hashlib
import json
import os
import subprocess

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--repository', type=Path, default=HERE.parents[2])
parser.add_argument('--out', type=Path, required=True)
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=False)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
for purpose in ['relative-namespace', 'operand-effects']:
    archived = HERE / purpose
    original = json.loads((archived / 'manifest.json').read_text())
    probe = archived / 'probe.tcl'
    assert sha(probe) == original['probe_sha256']
    output = args.out / purpose
    output.mkdir()
    rows = []
    for row in original['rows']:
        binary = args.repository / Path(row['argv'][0]).relative_to('/workspace/tcl-lsp')
        selected = {str(args.repository / Path(path).relative_to('/workspace/tcl-lsp')): expected
                    for path, expected in row['original_files_before'].items()}
        before = {path: sha(Path(path)) for path in selected}
        assert before == selected
        env = dict(os.environ)
        library = row.get('selected_original_library')
        if library:
            library = str(args.repository / Path(library).relative_to('/workspace/tcl-lsp'))
            env['TCL_LIBRARY'] = library
        argv = [str(binary), str(probe)]
        run = subprocess.run(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)
        stem = row['provider_label'].replace('.', '-')
        stdout, stderr = output / (stem + '.stdout'), output / (stem + '.stderr')
        stdout.write_bytes(run.stdout)
        stderr.write_bytes(run.stderr)
        after = {path: sha(Path(path)) for path in before}
        assert before == after
        rows.append({'provider_label': row['provider_label'], 'argv': argv,
                     'selected_original_library': library,
                     'original_files_before': before, 'original_files_after': after,
                     'exit_code': run.returncode, 'stdout_file': stdout.name,
                     'stderr_file': stderr.name, 'stdout_sha256': sha(stdout),
                     'stderr_sha256': sha(stderr)})
    manifest = {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'probe_sha256': sha(probe), 'original_manifest_sha256': sha(archived / 'manifest.json'),
                'scope': 'New fixed original-provider observations only; no Rust/LSP/Native/frame/edit result.',
                'rows': rows}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
