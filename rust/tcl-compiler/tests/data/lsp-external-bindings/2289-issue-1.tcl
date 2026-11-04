    proc New {} {
        catch {interp delete vectorCoreChild}
        interp create vectorCoreChild
        interp eval vectorCoreChild [list set auto_path $::auto_path]
    }
