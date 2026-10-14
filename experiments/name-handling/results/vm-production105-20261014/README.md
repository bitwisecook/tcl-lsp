# VM current consumer build

Original C5 and Jim provider preflight succeeds. The library build fails on two newly introduced descriptor-accessor API calls in the compiler shared write-effects helper, before any semantic controls execute. Full captured source/provider, compiled-input and artifact drift maps are empty. The correction uses the existing selected invocation and owned facts descriptor accessors; original ownership guards and assertions remain unchanged.
