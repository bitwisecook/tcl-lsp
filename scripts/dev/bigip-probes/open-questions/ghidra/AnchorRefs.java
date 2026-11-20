// For functions referencing given strings: list them, their callers and any
// data references to them (function-pointer tables), and decompile them.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.*;
import java.io.*;
import java.util.*;

public class AnchorRefs extends GhidraScript {
    public void run() throws Exception {
        PrintWriter w = new PrintWriter(new FileWriter(getScriptArgs()[0]));
        Memory mem = currentProgram.getMemory();
        FunctionManager fm = currentProgram.getFunctionManager();
        DecompInterface di = new DecompInterface(); di.openProgram(currentProgram);
        Set<Function> done = new HashSet<>();
        for (int i = 1; i < getScriptArgs().length; i++) {
            byte[] needle = getScriptArgs()[i].getBytes();
            for (MemoryBlock b : mem.getBlocks()) {
                if (!b.getName().equals(".rodata")) continue;
                Address hit = mem.findBytes(b.getStart(), b.getEnd(), needle, null, true, monitor);
                while (hit != null) {
                    Address st = hit; while (mem.getByte(st.subtract(1)) != 0) st = st.subtract(1);
                    for (Reference r : getReferencesTo(st)) {
                        Function f = fm.getFunctionContaining(r.getFromAddress());
                        if (f == null || !done.add(f)) continue;
                        w.println("===== " + getScriptArgs()[i] + " -> " + f.getName() + " @ " + f.getEntryPoint());
                        for (Reference cr : getReferencesTo(f.getEntryPoint())) {
                            Function cf = fm.getFunctionContaining(cr.getFromAddress());
                            w.println("  ref from " + cr.getFromAddress() + " type " + cr.getReferenceType() + " in " + (cf == null ? "(data)" : cf.getName() + " @ " + cf.getEntryPoint()));
                        }
                        DecompileResults res = di.decompileFunction(f, 120, monitor);
                        if (res.decompileCompleted()) w.println(res.getDecompiledFunction().getC());
                    }
                    hit = mem.findBytes(hit.add(1), b.getEnd(), needle, null, true, monitor);
                }
            }
        }
        w.close();
    }
}
