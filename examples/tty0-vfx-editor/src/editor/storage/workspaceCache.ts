import { TrackVisualizerConfig } from "../../types";

const WORKSPACE_STORAGE_KEY = "tty0.vfxEditor.workspace.v1";
const DB_NAME = "tty0-vfx-editor";
const DB_VERSION = 1;
const WORKSPACE_BLOB_STORE = "workspaceBlobs";
const CURRENT_AUDIO_BLOB_KEY = "current-audio-file";

export type WorkspaceSource =
  | { kind: "local-project"; projectId: string }
  | { kind: "imported-json"; sourceName: string | null }
  | { kind: "new-draft" };

export type StoredWorkspace = {
  version: 1;
  source: WorkspaceSource;
  visualizer: TrackVisualizerConfig;
  savedSnapshot: string | null;
  loadedName: string | null;
  audioPath: string | null;
  audioMode: "none" | "path" | "imported-file";
  importedAudioBlobKey: string | null;
  importedAudioFileName: string | null;
  dirty: boolean;
  restoredFromRecovery: boolean;
  updatedAt: string;
};

export type WorkspaceSaveInput = Omit<StoredWorkspace, "version" | "updatedAt">;

function canUseLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function canUseIndexedDb(): boolean {
  return typeof window !== "undefined" && typeof window.indexedDB !== "undefined";
}

function isWorkspaceSource(value: unknown): value is WorkspaceSource {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<WorkspaceSource>;
  if (candidate.kind === "local-project") {
    return typeof (candidate as { projectId?: unknown }).projectId === "string";
  }
  if (candidate.kind === "imported-json") {
    return (
      "sourceName" in (candidate as { sourceName?: unknown }) &&
      (typeof (candidate as { sourceName?: unknown }).sourceName === "string" ||
        (candidate as { sourceName?: unknown }).sourceName === null)
    );
  }
  return candidate.kind === "new-draft";
}

function isStoredWorkspace(value: unknown): value is StoredWorkspace {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<StoredWorkspace>;
  return (
    candidate.version === 1 &&
    isWorkspaceSource(candidate.source) &&
    "visualizer" in candidate &&
    ("savedSnapshot" in candidate ? typeof candidate.savedSnapshot === "string" || candidate.savedSnapshot === null : false) &&
    ("loadedName" in candidate ? typeof candidate.loadedName === "string" || candidate.loadedName === null : false) &&
    ("audioPath" in candidate ? typeof candidate.audioPath === "string" || candidate.audioPath === null : false) &&
    (candidate.audioMode === "none" || candidate.audioMode === "path" || candidate.audioMode === "imported-file") &&
    ("importedAudioBlobKey" in candidate
      ? typeof candidate.importedAudioBlobKey === "string" || candidate.importedAudioBlobKey === null
      : false) &&
    ("importedAudioFileName" in candidate
      ? typeof candidate.importedAudioFileName === "string" || candidate.importedAudioFileName === null
      : false) &&
    typeof candidate.dirty === "boolean" &&
    typeof candidate.restoredFromRecovery === "boolean" &&
    typeof candidate.updatedAt === "string"
  );
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (!canUseIndexedDb()) {
      reject(new Error("IndexedDB is unavailable."));
      return;
    }
    const request = window.indexedDB.open(DB_NAME, DB_VERSION);
    request.onerror = () => reject(request.error ?? new Error("Failed to open IndexedDB."));
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(WORKSPACE_BLOB_STORE)) {
        database.createObjectStore(WORKSPACE_BLOB_STORE);
      }
    };
    request.onsuccess = () => resolve(request.result);
  });
}

function withStore<T>(mode: IDBTransactionMode, run: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return openDatabase().then(
    (database) =>
      new Promise<T>((resolve, reject) => {
        const transaction = database.transaction(WORKSPACE_BLOB_STORE, mode);
        const store = transaction.objectStore(WORKSPACE_BLOB_STORE);
        const request = run(store);
        request.onerror = () => reject(request.error ?? new Error("IndexedDB request failed."));
        request.onsuccess = () => resolve(request.result);
        transaction.oncomplete = () => database.close();
        transaction.onerror = () => reject(transaction.error ?? new Error("IndexedDB transaction failed."));
      }),
  );
}

export async function loadWorkspace(): Promise<StoredWorkspace | null> {
  if (!canUseLocalStorage()) {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(WORKSPACE_STORAGE_KEY);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as unknown;
    return isStoredWorkspace(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export async function saveWorkspace(input: WorkspaceSaveInput): Promise<void> {
  if (!canUseLocalStorage()) {
    throw new Error("Workspace storage is unavailable.");
  }
  const payload: StoredWorkspace = {
    ...input,
    version: 1,
    updatedAt: new Date().toISOString(),
  };
  window.localStorage.setItem(WORKSPACE_STORAGE_KEY, JSON.stringify(payload));
}

export async function clearWorkspace(): Promise<void> {
  if (!canUseLocalStorage()) {
    return;
  }
  window.localStorage.removeItem(WORKSPACE_STORAGE_KEY);
}

export async function saveStoredAudioBlob(blob: Blob, key: string): Promise<string> {
  await withStore("readwrite", (store) => store.put(blob, key));
  return key;
}

export async function loadStoredAudioBlob(key: string): Promise<Blob | null> {
  if (!key) {
    return null;
  }
  try {
    const result = await withStore<Blob | undefined>("readonly", (store) => store.get(key));
    return result ?? null;
  } catch {
    return null;
  }
}

export async function clearStoredAudioBlob(key: string | null): Promise<void> {
  if (!key || !canUseIndexedDb()) {
    return;
  }
  try {
    await withStore("readwrite", (store) => store.delete(key));
  } catch {
    // Ignore cache cleanup failures.
  }
}

export async function saveWorkspaceAudioBlob(blob: Blob): Promise<string> {
  return saveStoredAudioBlob(blob, CURRENT_AUDIO_BLOB_KEY);
}

export async function loadWorkspaceAudioBlob(key: string): Promise<Blob | null> {
  return loadStoredAudioBlob(key);
}

export async function clearWorkspaceAudioBlob(key: string | null): Promise<void> {
  await clearStoredAudioBlob(key);
}
