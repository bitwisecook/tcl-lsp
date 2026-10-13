#!/usr/bin/env python3
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Run the bounded frozen-source Runtime namespace-forget controls."""
import datetime,gzip,hashlib,json,os,re,subprocess,time,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[5]
HERE=Path(__file__).resolve().parent
TARGET=ROOT/'tmp/name-target-issues'
ARGV=['cargo','test','--offline','-j','2','--manifest-path','runtime/rust/Cargo.toml','--lib','native_c86_forget_','--','--test-threads=1']
workspace=tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['dependencies']
roots=set()
def closure(crate,include_dev=False):
    crate=crate.resolve()
    if crate in roots:return
    roots.add(crate)
    data=tomllib.loads((crate/'Cargo.toml').read_text())
    tables=[data.get('dependencies',{}),data.get('build-dependencies',{})]
    if include_dev:tables.append(data.get('dev-dependencies',{}))
    for target in data.get('target',{}).values():
        tables += [target.get('dependencies',{}),target.get('build-dependencies',{})]
        if include_dev:tables.append(target.get('dev-dependencies',{}))
    for table in tables:
        for name,dep in table.items():
            if not isinstance(dep,dict):continue
            base=crate
            if dep.get('workspace'):
                dep=workspace[name];base=ROOT
            if isinstance(dep,dict) and 'path' in dep:closure(base/dep['path'])
closure(ROOT/'runtime/rust',True)
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def snapshot():
    paths={ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'rust-toolchain.toml'}
    for crate in roots:
        for name in ['Cargo.toml','Cargo.lock','build.rs']:
            if (crate/name).is_file():paths.add(crate/name)
        for subtree in ['src','include','csrc']:
            if (crate/subtree).is_dir():paths.update(p for p in (crate/subtree).rglob('*') if p.is_file())
    for subtree in [ROOT/'.cargo',ROOT/'runtime/rust/.cargo']:
        if subtree.is_dir():paths.update(p for p in subtree.rglob('*') if p.is_file())
    for relative in ['experiments/name-handling/results/namespace-forget-global-origin/issue-witness.tcl','experiments/name-handling/results/namespace-forget-global-origin/probe.tcl','experiments/name-handling/results/namespace-forget-global-origin/results/tcl8.6.18/stdout.txt','experiments/name-handling/results/namespace-forget-import-chain/probe.tcl','experiments/name-handling/results/namespace-forget-import-chain/results/tcl8.6.18/stdout.txt']:
        paths.add(ROOT/relative)
    return {str(p.relative_to(ROOT)):digest(p) for p in sorted(paths) if p.is_file()}
before=snapshot()
with gzip.open(HERE/'source-before.json.gz','wt') as out:json.dump(before,out,indent=2)
started=datetime.datetime.now(datetime.timezone.utc).isoformat();tick=time.monotonic()
with (HERE/'cargo-test.log').open('wb') as out:
    process=subprocess.run(ARGV,cwd=ROOT,stdout=out,stderr=subprocess.STDOUT,check=False,env=dict(os.environ,CARGO_TARGET_DIR=str(TARGET)))
after=snapshot()
with gzip.open(HERE/'source-after.json.gz','wt') as out:json.dump(after,out,indent=2)
log=(HERE/'cargo-test.log').read_text(errors='replace')
match=re.search(r'Running unittests .*?\(([^)]+)\)',log)
binary=None
if match:
    path=ROOT/match.group(1)
    if path.is_file():binary={'path':str(path),'sha256':digest(path),'size_bytes':path.stat().st_size}
changed=[p for p in sorted(set(before)|set(after)) if before.get(p)!=after.get(p)]
result={'question':'Three genuine Native C8.6 namespace-forget tests against retained native answers','argv':ARGV,'cwd':str(ROOT),'CARGO_TARGET_DIR':str(TARGET),'started_utc':started,'completed_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'elapsed_seconds':time.monotonic()-tick,'process_exit_code':process.returncode,'source_scope':'Conservative declared local normal/build/root-dev and target/optional dependency closure, source/include/csrc/manifests/config plus exact retained test inputs; not a claim that every recorded file compiled','local_crate_roots':[str(p.relative_to(ROOT)) for p in sorted(roots)],'source_file_count':len(before),'changed_source_paths':changed,'source_before_sha256':digest(HERE/'source-before.json.gz'),'source_after_sha256':digest(HERE/'source-after.json.gz'),'log_sha256':digest(HERE/'cargo-test.log'),'executed_test_binary':binary,'test_lines':[line for line in log.splitlines() if line.startswith('test cmd_namespace::tests::native_c86_forget_') or line.startswith('test result:')],'F5':'pending; not executed','broader_import_lifetimes':'#1778 remains unproved'}
(HERE/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'exit':process.returncode,'source_files':len(before),'source_changes':changed,'binary':binary,'test_lines':result['test_lines']},indent=2))
