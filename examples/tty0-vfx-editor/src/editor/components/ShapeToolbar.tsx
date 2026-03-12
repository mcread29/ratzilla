import { ShapeInteractionMode } from "../editor-types";
import { ShapeModeIcon } from "./icons";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

export function ShapeToolbar({
  interactionMode,
  onChangeMode,
}: {
  interactionMode: ShapeInteractionMode;
  onChangeMode: (mode: ShapeInteractionMode) => void;
}) {
  return (
    <div className="lfo-shape-toolbar">
      <ToggleGroup
        type="single"
        value={interactionMode}
        onValueChange={(value) => {
          if (value === "add" || value === "move" || value === "delete") {
            onChangeMode(value);
          }
        }}
        aria-label="LFO edit mode"
      >
        {(["add", "move", "delete"] as const).map((mode) => (
          <ToggleGroupItem
            key={mode}
            aria-label={`${mode[0].toUpperCase()}${mode.slice(1)} mode`}
            value={mode}
          >
            <ShapeModeIcon name={mode} />
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  );
}
