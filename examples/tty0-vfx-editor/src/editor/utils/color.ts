import { ChromaticBulgeGridShaderState, LaneId } from "../../types";

export function getBaseColor(
  state: ChromaticBulgeGridShaderState,
  lane: Extract<LaneId, "cold_color" | "hot_color">,
): [number, number, number] {
  return Array.from(state[lane]) as [number, number, number];
}

export function colorToHex(value: [number, number, number]): string {
  return `#${value
    .map((channel) => Math.round(clampColorChannel(channel) * 255).toString(16).padStart(2, "0"))
    .join("")}`;
}

export function hexToColor(value: string): [number, number, number] {
  const normalized = value.replace("#", "");
  if (normalized.length !== 6) {
    return [1, 1, 1];
  }
  return [
    parseInt(normalized.slice(0, 2), 16) / 255,
    parseInt(normalized.slice(2, 4), 16) / 255,
    parseInt(normalized.slice(4, 6), 16) / 255,
  ];
}

export function clampColorChannel(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
}

export function placementBackground(color: [number, number, number], selected: boolean): string {
  const fill = selected ? mixColor(color, [0.96, 0.73, 0.26], 0.42) : color;
  return `rgb(${fill.map((channel) => Math.round(clampColorChannel(channel) * 255)).join(" ")})`;
}

export function mixColor(
  left: [number, number, number],
  right: [number, number, number],
  amount: number,
): [number, number, number] {
  const t = Math.max(0, Math.min(1, amount));
  return [
    left[0] + (right[0] - left[0]) * t,
    left[1] + (right[1] - left[1]) * t,
    left[2] + (right[2] - left[2]) * t,
  ];
}
