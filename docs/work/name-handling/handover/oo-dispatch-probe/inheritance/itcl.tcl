lappend auto_path /opt/google-cloud-sdk/platform/bundledpythonunix/lib
package require itcl
itcl::class A { method m {} { return A } }
itcl::class B { inherit A; method m {} { return B } }
itcl::class C { inherit A; method m {} { return C } }
puts [catch {itcl::class D { inherit B C }} msg]:$msg
itcl::class P {}
itcl::class Q {}
itcl::class B2 { method m {} { return B2 } }
itcl::class C2 { method m {} { return C2 } }
itcl::class D2 { inherit B2 C2 }
D2 d
puts "D2(B2,C2) heritage: [d info heritage] m=[d m]"
itcl::class X { inherit P Q }
itcl::class Y { inherit Q P }
puts [catch {itcl::class Z { inherit X Y }} msg]:$msg
catch {Z z; puts "Z heritage: [z info heritage]"}
