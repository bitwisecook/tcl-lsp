// Decompile every function that references one of the needle strings.
// Output: <outdir>/<program>-refs.c
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import ghidra.program.model.address.*;
import java.io.*;
import java.util.*;

public class DumpRefs extends GhidraScript {
    String[] NEEDLES = {
        "starts_with", "ends_with", "matches_glob", "matches_regex", "contains", "equals", "matches",
        "missing a script after", "extra characters after close-brace", "extra characters after close-quote",
        "syntax error in expression", "unknown math function", "use curly braces to avoid double substitution",
        "command is not valid in the current scope", "command is not valid in current event context",
        "undefined procedure", "missing close-brace", "missing operand",
    };
    public void run() throws Exception {
        String outdir = getScriptArgs()[0];
        Set<String> needles = new HashSet<>(Arrays.asList(NEEDLES));
        Map<Function, Set<String>> hits = new LinkedHashMap<>();
        for (Data d : currentProgram.getListing().getDefinedData(true)) {
            if (!d.hasStringValue()) continue;
            Object v = d.getValue();
            if (!(v instanceof String)) continue;
            String s = (String) v;
            String match = null;
            for (String n : needles) { if (s.equals(n) || (n.length() > 12 && s.contains(n))) { match = n; break; } }
            if (match == null) continue;
            for (Reference r : getReferencesTo(d.getAddress())) {
                Function f = getFunctionContaining(r.getFromAddress());
                if (f == null) continue;
                hits.computeIfAbsent(f, k -> new TreeSet<>()).add(match + " @" + d.getAddress());
            }
        }
        DecompInterface di = new DecompInterface();
        di.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(new FileWriter(outdir + "/" + currentProgram.getName() + "-refs.c"))) {
            for (Map.Entry<Function, Set<String>> e : hits.entrySet()) {
                Function f = e.getKey();
                w.println("// ===== " + f.getName() + " @ " + f.getEntryPoint() + " refs: " + e.getValue());
                DecompileResults res = di.decompileFunction(f, 120, monitor);
                w.println(res.decompileCompleted() ? res.getDecompiledFunction().getC() : "// decompile failed: " + res.getErrorMessage());
            }
        }
        println("functions: " + hits.size());
    }
}
