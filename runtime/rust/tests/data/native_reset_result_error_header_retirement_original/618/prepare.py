from pathlib import Path
import copy, hashlib, json
BASE=Path(__file__).parent;OLD=Path('/workspace/.proofs/native-explicit-error-code-header-setter614c')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
original=json.loads((OLD/'request.json').read_bytes());request=copy.deepcopy(original)
launcher=(OLD/'launch.py').read_text()
launcher=launcher.replace('no output from614/614b is executable input.','all reset producers and own executable outputs are fresh. No setter ELF is reset evidence.')
launcher=launcher.replace('CLOSED_ORIGINAL_EXPLICIT_ERROR_CODE_HEADER_SETTER','CLOSED_ORIGINAL_RESET_RESULT_ERROR_HEADER_RETIREMENT')
launcher=launcher.replace("e['closed_cases'] == 4","e['closed_cases'] == request['expected_cases_per_provider']")
launcher=launcher.replace('Original selected setter/free/getter/public error phases remain bounded by their exact rows.','Original selected reset, free, compatibility trace, caller release and counted-getter phases remain bounded by their exact rows.')
launcher=launcher.replace('Original explicit error-code header API closure:', 'Original reset-result error-header API closure:')
compile(launcher,str(BASE/'launch.py'),'exec');(BASE/'launch.py').write_text(launcher)
source_windows=[]
for provider in original['providers']:
 path=Path('/workspace/tcl-lsp/tmp')/('tcl'+provider)/'generic/tclResult.c';raw=path.read_bytes();lines=raw.split(b'\n')
 for symbol in ['Tcl_ResetResult','ResetObjResult']:
  text=raw.decode();start=text.index('\n'+symbol+'(')+1;end=text.index('\n}\n',start)+3
  first=text[:start].count('\n')+1;last=text[:end].count('\n');window=b'\n'.join(lines[first-1:last])+b'\n'
  assert window.decode()==text[start:end]
  dest=BASE/'source-windows'/provider/(symbol+'.c');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(window)
  source_windows.append(dict(provider=provider,symbol=symbol,source=str(path),source_sha256=sha(path),source_bytes=len(raw),physical_LF_first=first,physical_LF_last=last,window=str(dest),window_sha256=sha(dest),window_bytes=len(window),source_join='Exact whole source and physical LF function extent; no configure/build replay or native process is inferred.'))
(BASE/'source-windows.json').write_text(json.dumps(source_windows,indent=2)+'\n')
selected=json.loads((OLD/'selected-makefile-builds.json').read_bytes())
(BASE/'selected-makefile-builds-original614c.json').write_bytes((OLD/'selected-makefile-builds.json').read_bytes())
request.update(question_id='naming.error.original-reset-result-error-header-retirement',document='docs/design/analysis/name-resolution-proofs/error-original-reset-result-error-header-retirement.md',fixture_root='runtime/rust/tests/data/native_reset_result_error_header_retirement_original',status='prepared-not-compiled-or-executed; Root alone launches after independent source/protocol review',problem_statement='For precisely prepared authentic original Interp/header roles, which code/info/result headers remain selected during public Tcl_ResetResult free callbacks, which compatibility variable-write/getter stages are reached, and which real caller/global/private holds remain after reset? Pure observation does not establish mutating reentry, Host/ABA, arbitrary header lifetime or a public lazy-info producer.',probe_file=str(BASE/'probe.c'),probe_sha256=sha(BASE/'probe.c'),launcher_file=str(BASE/'launch.py'),launcher_sha256=sha(BASE/'launch.py'),expected_cases_per_provider=5)
for build in request['provider_builds']:
 build['command']=[arg.replace(str(OLD),str(BASE)) for arg in build['command']]
 assert str(BASE/'probe.c') in build['command']
 assert build['command'][-1]==str(BASE/'providers'/build['provider']/'probe.elf')
request['cases']=[
 dict(id=0,input='Original lazy result only; code/info absent after explicit baseline reset/unset/reset. Original result has its genuine result-only reference.',purpose='Public reset and original result freeIntRep/empty-header role; no selected error-header work is invented.'),
 dict(id=1,input='Distinct original code/info headers with only their real selected private/global holds; caller holds explicitly released before reset.',purpose='Modern original code then info retirement, or actual C84 global persistence; no post-free header dereference.'),
 dict(id=2,input='Distinct original code/info headers with one declared caller hold each plus selected private/global storage.',purpose='Reset removes only selected native private holds; subsequent real caller release is a distinct phase.'),
 dict(id=3,input='Same distinct caller-held headers plus pure global write traces; modern ERR_LEGACY_COPY selected by explicit direct test flag. C84 has no private-copy flag.',purpose='Actual compatibility global setter/trace stages before private decrements; no public flag producer or C84 modern-private path is claimed.'),
 dict(id=4,input='The same original C header occupies code and info with two real selected holds plus one declared caller hold; I is absent. Original result is independent.',purpose='Aliased code/info roles are counted separately; this is not result/error aliasing or same-header setter self-reentry.')]
request['observers']=dict(free='Custom original freeIntRep callback records only passed header refcount plus raw current selected field pointers/flags. The hook alone does not prove object death versus in-place representation retirement. No interpreter mutation, variable worker, String getter or shell creation occurs in the observer.',global_write='Pure Tcl_TraceVar2 callback counter/current-field observer in case3 only. Registration is an explicit setup worker, with exact return codes; it is not an observation that no variable shell was created.',rows='Only direct actual Interp/global-table and live header fields; no public String rendering. Original pointers with any freeIntRep callback are never dereferenced afterward; current result is inspected independently.',getter='Separate Tcl_GetStringFromObj on precisely caller-held original headers after reset; counted bytes are retained without a byte/Unicode/parser equivalence claim.',teardown='Observers disabled before actual Tcl_DeleteInterp. Teardown results/callback counts are outside the measured phases.')
request['raw_payload_hex']='524553455400ff20484541444552'
request['source_windows']=dict(path=str(BASE/'source-windows.json'),sha256=sha(BASE/'source-windows.json'),count=len(source_windows))
request['scope']='Five fresh selected C producers call the actual public Tcl_ResetResult on deliberately prepared genuine original object and Interp fields. Modern errorInfo input uses direct actual private field installation with an explicit native +1; compatibility flag input uses the actual field macro, not an invented public producer. C84 selects its actual global code/info storage and has no modern private retirement. Original headers, result, private/global/caller reference roles, pure callback phases and later counted String presentation remain separate. No Host/ABA, mutating reentry, arbitrary pointer lifetime, Rust/VM parity, physical frame, whole-suite or performance outcome is supplied.'
request['independent_producer_configuration']='Exact actual selected SDK Makefile CC/CC_SWITCHES/LIBS and unix cwd retained from admitted614c input provenance, with only fresh reset probe/output paths replaced. Every selected header/link/runtime input is still strictly pinned; actual new maps and own ELFs are required. Original SDK configure/build replay remains absent.'
request['reused_admission_origin']=dict(request=str(OLD/'request.json'),request_sha256=sha(OLD/'request.json'),closure=str(OLD/'closure.json'),closure_sha256=sha(OLD/'closure.json'),scope='Selected SDK/compiler/system input provenance only; no setter own executable or setter observation is reset evidence.')
request['prior_independent_attempts']=[]
request['input_extension']='The exact614c required input map is retained as immutable SDK/compiler provenance, plus the fresh reset probe/launcher and ten independently rejoined reset/result helper windows. New producer/header/link/runtime verification remains mandatory.'
request['process_protocol']='Actual compiler-version then five independent own compile/dependencies/execute processes; save each original receipt/streams before post-verification. Closure requires every actual execute0, five genuine CLOSED_CASE markers and its reset-only final marker, all own/dependency identities and input SHA pins unchanged. This is process closure, not a semantic or Rust test pass.'
request['jim']='No original Jim Tcl_ResetResult C API process is selected, compiled or run; this is not a measured Jim unavailable-worker response or Jim result semantics.'
request['bigip']='No BIG-IP process executes this question.'
for p in [BASE/'probe.c',BASE/'launch.py',BASE/'source-windows.json',BASE/'selected-makefile-builds-original614c.json']+[Path(w['window']) for w in source_windows]:request['required_sha256'][str(p)]=sha(p)
for w in source_windows:request['required_sha256'][w['source']]=w['source_sha256']
(BASE/'request.json').write_text(json.dumps(request,indent=2)+'\n')
files=[BASE/'probe.c',BASE/'launch.py',BASE/'request.json',BASE/'source-windows.json',BASE/'selected-makefile-builds-original614c.json']+[Path(w['window']) for w in source_windows]
manifest=dict(status='READY fresh immutable source request for peer review; no native or compiler process launched by publisher',question=request['question_id'],request_sha256=sha(BASE/'request.json'),probe_sha256=request['probe_sha256'],launcher_sha256=request['launcher_sha256'],required_input_count=len(request['required_sha256']),selected_original_C_producers=5,cases_per_C=5,physical_LF_source_windows=10,files=[dict(path=str(p),sha256=sha(p),bytes=p.stat().st_size) for p in files],old_setter_executables_reused=False,SDK_configure_build_replay_available=False,Jim_or_BIGIP_execution_claim=False)
(BASE/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(dict(manifest_sha256=sha(BASE/'manifest.json'),request_sha256=manifest['request_sha256'],probe_sha256=manifest['probe_sha256'],launcher_sha256=manifest['launcher_sha256'],inputs=manifest['required_input_count'],native_launches=0)))
