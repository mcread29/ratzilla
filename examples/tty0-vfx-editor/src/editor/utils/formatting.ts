import { ChromaticBulgeGridClip, TrackVisualizerConfig } from "../../types";
import { isColorLane } from "../../vfx";
import { colorToHex } from "./color";

export function formatPlacementLabel(name: string, widthPx: number): string {
  if (widthPx < 30) {
    return "•";
  }
  if (widthPx < 76) {
    const initials = name
      .split(/\s+/)
      .filter(Boolean)
      .map((part) => part[0])
      .join("")
      .slice(0, 3)
      .toUpperCase();
    return initials || name.slice(0, 2).toUpperCase();
  }
  return name;
}

export function jsonFilename(name: string | null | undefined): string {
  const trimmed = (name ?? "").trim();
  if (!trimmed) {
    return "tty0-visualizer.json";
  }
  return trimmed.toLowerCase().endsWith(".json") ? trimmed : `${trimmed}.json`;
}

export function serializeVisualizer(config: TrackVisualizerConfig): string {
  return JSON.stringify(config);
}

export function clipListRangeLabel(clip: ChromaticBulgeGridClip): string {
  const source = clip.source?.kind === "lfo" ? clip.source : null;
  if (!source) {
    return "Legacy step clip";
  }
  if (isColorLane(source.lane)) {
    const colorSource = source as Extract<typeof source, { lane: "cold_color" | "hot_color" }>;
    return `${colorToHex(colorSource.start)}->${colorToHex(colorSource.end)}`;
  }
  const floatSource = source as Extract<typeof source, { start: number; end: number }>;
  return `${floatSource.start.toFixed(2)}-${floatSource.end.toFixed(2)}`;
}

export function formatClipBarLength(lengthBeats: number, beatsPerMeasure: number): string {
  const bars = lengthBeats / Math.max(1, beatsPerMeasure);
  const wholeBars = Math.round(bars);
  if (Math.abs(bars - wholeBars) < 0.0001) {
    return `${wholeBars}`;
  }

  for (const denominator of [2, 4, 8, 16]) {
    const numerator = Math.round(bars * denominator);
    if (Math.abs(bars - numerator / denominator) < 0.0001) {
      return `${numerator}/${denominator}`;
    }
  }

  return bars.toFixed(2);
}

export function formatMeasurePosition(currentBeat: number, beatsPerMeasure: number): string {
  const safeBeatsPerMeasure = Math.max(1, Math.round(beatsPerMeasure));
  const unitsPerBeat = 1;
  const totalUnits = Math.max(0, Math.round(currentBeat * unitsPerBeat));
  const unitsPerMeasure = safeBeatsPerMeasure * unitsPerBeat;
  const wholeMeasures = Math.floor(totalUnits / unitsPerMeasure);
  const remainderUnits = totalUnits % unitsPerMeasure;
  const displayedMeasure = wholeMeasures + 1;
  const displayedQuarter = remainderUnits + 1;
  return `${displayedMeasure} ${displayedQuarter}/${unitsPerMeasure}`;
}

export function formatDuration(seconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const remainderSeconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${remainderSeconds.toString().padStart(2, "0")}`;
  }

  return `${minutes}:${remainderSeconds.toString().padStart(2, "0")}`;
}
