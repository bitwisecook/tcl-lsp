#!/usr/bin/env python3
"""Produce exact original iRule source bytes without appliance access."""
import argparse
import hashlib
import json
import re
from pathlib import Path


def quote(text):
    """Tcl double-quoted runtime string; scripts are not braced compiler bodies."""
    return '"' + ''.join({'\\': '\\\\', '"': '\\"', '$': '\\$', '[': '\\[', ']': '\\]',
                          '{': '\\{', '}': '\\}', '\n': '\\n', '\r': '\\r', '\t': '\\t'}.get(c, c)
                         for c in text) + '"'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', required=True, help='unique ASCII lab identifier, 1-24 chars')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]{0,23}', args.run):
        parser.error('run must match [A-Za-z][A-Za-z0-9_]{0,23}')
    args.out.mkdir(parents=True, exist_ok=False)
    prefix = '__tcl_lsp_probe_2286_' + args.run
    ns = prefix
    records = []
    rows = []

    def emit(case, body, purpose, ending='lf', payload=None):
        name = prefix + '_' + case
        data = ('ltm rule /Common/' + name + ' {\n' + body + '\n}\n').encode('utf-8')
        if ending == 'crlf':
            data = data.replace(b'\n', b'\r\n')
        filename = case + '.conf'
        (args.out / filename).write_bytes(data)
        (args.out / (filename + '.hex')).write_text(data.hex() + '\n', encoding='ascii')
        record = dict(case=case, object='/Common/' + name, file=filename,
                      sha256=hashlib.sha256(data).hexdigest(), size=len(data),
                      line_endings=ending, purpose=purpose)
        if payload is not None:
            raw = payload.encode('utf-8')
            record.update(runtime_script_utf8_hex=raw.hex(),
                          runtime_script_sha256=hashlib.sha256(raw).hexdigest())
            (args.out / (case + '.runtime.tcl')).write_bytes(raw)
        records.append(record)
        rows.append(filename + '\t/Common/' + name)

    # Capability controls have no optional literal namespaces/commands.
    emit('minimal', '''when HTTP_REQUEST {
    log local0. "R2286|%s|minimal|[HTTP::uri]"
    HTTP::respond 200 content "R2286 minimal\\n" Connection close
}''' % args.run, 'HTTP/log/load baseline; no TMM identity claim')
    emit('identity', '''when HTTP_REQUEST {
    set u [TMM::cmp_unit]
    log local0. "R2286|%s|identity|unit=$u|[HTTP::uri]"
    HTTP::respond 200 content "unit=$u\\n" X-R2286-TMM $u Connection close
}''' % args.run, 'event identity capability; unit alone is not chassis-wide identity')
    emit('identity_group', '''when HTTP_REQUEST {
    set u [TMM::cmp_unit]
    set g [TMM::cmp_group]
    set n [TMM::cmp_count]
    log local0. "R2286|%s|identity_group|group=$g|unit=$u|count=$n|[HTTP::uri]"
    HTTP::respond 200 content "group=$g unit=$u count=$n\\n" X-R2286-TMM "$g:$u" Connection close
}''' % args.run, 'optional group/count capability; inspect actual active instance roster')
    emit('init_identity', '''when RULE_INIT {
    log local0. "R2286|%s|init_identity|group=[TMM::cmp_group]|unit=[TMM::cmp_unit]|count=[TMM::cmp_count]"
}''' % args.run, 'optional initialization identity; load rejection is an event capability result')
    emit('encoding', '''when HTTP_REQUEST {
    set x "é|é"
    binary scan $x H* hx
    log local0. "R2286|%s|encoding|hex=$hx|chars=[string length $x]"
    HTTP::respond 200 content "$hx\\n" Connection close
}''' % args.run, 'UTF-8 ingress and binary formatter capability; not physical-object representation proof')

    # Static alone, globals alone: their CMP consequences must not be conflated.
    for kind, variable in [('static', 'static::' + ns + '_cell'), ('static_group', 'static::' + ns + '_cell'), ('global', '::' + ns + '_cell')]:
        emit(kind, '''when RULE_INIT {
    set %s INIT_%s
    log local0. "R2286|%s|%s|INIT|seed=INIT_%s"
}
when HTTP_REQUEST {
    set u [TMM::cmp_unit]
    set action [URI::query [HTTP::uri] action]
    set target [URI::query [HTTP::uri] target]
    if {$target ne "" && $target ne $u} {set action read}
    set before_exists [info exists %s]
    set before MISSING
    if {$before_exists} {set before [set %s]}
    set rc 0
    set result NONE
    if {$action eq "write"} {set result [set %s "EVENT_$u"]}
    if {$action eq "unset"} {set rc [catch {unset %s} result]}
    if {$action eq "recreate"} {set result [set %s "RECREATED_$u"]}
    set after_exists [info exists %s]
    set after MISSING
    if {$after_exists} {set after [set %s]}
    set record [list R2286 %s %s $u $action $before_exists $before $rc $result $after_exists $after]
    log local0. $record
    HTTP::respond 200 content "$record\\n" X-R2286-TMM $u Connection close
}''' % (variable,args.run,args.run,kind,args.run,variable,variable,variable,variable,variable,
         variable,variable,args.run,kind),
             'RULE_INIT seed, event write/unset/recreate and TMM-local observations; global may alter CMP eligibility')
        if kind == 'static_group':
            record = records[-1]
            file = args.out / record['file']
            data = file.read_bytes().replace(b'set u [TMM::cmp_unit]', b'set u "[TMM::cmp_group]:[TMM::cmp_unit]"')
            file.write_bytes(data)
            (args.out / (record['file'] + '.hex')).write_text(data.hex() + '\n', encoding='ascii')
            record['sha256'] = hashlib.sha256(data).hexdigest()
            record['size'] = len(data)

    # Same static key, independent rule ownership and load order, with no responder conflict.
    collision = 'static::' + ns + '_collision'
    for who, priority in [('a',101), ('b',102)]:
        emit('collision_' + who, '''when RULE_INIT {
    set %s INIT_%s
    log local0. "R2286|%s|collision_%s|INIT|seed=INIT_%s"
}
when HTTP_REQUEST priority %d {
    set u [TMM::cmp_unit]
    set action [URI::query [HTTP::uri] action]
    if {$action eq "write_%s"} {set %s "EVENT_%s_$u"}
    set value MISSING
    if {[info exists %s]} {set value [set %s]}
    log local0. [list R2286 %s collision_%s $u $action $value]
}''' % (collision,who,args.run,who,who,priority,who,collision,who,collision,collision,args.run,who),
             'cross-rule same static name; test creation order separately from event priority')
    emit('collision_observer', '''when HTTP_REQUEST priority 110 {
    set u [TMM::cmp_unit]
    set value MISSING
    if {[info exists %s]} {set value [set %s]}
    log local0. [list R2286 %s collision_observer $u [HTTP::uri] $value]
    HTTP::respond 200 content "$value\\n" X-R2286-TMM $u Connection close
}''' % (collision,collision,args.run), 'final observer; attach A/B/observer only to an exclusive lab VIP')

    # F5 logical rule procedure ownership is different from ordinary Tcl namespace dispatch.
    emit('procedure_a', '''proc same {} {return A}
proc caller_link {name} {upvar 1 $name alias; set alias LINK_A; return $alias}
when HTTP_REQUEST priority 101 {
    set x INITIAL
    set rc [catch {call caller_link x} result]
    log local0. [list R2286 %s procedure_a [TMM::cmp_unit] $rc $result $x [call same]]
}''' % args.run, 'F5 local procedure call and real call-site upvar semantics')
    emit('procedure_b', '''proc same {} {return B}
when HTTP_REQUEST priority 102 {
    log local0. [list R2286 %s procedure_b [TMM::cmp_unit] [call same]]
}''' % args.run, 'same procedure spelling in another iRule')
    rule_a = '/Common/' + prefix + '_procedure_a'
    emit('procedure_caller', '''when HTTP_REQUEST priority 110 {
    set u [TMM::cmp_unit]
    set localname %s
    set absolute %s
    set r1 [catch {call $localname} v1]
    set r2 [catch {call $absolute} v2]
    set out [list R2286 %s procedure_caller $u $r1 $v1 $r2 $v2]
    log local0. $out
    HTTP::respond 200 content "$out\\n" X-R2286-TMM $u Connection close
}''' % (quote(prefix+'_procedure_a::same'),quote(rule_a+'::same'),args.run),
             'same-folder and absolute-folder F5 call ownership; record actual acceptance/results')
    emit('ordinary_literal', '''proc same {} {return LOCAL}
when HTTP_REQUEST {
    set rc [catch {same} result]
    log local0. [list R2286 %s ordinary_literal $rc $result]
    HTTP::respond 200 content "$rc $result\\n" Connection close
}''' % args.run, 'load-time ordinary command-head versus F5 call protocol; may reject at load')

    p = ns + '_'
    cases = {
        'namespace': 'namespace eval ::'+ns+' {variable x 11; proc p {} {variable x; return $x}}\n'+ns+'::p',
        'path_shadow': 'namespace eval ::'+ns+' {proc '+p+'pick {} {return LOCAL}; namespace path {::}}\nproc ::'+p+'pick {} {return GLOBAL}\nnamespace eval ::'+ns+' {'+p+'pick}',
        'path_provider': 'namespace eval ::'+ns+'_provider {proc '+p+'mark {} {return PATH}}\nproc ::'+p+'mark {} {return GLOBAL}\nnamespace eval ::'+ns+' {namespace path {::'+ns+'_provider}; '+p+'mark}',
        'global_alias': 'set ::'+p+'g 11\nproc '+p+'read {} {global '+p+'g; return $'+p+'g}\n'+p+'read',
        'upvar': 'proc '+p+'inner {n} {upvar 1 $n a; set a 9}\nproc '+p+'outer {} {set x 1; '+p+'inner x; return $x}\n'+p+'outer',
        'upvar_unset': 'proc '+p+'inner {n} {upvar 1 $n a; unset a; set a AGAIN}\nproc '+p+'outer {} {set x 1; '+p+'inner x; return $x}\n'+p+'outer',
        'uplevel': 'proc '+p+'inner {} {uplevel 1 {set x 42}}\nproc '+p+'outer {} {set x 1; '+p+'inner; return $x}\n'+p+'outer',
        'upvar_absolute': 'set ::'+p+'x GLOBAL\nproc '+p+'read {} {upvar #0 ::'+p+'x a; set a CHANGED; return $a}\nlist ['+p+'read] [set ::'+p+'x]',
        'upvar_zero': 'proc '+p+'outer {} {set x LOCAL; upvar 0 x a; set a CHANGED; return $x}\n'+p+'outer',
        'uplevel_absolute': 'proc '+p+'inner {} {uplevel #0 {set ::'+p+'x ABSOLUTE}}\n'+p+'inner\nset ::'+p+'x',
        'static_global_link': 'set static::'+p+'x STATIC\nproc '+p+'read {} {global static::'+p+'x; return $'+p+'x}\n'+p+'read',
        'alias': 'proc '+p+'one {x} {return "ONE:$x"}\ninterp alias {} '+p+'alias {} '+p+'one FIXED\n'+p+'alias',
        'alias_rename': 'proc '+p+'one {} {return ORIGINAL}\ninterp alias {} '+p+'alias {} '+p+'one\nrename '+p+'one '+p+'moved\nlist [catch {'+p+'alias} a] $a ['+p+'moved]',
        'rename_epoch': 'proc '+p+'one {} {return OLD}\nset c '+p+'one\nset before [eval $c]\nrename '+p+'one '+p+'moved\nproc '+p+'one {} {return NEW}\nlist $before [eval $c] ['+p+'moved]',
        'namespace_import': 'namespace eval ::'+ns+' {proc p {} {return EXPORTED}; namespace export p}\nnamespace eval ::'+ns+'_consumer {namespace import ::'+ns+'::p; p}',
        'namespace_delete': 'namespace eval ::'+ns+' {proc p {} {return OLD}}\nset c ::'+ns+'::p\nset before [eval $c]\nnamespace delete ::'+ns+'\nnamespace eval ::'+ns+' {proc p {} {return NEW}}\nlist $before [eval $c]',
        'trace_scalar': 'set ::'+p+'events {}\nproc '+p+'trace {n i op} {lappend ::'+p+'events [list $n $i $op]}\nset ::'+p+'x 1\ntrace variable ::'+p+'x rwu '+p+'trace\nset a [set ::'+p+'x]\nset ::'+p+'x 2\nunset ::'+p+'x\nset ::'+p+'x 3\nlist $a $::'+p+'events',
        'trace_upvar_array': 'set ::'+p+'events {}\nproc '+p+'trace {n i op} {lappend ::'+p+'events [list $n $i $op]}\nset ::'+p+'arr(k) 1\ntrace variable ::'+p+'arr rwu '+p+'trace\nproc '+p+'read {} {upvar #0 ::'+p+'arr(k) v; return $v}\nlist ['+p+'read] $::'+p+'events',
        'trace_static': 'set static::'+p+'events {}\nproc '+p+'trace {n i op} {lappend static::'+p+'events [list $n $i $op]}\nset static::'+p+'x 1\ntrace variable static::'+p+'x rwu '+p+'trace\nset a [set static::'+p+'x]\nunset static::'+p+'x\nset static::'+p+'x 3\nlist $a $static::'+p+'events',
        'package': 'list [package provide Tcl] [catch {package require Tcl} required] $required',
        'package_resolution': 'set script \"package provide '+p+'package 1.0\"\npackage ifneeded '+p+'package 1.0 $script\nlist [package require '+p+'package 1.0] [package provide '+p+'package]',
        'unicode_names': 'set '+p+'é PRECOMPOSED\nset '+p+'é DECOMPOSED\nlist [set '+p+'é] [set '+p+'é] [string equal '+p+'é '+p+'é]',
        'unicode_variable_syntax': 'set '+p+'é PRECOMPOSED\nset '+p+'é DECOMPOSED\nlist [catch {set a $'+p+'é} a] $a [catch {set b $'+p+'é} b] $b',
        'colon_names': 'namespace eval ::'+ns+' {variable x 7}\nlist [set ::'+ns+':::x] [set ::'+ns+'::x]',
        'read_failure': 'proc '+p+'failure {} {set x [set missing_'+args.run+']; set reached BAD}\nlist [catch {'+p+'failure} result] $result',
        'exists': 'proc '+p+'test {} {list [info exists missing_'+args.run+'] [array exists missing_'+args.run+']}\n'+p+'test',
        'expr': 'list [expr {3.5}] [expr {7 << 1}] [expr {abs(-3)}] [catch {expr {future_function(1)}} message] $message',
        'word_bytes': 'set x {A\\\n  B}\nset y "A\\\n  B"\nset z {\\n}\nlist $x $y $z',
    }
    for case, script in cases.items():
        emit('dynamic_' + case, '''when HTTP_REQUEST {
    set u [TMM::cmp_unit]
    set script %s
    set rc [catch {eval $script} result]
    set ec UNAVAILABLE
    if {[info exists errorCode]} {set ec $errorCode}
    binary scan $result H* result_hex
    binary scan $ec H* errorcode_hex
    log local0. "R2286|%s|dynamic_%s|unit=$u|rc=$rc|result_hex=$result_hex|errorcode_hex=$errorcode_hex|[HTTP::uri]"
    HTTP::respond 200 content "$rc $result_hex $errorcode_hex\\n" X-R2286-TMM $u Connection close
}''' % (quote(script),args.run,case),
             'optional runtime script; load acceptance and runtime behavior are independent', payload=script)

    # The physical original file itself contains these syntax edges.
    word_body = '''when HTTP_REQUEST {
    set brace {A\\
  B}
    set quote "A\\
  B"
    set literal {\\n}
    set unicode {é|é}
    set out [list $brace $quote $literal $unicode]
    binary scan $out H* hx
    log local0. "R2286|%s|physical_words|hex=$hx"
    HTTP::respond 200 content "$hx\\n" Connection close
}''' % args.run
    emit('physical_words_lf',word_body,'physical backslash-LF, braces, quote, literal escape and NFC/NFD source')
    emit('physical_words_crlf',word_body,'same source transformed only LF to CRLF',ending='crlf')
    for command in ['namespace', 'interp', 'trace', 'package', 'proc', 'rename']:
        # Isolated lexical capability, never mixed into the baseline.
        script = {'namespace':'namespace current','interp':'interp aliases','trace':'trace info variable x',
                  'package':'package provide Tcl','proc':'proc '+p+'literal {} {return X}',
                  'rename':'rename '+p+'absent '+p+'other'}[command]
        emit('literal_' + command,'when HTTP_REQUEST {\n    set rc [catch {'+script+'} result]\n    HTTP::respond 200 content "$rc $result\\n" Connection close\n}',
             'literal command inside catch may reject whole rule at load; not runtime absence proof')
    emit('event_frames', """when CLIENT_ACCEPTED {
    set event_local CLIENT_SEED
    log local0. [list R2286 %s event_frames CLIENT_ACCEPTED [TMM::cmp_unit] [IP::client_addr] [TCP::client_port] $event_local]
}
when HTTP_REQUEST {
    set rc [catch {set event_local} value]
    log local0. [list R2286 %s event_frames HTTP_REQUEST [TMM::cmp_unit] $rc $value [HTTP::uri]]
}
when LB_SELECTED {
    set rc [catch {set event_local} value]
    log local0. [list R2286 %s event_frames LB_SELECTED [TMM::cmp_unit] $rc $value]
}
when SERVER_CONNECTED {
    set rc [catch {set event_local} value]
    log local0. [list R2286 %s event_frames SERVER_CONNECTED [TMM::cmp_unit] $rc $value]
}
when HTTP_RESPONSE {
    set rc [catch {set event_local} value]
    log local0. [list R2286 %s event_frames HTTP_RESPONSE [TMM::cmp_unit] $rc $value [HTTP::status]]
}
when CLIENT_CLOSED {
    set rc [catch {set event_local} value]
    log local0. [list R2286 %s event_frames CLIENT_CLOSED [TMM::cmp_unit] $rc $value]
}""" % ((args.run,)*6), 'connection local frame continuity; requires real backend and legal event execution')
    names = [p + x for x in ['p','read','inner','outer','one','alias','moved','trace','failure','test','literal','pick','mark']]
    variables = ['::'+p+x for x in ['g','events','x','arr','é','é']]
    variables += ['static::'+p+x for x in ['events','x']]
    variables += ['static::'+ns+'_cell', '::'+ns+'_cell', collision]
    cleanup = 'set outcomes {}\n'
    for variable in variables:
        cleanup += 'lappend outcomes [list unset '+variable+' [catch {unset '+variable+'} e] $e]\n'
    for name in names:
        cleanup += 'lappend outcomes [list rename '+name+' [catch {rename '+name+' {}} e] $e]\n'
    for name in ['::'+ns, '::'+ns+'_consumer', '::'+ns+'_provider']:
        cleanup += 'lappend outcomes [list namespace '+name+' [catch {namespace delete '+name+'} e] $e]\n'
    cleanup += 'lappend outcomes [list package [catch {package forget '+p+'package} e] $e]\n'
    cleanup += 'set outcomes'
    emit('runtime_cleanup', """when HTTP_REQUEST {
    set u [TMM::cmp_unit]
    set script %s
    set rc [catch {eval $script} result]
    binary scan $result H* hx
    log local0. "R2286|%s|runtime_cleanup|unit=$u|rc=$rc|result_hex=$hx"
    HTTP::respond 200 content "$rc $hx\\n" X-R2286-TMM $u Connection close
}""" % (quote(cleanup),args.run), 'remove only exact generated runtime names on every reached TMM; preserve outcome errors', payload=cleanup)
    (args.out/'manifest.json').write_text(json.dumps({'run':args.run,'prefix':prefix,'encoding':'UTF-8',
                                                   'expected_appliance_results':'UNMEASURED','fixtures':records},indent=2)+'\n')
    (args.out/'rules.tsv').write_text('\n'.join(rows)+'\n',encoding='ascii')
    print('Generated',len(records),'isolated rules in',args.out)

if __name__ == '__main__':
    main()
