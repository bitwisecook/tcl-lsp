# W212/W216 registry-driven variable-name positions.
# Consumed by editors/vscode/src/test/precisionFalsePositives.test.ts.
proc link {arr} {
    upvar 1 remote ${arr}(x)
    catch {error boom} $res
}
