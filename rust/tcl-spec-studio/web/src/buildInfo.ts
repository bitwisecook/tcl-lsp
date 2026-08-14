// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Injected into both front-end bundles from `git describe`. */
declare const __SPEC_STUDIO_FRONTEND_VERSION__: string;

export interface BuildAsset {
  name: string;
  version: string;
  sha256: string;
  bytes: number;
}

export interface BuildInfo {
  version: string;
  assets: BuildAsset[];
}

const CACHE_BUST_PARAM = "spec-studio-cache-bust";

function reportFatal(message: string): void {
  console.error(`Spec Studio: ${message}`);
  const status = document.getElementById("status");
  if (status) {
    status.className = "status err";
    status.textContent = message;
  }
}

/** Reload an HTTP deployment once with a unique query after a version mismatch. */
function cacheBreak(reason: string, expectedVersion: string): boolean {
  const url = new URL(window.location.href);
  if (!/^https?:$/.test(url.protocol)) {
    reportFatal(`asset version mismatch in local file: ${reason}`);
    return false;
  }
  if (url.searchParams.has(CACHE_BUST_PARAM)) {
    reportFatal(`asset version mismatch after cache refresh: ${reason}`);
    return false;
  }

  url.searchParams.set(CACHE_BUST_PARAM, `${expectedVersion}-${Date.now()}`);
  console.warn(`Spec Studio: ${reason}; reloading with cache-busting URL ${url}`);
  window.location.replace(url.toString());
  return true;
}

/** Read and validate the embedded manifest before the WASM engine starts. */
export function verifyBuildInfo(): BuildInfo | null {
  const node = document.getElementById("studio-build");
  if (!node?.textContent) {
    reportFatal("embedded build manifest is missing");
    return null;
  }

  let info: BuildInfo;
  try {
    info = JSON.parse(node.textContent) as BuildInfo;
  } catch (error) {
    reportFatal(`embedded build manifest is invalid: ${String(error)}`);
    return null;
  }

  console.info(`Spec Studio ${info.version}`);
  console.table(
    info.assets.map((asset) => ({
      asset: asset.name,
      version: asset.version,
      sha256: asset.sha256,
      bytes: asset.bytes,
    })),
  );

  const wrongVersion = info.assets.find((asset) => asset.version !== info.version);
  if (wrongVersion) {
    cacheBreak(
      `${wrongVersion.name} is ${wrongVersion.version}, manifest is ${info.version}`,
      info.version,
    );
    return null;
  }
  if (__SPEC_STUDIO_FRONTEND_VERSION__ !== info.version) {
    cacheBreak(
      `controller is ${__SPEC_STUDIO_FRONTEND_VERSION__}, assets are ${info.version}`,
      info.version,
    );
    return null;
  }
  return info;
}

/** Compare a lazily-loaded bundle's own embedded version with the page. */
export function verifyAssetVersion(info: BuildInfo, name: string, actual: string): boolean {
  if (actual === info.version) return true;
  cacheBreak(`${name} is ${actual}, page is ${info.version}`, info.version);
  return false;
}

/** Give a deployed external asset a content-keyed URL so normal caches are safe. */
export function assetUrl(info: BuildInfo, name: string, relative: string): string {
  const asset = info.assets.find((candidate) => candidate.name === name);
  const url = new URL(relative, document.baseURI);
  url.searchParams.set("v", asset?.sha256 ?? info.version);
  return url.href;
}
