import { ChromaticBulgeGridClip, LfoPoint } from "../../types";
import { ClipMetadataForm } from "./ClipMetadataForm";
import { ShapeEditorPanel } from "./ShapeEditorPanel";

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
    <section className="panel clip-editor-panel">
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
        <p>Select or create a clip.</p>
      )}
    </section>
  );
}
