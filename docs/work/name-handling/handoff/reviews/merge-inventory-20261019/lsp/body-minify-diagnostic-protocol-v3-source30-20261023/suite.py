#!/usr/bin/env python3
"""Exact body minify diagnostic 5; immutable preparation, root GO required."""
from pathlib import Path
import datetime, fcntl, hashlib, importlib.util, json, os, re, shutil, subprocess, sys

ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
BASE=Path('/tmp/lsp-body-minify-diagnostic-v3-source30-direct5')
PLAN=BASE/'plan.json'
GO=BASE/'root-go.json'
MANIFEST=BASE/'runner-input-manifest.json'
SOURCE30=Path('/tmp/next-diagnostic-batch-composition-v3-20261023T0252Z/combined-manifest.json')
CAPTURE=ROOT/'docs/work/name-handling/validation-tools/capture-inputs.py'
LOCK=Path('/tmp/tcl-lsp-naming-build.lock')
CARGO=Path('/workspace/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/cargo')
RUNS=Path('/tmp/lsp-body-minify-diagnostic-v3-source30-direct5-runs')

def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()
def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def save(path,value): Path(path).write_text(json.dumps(value,sort_keys=True,indent=2)+'\n')
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def require(condition,message):
    if not condition: raise RuntimeError('STOP: '+message)
def source_view(go,go_sha,plan):
    require(sha(GO)==go_sha,'root GO changed')
    require(go.get('plan_sha256')==sha(PLAN) and go.get('runner_input_manifest_sha256')==sha(MANIFEST),'GO plan/runner pin mismatch')
    require(go.get('source30_manifest_sha256')==sha(SOURCE30)==plan['lifecycle']['source30_manifest_sha256'],'source30 manifest pin mismatch')
    approved=go.get('approved_integrated_head')
    require(isinstance(approved,str) and re.fullmatch(r'[0-9a-f]{40}',approved),'missing exact approved HEAD')
    require(approved==plan['lifecycle']['source30_integrated_head'],'GO approved HEAD differs from exact source30 integration')
    require(git('rev-parse',approved+'^{tree}')==go.get('approved_integrated_tree')==plan['lifecycle']['source30_integrated_tree'],'approved tree mismatch')
    require(subprocess.run(['git','merge-base','--is-ancestor','3b1a750be337',approved],cwd=ROOT,check=False).returncode==0,'approved HEAD not after baseline')
    current=git('rev-parse','HEAD')
    require(subprocess.run(['git','merge-base','--is-ancestor',approved,current],cwd=ROOT,check=False).returncode==0,'current HEAD not descendant of approved HEAD')
    commits=git('rev-list',approved+'..'+current).splitlines()
    for commit in commits:
        changed=git('diff-tree','--no-commit-id','--name-only','-r','-m',commit).splitlines()
        require(all(p.startswith('docs/work/name-handling/') for p in changed),'non-doc descendant commit: '+commit)
    tracked=subprocess.check_output(['git','diff','--name-only',approved,'--','.',':(exclude)docs/work/name-handling'],cwd=ROOT,text=True).splitlines()
    untracked=subprocess.check_output(['git','ls-files','--others','--exclude-standard','-z'],cwd=ROOT).split(b'\0')
    require(not tracked and all(not p or p.decode(errors='surrogateescape').startswith('docs/work/name-handling/') for p in untracked),'unapproved worktree product/untracked delta')
    for path,expected in json.loads(MANIFEST.read_text())['inputs'].items(): require(sha(path)==expected,'runner input changed: '+path)
    for item in plan['source_pins']: require(sha(ROOT/item['path'])==item['sha256'],'source pin changed: '+item['path'])
    final_hashes=json.loads(SOURCE30.read_text())['final_hashes']
    require(len(final_hashes)==30,'source30 inventory changed')
    for path,expected in final_hashes.items(): require(sha(ROOT/path)==expected,'source30 final hash changed: '+path)
    for key in ('post30_report14_evidence','post30_typecheck_evidence'):
        evidence=go.get(key)
        require(isinstance(evidence,dict) and evidence.get('path') and evidence.get('sha256') and sha(evidence['path'])==evidence['sha256'],'gate evidence missing/changed: '+key)
    return current

def disk(paths):
    result={}
    for raw in paths:
        path=Path(raw)
        while not path.exists(): path=path.parent
        result[str(raw)]=shutil.disk_usage(path).free
    require(min(result.values())>=2300000000,'actual mount below 2.3GB floor: '+str(result))
    return result

def snapshot(prefix):
    subprocess.run([sys.executable,str(CAPTURE),str(prefix)+'.source.json'],cwd=ROOT,check=True)
    subprocess.run([sys.executable,str(BASE/'providers.py'),str(prefix)+'.providers.json'],cwd=ROOT,check=True)
    return {'source':sha(str(prefix)+'.source.json'),'providers':sha(str(prefix)+'.providers.json')}

def parse_cargo(path):
    rows=[]
    for line in Path(path).read_text().splitlines():
        if line.strip(): rows.append(json.loads(line))
    return rows

lock=None; out=None; before=None; artifacts={}; results=[]; go_sha=None; exit_code=2
try:
    plan=json.loads(PLAN.read_text()); rows=plan['selectors']
    require(len(rows)==5 and len({x['selector'] for x in rows})==5 and all(x['expected_tests']==1 and x['exact'] for x in rows),'exact diagnostic5 inventory changed')
    require(sha(BASE/'guarded-exec.py')==plan['lifecycle']['executor_sha256'],'executor changed')
    finite=Path(plan['lifecycle']['body_finite_plan'])
    require(sha(finite)==plan['original_finite_plan_sha256'],'original finite plan changed')
    original=json.loads(finite.read_text())
    require(rows==[dict(x,expected_tests=x['expected_selected']) for x in original['filters']] and plan['diagnostic_env']==original['env'],'original filters/counts/trace changed')
    require(SOURCE30.is_file() and sha(SOURCE30)==plan['lifecycle']['source30_manifest_sha256'],'source30 manifest changed')
    require(GO.is_file(),'no root GO; no lock or Cargo')
    go=json.loads(GO.read_text()); go_sha=sha(GO)
    source_view(go,go_sha,plan)
    require(CARGO.is_file(),'pinned Cargo absent')
    target_dir=Path(os.environ['CARGO_TARGET_DIR']).resolve()
    disk((ROOT,'/tmp',target_dir,RUNS))
    lock=open(LOCK,'a')
    try: fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    except BlockingIOError: raise RuntimeError('STOP: build lock occupied; no queue')
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    out=RUNS/stamp; out.mkdir(parents=True,exist_ok=False)
    save(out/'plan.json',plan); save(out/'go.json',go)
    before=snapshot(out/'before')
    spec=importlib.util.spec_from_file_location('lsp_guarded_exec',BASE/'guarded-exec.py')
    require(spec is not None and spec.loader is not None,'executor loader missing')
    module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    cap=20000000
    for package,target,build_args,name,kind in [('tcl-lsp-core','lsp_edit_workspace',['--test','lsp_edit_workspace'],'lsp_edit_workspace',['test']),('tcl-lsp-core','minify_residual',['--test','minify_residual'],'minify_residual',['test']),('tcl-lsp-core','core_lib',['--lib'],'tcl_lsp_core',['lib']),('tcl-compiler','compiler_lib',['--lib'],'tcl_compiler',['lib'])]:
        source_view(go,go_sha,plan); require(snapshot(out/(target+'-prebuild'))==before,'source/provider changed before build')
        disk((ROOT,'/tmp',target_dir,out))
        prefix=out/('build-'+target)
        argv=[str(CARGO),'test','--locked','-p',package,*build_args,'--no-run','--message-format=json']
        save(str(prefix)+'.argv.json',argv)
        rec=module.run(argv,cwd=ROOT,env=os.environ.copy(),stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(CARGO),deadline_seconds=3600,cap_bytes=cap,disk_floor_bytes=2300000000,disk_paths=(ROOT,'/tmp',target_dir,out))
        require(rec['complete'] and rec['stop_reason'] is None and rec['exit_code']==0 and rec['group_empty_before_parent_reap'],'guarded Cargo incomplete')
        events=parse_cargo(str(prefix)+'.stdout')
        finished=[e for e in events if e.get('reason')=='build-finished']
        require(len(finished)==1 and finished[0].get('success'),'current Cargo build-finished missing/failure')
        warnings=[e for e in events if e.get('reason')=='compiler-message' and e.get('message',{}).get('level') in ('warning','error')]
        build_script_warnings=[e for e in events if e.get('reason')=='build-script-executed' and e.get('warnings')]
        stderr_warnings=[line for line in Path(str(prefix)+'.stderr').read_text(errors='replace').splitlines() if re.match(r'^warning:',line)]
        save(str(prefix)+'.warnings.json',{'compiler_messages':warnings,'build_script_warnings':build_script_warnings,'stderr_warning_lines':stderr_warnings})
        require(not warnings and not build_script_warnings and not stderr_warnings,'Cargo warning/error gate')
        candidates=[e for e in events if e.get('reason')=='compiler-artifact' and e.get('target',{}).get('name')==name and e.get('target',{}).get('kind')==kind and e.get('profile',{}).get('test') is True and e.get('executable') and Path(e.get('manifest_path','')).resolve()==ROOT/('rust/'+package+'/Cargo.toml')]
        require(len(candidates)==1,'fresh exact Cargo test artifact absent/ambiguous')
        exe=Path(candidates[0]['executable']).resolve()
        require(exe.is_file() and exe.is_relative_to(target_dir),'test executable outside actual target')
        server=None
        artifacts[target]={'test':str(exe),'test_sha256':sha(exe),'server':str(server) if server else None,'server_sha256':sha(server) if server else None,'event':candidates[0]}
        save(str(prefix)+'.artifact.json',artifacts[target])
        source_view(go,go_sha,plan); require(snapshot(out/(target+'-postbuild'))==before,'source/provider changed during build')
        for index,row in enumerate(x for x in rows if x['package']==package and x['target']==(' '.join(build_args))):
            source_view(go,go_sha,plan); require(snapshot(out/(target+'-pretest-'+str(index)))==before,'source/provider changed before selector')
            disk((ROOT,'/tmp',target_dir,out))
            require(sha(exe)==artifacts[target]['test_sha256'],'test artifact changed before selector')
            if server: require(sha(server)==artifacts[target]['server_sha256'],'server artifact changed before selector')
            env=os.environ.copy(); env.update(plan['diagnostic_env'])
            label=f'{target}-{index:02}'
            prefix=out/label
            argv=[str(exe),row['selector'],'--exact','--nocapture','--test-threads=1']
            rec=module.run(argv,cwd=ROOT,env=env,stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(exe),deadline_seconds=180,cap_bytes=cap,disk_floor_bytes=2300000000,disk_paths=(ROOT,'/tmp',target_dir,out))
            text=Path(str(prefix)+'.stdout').read_text(errors='replace')+'\n'+Path(str(prefix)+'.stderr').read_text(errors='replace')
            summaries=re.findall(r'^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
            require(len(summaries)==1 and re.findall(r'^running (\d+) tests?$',text,re.M)==['1'],'exact single test start/summary missing or duplicated')
            passed_count,failed_count,ignored_count=map(int,summaries[0])
            require(passed_count+failed_count==1 and ignored_count==0 and rec['exit_code'] in (0,101),'exact one original test outcome missing')
            require((passed_count==1)==(rec['exit_code']==0),'test summary/exit disagreement')
            test_warnings=bool(re.search(r'^warning:',text,re.M))
            passed=(rec['complete'] and rec['stop_reason'] is None and rec['exit_code']==0 and rec['group_empty_before_parent_reap'] and rec['timer_armed_before_spawn'] and sha(exe)==artifacts[target]['test_sha256'] and (not server or sha(server)==artifacts[target]['server_sha256']) and 'running 1 test' in text and 'test result: ok. 1 passed; 0 failed' in text and not test_warnings)
            result={'selector':row['selector'],'expected_tests':1,'actual_selected':passed_count+failed_count,'actual_passed':passed_count,'actual_failed':failed_count,'passed':passed,'test_warnings':test_warnings,'diagnostic_env':plan['diagnostic_env'],'process':rec,'test_artifact_sha256':sha(exe),'server_artifact_sha256':sha(server) if server else None,'stdout_sha256':sha(str(prefix)+'.stdout'),'stderr_sha256':sha(str(prefix)+'.stderr')}
            results.append(result); save(str(prefix)+'.result.json',result); save(out/'results.json',results)
            source_view(go,go_sha,plan); require(snapshot(out/(target+'-posttest-'+str(index)))==before,'source/provider changed after selector')
            require(rec['complete'] and rec['stop_reason'] is None and rec['group_empty_before_parent_reap'] and rec['timer_armed_before_spawn'],'selector lifecycle guard incomplete: '+row['selector'])
            require(sha(exe)==artifacts[target]['test_sha256'] and (not server or sha(server)==artifacts[target]['server_sha256']),'selector artifact changed: '+row['selector'])
    require(len(results)==5 and {x['selector'] for x in results}=={x['selector'] for x in rows},'exact 5 result inventory mismatch')
    source_view(go,go_sha,plan); require(snapshot(out/'after')==before,'final source/provider changed')
    require(all(sha(a['test'])==a['test_sha256'] and (not a['server'] or sha(a['server'])==a['server_sha256']) for a in artifacts.values()),'final artifact drift')
    all_passed=all(x['passed'] for x in results)
    save(out/'summary.json',{'passed':all_passed,'count':5,'source_provider_stable':True,'artifact_stable':True,'go_sha256':go_sha,'finished_utc':utc()})
    print(('PASS' if all_passed else 'FAIL')+' exact5 '+str(out),flush=True)
    exit_code=0 if all_passed else 1
except BaseException as error:
    print('STOP '+repr(error),file=sys.stderr,flush=True)
    if out is not None:
        final={'passed':False,'reason':repr(error),'results_observed':len(results),'finished_utc':utc(),'go_sha256':go_sha}
        try:
            final['after_snapshot']=snapshot(out/'failure-after')
            final['source_provider_stable']=before is not None and final['after_snapshot']==before
            final['artifact_stable']=all(sha(a['test'])==a['test_sha256'] and (not a['server'] or sha(a['server'])==a['server_sha256']) for a in artifacts.values())
        except Exception as guard_error:
            final['final_guard_incomplete']=repr(guard_error)
            final['source_provider_stable']=False; final['artifact_stable']=False
        save(out/'failure.json',final)
    exit_code=2
finally:
    if lock is not None:
        try:
            fcntl.flock(lock,fcntl.LOCK_UN); lock.close()
            if out is not None: save(out/'lock-released.json',{'utc':utc(),'pid':os.getpid(),'released':True})
        except Exception as release_error:
            if out is not None: save(out/'lock-release-incomplete.json',{'utc':utc(),'reason':repr(release_error),'released':False})
            exit_code=2
raise SystemExit(exit_code)
