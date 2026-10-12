# Stock Wiki original-body evidence review

The 43 complete body-return receipts can answer the original selected Wiki bodies in the matrix, scoped to the recorded real harness and execution frame. The execution trace is instrumentation around the unchanged original helper; it is not a replacement Tcl body. This qualification applies to completion/result bytes. Inner child error options remain unavailable.

Both stock2 and stock3 evaluate the byte-identical complete [Wiki suite](../../wiki-probes.tcl), SHA-256 `bd322efe1bb54898e620354eec8183210d94d74e5c96358d4c58bf36c12e64a5`. Selected source ranges and their hashes match the authored questions. The setup uses real `tcltest`, selects the exact test with `configure -match`, and adds an execution-leave observer only after the original `isolated` procedure is defined. That unchanged helper creates a child, evaluates the original body there, deletes the child and returns its completion/result. Every observed selected body consists of that isolated invocation. The observer captures its leave completion and value; its parent trace does not rewrite the original child script.

The actual executables and harness receipts are:

| Context | Interpreter | Patchlevel | Library | Real tcltest | Reached selected bodies |
| --- | --- | --- | --- | --- | --- |
| Stock 8.4 | `/usr/bin/tclsh8.4` | 8.4.13 | `/usr/share/compat-tcl8.4` | 2.2.8 | 21 |
| Stock 8.5 | `/usr/bin/tclsh8.5` | 8.5.13 | `/usr/share/tcl8.5` | 2.3.4 | 22 |

All 43 reached bodies return completion 0, have complete result captures, and the real matching test reports PASS. Stock2 and stock3 agree on all 43 completion/encoding/hex results. This covers 22 distinct canonical Wiki cases. Constraint skips remain unreached: `wiki-code-7` (`noAunt`) and `wiki-oo-1` through `wiki-oo-7` (`tcloo`) on both versions, plus `wiki-prefix-2` (`tcl85`) on 8.4. No constraint skip becomes unsupported evidence.

The primary records are [stock2/records.json](nh20261012stock2/records.json), SHA-256 `2e413c93a1a358d84a0b35f9bfd4ee7f2d0ba76a3f2f0ea4a7e385159313fcf1`. Independent repeat records are [stock3/records.json](nh20261012stock3/records.json), SHA-256 `5b8f8dd9d2e125cc83c630dc6603e174906204c3a56b3772fbca035f6486b408`. JSON pointers below identify exact measurement records, including original selected ranges, actual input hashes, raw runtime/provider hashes, complete capture fields and constraints. Sealed run manifests remain unchanged.

| Selected original case | Canonical question | Original lines / range SHA-256 | Stock 8.4 record | Stock 8.5 record |
| --- | --- | --- | --- | --- |
| `wiki-alias-1` | `naming.wiki.alias-1` | [128, 137] / `54e4fdbd494fcdd6b4adaebae0a834a6d37e70b4e4053e986766da9e8152eef9` | observed code 0; primary `/27`, repeat `/27` | observed code 0; primary `/72`, repeat `/72` |
| `wiki-alias-2` | `naming.wiki.alias-2` | [139, 147] / `3454d32d18bf6f3948b659448ce4d9970efdcb39763c91d7a0691c6ae46d8001` | observed code 0; primary `/28`, repeat `/28` | observed code 0; primary `/73`, repeat `/73` |
| `wiki-autoload-1` | `naming.wiki.autoload-1` | [207, 212] / `6ef819ad7f7ed4b6a7ceb80899b04f9a3381e4b140a625b9ac613834238bf508` | observed code 0; primary `/36`, repeat `/36` | observed code 0; primary `/81`, repeat `/81` |
| `wiki-code-1` | `naming.wiki.code-1` | [48, 57] / `4747eb50352e396c88ab56c0544cc4fc2ceebd04b8b2ffc877dcabe179aafa8e` | observed code 0; primary `/18`, repeat `/18` | observed code 0; primary `/63`, repeat `/63` |
| `wiki-code-2` | `naming.wiki.code-2` | [59, 67] / `7a235c9b080865d48b5bb74931520176c3a4ff60ae6032277c763ac133cf545e` | observed code 0; primary `/19`, repeat `/19` | observed code 0; primary `/64`, repeat `/64` |
| `wiki-code-3` | `naming.wiki.code-3` | [69, 76] / `752537491e1d6618abe182a5f1ce9a15357c31ea01774560d01d9b960b2dbbf3` | observed code 0; primary `/20`, repeat `/20` | observed code 0; primary `/65`, repeat `/65` |
| `wiki-code-4` | `naming.wiki.code-4` | [78, 84] / `0d3da2d817d70adf67f9b08fd68e0e54c9727bf55daaca872625811958e35fbd` | observed code 0; primary `/21`, repeat `/21` | observed code 0; primary `/66`, repeat `/66` |
| `wiki-code-5` | `naming.wiki.code-5` | [86, 93] / `989d7f32b40c856785d13288a928c9eb0de2817262c6fa1b319f3c8a4244630f` | observed code 0; primary `/22`, repeat `/22` | observed code 0; primary `/67`, repeat `/67` |
| `wiki-code-6` | `naming.wiki.code-6` | [95, 100] / `ae20ae94bbfcebd10d253c5c8d5593eeae954747f240978b89e721fcf186c4e5` | observed code 0; primary `/23`, repeat `/23` | observed code 0; primary `/68`, repeat `/68` |
| `wiki-code-7` | `naming.wiki.code-6` | [102, 107] / `d024673d2d5dd3a2146f1d20251fc614ac671e952ff4e2227493dd41e2973ef0` | unreached: noAunt; primary `/24`, repeat `/24` | unreached: noAunt; primary `/69`, repeat `/69` |
| `wiki-factory-1` | `naming.wiki.factory-1` | [109, 116] / `24e1d799ad682fa0a163d2dddd9510b9817d1d4c4022dbc09ac425304acba8c7` | observed code 0; primary `/25`, repeat `/25` | observed code 0; primary `/70`, repeat `/70` |
| `wiki-factory-2` | `naming.wiki.factory-2` | [118, 126] / `713516977b70ed63c25c31983351fd778c72104dd8a83cee8f49688562a069db` | observed code 0; primary `/26`, repeat `/26` | observed code 0; primary `/71`, repeat `/71` |
| `wiki-global-1` | `naming.wiki.global-1` | [198, 205] / `a1cf73d261b5f79fe0991007bbb1608e99114e6fbf1597b6d32479814a842b7e` | observed code 0; primary `/35`, repeat `/35` | observed code 0; primary `/80`, repeat `/80` |
| `wiki-oo-1` | `naming.wiki.oo-1` | [221, 232] / `b2db139dec305c8c92207990b5c5a6a6ab3e0839fb2e06977863fd8fcde2a50d` | unreached: tcloo; primary `/38`, repeat `/38` | unreached: tcloo; primary `/83`, repeat `/83` |
| `wiki-oo-2` | `naming.wiki.oo-2` | [234, 242] / `d3fd9b19d5a4f306420ba3679ea5e24f39e1ba4ab1242c1060d7e3d5303140af` | unreached: tcloo; primary `/39`, repeat `/39` | unreached: tcloo; primary `/84`, repeat `/84` |
| `wiki-oo-3` | `naming.wiki.oo-3` | [244, 253] / `b1e0208b130f11be8b34263973216639ccacf8e0d93832a1af07834a2afc627d` | unreached: tcloo; primary `/40`, repeat `/40` | unreached: tcloo; primary `/85`, repeat `/85` |
| `wiki-oo-4` | `naming.wiki.oo-4` | [255, 266] / `e4022a69295f5f111d7fc7bc6f4894bdbed725d41ee16bbb701d53804827dbcb` | unreached: tcloo; primary `/41`, repeat `/41` | unreached: tcloo; primary `/86`, repeat `/86` |
| `wiki-oo-5` | `naming.wiki.oo-5` | [268, 275] / `dbec0dca659d0af0dca734f6e0295e02a1c5029ccccdf2fe0c616fd6177a4183` | unreached: tcloo; primary `/42`, repeat `/42` | unreached: tcloo; primary `/87`, repeat `/87` |
| `wiki-oo-6` | `naming.wiki.oo-6` | [277, 287] / `126b5ef70e365ae7ab5e06b5f5423d658bc6c7406c9262ae28072f131223b1dc` | unreached: tcloo; primary `/43`, repeat `/43` | unreached: tcloo; primary `/88`, repeat `/88` |
| `wiki-oo-7` | `naming.wiki.oo-7` | [289, 297] / `ee1b3c451d0b7a90bd316cd4367a7f6a18c56bce8c017e9274005e32030eb0f4` | unreached: tcloo; primary `/44`, repeat `/44` | unreached: tcloo; primary `/89`, repeat `/89` |
| `wiki-prefix-1` | `naming.wiki.prefix-1` | [20, 26] / `98bfa7f8f14a9066e13bbfae4c18401b5f7b79f10b9f8a49c40f46786b33954e` | observed code 0; primary `/15`, repeat `/15` | observed code 0; primary `/60`, repeat `/60` |
| `wiki-prefix-2` | `naming.wiki.prefix-2` | [28, 35] / `834ac1166c505c4b2f483abe7499e534286c02252779c566fffec7938e76aabc` | unreached: tcl85; primary `/16`, repeat `/16` | observed code 0; primary `/61`, repeat `/61` |
| `wiki-prefix-3` | `naming.wiki.prefix-3` | [37, 46] / `0c1d3cbca8bad806f0a44cf3094f94e2e41f9aefa138d7066e95bc0f33ea70f5` | observed code 0; primary `/17`, repeat `/17` | observed code 0; primary `/62`, repeat `/62` |
| `wiki-rename-1` | `naming.wiki.rename-1` | [149, 156] / `a022868f907ccc9197b9b2b65394b6cd45ab6fb649d141de27b5c45ce1d0e815` | observed code 0; primary `/29`, repeat `/29` | observed code 0; primary `/74`, repeat `/74` |
| `wiki-unknown-1` | `naming.wiki.unknown-1` | [214, 219] / `7b98978f3a8e113b39beca291f81b55dc568abe4fa5db1c0eae249603ead17bc` | observed code 0; primary `/37`, repeat `/37` | observed code 0; primary `/82`, repeat `/82` |
| `wiki-upvar-1` | `naming.wiki.upvar-1` | [158, 164] / `0c7260b31f277487f57b4e1d5b8e4ea790a7e718f2acf088eca4baaf3ef32a8e` | observed code 0; primary `/30`, repeat `/30` | observed code 0; primary `/75`, repeat `/75` |
| `wiki-upvar-2` | `naming.wiki.upvar-2` | [166, 172] / `9f4f2d1b7e7df95c306b71c96c4f78d3474ffef15fb9e0654de42e2071dce97b` | observed code 0; primary `/31`, repeat `/31` | observed code 0; primary `/76`, repeat `/76` |
| `wiki-upvar-3` | `naming.wiki.upvar-3` | [174, 181] / `8db7792c1d8e8b377f186238b6d2f4c03f2cdcbf64c7a42c8db2a38ec6ff1c46` | observed code 0; primary `/32`, repeat `/32` | observed code 0; primary `/77`, repeat `/77` |
| `wiki-upvar-4` | `naming.wiki.upvar-4` | [183, 190] / `42674ec7bcac3062c5c4e2f3dd5bb0187f32d8537ea0fbc4b49a15b5b2c595c9` | observed code 0; primary `/33`, repeat `/33` | observed code 0; primary `/78`, repeat `/78` |
| `wiki-variable-1` | `naming.wiki.variable-1` | [192, 196] / `ec554656efbdbe8f7b48066810d3ea365fa9f28cbcaf0fb9fa31fb62732f7c07` | observed code 0; primary `/34`, repeat `/34` | observed code 0; primary `/79`, repeat `/79` |

Each observed row supports `original_selected_body_observed=true` for that exact case/context, retaining the observer variant and real-harness frame scope. It does not claim arbitrary uninstrumented execution, a synthetic harness, another Tcl release, hosted iApp/TMM execution, or available inner error options.
