#!/usr/bin/env python3
"""Exact LSP startup/UPF 4; immutable preparation, root GO required."""
from pathlib import Path
import datetime, fcntl, hashlib, importlib.util, json, os, re, shutil, subprocess, sys

ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
BASE=Path('/tmp/vm-notom16-runtime-template-v1')
PLAN=BASE/'plan.json'
GO=BASE/'root-go.json'
MANIFEST=BASE/'runner-input-manifest.json'
SOURCE122=Path('/tmp/naming-reviewed-product-actual-identity-v11/actual-source122-manifest.json')
CAPTURE=ROOT/'docs/work/name-handling/validation-tools/capture-inputs.py'
LOCK=Path('/tmp/tcl-lsp-naming-build.lock')
CARGO=Path('/workspace/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/cargo')
RUNS=Path('/workspace/naming-validation-runs/vm-notom16-runtime-template-v1')

def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()
def unique_json(pairs):
    value={}
    for key,child in pairs:
        if key in value:raise RuntimeError('STOP: duplicate JSON key '+key)
        value[key]=child
    return value
def read_json_text(text):return json.loads(text,object_pairs_hook=unique_json)
def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def save(path,value): Path(path).write_text(json.dumps(value,sort_keys=True,indent=2)+'\n')
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def require(condition,message):
    if not condition: raise RuntimeError('STOP: '+message)
def environment_override_keys(plan, environment):
    exact=set(plan['refused_environment_exact'])
    return sorted(key for key in environment if key in exact or any(key.startswith(pattern['prefix']) and key.endswith(pattern['suffix']) for pattern in plan['refused_environment_patterns']))

def source_view(go,go_sha,plan):
    global last_head, head_samples
    require(sha(GO)==go_sha,'root GO changed')
    require(not environment_override_keys(plan,os.environ),'unsupported build environment override')
    require(str(CARGO)==plan['cargo_path'] and sha(CARGO)==plan['cargo_sha256'],'pinned Cargo path/hash changed')
    require(sha(plan['rustc_path'])==plan['rustc_sha256'],'pinned rustc changed')
    require(all(os.environ.get(key)==value for key,value in plan['required_environment'].items()),'required build environment mismatch')
    require(os.environ.get('PATH','').startswith(plan['path_prefix']+':'),'required tool PATH prefix absent')
    require(go.get('plan_sha256')==sha(PLAN) and go.get('runner_input_manifest_sha256')==sha(MANIFEST),'GO plan/runner pin mismatch')
    require(go.get('source122_manifest_sha256')==sha(SOURCE122)==plan['lifecycle']['source122_manifest_sha256'],'source122 manifest pin mismatch')
    identity=plan['actual_identity']
    require(sha(identity['path'])==identity['sha256'],'actual source122 identity changed')
    current_identity=read_json_text(Path(identity['path']).read_text())
    require(current_identity.get('approved_integrated_head')==plan['approved_product_head'] and current_identity.get('approved_integrated_tree')==plan['approved_product_tree'] and current_identity.get('path_count')==plan['source_path_count'],'actual identity product/count mismatch')
    typing_plan=plan['typecheck_plan']
    require(sha(typing_plan['path'])==typing_plan['sha256'] and sha(typing_plan['runner_inputs_path'])==typing_plan['runner_inputs_sha256'],'current typecheck protocol changed')
    for path,pin in json.loads(Path(typing_plan['runner_inputs_path']).read_text())['inputs'].items():
        require(sha(path)==pin,'typecheck transitive input changed: '+path)
    approved=go.get('approved_integrated_head')
    require(isinstance(approved,str) and re.fullmatch(r'[0-9a-f]{40}',approved),'missing exact approved HEAD')
    require(approved==plan['approved_product_head'],'GO differs from actual integrated source122 product')
    approved_tree=go.get('approved_integrated_tree')
    require(isinstance(approved_tree,str) and re.fullmatch(r'[0-9a-f]{40}',approved_tree),'missing exact approved tree')
    require(git('rev-parse',approved+'^{tree}')==approved_tree==plan['approved_product_tree'],'approved tree mismatch')
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
    for path,expected in read_json_text(MANIFEST.read_text())['inputs'].items(): require(sha(path)==expected,'runner input changed: '+path)
    for item in plan['source_pins']: require(sha(ROOT/item['path'])==item['sha256'],'source pin changed: '+item['path'])
    source_rows=read_json_text(SOURCE122.read_text())['paths']
    require(len(source_rows)==plan['source_path_count'] and len({row['path'] for row in source_rows})==plan['source_path_count'],'source122 inventory changed')
    for row in source_rows:
        path=row['path']; expected=row['after_sha256']
        require(sha(ROOT/path)==expected,'source122 final hash changed: '+path)
        committed=subprocess.run(['git','show',approved+':'+path],cwd=ROOT,capture_output=True)
        require(committed.returncode==0 and hashlib.sha256(committed.stdout).hexdigest()==expected,'approved product source122 blob mismatch: '+path)
    require(subprocess.run(['git','diff','--quiet',approved,'--','.',':(exclude)docs/work/name-handling'],cwd=ROOT).returncode==0,'worktree product differs from approved HEAD/tree')
    for key in ('post122_eight_package_typecheck_evidence',):
        evidence=go.get(key)
        require(isinstance(evidence,dict) and evidence.get('path') and evidence.get('sha256') and sha(evidence['path'])==evidence['sha256'],'gate evidence missing/changed: '+key)
    typing=read_json_text(Path(go['post122_eight_package_typecheck_evidence']['path']).read_text())
    require(typing.get('status')=='PASS' and typing.get('overall_exit')==0 and typing.get('cargo_exit_code')==0,'eight-package check not PASS0')
    require(all(typing.get(key) is True for key in ('guard_valid','warning_free_json_stderr_stdout','source_stable','providers_stable','root_go_stable','head_docs_descendant_valid')),'eight-package check guard/warning seal incomplete')
    require(typing.get('plan_sha256')==plan['lifecycle']['post122_eight_package_typecheck_plan_sha256'],'wrong eight-package check plan')
    require(typing.get('approved_product_head')==approved and typing.get('approved_product_tree')==approved_tree,'eight-package product HEAD/tree mismatch')
    require(typing.get('selected_packages')==plan['required_typecheck_packages'],'eight-package check package inventory mismatch')
    if last_head is not None:
        require(subprocess.run(['git','merge-base','--is-ancestor',last_head,current],cwd=ROOT).returncode==0,'actual HEAD moved outside monotonic docs-only history')
    head_samples.append({'utc':utc(),'actual_head':current})
    last_head=current
    return current

def disk(paths):
    result={}
    for raw in paths:
        path=Path(raw)
        while not path.exists(): path=path.parent
        result[str(raw)]=shutil.disk_usage(path).free
    require(min(result.values())>=2*1024**3,'actual mount below 2GiB floor: '+str(result))
    return result

def snapshot(prefix):
    subprocess.run([sys.executable,str(CAPTURE),str(prefix)+'.source.json'],cwd=ROOT,check=True)
    subprocess.run([sys.executable,str(BASE/'providers.py'),str(prefix)+'.providers.json'],cwd=ROOT,check=True)
    return {'source':sha(str(prefix)+'.source.json'),'providers':sha(str(prefix)+'.providers.json')}

def parse_cargo(path):
    rows=[]
    for line in Path(path).read_text().splitlines():
        if line.strip(): rows.append(read_json_text(line))
    return rows

MIRROR=Path('/tmp/vm-runtime-ci5205-no-tommath')
MIRROR_LINKS=None
def runtime_files(root):
    return {str(p.relative_to(root)):sha(p) for p in root.rglob('*') if p.is_file() and 'target' not in p.relative_to(root).parts}
def mirror_check():
    if MIRROR_LINKS is not None:require({n:os.readlink(MIRROR/n)for n in MIRROR_LINKS}==MIRROR_LINKS,'mirror lexical links changed')
    for name in ['rust','experiments','Cargo.toml','Cargo.lock']:
        p=MIRROR/name;require(p.is_symlink() and p.resolve()==ROOT/name,'mirror root link changed '+name)
        if p.is_file():require(sha(p)==sha(ROOT/name),'mirror manifest bytes changed')
    require(not (MIRROR/'tmp').exists(),'mirror fallback source exists')
    require(runtime_files(MIRROR/'runtime/rust')==runtime_files(ROOT/'runtime/rust'),'mirror source differs')
def mirror_prepare(out):
    global MIRROR_LINKS
    MIRROR.mkdir(exist_ok=True)
    for name in ['rust','experiments','Cargo.toml','Cargo.lock']:
        p=MIRROR/name
        if not p.exists() and not p.is_symlink():p.symlink_to(ROOT/name,target_is_directory=(ROOT/name).is_dir())
    root=MIRROR/'runtime/rust';root.mkdir(parents=True,exist_ok=True);actual=runtime_files(ROOT/'runtime/rust');prior=runtime_files(root);require(not set(prior)-set(actual),'mirror removed source needs successor, no deletion')
    for n,h in actual.items():
        dst=root/n
        if prior.get(n)==h:continue
        if dst.exists():
            savepath=out/'prior-mirror-source'/(n+'.txt');savepath.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(dst,savepath);require(sha(savepath)==prior[n],'prior source retention failed')
        dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/'runtime/rust'/n,dst)
    mirror_check();MIRROR_LINKS={n:os.readlink(MIRROR/n)for n in ['rust','experiments','Cargo.toml','Cargo.lock']};save(out/'mirror.json',{'prior':prior,'actual':actual,'links':{n:os.readlink(MIRROR/n)for n in ['rust','experiments','Cargo.toml','Cargo.lock']}})
lock=None;out=None;before=None;results=[];artifact=None;go_sha=None;last_head=None;head_samples=[];remaining=20*1024**2;control_results=[]
try:
    plan=read_json_text(PLAN.read_text());rows=plan['selectors'];require(len(rows)==16 and len({x['selector']for x in rows})==16,'original16 inventory mismatch');require(sha(BASE/'guarded-exec.py')==plan['lifecycle']['executor_sha256'],'executor drift');require(GO.is_file(),'no root GO; no lock or Cargo');go=read_json_text(GO.read_text());go_sha=sha(GO);source_view(go,go_sha,plan)
    target_dir=Path(os.environ['CARGO_TARGET_DIR']).resolve();disk((ROOT,'/tmp',target_dir,RUNS));lock=open(LOCK,'a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);out=RUNS/datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ');out.mkdir(parents=True);save(out/'plan.json',plan);save(out/'go.json',go);before=snapshot(out/'before');mirror_prepare(out)
    spec=importlib.util.spec_from_file_location('guarded',BASE/'guarded-exec.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    def execute(argv,prefix,pin,seconds,env):
        global remaining
        require(remaining>0,'aggregate log cap exhausted');r=module.run(argv,cwd=ROOT,env=env,stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=str(pin),deadline_seconds=seconds,cap_bytes=remaining,disk_floor_bytes=2*1024**3,disk_paths=(ROOT,'/tmp',target_dir,out));n=r.get('retained_bytes');require(isinstance(n,int)and 0<=n<=remaining,'invalid log accounting');remaining-=n;save(out/'aggregate-log-budget.json',{'cap_bytes':20*1024**2,'remaining_bytes':remaining});require(remaining>0 and r['complete']and r['stop_reason']is None and r['group_empty_before_parent_reap']and r['timer_armed_before_spawn']and r['artifact_unchanged'],'process guard incomplete');return r
    baseline_outdir=None;build_index=0
    def build(mode):
        global baseline_outdir,build_index
        source_view(go,go_sha,plan);mirror_check();disk((ROOT,'/tmp',target_dir,out));build_index+=1;prefix=out/(f'build-{build_index:02d}-'+mode);prior_cap=baseline_outdir/'tommath-capability.json' if baseline_outdir is not None else None;require(mode=='intentional' or (prior_cap is not None and prior_cap.is_file() and read_json_text(prior_cap.read_text())=={'detected':False,'intentional_absence':True}),'transition lacks freshly rearmed capability');prior_cap_sha=sha(prior_cap)if prior_cap is not None and prior_cap.exists()else None;env=os.environ.copy();env.pop('TCL_TOMMATH_DIR',None);env.pop('TCL_RUNTIME_EXPECT_NO_TOMMATH',None)
        if mode in ['intentional','mismatch']:env['TCL_RUNTIME_EXPECT_NO_TOMMATH']='1'
        if mode=='invalid':env['TCL_RUNTIME_EXPECT_NO_TOMMATH']='invalid'
        if mode in ['full','mismatch']:env['TCL_TOMMATH_DIR']=plan['normal_tommath_source']
        original=plan['original_build_argv'];require(original==['cargo','test','--locked','--manifest-path',str(MIRROR/'runtime/rust/Cargo.toml'),'--lib','--no-run','--message-format=json','-j','2'],'original args changed');argv=[str(CARGO)]+original[1:];save(str(prefix)+'.argv.json',argv);save(str(prefix)+'.environment.json',{k:env.get(k)for k in ['TCL_RUNTIME_EXPECT_NO_TOMMATH','TCL_TOMMATH_DIR','CARGO_TARGET_DIR']});rec=execute(argv,prefix,CARGO,3600,env);events=parse_cargo(str(prefix)+'.stdout');text=Path(str(prefix)+'.stdout').read_text(errors='replace')+'\n'+Path(str(prefix)+'.stderr').read_text(errors='replace');success=mode not in ['invalid','mismatch'];require(rec['exit_code']==(0 if success else 101),'build exit does not match diagnostic case');finished=[x for x in events if x.get('reason')=='build-finished'];require(len(finished)==1 and finished[0]['success']==success,'terminal Cargo status mismatch')
        messages=[x for x in events if x.get('reason')=='compiler-message' and x.get('message',{}).get('level')in ['warning','error']];warnings=re.findall(r'^warning:.*',text,re.M)
        if mode=='unexpected':require(not messages and warnings and all('libtommath source not found; bignum backend disabled'in x for x in warnings),'unexpected absence must retain only original warning')
        else:require(not messages and not warnings,'strict Rust0W; no warning waiver')
        script_events=[x for x in events if x.get('reason')=='build-script-executed' and 'tcl-runtime' in x.get('package_id','')]
        if success:
            require(len(script_events)==1,'actual runtime buildscript event absent/ambiguous');bs=script_events[0];od=Path(bs['out_dir']).resolve();require(od.is_relative_to(target_dir),'OUT_DIR outside target')
            if baseline_outdir is None:baseline_outdir=od
            require(od==baseline_outdir,'same OUT_DIR transition proof refused');cfg=bs.get('cfgs',[]);require(('have_tommath'in cfg)==(mode=='full'),'actual backend cfg differs');output=od.parent/'output';require(output.is_file(),'actual buildscript stdout missing');raw=output.read_text(errors='replace');require(('cargo:rustc-cfg=have_tommath'in raw)==(mode=='full'),'buildscript cfg raw mismatch')
        require(baseline_outdir is not None,'no established original OUT_DIR');cap=baseline_outdir/'tommath-capability.json'
        if mode=='intentional':require(cap.is_file()and read_json_text(cap.read_text())=={'detected':False,'intentional_absence':True},'fresh intentional capability missing');require('tommath_capability=absent-intentional'in raw,'capability Cargo metadata missing')
        else:require(not cap.exists(),'stale capability metadata survived transition')
        if mode=='invalid':require('TCL_RUNTIME_EXPECT_NO_TOMMATH must be absent or exactly 1'in text,'invalid expectation actual cause missing')
        if mode=='mismatch':require('TomMath sources found despite TCL_RUNTIME_EXPECT_NO_TOMMATH=1'in text,'present mismatch actual cause missing')
        save(str(prefix)+'.capability.json',{'mode':mode,'intentional_capability_before_sha256':prior_cap_sha,'out_dir':str(baseline_outdir),'capability_exists':cap.exists(),'capability_sha256':sha(cap)if cap.exists()else None,'buildscript_events':script_events,'warnings':warnings,'diagnostic_only':mode!='intentional','process':rec});source_view(go,go_sha,plan);mirror_check();return events
    for transition in ['unexpected','full','invalid','mismatch']:
        build('intentional');build(transition);control_results.append({'transition':transition,'same_actual_OUT_DIR':str(baseline_outdir),'passed':True,'release_admission':False});save(out/'controls.json',control_results)
    events=build('intentional');candidates=[x for x in events if x.get('reason')=='compiler-artifact'and x.get('target',{}).get('name')=='tcl_runtime'and set(x.get('target',{}).get('kind',[]))=={'rlib','cdylib','staticlib'}and x.get('profile',{}).get('test')is True and x.get('executable')and Path(x.get('manifest_path','')).resolve()==MIRROR/'runtime/rust/Cargo.toml'];require(len(candidates)==1,'fresh standalone artifact absent/ambiguous');event=candidates[0];exe=Path(event['executable']).resolve();require(exe.is_relative_to(target_dir)and exe.is_file(),'artifact outside actual target');artifact={'path':str(exe),'sha256':sha(exe),'event':event};save(out/'artifact.json',artifact)
    prefix=out/'roster';r=execute([str(exe),'--list','--format','terse'],prefix,exe,180,os.environ.copy());require(r['exit_code']==0,'roster failed');names=re.findall(r'^(.+): test$',Path(str(prefix)+'.stdout').read_text(),re.M);require(names and len(names)==len(set(names)),'empty/duplicate actual roster');save(out/'roster.json',names)
    for row in rows:
        name=row['selector'];require(names.count(name)==1,'exact original selector absent/duplicate');source_view(go,go_sha,plan);mirror_check();require(sha(exe)==artifact['sha256'],'artifact drift');prefix=out/(f'{len(results)+1:02d}-'+name);argv=[str(exe),name,'--exact','--nocapture','--test-threads=1'];save(str(prefix)+'.argv.json',argv);rec=execute(argv,prefix,exe,180,os.environ.copy());text=Path(str(prefix)+'.stdout').read_text(errors='replace')+'\n'+Path(str(prefix)+'.stderr').read_text(errors='replace');started=[int(x)for x in re.findall(r'^running (\d+) tests?$',text,re.M)];printed=re.findall(r'^test ([^ ]+) \.\.\.',text,re.M);summary=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M);require(started==[1]and printed==[name]and len(summary)==1,'libtest exact identity/count mismatch');status,passed,failed,ignored=summary[0];passed,failed,ignored=map(int,(passed,failed,ignored));require(ignored==0 and (status,passed,failed,rec['exit_code'])in [('ok',1,0,0),('FAILED',0,1,101)],'libtest status/exit mismatch');require(not re.search(r'^warning:',text,re.M),'runtime warning');source_view(go,go_sha,plan);require(sha(exe)==artifact['sha256'],'artifact drift');result={'selector':name,'passed':passed==1,'summary':summary,'started':started,'printed':printed,'process':rec,'provisional_until_final_seals':True};results.append(result);save(str(prefix)+'.result.json',result);save(out/'results.json',results);print('RESULT',len(results),name,'PASS'if passed else'FAIL',flush=True)
    require(len(results)==16 and len(control_results)==4,'terminal inventory mismatch');mirror_check();source_view(go,go_sha,plan);require(snapshot(out/'after')==before,'final full source/provider drift');require(sha(exe)==artifact['sha256'],'final artifact drift');source_view(go,go_sha,plan);save(out/'head-samples.json',head_samples);save(out/'final.json',{'count':16,'PASS':sum(x['passed']for x in results),'FAIL':sum(not x['passed']for x in results),'controls':control_results,'artifact':artifact,'source_provider_stable':True,'artifact_stable':True,'warnings':False,'diagnostic_warnings_separate':True,'finished_utc':utc(),'go_sha256':go_sha});exit_code=0 if all(x['passed']for x in results)else 1;print('COMPLETE original16 overall='+str(exit_code)+' receipt='+str(out),flush=True)
except BaseException as error:
    print('STOP '+repr(error),file=sys.stderr,flush=True);exit_code=2
    if out is not None:
        failure={'protocol_failure':True,'reason':repr(error),'results_observed':len(results),'finished_utc':utc()}
        try:failure['after_snapshot']=snapshot(out/'failure-after');failure['source_provider_stable']=before is not None and failure['after_snapshot']==before
        except Exception as guard_error:failure['final_guard_incomplete']=repr(guard_error)
        save(out/'failure.json',failure)
finally:
    if lock is not None:fcntl.flock(lock,fcntl.LOCK_UN);lock.close();print('LOCK_RELEASED '+utc(),flush=True)
sys.exit(exit_code)
