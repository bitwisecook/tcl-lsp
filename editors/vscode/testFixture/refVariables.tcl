proc process {dataName} {
    upvar 1 $dataName localdata
    dict update localdata count cnt total sum {
        incr cnt
    }
}
