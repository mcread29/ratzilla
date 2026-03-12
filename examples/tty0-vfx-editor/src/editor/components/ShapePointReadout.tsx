import { LfoPoint } from "../../types";

export function ShapePointReadout({
  selectedPoint,
  selectedPointIndex,
}: {
  selectedPoint: LfoPoint | null;
  selectedPointIndex: number | null;
}) {
  return (
    <div className="shape-point-readout">
      {selectedPoint && selectedPointIndex != null ? (
        <>
          <strong>{`Point ${selectedPointIndex + 1}`}</strong>
          <span>{`x ${selectedPoint.phase.toFixed(3)}`}</span>
          <span>{`y ${selectedPoint.value.toFixed(3)}`}</span>
        </>
      ) : (
        <span className="empty-copy">No point selected.</span>
      )}
    </div>
  );
}
