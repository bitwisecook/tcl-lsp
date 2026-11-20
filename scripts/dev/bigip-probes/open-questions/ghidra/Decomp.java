import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.io.*;
public class Decomp extends GhidraScript {
    public void run() throws Exception {
        PrintWriter w = new PrintWriter(new FileWriter(getScriptArgs()[0]));
        DecompInterface di = new DecompInterface(); di.openProgram(currentProgram);
        for (int i = 1; i < getScriptArgs().length; i++) {
            Function f = getFunctionAt(toAddr(Long.decode(getScriptArgs()[i])));
            if (f == null) { w.println("// no function at " + getScriptArgs()[i]); continue; }
            DecompileResults r = di.decompileFunction(f, 180, monitor);
            w.println("// ===== " + f.getName() + " @ " + f.getEntryPoint());
            w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// failed " + r.getErrorMessage());
        }
        w.close();
    }
}
