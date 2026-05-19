| stem | sub | bundle KB | wasm KB | compile (ms) | wasm run (ms) | tcl run (ms) | wasm: P/F/S of T | tcl: P/F/S of T | status |
|---|---|---:|---:|---:|---:|---:|---|---|---|
| parse | parsing | 151 | 162 | 407 | 395 | 107 | TRAP | 90/0/181 of 271 | run-trap |
| parseOld | parsing | 117 | 122 | 355 | 237 | 103 | 53/105/0 of 158 | 158/0/0 of 158 | partial |
| parseExpr | parsing | 168 | 179 | 407 | 115 | 106 | TRAP | 67/0/219 of 286 | run-trap |
| subst | parsing | 113 | 116 | 311 | 168 | 447 | TRAP | 62/0/1 of 63 | run-trap |
| word | parsing | 109 | 110 | 332 | 227 | 114 | 0/55/0 of 55 | 55/0/0 of 55 | partial |
| list | list | 108 | 110 | 366 | 200 | 95 | TRAP | 78/0/0 of 78 | run-trap |
| listObj | list | 114 | 117 | 335 | 106 | 104 | TRAP | 42/0/17 of 59 | run-trap |
| listRep | list | 200 | 221 | 509 | 137 | 128 | TRAP | 4/0/227 of 231 | run-trap |
| llength | list | 104 | 103 | 328 | 121 | 91 | 3/3/0 of 6 | 6/0/0 of 6 | partial |
| lindex | list | 116 | 120 | 357 | 221 | 97 | 9/75/0 of 84 | 47/0/37 of 84 | partial |
| linsert | list | 106 | 107 | 348 | 189 | 100 | 18/10/0 of 28 | 28/0/0 of 28 | partial |
| lrange | list | 113 | 114 | 311 | 464 | 167 | TRAP | 1764/0/2 of 1766 | run-trap |
| lreplace | list | 119 | 123 | 355 | 496 | 254 | TRAP | 3579/0/0 of 3579 | run-trap |
| lsearch | list | 129 | 135 | 426 | 246 | 106 | 11/154/0 of 165 | 165/0/0 of 165 | partial |
| lset | list | 119 | 122 | 329 | 137 | 100 | 0/89/0 of 89 | 0/0/89 of 89 | partial |
| lsetComp | list | 119 | 119 | 336 | 90 | 97 | 0/19/0 of 19 | 19/0/0 of 19 | partial |
| lmap | list | 115 | 117 | 346 | 112 | 953 | 17/49/0 of 66 | 66/0/0 of 66 | partial |
| lpop | list | 107 | 107 | 304 | 98 | 95 | 0/19/0 of 19 | 17/0/2 of 19 | partial |
| lseq | list | 143 | 150 | 377 | 83 | 134 | TRAP | 132/0/2 of 134 | run-trap |
| lrepeat | list | 105 | 104 | 330 | 67 | 100 | TRAP | 11/0/1 of 12 | run-trap |
| foreach | list | 110 | 112 | 311 | 91 | 98 | TRAP | 43/0/0 of 43 | run-trap |
| abstractlist | list | 129 | 129 | 369 | 205 | 96 | 0/123/0 of 123 | 0/0/123 of 123 | partial |
| dict | dict | 167 | 184 | 439 | 72 | 361 | TRAP | 367/0/6 of 373 | run-trap |
| string | string | 203 | 0 | 429 | 0 | 127 | COMPILE-FAIL | 693/0/12 of 705 | compile-fail |
| stringObj | string | 123 | 127 | 327 | 122 | 96 | 9/72/0 of 81 | 0/0/81 of 81 | partial |
| format | string | 125 | 131 | 356 | 148 | 107 | TRAP | 269/0/0 of 269 | run-trap |
| scan | string | 134 | 143 | 378 | 245 | 134 | 25/160/0 of 185 | 184/0/1 of 185 | partial |
| regexp | string | 146 | 159 | 423 | 75 | 23395 | TRAP | 253/0/4 of 257 | run-trap |
| regexpComp | string | 129 | 136 | 325 | 217 | 215 | 2/177/0 of 179 | 179/0/0 of 179 | partial |
| reg | string | 145 | 156 | 417 | 73 | 6609 | TRAP | 34/0/1107 of 1141 | run-trap |
| get | string | 109 | 109 | 365 | 66 | 106 | TRAP | 6/0/17 of 23 | run-trap |
| split | string | 105 | 105 | 329 | 92 | 103 | 10/8/0 of 18 | 18/0/0 of 18 | partial |
| join | string | 105 | 104 | 337 | 76 | 112 | 7/3/0 of 10 | 10/0/0 of 10 | partial |
| expr | expr | 446 | 503 | 827 | 74 | 596 | TRAP | 2137/0/31 of 2168 | run-trap |
| expr-old | expr | 150 | 166 | 435 | 64 | 215 | TRAP | 430/0/31 of 461 | run-trap |
| compExpr | expr | 120 | 123 | 361 | 132 | 142 | 19/63/0 of 82 | 80/0/2 of 82 | partial |
| compExpr-old | expr | 138 | 147 | 411 | 101 | 242 | TRAP | 183/0/1 of 184 | run-trap |
| mathop | expr | 159 | 178 | 445 | 64 | 206 | TRAP | 385/0/0 of 385 | run-trap |
| if | control | 132 | 135 | 358 | 120 | 110 | 0/73/0 of 73 | 73/0/0 of 73 | partial |
| if-old | control | 108 | 108 | 308 | 97 | 100 | 13/20/0 of 33 | 33/0/0 of 33 | partial |
| for | control | 141 | 146 | 369 | 83 | 112 | TRAP | 64/0/24 of 88 | run-trap |
| for-old | control | 105 | 104 | 344 | 78 | 103 | 5/4/0 of 9 | 9/0/0 of 9 | partial |
| while | control | 118 | 120 | 306 | 101 | 111 | 1/45/0 of 46 | 46/0/0 of 46 | partial |
| while-old | control | 106 | 106 | 331 | 80 | 103 | 6/9/0 of 15 | 15/0/0 of 15 | partial |
| switch | control | 124 | 131 | 363 | 163 | 108 | TRAP | 113/0/0 of 113 | run-trap |
| error | control | 140 | 149 | 358 | 75 | 117 | TRAP | 309/0/8 of 317 | run-trap |
| result | control | 108 | 109 | 330 | 85 | 102 | 0/26/0 of 26 | 4/0/22 of 26 | partial |
| set | variable | 120 | 124 | 345 | 79 | 106 | TRAP | 63/0/1 of 64 | run-trap |
| set-old | variable | 133 | 139 | 400 | 180 | 106 | 26/127/0 of 153 | 153/0/0 of 153 | partial |
| var | variable | 165 | 177 | 383 | 67 | 134 | TRAP | 198/0/21 of 219 | run-trap |
| upvar | variable | 120 | 123 | 345 | 121 | 109 | 17/53/0 of 70 | 62/0/8 of 70 | partial |
| uplevel | variable | 111 | 114 | 338 | 114 | 161 | 19/38/0 of 57 | 57/0/0 of 57 | partial |
| namespace | variable | 215 | 233 | 464 | 67 | 210 | TRAP | 311/0/3 of 314 | run-trap |
| namespace-old | variable | 138 | 147 | 389 | 357 | 110 | TRAP | 126/0/0 of 126 | run-trap |
| trace | variable | 185 | 203 | 417 | 69 | 193 | TRAP | 273/0/17 of 290 | run-trap |
| resolver | variable | 111 | 110 | 343 | 77 | 105 | 0/10/0 of 10 | 0/0/10 of 10 | partial |
| proc | proc | 116 | 117 | 359 | 98 | 117 | 3/35/0 of 38 | 29/0/9 of 38 | partial |
| proc-old | proc | 117 | 120 | 390 | 122 | 99 | 23/51/0 of 74 | 74/0/0 of 74 | partial |
| apply | proc | 113 | 114 | 319 | 70 | 94 | no-summary | 38/0/4 of 42 | no-summary |
| info | proc | 183 | 226 | 466 | 292 | 207 | TRAP | 282/0/5 of 287 | run-trap |
| cmdInfo | proc | 106 | 106 | 354 | 81 | 148 | 0/12/0 of 12 | 0/0/12 of 12 | partial |
| rename | proc | 109 | 109 | 350 | 78 | 151 | TRAP | 11/0/8 of 19 | run-trap |
| unknown | proc | 105 | 104 | 301 | 80 | 121 | 0/7/0 of 7 | 7/0/0 of 7 | partial |
| eval | eval | 105 | 105 | 336 | 78 | 127 | 8/4/0 of 12 | 12/0/0 of 12 | partial |
| compile | eval | 141 | 188 | 424 | 282 | 302 | TRAP | 138/0/33 of 171 | run-trap |
| execute | eval | 139 | 147 | 401 | 71 | 197 | TRAP | 79/0/78 of 157 | run-trap |
| basic | eval | 133 | 182 | 363 | 662 | 167 | TRAP | no-summary | run-trap |
| cmdAH | cmd-dispatch | 178 | 235 | 525 | 138 | 6233 | TRAP | 16820/0/181 of 17001 | run-trap |
| cmdIL | cmd-dispatch | 134 | 140 | 343 | 68 | 280 | TRAP | 163/0/5 of 168 | run-trap |
| cmdMZ | cmd-dispatch | 123 | 126 | 388 | 154 | 355 | TRAP | 96/0/1 of 97 | run-trap |
| oo | object | 263 | 282 | 518 | 94 | 185 | TRAP | 372/0/16 of 388 | run-trap |
| ooNext2 | object | 128 | 129 | 355 | 100 | 116 | TRAP | 57/0/5 of 62 | run-trap |
| ooProp | object | 132 | 132 | 369 | 117 | 114 | 0/55/0 of 55 | 55/0/0 of 55 | partial |
| ooUtil | object | 118 | 119 | 319 | 100 | 142 | 0/33/0 of 33 | 33/0/0 of 33 | partial |
| coroutine | coroutine | 130 | 135 | 376 | 141 | 142 | 0/77/0 of 77 | 74/0/3 of 77 | partial |
| nre | coroutine | 113 | 114 | 355 | 99 | 178 | 0/28/0 of 28 | 5/0/23 of 28 | partial |
| tailcall | coroutine | 119 | 121 | 335 | 133 | 129 | 0/37/0 of 37 | 29/0/8 of 37 | partial |
| interp | interp | 207 | 259 | 513 | 130 | 1493 | TRAP | 340/0/14 of 354 | run-trap |
| safe | interp | 244 | 305 | 553 | 245 | 2222 | TRAP | 147/0/8 of 155 | run-trap |
| safe-stock | interp | 120 | 164 | 361 | 160 | 373 | TRAP | 11/0/0 of 11 | run-trap |
| safe-stock86 | interp | 103 | 102 | 330 | 66 | 90 | no-summary | no-summary | no-summary |
| source | interp | 112 | 112 | 345 | 87 | 131 | 0/23/0 of 23 | 23/0/0 of 23 | partial |
| append | misc | 113 | 114 | 334 | 99 | 137 | 19/33/0 of 52 | 49/0/3 of 52 | partial |
| appendComp | misc | 115 | 116 | 340 | 114 | 135 | 18/30/0 of 48 | 43/0/5 of 48 | partial |
| concat | misc | 104 | 104 | 401 | 76 | 131 | 9/0/0 of 9 | 9/0/0 of 9 | pass |
| incr | misc | 123 | 126 | 500 | 102 | 99 | TRAP | 69/0/0 of 69 | run-trap |
| incr-old | misc | 106 | 105 | 295 | 76 | 96 | 5/9/0 of 14 | 14/0/0 of 14 | partial |
| obj | misc | 125 | 130 | 368 | 59 | 148 | TRAP | 8/0/76 of 84 | run-trap |
| indexObj | misc | 113 | 115 | 350 | 54 | 92 | TRAP | 0/0/65 of 65 | run-trap |
| dstring | misc | 119 | 120 | 314 | 98 | 94 | 0/46/0 of 46 | 0/0/46 of 46 | partial |
| assocd | misc | 105 | 105 | 329 | 70 | 93 | 0/11/0 of 11 | 0/0/11 of 11 | partial |
| opt | misc | 110 | 111 | 340 | 80 | 102 | TRAP | 31/0/0 of 31 | run-trap |
| stack | misc | 105 | 104 | 363 | 70 | 575 | 0/3/0 of 3 | 3/0/0 of 3 | partial |
| misc | misc | 105 | 104 | 377 | 305 | 136 | 0/301/0 of 301 | 2/0/299 of 301 | partial |
| brodnik | misc | 105 | 104 | 325 | 231 | 116 | 0/257/0 of 257 | no-summary | partial |
| range | misc | 103 | 102 | 326 | 67 | 97 | no-summary | no-summary | no-summary |
| aaa_exit | misc | 104 | 104 | 337 | 65 | 268 | 0/2/0 of 2 | 2/0/0 of 2 | partial |
