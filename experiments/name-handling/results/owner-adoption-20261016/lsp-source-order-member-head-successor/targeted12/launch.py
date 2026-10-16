#!/usr/bin/env python3
"""Root-only execution launcher for twelve targeted ordinary controls.

Prepared by source/hash reads only. Requires exact actual frozen input/ELF/env
preflight; delegates listing/exact execution180s/list30s to the canonical runner.
No build, Git, network, source rewrite, future selection or manifest fallback.
"""
import argparse,datetime,hashlib,json,os,shutil,subprocess
from pathlib import Path
ROOT=Path('/workspace/tcl-lsp')
PACKAGE=Path('/tmp/lsp-member-source-order-targeted-replay')
MANIFEST=PACKAGE/'selection.json'
EXPECTED='c4e0c71c4de30d05122906e00e33e98f4766355cb33411339bb12dc383efa1c2'

def sha(p):
    with Path(p).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def exact_source(rows):
    for x in rows:
        p=ROOT/x['path']
        assert p.is_file() and sha(p)==x['sha256'], 'Frozen ordinary source drift: '+x['path']

def exact_artifacts(rows,destination):
    for target,x in rows.items():
        p=Path(x['path'])
        assert p.resolve().is_relative_to((destination/'frozen-binaries').resolve())
        assert p.is_file() and p.stat().st_size==x['bytes'] and sha(p)==x['sha256'], 'Frozen ELF drift: '+target
        with p.open('rb') as stream:assert stream.read(4)==b'\x7fELF', 'Not actual native ELF: '+target

if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--out',required=True,type=Path,help='Root explicit NEW result directory; canonical runner refuses existing results');args=ap.parse_args()
    out=args.out.resolve();assert not out.exists(), 'Never overwrite replay evidence';assert not out.is_relative_to(ROOT.resolve()), 'Repository output forbidden'
    assert sha(MANIFEST)==EXPECTED, 'Targeted selection digest mismatch'
    m=json.loads(MANIFEST.read_text());assert m['total_exact_selectors']==12 and len(m['execution_order'])==12
    assert m['test_timeout_seconds']==180 and m['listing_timeout_seconds']==30
    assert not [k for k,v in os.environ.items() if v and (k.startswith('TCL_LSP_TRACE') or k=='TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC')], 'Instrumentation environment cannot run ordinary replay'
    current=m['actual_successful_five_library_source'];destination=Path(current['destination'])
    for key in ['source_binding','input_manifest','artifacts_manifest','source_diff']:
        x=current[key];assert sha(x['path'])==x['sha256'], 'Frozen receipt drift: '+key
    build=json.loads(Path(current['source_binding']['path']).read_text());assert build['ordinary'] is True and build['instrumentation'] is False and build['exit_code']==0 and build['input_drift_count']==0
    assert build['input_before_count']==build['input_after_count']==39001
    assert {p.replace('-','_') for p in build['packages']}=={'tcl_compiler','tcl_registry','tcl_lsp_core','tcl_lsp_db','tcl_lsp_server'} and len(build['packages'])==5
    assert sha(current['source_diff']['path'])==build['rust_diff_sha256']
    inputs=json.loads(Path(current['input_manifest']['path']).read_text());assert len(inputs)==39001
    exact_source(inputs)
    frozen={x['path']:x['sha256'] for x in inputs}
    for x in m['current_source_files']:assert frozen[x['path']]==x['sha256']
    assert all(x['current_native_linux_source_eligible'] for x in m['current_source_evidence'])
    artifacts=json.loads(Path(current['artifacts_manifest']['path']).read_text());assert len(artifacts)==5
    exact_artifacts(artifacts,destination)
    for x in m['reviewed_controls']:assert sha(x['path'])==x['sha256'], 'Reviewed selector provenance drift'
    prior=m['retained460'];assert sha(prior['path'])==prior['sha256']
    old=json.loads(Path(prior['path']).read_text())
    for x in m['retained_original_rows']:assert old['execution_order'][x['old_ordinal']-1]==x['row_verbatim']
    assert m['execution_order'][:4]==[x['row_verbatim'] for x in m['retained_original_rows']]
    setup=m['provider_setup'];assert sha(setup['path'])==setup['sha256']
    required=setup['required_environment'];assert all(os.environ.get(k)==v for k,v in required.items()), 'Original mandatory provider environment mismatch'
    assert shutil.which('tclsh9.0')=='/workspace/.toolchains/tcl/bin/tclsh9.0'
    assert shutil.which('tclsh8.6')=='/workspace/.toolchains/tcl/bin/tclsh8.6'
    assert Path(required['TCL_LSP_JIMSH']).is_file()
    runner=m['canonical_runner'];assert sha(runner['path'])==runner['sha256'], 'Canonical replay runner drift'
    argv=['python3',runner['path'],'--manifest',str(MANIFEST),'--manifest-sha256',EXPECTED,'--artifacts',current['artifacts_manifest']['path'],'--root',str(ROOT),'--out',str(out)]
    receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'PREFLIGHT_PASS_TARGETED12_ONLY','argv':argv,'manifest_sha256':EXPECTED,'actual_build_binding':current,'all39001_current_input_hashes_match':True,'all_five_actual_frozen_elf_hashes_match':True,'original_provider_environment':required,'whole460_selected':False,'scope':'Four retained originals222/223/224/242 plus one actual member issuer control and seven source-order controls only; no cumulative/future/Native completion claim'}
    out.parent.mkdir(parents=True,exist_ok=True);receiptpath=out.with_name(out.name+'-invocation.json');assert not receiptpath.exists(),'Never overwrite invocation evidence';receiptpath.write_text(json.dumps(receipt,indent=2)+'\n')
    print(json.dumps({'status':receipt['status'],'controls':12,'manifest':EXPECTED}),flush=True)
    code=subprocess.run(argv,cwd=ROOT).returncode
    # Canonical runner independently records exact singleton/listing/timeouts
    # and selected ELF afterhashes. Preserve all-five/source postflight too.
    exact_source(inputs);exact_artifacts(artifacts,destination)
    receipt.update(canonical_exit_code=code,all39001_input_hashes_still_match=True,all_five_actual_elf_hashes_still_match=True,completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat());receiptpath.write_text(json.dumps(receipt,indent=2)+'\n')
    raise SystemExit(code)
