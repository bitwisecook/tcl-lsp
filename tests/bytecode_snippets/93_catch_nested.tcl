set status2 [catch {
    set status1 [catch {
        error "inner"
    } result1]
} result2]
