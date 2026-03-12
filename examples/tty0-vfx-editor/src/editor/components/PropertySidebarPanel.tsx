import { ChromaticBulgeGridShaderState, LaneId } from "../../types";
import { isColorLane } from "../../vfx";
import { colorToHex, clampColorChannel, getBaseColor, hexToColor } from "../utils/color";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

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
      <Label>
        {label}
        <Input
          className="color-picker"
          type="color"
          value={colorToHex(value)}
          onChange={(event) => onChange(hexToColor(event.target.value))}
        />
      </Label>
      <div className="color-channel-grid">
        {(["R", "G", "B"] as const).map((channel, index) => (
          <Label key={channel}>
            {channel}
            <Input
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
          </Label>
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
    <Card className="panel property-sidebar-panel">
      <CardHeader className="p-0">
        <div className="property-sidebar-header">
          <CardTitle className="text-base">{selectedLaneMeta.label}</CardTitle>
          <Badge variant="amber" className="property-variable-name font-mono normal-case tracking-[0.04em]">
            {selectedLane}
          </Badge>
        </div>
        <CardDescription>{selectedLaneMeta.description}</CardDescription>
      </CardHeader>
      <CardContent className="base-panel p-0">
        {baseState ? (
          isColorLane(selectedLane) ? (
            <ColorEditor
              label="Base Value"
              value={getBaseColor(baseState, selectedLane as "cold_color" | "hot_color")}
              onChange={(nextColor) => onChangeBaseValue(selectedLane, nextColor)}
            />
          ) : (
            <Label>
              Base Value
              <Input
                type="number"
                step={0.01}
                value={Number(baseState[selectedLane])}
                onChange={(event) => onChangeBaseValue(selectedLane, Number(event.target.value))}
              />
            </Label>
          )
        ) : null}
      </CardContent>
    </Card>
  );
}
