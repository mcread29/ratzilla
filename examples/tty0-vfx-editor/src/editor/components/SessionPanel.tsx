import { ChangeEvent, RefObject, useMemo, useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";
import { MountedAudioState } from "../editor-types";
import { StoredVfxProject } from "../storage/localProjects";

type SaveDialogState = {
  open: boolean;
  mode: "save" | "save-as";
  name: string;
  error: string | null;
  submitting: boolean;
};

function summarizeAudioPath(audioPath: string): string {
  const trimmed = audioPath.trim();
  const parts = trimmed.split(/[\\/]/).filter(Boolean);
  const label = parts.length > 0 ? parts[parts.length - 1] : trimmed;
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    return `External audio: ${label}`;
  }
  return `Path audio: ${label}`;
}

function sourceBadgeLabel({
  dirty,
  recoveredWorkspace,
  sourceKind,
}: {
  dirty: boolean;
  recoveredWorkspace: boolean;
  sourceKind: "local-project" | "imported-json" | "new-draft" | null;
}): string {
  if (recoveredWorkspace) {
    return "Recovered";
  }
  if (dirty) {
    return "Unsaved";
  }
  if (sourceKind === "local-project") {
    return "Saved";
  }
  if (sourceKind === "imported-json") {
    return "Imported";
  }
  return "Unsaved";
}

function sourceDescription({
  recoveredWorkspace,
  sourceKind,
}: {
  recoveredWorkspace: boolean;
  sourceKind: "local-project" | "imported-json" | "new-draft" | null;
}): string {
  if (recoveredWorkspace) {
    return "Recovered unsaved session";
  }
  if (sourceKind === "local-project") {
    return "Saved in browser";
  }
  if (sourceKind === "imported-json") {
    return "Imported JSON draft";
  }
  return "New draft";
}

export function SessionPanel({
  loadedName,
  sourceKind,
  dirty,
  recoveredWorkspace,
  hasLoadedDocument,
  message,
  messageTone,
  fileInputRef,
  audioInputRef,
  projects,
  activeProjectId,
  mountedAudio,
  audioPath,
  saveDialog,
  onNewVisualizer,
  onImportRecord,
  onImportAudio,
  onSave,
  onSaveAs,
  onExportJson,
  onSelectProject,
  onSaveDialogClose,
  onSaveDialogNameChange,
  onSubmitSaveDialog,
  onRevert,
}: {
  loadedName: string | null;
  sourceKind: "local-project" | "imported-json" | "new-draft" | null;
  dirty: boolean;
  recoveredWorkspace: boolean;
  hasLoadedDocument: boolean;
  message: string;
  messageTone: "info" | "success" | "warning" | "error";
  fileInputRef: RefObject<HTMLInputElement>;
  audioInputRef: RefObject<HTMLInputElement>;
  projects: StoredVfxProject[];
  activeProjectId: string;
  mountedAudio: MountedAudioState;
  audioPath: string | null;
  saveDialog: SaveDialogState;
  onNewVisualizer: () => void;
  onImportRecord: (event: ChangeEvent<HTMLInputElement>) => void;
  onImportAudio: (event: ChangeEvent<HTMLInputElement>) => void;
  onSave: () => void;
  onSaveAs: () => void;
  onExportJson: () => void;
  onSelectProject: (projectId: string) => void;
  onSaveDialogClose: () => void;
  onSaveDialogNameChange: (name: string) => void;
  onSubmitSaveDialog: () => void;
  onRevert: () => void;
}) {
  const [pendingProjectId, setPendingProjectId] = useState<string | null>(null);

  const audioSummary = useMemo(() => {
    if (mountedAudio.kind === "imported-file") {
      return `Imported audio: ${mountedAudio.fileName}`;
    }
    if (audioPath) {
      return summarizeAudioPath(audioPath);
    }
    return "No audio mounted";
  }, [audioPath, mountedAudio]);

  const selectValue = activeProjectId || "__unsaved__";

  function handleProjectSelect(nextValue: string) {
    if (nextValue === "__unsaved__" || nextValue === activeProjectId) {
      return;
    }
    if (dirty) {
      setPendingProjectId(nextValue);
      return;
    }
    onSelectProject(nextValue);
  }

  function confirmProjectSwitch() {
    if (!pendingProjectId) {
      return;
    }
    onSelectProject(pendingProjectId);
    setPendingProjectId(null);
  }

  return (
    <Card className="panel utility-panel">
      <CardHeader className="p-0">
        <div className="session-summary">
          <div className="flex items-start justify-between gap-3">
            <CardTitle className="text-base">{loadedName ?? "Untitled effect"}</CardTitle>
            <Badge variant={dirty ? "amber" : "cyan"}>
              {sourceBadgeLabel({
                dirty,
                recoveredWorkspace,
                sourceKind,
              })}
            </Badge>
          </div>
          <CardDescription>
            {dirty ? "Unsaved changes" : sourceDescription({ recoveredWorkspace, sourceKind })}
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-3 p-0">
        <div className="session-project-shelf">
          <Label htmlFor="project-picker">Project</Label>
          <NativeSelect id="project-picker" value={selectValue} onChange={(event) => handleProjectSelect(event.target.value)}>
            <option value="__unsaved__">Unsaved draft</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </NativeSelect>
        </div>
        <div className="session-audio-summary">{audioSummary}</div>
        <div
          className={[
            "rounded-lg border px-3 py-2 text-sm",
            messageTone === "error"
              ? "border-destructive/45 bg-destructive/10 text-destructive-foreground"
              : messageTone === "warning"
                ? "border-editor-amber/35 bg-editor-amber/8 text-foreground"
                : messageTone === "success"
                  ? "border-editor-cyan/35 bg-editor-cyan/8 text-foreground"
                  : "border-border/80 bg-muted/35 text-muted-foreground",
          ].join(" ")}
        >
          {message}
        </div>
        <div className="action-grid action-grid-wide">
          <Button onClick={onNewVisualizer} variant="secondary">
            New Effect
          </Button>
          <Button onClick={() => fileInputRef.current?.click()} variant="outline">
            Import JSON
          </Button>
          <Button onClick={() => audioInputRef.current?.click()} variant="outline">
            Import Audio
          </Button>
          <Button onClick={onSave}>Save</Button>
          <Button onClick={onSaveAs} variant="secondary">
            Save As
          </Button>
          <Button onClick={onExportJson} variant="outline">
            Export JSON
          </Button>
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="ghost" disabled={!hasLoadedDocument || !dirty}>
                Revert
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Revert unsaved changes?</AlertDialogTitle>
                <AlertDialogDescription>
                  This restores the last loaded or manually saved state for the current effect and resets mounted audio to the
                  saved baseline.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction onClick={onRevert}>Revert draft</AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </CardContent>
      <input
        ref={fileInputRef}
        type="file"
        accept=".json,application/json"
        hidden
        onChange={onImportRecord}
      />
      <input
        ref={audioInputRef}
        type="file"
        accept="audio/*"
        hidden
        onChange={onImportAudio}
      />

      <AlertDialog open={pendingProjectId != null} onOpenChange={(open) => !open && setPendingProjectId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Switch projects and discard unsaved changes?</AlertDialogTitle>
            <AlertDialogDescription>
              The current draft has unsaved changes. Switching projects will replace the open workspace with the selected
              browser project.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmProjectSwitch}>Switch project</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog open={saveDialog.open} onOpenChange={(open) => !open && onSaveDialogClose()}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{saveDialog.mode === "save-as" ? "Save as new browser project" : "Save browser project"}</AlertDialogTitle>
            <AlertDialogDescription>
              Manual project save stores the visualizer and restores imported audio from browser storage automatically.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="session-dialog-fields">
            <div className="session-field">
              <Label htmlFor="project-name">Project name</Label>
              <Input
                id="project-name"
                value={saveDialog.name}
                onChange={(event) => onSaveDialogNameChange(event.target.value)}
                placeholder="Untitled effect"
              />
            </div>
            {saveDialog.error ? <p className="session-dialog-error">{saveDialog.error}</p> : null}
          </div>
          <AlertDialogFooter>
            <Button variant="outline" onClick={onSaveDialogClose} disabled={saveDialog.submitting}>
              Cancel
            </Button>
            <Button onClick={onSubmitSaveDialog} disabled={saveDialog.submitting}>
              {saveDialog.submitting ? "Saving..." : "Save project"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
