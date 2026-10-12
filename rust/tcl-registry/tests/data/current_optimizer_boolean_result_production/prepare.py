from pathlib import Path
import hashlib,json,re
ROOT=Path('/workspace/tcl-lsp')
OUT=Path('/workspace/.proofs/optimizer-source-interpretation471')
CAP=Path('/workspace/.proofs/native-boolean-logical-sites465')
def digest(b):return hashlib.sha256(b).hexdigest()
def pin(path):
 p=Path(path);b=p.read_bytes();return {'path':str(p),'sha256':digest(b),'bytes':len(b)}
def retain(path,destination):
 p=Path(path);b=p.read_bytes();dest=OUT/destination;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(b);assert dest.read_bytes()==b
 return dict(pin(p),retained_path=str(destination),retained_sha256=digest(dest.read_bytes()))
def window(source,dest,first,last):
 b=Path(source).read_bytes();lines=b.splitlines(keepends=True);selected=b''.join(lines[first-1:last]);f=OUT/dest;f.parent.mkdir(parents=True,exist_ok=True);f.write_bytes(selected)
 return {'source_path':str(source),'source_sha256':digest(b),'first_line':first,'last_line':last,'retained_path':str(dest),'window_sha256':digest(selected)}
providers=[];inputs=[];windows=[]
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 root=ROOT/'tmp'/('tcl'+version);receipt_path=CAP/version/'receipt.json';receipt=json.loads(receipt_path.read_text());pins=receipt['inputs_verified_before_and_after']
 version_path=CAP/version/'cli-version.stdout';vb=version_path.read_bytes()
 assert digest(vb)==receipt['cli_version']['stdout_sha256']
 header=root/'generic/tcl.h';hb=header.read_bytes();match=re.search(rb'^#\s*define\s+TCL_PATCH_LEVEL\s+"([^"]+)"',hb,re.M);assert match and match[1].decode()==version
 rows=[]
 for relative in ['generic/tcl.h','generic/tclInt.h','generic/tclCompile.h','generic/tclBasic.c','generic/tclCompile.c','generic/tclCompExpr.c','unix/Makefile','unix/tclConfig.sh']:
  source=root/relative;row=retain(source,Path('providers')/version/'sources'/relative)
  old=pins.get(str(source));row['same_whole_sha_as_original465_launch_input']=old is not None
  if old is not None:assert old==row['sha256'];row['original465_recorded_sha256']=old
  else:row['original465_recorded_sha256']=None
  rows.append(row);inputs.append(row)
 optimizer=root/'generic/tclOptimize.c'
 if optimizer.is_file():
  row=retain(optimizer,Path('providers')/version/'sources/generic/tclOptimize.c');assert str(optimizer) not in pins
  row.update(same_whole_sha_as_original465_launch_input=False,original465_recorded_sha256=None);rows.append(row);inputs.append(row)
  lines=optimizer.read_bytes().splitlines();at=next(i for i,l in enumerate(lines,1) if b'case INST_TRY_CVT_TO_NUMERIC:' in l)
  windows.append(window(optimizer,Path('providers')/version/'windows/try-numeric-next-instruction.txt',at-4,at+51))
  for name,needle,radius in [('optimizer-installation','optimizer = TclOptimizeBytecode',12),('optimizer-call','optimizer)(&compEnv)',10)]:
   source=root/'generic'/('tclBasic.c' if name=='optimizer-installation' else 'tclCompile.c')
   slines=source.read_bytes().splitlines();line=next(i for i,l in enumerate(slines,1) if needle.encode() in l)
   windows.append(window(source,Path('providers')/version/('windows/'+name+'.txt'),line-radius,line+radius))
  existence={'exists':True,'source_path':str(optimizer),'source_sha256':row['sha256']}
 else:
  assert version in ['8.4.20','8.5.19']
  existence={'exists':False,'selected_path':str(optimizer),'scope':'Selected file absent in this inspected configured source tree; this proves no execution, opcode absence, compile absence, or absence of optimisation elsewhere.'}
 library=root/'unix'/('libtcl'+version.rsplit('.',1)[0]+'.a');lp=pin(library);assert pins[str(library)]==lp['sha256'];lp['same_whole_sha_as_original465_launch_input']=True;lp['original465_recorded_sha256']=pins[str(library)]
 lp['retention']='Exact library is an existing original465 required input; this source interpretation keeps a hash reference, not a new duplicate or new source-to-library build proof.'
 inputs.append(lp)
 providers.append({'provider':version,'declared_patch_level':match[1].decode(),'source_revision':'Exact whole-file content hashes are the inspected revision; no upstream commit identifier is inferred.',
 'original465_receipt':retain(receipt_path,Path('providers')/version/'context/original465-receipt.json'),
 'original465_version_stream':retain(version_path,Path('providers')/version/'context/original465-cli-version.stdout'),
 'original465_cli_version_sha256':receipt['cli_version']['stdout_sha256'],'selected_optimizer_file':existence,'files':rows,'library':lp,
 'purpose':'Current source interpretation only. Original465 execution records remain independent, unmodified, and supply no retroactive optimizer source pin.'})
request={'question_id':'naming.numeric.current-optimizer-boolean-result-production','question':'Which current configured C Tcl sources select numeric-conversion removal before a Boolean jump, and how does that source-level rule distinguish a reached TRY_CVT_TO_NUMERIC from inline and public-API result production?',
'problem':'A reached numeric instruction must execute conversion. An inline Boolean condition may have that instruction removed before execution. Applying one result recipe to both changes original cache/String effects; a public expression API has its own independent result producer.',
'kind':'source-interpretation','external_execution_statuses':{v:'not-tested' for v in ['C Tcl8.4','C Tcl8.5','C Tcl8.6','C Tcl9.0','C Tcl9.1','Jim','BIG-IP']},
'providers':providers,'source_windows':windows,'required_inputs':inputs,
'original465_context_request':retain(CAP/'request.json',Path('context/original465-request.json')),
'original465_context_closure':retain(CAP/'closure.json',Path('context/original465-closure.json')),
'conclusion':'The retained current C8.6/9.0/9.1 optimizer switch selects TRY_CVT_TO_NUMERIC for blanking when its next instruction is a conditional jump or listed numeric/logical instruction. Current tclBasic installs the optimizer and tclCompile invokes it. This explains the current selected source recipe; the independent original465 rows measure route/cache differences without launch-pinning this optimizer file. C8.4/C8.5 selected optimizer-file absence is only that filesystem fact.',
'limits':['No native process, compiler invocation, version command, or Rust test is run by this request.','No opcode listing or causal source-to-library rebuild equivalence is measured.','Matching header/config/library byte hashes establish correspondence with existing465 inputs where explicitly marked; optimizer and other previously unpinned sources are current independent source bytes.','No Jim/BIG-IP implementation or execution behaviour is inferred.','Original465 launch inputs, receipts and raw streams are immutable and remain a separate proof.'],
'implementation_owners':['rust/tcl-syntax/src/native_boolean_truth.rs::NativeExpressionResultProducer','rust/tcl-registry/src/native_boolean_truth.rs::native_numeric_instruction_result_protocol','rust/tcl-cmd-core/src/native_boolean_truth.rs::original_boolean_expression_result'],
'launches':0}
(OUT/'request.json').write_text(json.dumps(request,indent=2)+'\n')
(OUT/'manifest.json').write_text(json.dumps({'request':pin(OUT/'request.json'),'preparer':pin(OUT/'prepare.py'),'retained_files':[pin(p) for p in sorted((OUT/'providers').rglob('*')) if p.is_file()]+[pin(p) for p in sorted((OUT/'context').rglob('*')) if p.is_file()], 'required_external_library_inputs':[p for p in inputs if 'retention' in p],'windows':len(windows),'launches':0},indent=2)+'\n')
print(json.dumps({'request_sha256':digest((OUT/'request.json').read_bytes()),'providers':len(providers),'windows':len(windows),'retained_bytes':sum(p.stat().st_size for p in (OUT/'providers').rglob('*') if p.is_file())},indent=2))
