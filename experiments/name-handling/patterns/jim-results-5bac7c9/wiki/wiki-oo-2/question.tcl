
        oo::class create C {method hidden {} {return ok}; method prefix {} {namespace code {my hidden}}}
        set obj [C new]
        set prefix [$obj prefix]
        $obj destroy
        catch {eval $prefix}
    
