import { TimelineTool } from "../editor-types";
import { TimelineActionIcon, TimelineFieldIcon, TimelineToolIcon } from "./icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function ArrangementToolbar({
  bpm,
  measures,
  beatsPerMeasure,
  timelineTool,
  hasSelectedClip,
  selectedPlacementCount,
  onChangeBpm,
  onChangeMeasures,
  onChangeBeatsPerMeasure,
  onChangeTimelineTool,
  onAddPlacement,
  onCopyPlacements,
  onDeletePlacements,
}: {
  bpm: number;
  measures: number;
  beatsPerMeasure: number;
  timelineTool: TimelineTool;
  hasSelectedClip: boolean;
  selectedPlacementCount: number;
  onChangeBpm: (value: number) => void;
  onChangeMeasures: (value: number) => void;
  onChangeBeatsPerMeasure: (value: number) => void;
  onChangeTimelineTool: (tool: TimelineTool) => void;
  onAddPlacement: () => void;
  onCopyPlacements: () => void;
  onDeletePlacements: () => void;
}) {
  return (
    <div className="inline-actions arrangement-toolbar">
      <div className="arrangement-toolbar-fields">
        <div className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="bpm" />
          </span>
          <Input
            type="number"
            step={0.1}
            aria-label="BPM"
            className="h-8 w-[54px] px-1.5 text-center"
            value={bpm}
            onChange={(event) => onChangeBpm(Number(event.target.value))}
          />
        </div>
        <div className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="measures" />
          </span>
          <Input
            type="number"
            step={1}
            min={1}
            aria-label="Measures"
            className="h-8 w-[54px] px-1.5 text-center"
            value={measures}
            onChange={(event) => onChangeMeasures(Number(event.target.value))}
          />
        </div>
        <div className="compact-field">
          <span className="compact-field-icon" aria-hidden="true">
            <TimelineFieldIcon name="time" />
          </span>
          <Input
            type="number"
            step={1}
            min={1}
            aria-label="Time"
            className="h-8 w-[54px] px-1.5 text-center"
            value={beatsPerMeasure}
            onChange={(event) => onChangeBeatsPerMeasure(Number(event.target.value))}
          />
        </div>
      </div>
      <div className="arrangement-toolbar-actions">
        <ToggleGroup
          type="single"
          value={timelineTool}
          onValueChange={(value) => {
            if (value === "select" || value === "pencil") {
              onChangeTimelineTool(value);
            }
          }}
          aria-label="Timeline tool"
        >
          <ToggleGroupItem value="select" aria-label="Select tool">
            <TimelineToolIcon name="select" />
          </ToggleGroupItem>
          <ToggleGroupItem value="pencil" aria-label="Pencil tool">
            <TimelineToolIcon name="pencil" />
          </ToggleGroupItem>
        </ToggleGroup>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="icon-button"
              onClick={onAddPlacement}
              disabled={!hasSelectedClip}
              type="button"
              variant="toolbar"
              size="icon"
              aria-label="Place selected clip"
            >
              <TimelineActionIcon name="place" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Place selected clip</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="icon-button"
              onClick={onCopyPlacements}
              disabled={!selectedPlacementCount}
              type="button"
              variant="toolbar"
              size="icon"
              aria-label={`Copy placement${selectedPlacementCount === 1 ? "" : "s"}`}
            >
              <TimelineActionIcon name="copy" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{`Copy placement${selectedPlacementCount === 1 ? "" : "s"}`}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="icon-button"
              onClick={onDeletePlacements}
              disabled={!selectedPlacementCount}
              type="button"
              variant="toolbar"
              size="icon"
              aria-label={`Delete placement${selectedPlacementCount === 1 ? "" : "s"}`}
            >
              <TimelineActionIcon name="delete" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{`Delete placement${selectedPlacementCount === 1 ? "" : "s"}`}</TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}
