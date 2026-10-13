from pathlib import Path
import hashlib,json
p=Path(__file__).parent
checks=[]
for provider in ['c84','c85','c86','c90','c91','jim']:
 def stdout(group):return (p/group/provider/'stdout.bin').read_text()
 text=stdout('shared-family-controls');names=[line.split('\t')[2] for line in text.splitlines() if line.startswith('SPEC-PROBE\tadded\t')];want=[':lost','N0::pZ','N[0]::p*','__spec_probeOther'] if provider=='jim' else ['::N0::pZ','::N[0]::p*',':lost','__spec_probeOther'];assert names==want and 'SPEC-PROBE\tdone\t4\t' in text
 checks.append({'provider':provider,'selector':'package original root operands','expected':want,'passed':True})
 text=stdout('shared-family-ambiguous-controls');assert ('SPEC-PROBE\tadded\tN:::x\t' in text and 'SPEC-PROBE\tdone\t1\t' in text) if provider=='jim' else ('command inventory has no unambiguous namespace address' in text and 'SPEC-PROBE\tdone\t0\t' in text)
 checks.append({'provider':provider,'selector':'ambiguous namespace original context','expected':'Jim flat callable N:::x' if provider=='jim' else 'C whole-inventory explicit refusal','passed':True})
 text=stdout('shared-family-planned-source-controls');missing=[line.split('\t')[2] for line in text.splitlines() if line.startswith('SPEC-TEST\tmissing\t')];assert missing==['::N[missing]::absent','::N[0]::absent*','absent*'];assert 'SPEC-TEST\tdone\t-\t14' in text
 checks.append({'provider':provider,'selector':'presence original selected shared-family script forms','observations':14,'passed':True})
 text=stdout('shared-family-root-regression-controls');rows=[line for line in text.splitlines() if line.startswith('PRESENCE ')];assert len(rows)==8;assert [line for line in rows if line.endswith(' 1')]==['PRESENCE ghost* 1','PRESENCE quer? 1']
 checks.append({'provider':provider,'selector':'existing root presence regressions','observations':8,'passed':True})
 for group in ['shared-family-controls','shared-family-ambiguous-controls','shared-family-planned-source-controls','shared-family-root-regression-controls']:
  r=json.loads((p/group/provider/'provenance.json').read_text());assert r['exit_code']==0 and r['seal_changes']=={} and (p/group/provider/'stderr.bin').stat().st_size==0
validation={'scope':'Original executable script forms only; CLI/Studio Rust controls authored/unexecuted. No Runtime Native admission, F5, tcltest or WASM acceptance.','checks':checks,'raw_capture_dirs':['shared-family-controls','shared-family-ambiguous-controls','shared-family-planned-source-controls','shared-family-root-regression-controls'],'older_failures_preserved':['ordinary/jim','ambiguous/jim'],'all_immutable_binary_linked_library_and_init_seals_unchanged':True}
(p/'validation.json').write_text(json.dumps(validation,indent=2)+'\n')
print('24 scoped groups: 132 presence observations + 6 original package outputs + 6 provider-distinct ambiguous-address outputs PASS')
