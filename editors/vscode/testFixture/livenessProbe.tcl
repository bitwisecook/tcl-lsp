# Asked about only by the liveness probe in src/test/helper.ts (issue #1294),
# never driven by a test — so a hover on it can never be queued behind another
# test's edits, and an answer here means the server's document pipeline is
# draining for documents other than the stalled one.
set probe 1
