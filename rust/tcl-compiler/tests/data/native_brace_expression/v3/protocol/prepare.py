from pathlib import Path
import hashlib, json
root=Path('/workspace/.proofs/native-brace-expression129-v3')
original=Path('/workspace/.proofs/recovered-naming-literal-expression129/native-probe/case-pairs.json')
cases=json.loads(original.read_text())
for entry in cases:
 entry['original_hex']=entry['original'].encode().hex()
 entry['candidate_hex']=entry['candidate'].encode().hex()
cases.extend([
 {'id':'raw-zero-literal-string','original_hex':b'expr "\\"A\0B\\" eq \\"A\0B\\""'.hex(),'candidate_hex':b'expr {"A\0B" eq "A\0B"}'.hex(),'claim_scope':'actual counted source/raw00; no source escape substitution'},
 {'id':'modified-zero-literal-string','original_hex':b'expr "\\"A\xc0\x80B\\" eq \\"A\xc0\x80B\\""'.hex(),'candidate_hex':b'expr {"A\xc0\x80B" eq "A\xc0\x80B"}'.hex(),'claim_scope':'actual C080 bytes independent of raw00/source numeric escape'},
 {'id':'escaped-zero-literal-string','original':r'expr "\"A\u0000B\" eq \"A\u0000B\""','candidate':r'expr {"A\u0000B" eq "A\u0000B"}','claim_scope':'outer versus expression numeric escape scheduling; raw byte equality must be measured'},
 {'id':'execution-observer-source','original':'proc observe {command op} {set ::seen $command}; trace add execution expr enter observe; expr "1 + 2"','candidate':'proc observe {command op} {set ::seen $command}; trace add execution expr enter observe; expr {1 + 2}','claim_scope':'observer receives source command; no equivalence permission'},
 {'id':'shadowed-handler-body','original':'proc expr {x} {uplevel 1 {info body p}}; proc p {} {expr "1 + 2"}; p','candidate':'proc expr {x} {uplevel 1 {info body p}}; proc p {} {expr {1 + 2}}; p','claim_scope':'user handler inspects calling body; differing result is meaningful negative'},
])
header=[]
records=[]
for index,case in enumerate(cases):
 item={k:v for k,v in case.items() if k not in ['original','candidate']}
 values=[]
 for variant in ['original','candidate']:
  value=bytes.fromhex(case[variant+'_hex']) if variant+'_hex' in case else case[variant].encode()
  split=value.rfind(b'; ')
  prefix=value[:split+2] if split>=0 else b''
  invocation=value[split+2:] if split>=0 else value
  operand=invocation.split(b' ',1)
  # Input-only collector uses exactly this bounded final command's written operands.
  collector=prefix+b'__argv'+(b' '+operand[1] if len(operand)==2 else b'')
  if case['id']=='shadowed-handler-body':
   collector=b'__argv "1 + 2"' if variant=='original' else b'__argv {1 + 2}'
  for role,data in [('source',value),('argv',collector)]:
   path=root/'inputs'/case['id']/(variant+'.'+role)
   path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
   item[variant+'_'+role+'_hex']=data.hex()
   item[variant+'_'+role+'_sha256']=hashlib.sha256(data).hexdigest()
   symbol=f'case_{index}_{variant}_{role}'
   header.append('static const unsigned char '+symbol+'[] = {'+','.join(str(c) for c in data)+',0};')
   values.extend([symbol,str(len(data))])
 header.append('')
 records.append((case['id'],values))
 item['id']=case['id'];case.clear();case.update(item)
header.append('static const Case cases[] = {')
for name,values in records:header.append('{"'+name+'",'+','.join(values)+'},')
header.append('};')
(root/'cases.h').write_text('\n'.join(header)+'\n')
(root/'inputs.json').write_text(json.dumps({'original_case_pairs_sha256':hashlib.sha256(original.read_bytes()).hexdigest(),'cases':cases},indent=2)+'\n')
print('prepared',len(cases),'pairs, counted source and separately authored argv-collector inputs')
