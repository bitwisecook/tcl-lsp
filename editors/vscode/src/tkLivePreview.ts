/**
 * Tk live preview via `wish`.
 *
 * When `wish` is available on the system PATH, this module can run Tk source
 * in a subprocess, capture a screenshot, and return the image data for display
 * in the Tk Preview webview panel.
 */

import { execFile } from "child_process";
import { existsSync, readFileSync, unlinkSync, writeFileSync } from "fs";
import { tmpdir } from "os";
import * as path from "path";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

const WISH_TIMEOUT_MS = 5000;

let wishAvailable: boolean | undefined;

/**
 * Check whether `wish` (the Tk interpreter) is available on the system PATH.
 * The result is cached after the first check.
 */
export async function isWishAvailable(): Promise<boolean> {
  if (wishAvailable !== undefined) {
    return wishAvailable;
  }

  try {
    await execFileAsync("wish", ["-version"], { timeout: 2000 });
    wishAvailable = true;
  } catch {
    // wish may not support -version; try a simple eval instead
    try {
      await execFileAsync("wish", ["-e", "exit"], { timeout: 2000 });
      wishAvailable = true;
    } catch {
      wishAvailable = false;
    }
  }

  return wishAvailable;
}

/**
 * Run the given Tk source in `wish` and capture a screenshot.
 *
 * Appends Tk screenshot code that writes a PPM file, then converts to
 * base64 PNG for webview display. Gracefully returns `undefined` when
 * `wish` is unavailable or the process times out.
 */
export async function captureTkScreenshot(source: string): Promise<string | undefined> {
  if (!(await isWishAvailable())) {
    return undefined;
  }

  const tmpDir = tmpdir();
  const srcFile = path.join(tmpDir, `tk-preview-${process.pid}.tcl`);
  const imgFile = path.join(tmpDir, `tk-preview-${process.pid}.ppm`);

  // Append screenshot capture code to the source.
  // This waits for the window to render, captures the root window to an
  // image, saves it as PPM (always available in Tk), then exits.
  const captureCode = `
# -- Live preview capture --
after 500 {
  update idletasks
  set img [image create photo -format window -data .]
  $img write {${imgFile}} -format ppm
  exit 0
}
`;

  const fullSource = source + "\n" + captureCode;

  try {
    writeFileSync(srcFile, fullSource, "utf-8");

    await execFileAsync("wish", [srcFile], { timeout: WISH_TIMEOUT_MS });

    if (!existsSync(imgFile)) {
      return undefined;
    }

    // Read the PPM and return as base64 data URI
    const imageData = readFileSync(imgFile);
    const base64 = imageData.toString("base64");
    return `data:image/x-portable-pixmap;base64,${base64}`;
  } catch {
    // Timeout or wish failure — degrade gracefully
    return undefined;
  } finally {
    // Clean up temp files
    try {
      unlinkSync(srcFile);
    } catch {
      // ignore
    }
    try {
      unlinkSync(imgFile);
    } catch {
      // ignore
    }
  }
}
