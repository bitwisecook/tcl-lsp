# Fixture for issues #1329 and #1330 — bareword `my <method>` must be
# diagnosed like every other same-object dispatch spelling, and the range
# every dispatch diagnostic reports must cover the offending word exactly.
#
# The companion test (issue1329SelfDispatch.test.ts) locates each expected
# range by searching this document for the offending word, so the assertions
# stay pinned to the real text rather than to hand-counted line numbers.
# Names are unique to this fixture so the project-wide index cannot
# (correctly) abstain on a cross-file ambiguity and mask what is under test.
#
#   my nosuchissue1329method    -> W308, underlining the method word alone
#   my issue1329Tick            -> a real method: silent
#   my variable issue1329State  -> oo::object's unexported member: silent
#   [Issue1329Dog new] nosuchissue1329bark -> W308, method word alone
oo::class create Issue1329Class {
    method issue1329Tick {} { return {} }
    method issue1329Anim {} {
        my nosuchissue1329method
        my issue1329Tick
        my variable issue1329State
        return {}
    }
}
oo::class create Issue1329Dog {
    method issue1329Bark {} { return {} }
}
proc issue1329Handler {} {
    [Issue1329Dog new] nosuchissue1329bark
}
