import { ChromaticBulgeGridClip, LfoPoint } from "../../types";
import { ClipMetadataForm } from "./ClipMetadataForm";
import { ShapeEditorPanel } from "./ShapeEditorPanel";
import { Card, CardContent } from "@/components/ui/card";

export function ClipEditorPanel({
  selectedClip,
  selectedShape,
  selectedPointIndex,
  selectedClipBeatOption,
  selectedClipBeatValue,
  clipLengthOptions,
  onSelectPoint,
  onCommitShape,
  onChangeClipName,
  onChangeClipBeatValue,
  onChangeClipMin,
  onChangeClipMax,
}: {
  selectedClip: ChromaticBulgeGridClip | null;
  selectedShape: { interpolation: "linear"; points: LfoPoint[] } | null;
  selectedPointIndex: number | null;
  selectedClipBeatOption: string;
  selectedClipBeatValue: number;
  clipLengthOptions: Array<{ label: string; beats: number }>;
  onSelectPoint: (index: number | null) => void;
  onCommitShape: (points: LfoPoint[]) => void;
  onChangeClipName: (value: string) => void;
  onChangeClipBeatValue: (value: number) => void;
  onChangeClipMin: (value: number) => void;
  onChangeClipMax: (value: number) => void;
}) {
  return (
    <Card className="panel clip-editor-panel">
      {selectedClip ? (
        <ShapeEditorPanel
          onCommitShape={onCommitShape}
          onSelectPoint={onSelectPoint}
          selectedPointIndex={selectedPointIndex}
          shape={selectedShape}
          sidebarTop={
            <ClipMetadataForm
              selectedClip={selectedClip}
              selectedClipBeatOption={selectedClipBeatOption}
              selectedClipBeatValue={selectedClipBeatValue}
              clipLengthOptions={clipLengthOptions}
              onChangeName={onChangeClipName}
              onChangeBeatValue={onChangeClipBeatValue}
              onChangeMin={onChangeClipMin}
              onChangeMax={onChangeClipMax}
            />
          }
        />
      ) : (
        <CardContent className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border/80 bg-muted/20 p-6 text-sm text-muted-foreground">
          Select or create a clip.
        </CardContent>
      )}
    </Card>
  );
}
