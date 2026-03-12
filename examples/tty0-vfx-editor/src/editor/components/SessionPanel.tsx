import { ChangeEvent, RefObject } from "react";
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

export function SessionPanel({
  loadedName,
  dirty,
  hasLoadedDocument,
  message,
  messageTone,
  fileInputRef,
  audioInputRef,
  onNewVisualizer,
  onImportRecord,
  onImportAudio,
  onSave,
  onRevert,
}: {
  loadedName: string | null;
  dirty: boolean;
  hasLoadedDocument: boolean;
  message: string;
  messageTone: "info" | "success" | "warning" | "error";
  fileInputRef: RefObject<HTMLInputElement>;
  audioInputRef: RefObject<HTMLInputElement>;
  onNewVisualizer: () => void;
  onImportRecord: (event: ChangeEvent<HTMLInputElement>) => void;
  onImportAudio: (event: ChangeEvent<HTMLInputElement>) => void;
  onSave: () => void;
  onRevert: () => void;
}) {
  return (
    <Card className="panel utility-panel">
      <CardHeader className="p-0">
        <div className="session-summary">
          <div className="flex items-start justify-between gap-3">
            <CardTitle className="text-base">{loadedName ?? "Untitled effect"}</CardTitle>
            <Badge variant={dirty ? "amber" : "cyan"}>{dirty ? "Unsaved" : loadedName ? "Saved" : "New"}</Badge>
          </div>
          <CardDescription>{dirty ? "Unsaved changes" : loadedName ? "Saved draft" : "New draft"}</CardDescription>
        </div>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-3 p-0">
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
        <div className="action-grid">
          <Button onClick={onNewVisualizer} variant="secondary">
            New Effect
          </Button>
          <Button onClick={() => fileInputRef.current?.click()} variant="outline">
            Load JSON
          </Button>
          <Button onClick={() => audioInputRef.current?.click()} variant="outline">
            Import Audio
          </Button>
          <Button onClick={onSave}>Save JSON</Button>
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
                  This will restore the last loaded or saved JSON state for the current effect.
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
    </Card>
  );
}
