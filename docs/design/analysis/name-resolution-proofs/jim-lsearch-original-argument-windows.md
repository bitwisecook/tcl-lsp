# naming.jim.lsearch-original-argument-windows

Kind: `native-observation`

## Problem statement

Search options, comparison callbacks and a nonnormal callback can consume or prepare argument objects differently. A final search index cannot identify those physical transitions.

## Question

What completion, result and original argument/cache windows does the captured Jim lsearch vector produce for its fixed cases?

## Conclusion

The actual original-vector calls retain per-case completion, callback and primary/resident/reference fields before result observation. Only this pinned Jim probe is covered; C engines and BIG-IP were not run for these physical windows.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.4.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.5.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl8.6.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.0.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: tcl9.1.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

### jim

Status: `observed`. Version: Jim; exact source/library/configuration hashes recorded; version query absent. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector. Exact lengths, flags and input constructors remain in the source.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-0`):

```json
{
  "compile_exit": 0,
  "stderr": "",
  "captures": [
    {
      "case": 0,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tlist\t1\t1\nargument\t2\tstring\t1\t1\nbytes\t31\n",
      "stderr": ""
    },
    {
      "case": 1,
      "exit": 0,
      "stdout": "code\t1\nresult\tstring\t1\t1\nargument\t1\tnone\t1\t2\nargument\t2\tnone\t1\t2\nargument\t3\tnone\t1\t2\nbytes\t626164206f7074696f6e20222d67223a206d757374206265202d616c6c2c202d626f6f6c2c202d636f6d6d616e642c202d65786163742c202d676c6f622c202d696e6465782c202d696e6c696e652c202d6e6f636173652c202d6e6f742c202d7265676578702c206f72202d737472696465\n",
      "stderr": ""
    },
    {
      "case": 2,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tnone\t1\t1\nbytes\t30\n",
      "stderr": ""
    },
    {
      "case": 3,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tregexp\t1\t1\nbytes\t30\n",
      "stderr": ""
    },
    {
      "case": 4,
      "exit": 0,
      "stdout": "callback\t3\tcommand:4\tnone:3\tsource:2\ncallback\t3\tcommand:4\tnone:3\tsource:2\ncode\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tcommand\t1\t1\nargument\t3\tlist\t1\t1\nargument\t4\tnone\t1\t1\nbytes\t2d31\n",
      "stderr": ""
    },
    {
      "case": 5,
      "exit": 0,
      "stdout": "callback\t3\tcommand:4\tnone:3\tsource:2\ncode\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tcommand\t1\t1\nargument\t4\tlist\t1\t1\nargument\t5\tnone\t1\t1\nbytes\t32\n",
      "stderr": ""
    },
    {
      "case": 6,
      "exit": 0,
      "stdout": "callback\t3\tcommand:4\tnone:3\tsource:2\ncallback\t3\tcommand:4\tnone:3\tsource:2\ncode\t0\nresult\tlist\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tget-enum\t1\t1\nargument\t4\tcommand\t1\t1\nargument\t5\tlist\t1\t1\nargument\t6\tnone\t1\t1\nbytes\t322032\n",
      "stderr": ""
    },
    {
      "case": 7,
      "exit": 0,
      "stdout": "code\t0\nresult\tnone\t1\t5\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tstring\t1\t1\nbytes\t\n",
      "stderr": ""
    },
    {
      "case": 8,
      "exit": 0,
      "stdout": "callback\t4\tcommand:2\tnone:1\tnone:3\tsource:2\ncallback\t4\tcommand:2\tnone:1\tnone:3\tsource:2\ncode\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tlist\t1\t1\nargument\t4\tnone\t1\t1\nbytes\t32\n",
      "stderr": ""
    },
    {
      "case": 9,
      "exit": 0,
      "stdout": "code\t0\nresult\tlist\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tget-enum\t1\t1\nargument\t4\tint\t1\t1\nargument\t5\tlist\t1\t1\nargument\t6\tstring\t1\t1\nbytes\t612041\n",
      "stderr": ""
    },
    {
      "case": 10,
      "exit": 0,
      "stdout": "code\t0\nresult\tlist\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tget-enum\t1\t1\nargument\t4\tint\t1\t1\nargument\t5\tget-enum\t1\t1\nargument\t6\tlist\t1\t1\nargument\t7\tlist\t1\t1\nargument\t8\tstring\t1\t1\nbytes\t612041\n",
      "stderr": ""
    },
    {
      "case": 11,
      "exit": 0,
      "stdout": "code\t0\nresult\tlist\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tint\t1\t1\nargument\t4\tget-enum\t1\t1\nargument\t5\tlist\t1\t1\nargument\t6\tlist\t1\t1\nargument\t7\tstring\t1\t1\nbytes\t612041\n",
      "stderr": ""
    },
    {
      "case": 12,
      "exit": 0,
      "stdout": "code\t0\nresult\tstring\t1\t2\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tlist\t1\t1\nargument\t4\tlist\t1\t1\nargument\t5\tstring\t1\t1\nbytes\t41\n",
      "stderr": ""
    },
    {
      "case": 13,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tlist\t1\t1\nargument\t4\tstring\t1\t1\nbytes\t31\n",
      "stderr": ""
    },
    {
      "case": 14,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tlist\t1\t1\nargument\t4\tstring\t1\t1\nbytes\t30\n",
      "stderr": ""
    },
    {
      "case": 15,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tstring\t1\t1\nbytes\t31\n",
      "stderr": ""
    },
    {
      "case": 16,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tstring\t1\t1\nbytes\t30\n",
      "stderr": ""
    },
    {
      "case": 17,
      "exit": 0,
      "stdout": "code\t0\nresult\tint\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tnone\t1\t1\nbytes\t2d31\n",
      "stderr": ""
    },
    {
      "case": 18,
      "exit": 0,
      "stdout": "code\t1\nresult\tnone\t1\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tlist\t1\t1\nargument\t3\tnone\t1\t2\nbytes\t636f756c646e277420636f6d70696c6520726567756c61722065787072657373696f6e207061747465726e3a20627261636b657473205b5d206e6f742062616c616e636564\n",
      "stderr": ""
    },
    {
      "case": 19,
      "exit": 0,
      "stdout": "code\t1\nresult\tnone\t1\t1\nargument\t1\tget-enum\t1\t2\nargument\t2\tcommand\t1\t2\nargument\t3\tlist\t1\t2\nargument\t4\tnone\t1\t2\nbytes\t4f524947494e414c5f52455455524e\n",
      "stderr": ""
    },
    {
      "case": 20,
      "exit": 0,
      "stdout": "code\t0\nresult\tlist\t0\t1\nargument\t1\tget-enum\t1\t1\nargument\t2\tget-enum\t1\t1\nargument\t3\tget-enum\t1\t1\nargument\t4\tint\t1\t1\nargument\t5\tlist\t1\t1\nargument\t6\tstring\t1\t1\nbytes\t\n",
      "stderr": ""
    },
    {
      "case": 21,
      "exit": 0,
      "stdout": "code\t1\nresult\tstring\t1\t1\nargument\t1\tnone\t1\t2\nargument\t2\tnone\t1\t2\nargument\t3\tnone\t1\t2\nbytes\t626164206f7074696f6e20222d2d223a206d757374206265202d616c6c2c202d626f6f6c2c202d636f6d6d616e642c202d65786163742c202d676c6f622c202d696e6465782c202d696e6c696e652c202d6e6f636173652c202d6e6f742c202d7265676578702c206f72202d737472696465\n",
      "stderr": ""
    },
    {
      "case": 22,
      "exit": 0,
      "stdout": "code\t1\nresult\tnone\t1\t1\nargument\t1\tget-enum\t1\t2\nargument\t2\tget-enum\t1\t2\nargument\t3\tint\t1\t2\nargument\t4\tlist\t1\t2\nargument\t5\tnone\t1\t2\nbytes\t6c6973742073697a65206d7573742062652061206d756c7469706c65206f662074686520737472696465206c656e677468\n",
      "stderr": ""
    },
    {
      "case": 23,
      "exit": 0,
      "stdout": "code\t1\nresult\tnone\t1\t1\nargument\t1\tget-enum\t1\t2\nargument\t2\tlist\t1\t2\nargument\t3\tlist\t1\t2\nargument\t4\tnone\t1\t2\nbytes\t696e6465782022656e642b3122206f7574206f662072616e6765\n",
      "stderr": ""
    }
  ]
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-cmd-core/tests/data/native_jim_lsearch/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_jim_lsearch/manifest.json). SHA-256 `21dbae4454a92b9e8443639d5cb566932460de38165c20035231d94c8400915c`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-1` (input): [rust/tcl-cmd-core/tests/data/native_jim_lsearch/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_jim_lsearch/probe.c). SHA-256 `f2637ff9cf2a2088e6ac2394d247a960ff2b90f9ecf00312dcb933843181a899`. Exact retained input/program bytes; purpose is limited to this question.
- `file-2` (observation): [rust/tcl-cmd-core/tests/data/native_jim_lsearch/rows.txt](../../../../rust/tcl-cmd-core/tests/data/native_jim_lsearch/rows.txt). SHA-256 `1f016da6a27cdbd08df027c1728c0ab00cabf30b426f43187446b2215af815d9`. Exact retained capture/provenance artifact; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.
