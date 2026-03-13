import { convertFileSrc, isTauri } from "@tauri-apps/api/core";

const WINDOWS_DRIVE_PATH_RE = /^[a-zA-Z]:[\\/]/;
const WINDOWS_UNC_PATH_RE = /^\\\\/;

function isFileSystemPath(source: string): boolean {
  return source.startsWith("file://") || source.startsWith("/") || WINDOWS_DRIVE_PATH_RE.test(source) || WINDOWS_UNC_PATH_RE.test(source);
}

function normalizeFileSystemPath(source: string): string {
  if (!source.startsWith("file://")) {
    return source;
  }

  try {
    const url = new URL(source);
    if (url.protocol !== "file:") {
      return source;
    }

    const decodedPath = decodeURIComponent(url.pathname);
    if (url.host) {
      return `\\\\${url.host}${decodedPath.replace(/\//g, "\\")}`;
    }
    if (/^\/[a-zA-Z]:/.test(decodedPath)) {
      return decodedPath.slice(1);
    }
    return decodedPath;
  } catch {
    return source;
  }
}

export function resolveAudioSourceUrl(source: string | null | undefined): string | null {
  const trimmed = source?.trim() ?? "";
  if (!trimmed) {
    return null;
  }

  if (
    trimmed.startsWith("http://") ||
    trimmed.startsWith("https://") ||
    trimmed.startsWith("blob:") ||
    trimmed.startsWith("data:") ||
    trimmed.startsWith("asset:") ||
    trimmed.startsWith("http://asset.localhost") ||
    trimmed.startsWith("https://asset.localhost")
  ) {
    return trimmed;
  }

  if (!isFileSystemPath(trimmed)) {
    return trimmed;
  }

  if (!isTauri()) {
    return null;
  }

  return convertFileSrc(normalizeFileSystemPath(trimmed));
}
