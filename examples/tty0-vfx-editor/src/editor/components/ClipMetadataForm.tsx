import {
  ColorValue,
  ChromaticBulgeGridClip,
  ChromaticBulgeGridColorLfoClip,
  ChromaticBulgeGridFloatLfoClip,
} from "../../types";
import { isColorLane, isLegacyClip } from "../../vfx";
import { colorToHex, hexToColor } from "../utils/color";
import { ClipFieldIcon } from "./icons";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { NativeSelect } from "@/components/ui/native-select";

export function ClipMetadataForm({
  selectedClip,
  selectedClipBeatOption,
  selectedClipBeatValue,
  clipLengthOptions,
  onChangeName,
  onChangeBeatValue,
  onChangeStartValue,
  onChangeEndValue,
  onChangeHoldAfter,
}: {
  selectedClip: ChromaticBulgeGridClip;
  selectedClipBeatOption: string;
  selectedClipBeatValue: number;
  clipLengthOptions: Array<{ label: string; beats: number }>;
  onChangeName: (value: string) => void;
  onChangeBeatValue: (value: number) => void;
  onChangeStartValue: (value: number | ColorValue) => void;
  onChangeEndValue: (value: number | ColorValue) => void;
  onChangeHoldAfter: (value: boolean) => void;
}) {
  const source = selectedClip.source?.kind === "lfo" ? selectedClip.source : null;
  const colorSource: ChromaticBulgeGridColorLfoClip | null =
    source && isColorLane(source.lane) ? (source as ChromaticBulgeGridColorLfoClip) : null;
  const floatSource: ChromaticBulgeGridFloatLfoClip | null =
    source && !isColorLane(source.lane) ? (source as ChromaticBulgeGridFloatLfoClip) : null;

  return (
    <div className="step-list lfo-clip-fields">
      <div className="clip-field-grid">
        <Label className="clip-input-field field-span-2" title="Clip name">
          <span className="compact-field-icon clip-input-icon" aria-hidden="true">
            <ClipFieldIcon name="name" />
          </span>
          <Input aria-label="Clip name" value={selectedClip.name} onChange={(event) => onChangeName(event.target.value)} />
        </Label>
        <Label className="clip-input-field field-span-2" title="Length / period">
          <span className="compact-field-icon clip-input-icon" aria-hidden="true">
            <ClipFieldIcon name="length" />
          </span>
          <NativeSelect
            aria-label="Length / period"
            value={selectedClipBeatOption}
            onChange={(event) => onChangeBeatValue(Number(event.target.value))}
          >
            {selectedClipBeatOption ? null : (
              <option value="" disabled>
                {`${selectedClipBeatValue.toFixed(2)} beats (custom)`}
              </option>
            )}
            {clipLengthOptions.map((option) => (
              <option key={option.label} value={option.beats}>
                {option.label}
              </option>
            ))}
          </NativeSelect>
        </Label>
        {isLegacyClip(selectedClip) ? (
          <p className="field-span-2 rounded-lg border border-editor-amber/30 bg-editor-amber/8 px-3 py-2 text-sm text-muted-foreground">
            Legacy step clip detected. This layout preserves playback, but LFO editing is only available for LFO clips.
          </p>
        ) : (
          <>
            <Label className="clip-input-field" title="Start value">
              <span className="compact-field-icon clip-input-icon" aria-hidden="true">
                <ClipFieldIcon name="min" />
              </span>
              {colorSource ? (
                <Input
                  aria-label="Start value"
                  className="color-picker"
                  type="color"
                  value={colorToHex(colorSource.start)}
                  onChange={(event) => onChangeStartValue(hexToColor(event.target.value))}
                />
              ) : (
                <Input
                  aria-label="Start value"
                  className="no-spinner-input"
                  type="number"
                  step={0.01}
                  value={floatSource?.start ?? 0}
                  onChange={(event) => onChangeStartValue(Number(event.target.value))}
                />
              )}
            </Label>
            <Label className="clip-input-field" title="End value">
              <span className="compact-field-icon clip-input-icon" aria-hidden="true">
                <ClipFieldIcon name="max" />
              </span>
              {colorSource ? (
                <Input
                  aria-label="End value"
                  className="color-picker"
                  type="color"
                  value={colorToHex(colorSource.end)}
                  onChange={(event) => onChangeEndValue(hexToColor(event.target.value))}
                />
              ) : (
                <Input
                  aria-label="End value"
                  className="no-spinner-input"
                  type="number"
                  step={0.01}
                  value={floatSource?.end ?? 0}
                  onChange={(event) => onChangeEndValue(Number(event.target.value))}
                />
              )}
            </Label>
            <label className="clip-toggle-field field-span-2">
              <input
                aria-label="Hold value after clip ends"
                checked={selectedClip.hold_after === true}
                onChange={(event) => onChangeHoldAfter(event.target.checked)}
                type="checkbox"
              />
              <span>Hold value after end</span>
            </label>
          </>
        )}
      </div>
    </div>
  );
}
