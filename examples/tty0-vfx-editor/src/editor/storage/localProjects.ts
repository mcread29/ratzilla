import { TrackVisualizerConfig } from "../../types";
import { serializeVisualizer } from "../utils/formatting";

const PROJECTS_STORAGE_KEY = "tty0.vfxEditor.projects.v1";
const LAST_PROJECT_STORAGE_KEY = "tty0.vfxEditor.lastProjectId.v1";

export type StoredProjectAudioMode = "none" | "path" | "imported-file";

export type StoredVfxProject = {
  id: string;
  name: string;
  visualizer: TrackVisualizerConfig;
  savedSnapshot: string;
  audioMode: StoredProjectAudioMode;
  audioPath: string | null;
  importedAudioBlobKey: string | null;
  importedAudioFileName: string | null;
  createdAt: string;
  updatedAt: string;
};

type StoredVfxProjectIndex = {
  version: 2;
  projects: StoredVfxProject[];
};

type LegacyStoredVfxProject = {
  id: string;
  name: string;
  visualizer: TrackVisualizerConfig;
  savedSnapshot: string;
  audioPath: string | null;
  createdAt: string;
  updatedAt: string;
};

function canUseLocalStorage(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function normalizeProjectName(name: string): string {
  return name.trim().replace(/\s+/g, " ");
}

function sortProjects(projects: StoredVfxProject[]): StoredVfxProject[] {
  return [...projects].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

function isLegacyStoredProject(value: unknown): value is LegacyStoredVfxProject {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<LegacyStoredVfxProject>;
  return (
    typeof candidate.id === "string" &&
    typeof candidate.name === "string" &&
    typeof candidate.savedSnapshot === "string" &&
    typeof candidate.createdAt === "string" &&
    typeof candidate.updatedAt === "string" &&
    "visualizer" in candidate &&
    ("audioPath" in candidate ? typeof candidate.audioPath === "string" || candidate.audioPath === null : true)
  );
}

function isStoredProject(value: unknown): value is StoredVfxProject {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<StoredVfxProject>;
  return (
    typeof candidate.id === "string" &&
    typeof candidate.name === "string" &&
    typeof candidate.savedSnapshot === "string" &&
    typeof candidate.createdAt === "string" &&
    typeof candidate.updatedAt === "string" &&
    "visualizer" in candidate &&
    (candidate.audioMode === "none" || candidate.audioMode === "path" || candidate.audioMode === "imported-file") &&
    ("audioPath" in candidate ? typeof candidate.audioPath === "string" || candidate.audioPath === null : false) &&
    ("importedAudioBlobKey" in candidate
      ? typeof candidate.importedAudioBlobKey === "string" || candidate.importedAudioBlobKey === null
      : false) &&
    ("importedAudioFileName" in candidate
      ? typeof candidate.importedAudioFileName === "string" || candidate.importedAudioFileName === null
      : false)
  );
}

function normalizeStoredProject(value: unknown): StoredVfxProject | null {
  if (isStoredProject(value)) {
    return {
      ...value,
      audioPath: value.audioPath?.trim() || null,
      importedAudioBlobKey: value.importedAudioBlobKey?.trim() || null,
      importedAudioFileName: value.importedAudioFileName?.trim() || null,
    };
  }

  if (isLegacyStoredProject(value)) {
    const audioPath = value.audioPath?.trim() || null;
    return {
      ...value,
      audioMode: audioPath ? "path" : "none",
      audioPath,
      importedAudioBlobKey: null,
      importedAudioFileName: null,
    };
  }

  return null;
}

function readProjectIndex(): StoredVfxProjectIndex {
  if (!canUseLocalStorage()) {
    return { version: 2, projects: [] };
  }
  try {
    const raw = window.localStorage.getItem(PROJECTS_STORAGE_KEY);
    if (!raw) {
      return { version: 2, projects: [] };
    }
    const parsed = JSON.parse(raw) as { version?: number; projects?: unknown };
    if (!Array.isArray(parsed.projects)) {
      return { version: 2, projects: [] };
    }
    return {
      version: 2,
      projects: sortProjects(parsed.projects.map(normalizeStoredProject).filter((project): project is StoredVfxProject => project !== null)),
    };
  } catch {
    return { version: 2, projects: [] };
  }
}

function writeProjectIndex(index: StoredVfxProjectIndex): void {
  if (!canUseLocalStorage()) {
    throw new Error("Browser project storage is unavailable.");
  }
  window.localStorage.setItem(PROJECTS_STORAGE_KEY, JSON.stringify(index));
}

function createProjectId(): string {
  return `project_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

export function listProjects(): StoredVfxProject[] {
  return readProjectIndex().projects;
}

export function getProject(projectId: string): StoredVfxProject | null {
  return readProjectIndex().projects.find((project) => project.id === projectId) ?? null;
}

export function nameExists(name: string, excludeProjectId?: string): boolean {
  const normalizedName = normalizeProjectName(name).toLocaleLowerCase();
  return readProjectIndex().projects.some(
    (project) => project.id !== excludeProjectId && project.name.toLocaleLowerCase() === normalizedName,
  );
}

export function createProject(input: {
  name: string;
  visualizer: TrackVisualizerConfig;
  audioMode: StoredProjectAudioMode;
  audioPath: string | null;
  importedAudioBlobKey: string | null;
  importedAudioFileName: string | null;
}): StoredVfxProject {
  const name = normalizeProjectName(input.name);
  if (!name) {
    throw new Error("Project name is required.");
  }
  if (nameExists(name)) {
    throw new Error(`A project named "${name}" already exists.`);
  }
  const timestamp = new Date().toISOString();
  const project: StoredVfxProject = {
    id: createProjectId(),
    name,
    visualizer: structuredClone(input.visualizer),
    savedSnapshot: serializeVisualizer(input.visualizer),
    audioMode: input.audioMode,
    audioPath: input.audioPath?.trim() || null,
    importedAudioBlobKey: input.importedAudioBlobKey?.trim() || null,
    importedAudioFileName: input.importedAudioFileName?.trim() || null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  const index = readProjectIndex();
  index.projects.push(project);
  writeProjectIndex({
    version: 2,
    projects: sortProjects(index.projects),
  });
  return project;
}

export function updateProject(
  projectId: string,
  input: {
    name?: string;
    visualizer: TrackVisualizerConfig;
    audioMode: StoredProjectAudioMode;
    audioPath: string | null;
    importedAudioBlobKey: string | null;
    importedAudioFileName: string | null;
  },
): StoredVfxProject {
  const index = readProjectIndex();
  const existing = index.projects.find((project) => project.id === projectId);
  if (!existing) {
    throw new Error("Project not found.");
  }
  const name = normalizeProjectName(input.name ?? existing.name);
  if (!name) {
    throw new Error("Project name is required.");
  }
  if (nameExists(name, projectId)) {
    throw new Error(`A project named "${name}" already exists.`);
  }
  const updatedProject: StoredVfxProject = {
    ...existing,
    name,
    visualizer: structuredClone(input.visualizer),
    savedSnapshot: serializeVisualizer(input.visualizer),
    audioMode: input.audioMode,
    audioPath: input.audioPath?.trim() || null,
    importedAudioBlobKey: input.importedAudioBlobKey?.trim() || null,
    importedAudioFileName: input.importedAudioFileName?.trim() || null,
    updatedAt: new Date().toISOString(),
  };
  writeProjectIndex({
    version: 2,
    projects: sortProjects(index.projects.map((project) => (project.id === projectId ? updatedProject : project))),
  });
  return updatedProject;
}

export function setLastProjectId(projectId: string | null): void {
  if (!canUseLocalStorage()) {
    return;
  }
  if (projectId) {
    window.localStorage.setItem(LAST_PROJECT_STORAGE_KEY, projectId);
    return;
  }
  window.localStorage.removeItem(LAST_PROJECT_STORAGE_KEY);
}

export function getLastProjectId(): string | null {
  if (!canUseLocalStorage()) {
    return null;
  }
  try {
    return window.localStorage.getItem(LAST_PROJECT_STORAGE_KEY);
  } catch {
    return null;
  }
}
