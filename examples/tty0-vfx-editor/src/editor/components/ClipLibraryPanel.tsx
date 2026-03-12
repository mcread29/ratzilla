import { DragEvent } from "react";
import { ChromaticBulgeGridClip, LaneId } from "../../types";
import { ClipDropIndicator } from "../editor-types";
import { ClipActionIcon } from "./icons";
import { ClipCard } from "./ClipCard";
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
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function ClipLibraryPanel({
  laneClips,
  selectedClipId,
  selectedLane,
  timelineBeatsPerMeasure,
  draggedClipId,
  clipDropIndicator,
  onSelectClip,
  onAddClip,
  onDuplicateClip,
  onDeleteClip,
  onClipDragStart,
  onClipDragOver,
  onClipDrop,
  onClearClipDragState,
  hasSelectedClip,
  selectedClipName,
}: {
  laneClips: ChromaticBulgeGridClip[];
  selectedClipId: string | null;
  selectedLane: LaneId;
  timelineBeatsPerMeasure: number;
  draggedClipId: string | null;
  clipDropIndicator: ClipDropIndicator | null;
  onSelectClip: (clipId: string, lane: LaneId) => void;
  onAddClip: () => void;
  onDuplicateClip: () => void;
  onDeleteClip: () => void;
  onClipDragStart: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClipDragOver: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClipDrop: (event: DragEvent<HTMLButtonElement>, clipId: string) => void;
  onClearClipDragState: () => void;
  hasSelectedClip: boolean;
  selectedClipName: string | null;
}) {
  return (
    <Card className="panel library-panel clips-panel">
      <CardHeader className="panel-header p-0">
        <CardTitle className="text-base">Clips</CardTitle>
        <div className="inline-actions">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button className="icon-button" onClick={onAddClip} type="button" variant="toolbar" size="icon" aria-label="Add clip">
                <ClipActionIcon name="add" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Add clip</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                className="icon-button"
                onClick={onDuplicateClip}
                disabled={!hasSelectedClip}
                type="button"
                variant="toolbar"
                size="icon"
                aria-label="Copy clip"
              >
                <ClipActionIcon name="copy" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Copy clip</TooltipContent>
          </Tooltip>
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button
                className="icon-button"
                disabled={!hasSelectedClip}
                type="button"
                variant="toolbar"
                size="icon"
                aria-label="Delete clip"
              >
                <ClipActionIcon name="delete" />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Delete selected clip?</AlertDialogTitle>
                <AlertDialogDescription>
                  {selectedClipName
                    ? `This will remove ${selectedClipName} if it is not placed on the timeline.`
                    : "This will remove the selected clip if it is not placed on the timeline."}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction onClick={onDeleteClip}>Delete clip</AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </CardHeader>
      <CardContent className="scroll-shell p-0">
        <ScrollArea className="h-full rounded-lg">
          <div className="list pr-3">
            {laneClips.map((clip) => (
              <ClipCard
                key={clip.id}
                clip={clip}
                selectedClipId={selectedClipId}
                selectedLane={selectedLane}
                timelineBeatsPerMeasure={timelineBeatsPerMeasure}
                draggedClipId={draggedClipId}
                clipDropIndicator={clipDropIndicator}
                onSelect={onSelectClip}
                onDragStart={onClipDragStart}
                onDragOver={onClipDragOver}
                onDrop={onClipDrop}
                onDragEnd={onClearClipDragState}
              />
            ))}
            {!laneClips.length ? <p className="empty-copy rounded-lg border border-dashed border-border/70 px-3 py-4 text-sm">No clips for this track yet.</p> : null}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
