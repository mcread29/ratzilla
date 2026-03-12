import { ChromaticBulgeGridClip } from "../../types";
import { isLegacyClip } from "../../vfx";
import { ClipFieldIcon } from "./icons";

export function ClipMetadataForm({
  selectedClip,
  selectedClipBeatOption,
  selectedClipBeatValue,
  clipLengthOptions,
  onChangeName,
  onChangeBeatValue,
  onChangeMin,
  onChangeMax,
}: {
  selectedClip: ChromaticBulgeGridClip;
  selectedClipBeatOption: string;
  selectedClipBeatValue: number;
  clipLengthOptions: Array<{ label: string; beats: number }>;
  onChangeName: (value: string) => void;
  onChangeBeatValue: (value: number) => void;
  onChangeMin: (value: number) => void;
  onChangeMax: (value: number) => void;
}) {
  return (
    <div className="step-list lfo-clip-fields">
      <div className="clip-field-grid">
        <label className="clip-input-field field-span-2" title="Clip name">
          <span className="compact-field-icon clip-input-icon" aria-hidden="true">
            <ClipFieldIcon name="name" />
          </span>
          <input aria-label="Clip name" value={selectedClip.name} onChange={(event) => onChangeName(event.target.value)} />
        </label>
        <label className="clip-input-field field-span-2" title="Length / period">
          <span className="compact-field-icon clip-input-icon" aria-hidden="true">
            <ClipFieldIcon name="length" />
          </span>
          <select
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
          </select>
        </label>
        {isLegacyClip(selectedClip) ? (
          <p className="empty-copy field-span-2">
            Legacy step clip detected. This layout preserves playback, but LFO editing is only available for LFO clips.
          </p>
        ) : (
          <>
            <label className="clip-input-field" title="Minimum value">
              <span className="compact-field-icon clip-input-icon" aria-hidden="true">
                <ClipFieldIcon name="min" />
              </span>
              <input
                aria-label="Minimum value"
                type="number"
                step={0.01}
                value={selectedClip.source?.min ?? 0}
                onChange={(event) => onChangeMin(Number(event.target.value))}
              />
            </label>
            <label className="clip-input-field" title="Maximum value">
              <span className="compact-field-icon clip-input-icon" aria-hidden="true">
                <ClipFieldIcon name="max" />
              </span>
              <input
                aria-label="Maximum value"
                type="number"
                step={0.01}
                value={selectedClip.source?.max ?? 0}
                onChange={(event) => onChangeMax(Number(event.target.value))}
              />
            </label>
          </>
        )}
      </div>
    </div>
  );
}
