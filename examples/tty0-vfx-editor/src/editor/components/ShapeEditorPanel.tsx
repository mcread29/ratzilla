import { ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { LfoPoint } from "../../types";
import {
  curveFromControlValue,
  normalizeClipLfoShape,
  phaseToEditorX,
  shapePath,
  shapeSegments,
  valueToEditorY,
} from "../../vfx";
import {
  LFO_POINT_SNAP,
  SHAPE_EDITOR_BOUND_INSET,
  SHAPE_EDITOR_HANDLE_INSET,
  SHAPE_EDITOR_HEIGHT,
  SHAPE_EDITOR_VERTICAL_PADDING,
  SHAPE_EDITOR_WIDTH,
} from "../constants";
import { ShapeInteractionMode } from "../editor-types";
import { ShapePointReadout } from "./ShapePointReadout";
import { ShapeToolbar } from "./ShapeToolbar";

function shapeEditorY(value: number): number {
  return valueToEditorY(value, SHAPE_EDITOR_HEIGHT, SHAPE_EDITOR_VERTICAL_PADDING + SHAPE_EDITOR_HANDLE_INSET);
}

function shapeEditorX(phase: number): number {
  return phaseToEditorX(phase, SHAPE_EDITOR_WIDTH, SHAPE_EDITOR_BOUND_INSET + SHAPE_EDITOR_HANDLE_INSET);
}

function snapLfoPointCoordinate(value: number): number {
  return Math.max(0, Math.min(1, Math.round(value / LFO_POINT_SNAP) * LFO_POINT_SNAP));
}

function editorXToPhase(editorX: number): number {
  const padding = SHAPE_EDITOR_BOUND_INSET + SHAPE_EDITOR_HANDLE_INSET;
  const usableWidth = Math.max(1, SHAPE_EDITOR_WIDTH - padding * 2);
  return Math.max(0, Math.min(1, (editorX - padding) / usableWidth));
}

function editorYToValue(editorY: number): number {
  const padding = SHAPE_EDITOR_VERTICAL_PADDING + SHAPE_EDITOR_HANDLE_INSET;
  const usableHeight = Math.max(1, SHAPE_EDITOR_HEIGHT - padding * 2);
  return Math.max(0, Math.min(1, 1 - (editorY - padding) / usableHeight));
}

function pointerToPoint(svg: SVGSVGElement, clientX: number, clientY: number): LfoPoint {
  const svgPoint = svg.createSVGPoint();
  svgPoint.x = clientX;
  svgPoint.y = clientY;
  const inverse = svg.getScreenCTM()?.inverse();
  if (!inverse) {
    return { phase: 0, value: 0 };
  }
  const local = svgPoint.matrixTransform(inverse);
  return {
    phase: snapLfoPointCoordinate(editorXToPhase(local.x)),
    value: snapLfoPointCoordinate(editorYToValue(local.y)),
  };
}

export function ShapeEditorPanel({
  onCommitShape,
  onSelectPoint,
  selectedPointIndex,
  sidebarTop,
  shape,
}: {
  onCommitShape: (points: LfoPoint[]) => void;
  onSelectPoint: (index: number | null) => void;
  selectedPointIndex: number | null;
  sidebarTop?: ReactNode;
  shape: { interpolation: "linear"; points: LfoPoint[] } | null;
}) {
  const shapeSvgRef = useRef<SVGSVGElement | null>(null);
  const shapeInspectorRef = useRef<HTMLDivElement | null>(null);
  const draftPointsRef = useRef<LfoPoint[]>(shape?.points ?? []);
  const [draftPoints, setDraftPoints] = useState<LfoPoint[]>(shape?.points ?? []);
  const [interactionMode, setInteractionMode] = useState<ShapeInteractionMode>("add");
  const [shapeDragIndex, setShapeDragIndex] = useState<number | null>(null);
  const [curveDragIndex, setCurveDragIndex] = useState<number | null>(null);
  const [selectedSegmentIndex, setSelectedSegmentIndex] = useState<number | null>(null);
  const [shapeViewport, setShapeViewport] = useState({ width: SHAPE_EDITOR_WIDTH, height: SHAPE_EDITOR_HEIGHT });

  useEffect(() => {
    const nextPoints = shape?.points ?? [];
    draftPointsRef.current = nextPoints;
    setDraftPoints(nextPoints);
    setShapeDragIndex(null);
    setCurveDragIndex(null);
    setSelectedSegmentIndex(null);
  }, [shape]);

  useEffect(() => {
    const element = shapeInspectorRef.current;
    if (!element) return;

    const aspectRatio = SHAPE_EDITOR_WIDTH / SHAPE_EDITOR_HEIGHT;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;

      const availableWidth = Math.max(1, entry.contentRect.width);
      const availableHeight = Math.max(1, entry.contentRect.height);
      const fittedHeight = Math.min(availableHeight, availableWidth / aspectRatio);
      const fittedWidth = Math.min(availableWidth, fittedHeight * aspectRatio);

      setShapeViewport({
        width: Math.max(1, Math.floor(fittedWidth)),
        height: Math.max(1, Math.floor(fittedHeight)),
      });
    });

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const normalizedShape = useMemo(
    () => (shape ? normalizeClipLfoShape({ interpolation: shape.interpolation, points: draftPoints }) : null),
    [draftPoints, shape],
  );
  const segments = useMemo(
    () =>
      normalizedShape
        ? shapeSegments(
            normalizedShape,
            SHAPE_EDITOR_WIDTH,
            SHAPE_EDITOR_HEIGHT,
            SHAPE_EDITOR_VERTICAL_PADDING + SHAPE_EDITOR_HANDLE_INSET,
            SHAPE_EDITOR_BOUND_INSET + SHAPE_EDITOR_HANDLE_INSET,
          )
        : [],
    [normalizedShape],
  );
  const selectedPoint =
    normalizedShape && selectedPointIndex != null ? normalizedShape.points[selectedPointIndex] ?? null : null;

  function commit(points: LfoPoint[]) {
    const normalized = normalizeClipLfoShape({ interpolation: "linear", points });
    draftPointsRef.current = normalized.points;
    setDraftPoints(normalized.points);
    onCommitShape(normalized.points);
  }

  function updateDraftPoints(mutator: (current: LfoPoint[]) => LfoPoint[]) {
    setDraftPoints((current) => {
      const next = mutator(current);
      draftPointsRef.current = next;
      return next;
    });
  }

  function setCurve(point: LfoPoint, curveToNext: number): LfoPoint {
    if (Math.abs(curveToNext) < 0.0001) {
      return { phase: point.phase, value: point.value };
    }
    return { ...point, curve_to_next: Math.max(-1, Math.min(1, curveToNext)) };
  }

  function finalizeDrag() {
    if (shapeDragIndex != null || curveDragIndex != null) {
      commit(draftPointsRef.current);
    }
    setShapeDragIndex(null);
    setCurveDragIndex(null);
  }

  function pointIndex(points: LfoPoint[], candidate: LfoPoint): number {
    return points.findIndex(
      (point) => Math.abs(point.phase - candidate.phase) < 0.0001 && Math.abs(point.value - candidate.value) < 0.0001,
    );
  }

  function deletePoint(index: number) {
    if (!normalizedShape || normalizedShape.points.length <= 1) {
      return;
    }
    const nextPoints = normalizedShape.points.filter((_, pointIndex) => pointIndex !== index);
    updateDraftPoints(() => nextPoints);
    onSelectPoint(nextPoints.length ? Math.min(index, nextPoints.length - 1) : null);
    setSelectedSegmentIndex(null);
    commit(nextPoints);
  }

  return (
    <div className="clip-editor">
      <div className="clip-editor-sidebar">
        {sidebarTop}
        <div className="clip-editor-divider" />
        <div className="shape-point-column">
          {normalizedShape ? (
            <>
              <ShapeToolbar
                interactionMode={interactionMode}
                onChangeMode={(mode) => {
                  finalizeDrag();
                  setInteractionMode(mode);
                }}
              />
              <ShapePointReadout selectedPoint={selectedPoint} selectedPointIndex={selectedPointIndex} />
            </>
          ) : (
            <p className="empty-copy">Select a shape to edit.</p>
          )}
        </div>
      </div>
      <div ref={shapeInspectorRef} className="step-inspector lfo-shape-inspector">
        {normalizedShape ? (
          <div
            className="shape-editor-frame"
            style={{
              width: `${shapeViewport.width}px`,
              height: `${shapeViewport.height}px`,
            }}
          >
            <svg
              ref={shapeSvgRef}
              className={`shape-editor shape-editor-${interactionMode}`}
              viewBox={`0 0 ${SHAPE_EDITOR_WIDTH} ${SHAPE_EDITOR_HEIGHT}`}
              preserveAspectRatio="xMidYMid meet"
              onPointerDown={(event) => {
                const svg = shapeSvgRef.current;
                if (!svg) return;
                const role = event.target instanceof SVGElement ? event.target.dataset.shapeRole : null;
                if (role || interactionMode !== "add") {
                  return;
                }
                event.currentTarget.setPointerCapture(event.pointerId);
                const point = pointerToPoint(svg, event.clientX, event.clientY);
                const nextPoints = [...normalizedShape.points];
                let insertionIndex = nextPoints.findIndex((candidate) => point.phase < candidate.phase);
                if (insertionIndex < 0) insertionIndex = nextPoints.length;
                nextPoints.splice(insertionIndex, 0, point);
                if (insertionIndex > 0 && insertionIndex < nextPoints.length - 1) {
                  nextPoints[insertionIndex - 1] = setCurve(nextPoints[insertionIndex - 1], 0);
                }
                const normalized = normalizeClipLfoShape({ interpolation: "linear", points: nextPoints }).points;
                updateDraftPoints(() => normalized);
                const nextIndex = pointIndex(normalized, point);
                onSelectPoint(nextIndex >= 0 ? nextIndex : null);
                setSelectedSegmentIndex(null);
                commit(normalized);
              }}
              onPointerMove={(event) => {
                const svg = shapeSvgRef.current;
                if (!svg) return;
                if (shapeDragIndex != null) {
                  const point = pointerToPoint(svg, event.clientX, event.clientY);
                  updateDraftPoints((current) =>
                    current.map((entry, index) =>
                      index === shapeDragIndex ? { ...point, curve_to_next: entry.curve_to_next } : entry,
                    ),
                  );
                  return;
                }
                if (curveDragIndex == null) return;
                const controlValue = pointerToPoint(svg, event.clientX, event.clientY).value;
                updateDraftPoints((current) => {
                  const next = normalizeClipLfoShape({ interpolation: "linear", points: current }).points;
                  const left = next[curveDragIndex];
                  const right = next[curveDragIndex + 1];
                  if (!left || !right) return next;
                  next[curveDragIndex] = setCurve(left, curveFromControlValue(left, right, controlValue));
                  return next;
                });
              }}
              onPointerUp={(event) => {
                if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                  event.currentTarget.releasePointerCapture(event.pointerId);
                }
                finalizeDrag();
              }}
              onPointerCancel={finalizeDrag}
              onLostPointerCapture={finalizeDrag}
            >
              <rect x="0" y="0" width={SHAPE_EDITOR_WIDTH} height={SHAPE_EDITOR_HEIGHT} />
              <path
                className="shape-grid shape-grid-bound"
                d={`M ${SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_VERTICAL_PADDING} L ${SHAPE_EDITOR_WIDTH - SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_VERTICAL_PADDING}`}
              />
              <path
                className="shape-grid shape-grid-bound"
                d={`M ${SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING} L ${SHAPE_EDITOR_WIDTH - SHAPE_EDITOR_BOUND_INSET} ${SHAPE_EDITOR_HEIGHT - SHAPE_EDITOR_VERTICAL_PADDING}`}
              />
              <path className="shape-grid" d={`M 0 ${SHAPE_EDITOR_HEIGHT / 2} L ${SHAPE_EDITOR_WIDTH} ${SHAPE_EDITOR_HEIGHT / 2}`} />
              <path className="shape-grid" d={`M ${SHAPE_EDITOR_WIDTH / 2} 0 L ${SHAPE_EDITOR_WIDTH / 2} ${SHAPE_EDITOR_HEIGHT}`} />
              <path
                className="shape-curve"
                d={shapePath(
                  normalizedShape,
                  SHAPE_EDITOR_WIDTH,
                  SHAPE_EDITOR_HEIGHT,
                  SHAPE_EDITOR_VERTICAL_PADDING + SHAPE_EDITOR_HANDLE_INSET,
                  SHAPE_EDITOR_BOUND_INSET + SHAPE_EDITOR_HANDLE_INSET,
                )}
              />
              {segments.map((segment) => {
                const active = selectedSegmentIndex === segment.index || curveDragIndex === segment.index;
                return (
                  <g key={`segment-${segment.index}`}>
                    {active ? (
                      <line
                        className="shape-curve-guide"
                        x1={segment.midpointX}
                        y1={segment.midpointY}
                        x2={segment.controlX}
                        y2={segment.controlY}
                      />
                    ) : null}
                    <circle
                      data-shape-role="curve-handle"
                      className={active ? "shape-curve-handle selected" : "shape-curve-handle"}
                      cx={segment.controlX}
                      cy={segment.controlY}
                      r={4}
                      onPointerDown={(event) => {
                        if (interactionMode !== "move") {
                          return;
                        }
                        event.stopPropagation();
                        event.currentTarget.setPointerCapture(event.pointerId);
                        setSelectedSegmentIndex(segment.index);
                        setCurveDragIndex(segment.index);
                      }}
                      onDoubleClick={(event) => {
                        if (interactionMode !== "move") {
                          return;
                        }
                        event.stopPropagation();
                        const next = normalizedShape.points.map((point, index) =>
                          index === segment.index ? setCurve(point, 0) : point,
                        );
                        updateDraftPoints(() => next);
                        setSelectedSegmentIndex(segment.index);
                        commit(next);
                      }}
                    />
                  </g>
                );
              })}
              {normalizedShape.points.map((point, index) => (
                <circle
                  key={`${index}-${point.phase}-${point.value}`}
                  data-shape-role="point"
                  className={selectedPointIndex === index ? "shape-point selected" : "shape-point"}
                  cx={shapeEditorX(point.phase)}
                  cy={shapeEditorY(point.value)}
                  r={5}
                  onPointerDown={(event) => {
                    event.stopPropagation();
                    if (interactionMode === "delete") {
                      deletePoint(index);
                      return;
                    }
                    if (interactionMode !== "add" && interactionMode !== "move") {
                      return;
                    }
                    event.currentTarget.setPointerCapture(event.pointerId);
                    onSelectPoint(index);
                    setSelectedSegmentIndex(null);
                    setShapeDragIndex(index);
                  }}
                />
              ))}
            </svg>
          </div>
        ) : null}
      </div>
    </div>
  );
}
