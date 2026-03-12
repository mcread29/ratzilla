import { ChangeEvent, RefObject } from "react";

export function SessionPanel({
  loadedName,
  dirty,
  hasLoadedDocument,
  message,
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
  fileInputRef: RefObject<HTMLInputElement>;
  audioInputRef: RefObject<HTMLInputElement>;
  onNewVisualizer: () => void;
  onImportRecord: (event: ChangeEvent<HTMLInputElement>) => void;
  onImportAudio: (event: ChangeEvent<HTMLInputElement>) => void;
  onSave: () => void;
  onRevert: () => void;
}) {
  return (
    <section className="panel utility-panel">
      <div className="session-summary">
        <strong>{loadedName ?? "Untitled effect"}</strong>
        <span>{dirty ? "Unsaved changes" : loadedName ? "Saved draft" : "New draft"}</span>
      </div>
      <p className="empty-copy">{message}</p>
      <div className="action-grid">
        <button onClick={onNewVisualizer}>New Effect</button>
        <button onClick={() => fileInputRef.current?.click()}>Load JSON</button>
        <button onClick={() => audioInputRef.current?.click()}>Import Audio</button>
        <button onClick={onSave}>Save JSON</button>
        <button onClick={onRevert} disabled={!hasLoadedDocument || !dirty}>
          Revert
        </button>
      </div>
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
    </section>
  );
}
