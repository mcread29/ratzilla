import { ChromaticBulgeGridShaderState, LaneId } from "../../types";
import { isColorLane } from "../../vfx";
import { colorToHex, clampColorChannel, getBaseColor, hexToColor } from "../utils/color";

function ColorEditor({
  label,
  value,
  onChange,
}: {
  label: string;
  value: [number, number, number];
  onChange: (value: [number, number, number]) => void;
}) {
  return (
    <div className="color-editor">
      <label>
        {label}
        <input
          className="color-picker"
          type="color"
          value={colorToHex(value)}
          onChange={(event) => onChange(hexToColor(event.target.value))}
        />
      </label>
      <div className="color-channel-grid">
        {(["R", "G", "B"] as const).map((channel, index) => (
          <label key={channel}>
            {channel}
            <input
              type="number"
              min={0}
              max={1}
              step={0.01}
              value={value[index]}
              onChange={(event) => {
                const next = [...value] as [number, number, number];
                next[index] = clampColorChannel(Number(event.target.value));
                onChange(next);
              }}
            />
          </label>
        ))}
      </div>
    </div>
  );
}

export function PropertySidebarPanel({
  selectedLane,
  selectedLaneMeta,
  baseState,
  onChangeBaseValue,
}: {
  selectedLane: LaneId;
  selectedLaneMeta: { label: string; description: string };
  baseState: ChromaticBulgeGridShaderState | undefined;
  onChangeBaseValue: (lane: LaneId, value: number | [number, number, number]) => void;
}) {
  return (
    <section className="panel property-sidebar-panel">
      <div className="property-sidebar-header">
        <h2>{selectedLaneMeta.label}</h2>
        <code className="property-variable-name">{selectedLane}</code>
      </div>
      <p className="empty-copy">{selectedLaneMeta.description}</p>
      <div className="base-panel">
        {baseState ? (
          isColorLane(selectedLane) ? (
            <ColorEditor
              label="Base Value"
              value={getBaseColor(baseState, selectedLane as "cold_color" | "hot_color")}
              onChange={(nextColor) => onChangeBaseValue(selectedLane, nextColor)}
            />
          ) : (
            <label>
              Base Value
              <input
                type="number"
                step={0.01}
                value={Number(baseState[selectedLane])}
                onChange={(event) => onChangeBaseValue(selectedLane, Number(event.target.value))}
              />
            </label>
          )
        ) : null}
      </div>
    </section>
  );
}
