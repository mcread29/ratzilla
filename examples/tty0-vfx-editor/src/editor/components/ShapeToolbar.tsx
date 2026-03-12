import { ShapeInteractionMode } from "../editor-types";
import { ShapeModeIcon } from "./icons";

export function ShapeToolbar({
  interactionMode,
  onChangeMode,
}: {
  interactionMode: ShapeInteractionMode;
  onChangeMode: (mode: ShapeInteractionMode) => void;
}) {
  return (
    <div className="lfo-shape-toolbar">
      <div className="tool-toggle" role="group" aria-label="LFO edit mode">
        {(["add", "move", "delete"] as const).map((mode) => (
          <button
            key={mode}
            className={interactionMode === mode ? "active" : ""}
            onClick={() => onChangeMode(mode)}
            type="button"
            title={`${mode[0].toUpperCase()}${mode.slice(1)} mode`}
            aria-label={`${mode[0].toUpperCase()}${mode.slice(1)} mode`}
            aria-pressed={interactionMode === mode}
          >
            <ShapeModeIcon name={mode} />
          </button>
        ))}
      </div>
    </div>
  );
}
