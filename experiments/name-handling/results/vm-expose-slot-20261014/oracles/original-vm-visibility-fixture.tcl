puts [proc outcome script {
                 set code [catch {uplevel 1 $script} message options]
                 list $code $message [dict get $options -errorcode]
             }
             proc p {} {return P}
             namespace eval N {proc p {} {return NP}}
             interp create kid
             kid eval {
                 proc p {} {return P}
                 namespace eval N {proc p {} {return NP}}
             }
             list [outcome {interp hide {} N::p}] [outcome {interp expose {} N::p}] [outcome {interp hide kid N::p}] [outcome {interp expose kid N::p}] [outcome {kid hide N::p}] [outcome {kid expose N::p}] [outcome {interp hide kid p N::held}] [outcome {interp expose kid N::held q}] [outcome {interp hide kid N::p held}] [outcome {interp expose kid held N::q}]]
