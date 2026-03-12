import { TrackVisualizerConfig } from "../../types";
import { buildTimelineIndex } from "../../vfx";
import { PreviewCanvas } from "./PreviewCanvas";

export function PreviewPanel({
  config,
  audioRef,
  playbackTimeRef,
  isPlaying,
  onReady,
  timelineIndex,
}: {
  config: TrackVisualizerConfig;
  audioRef: { current: HTMLAudioElement | null };
  playbackTimeRef: { current: number };
  isPlaying: boolean;
  onReady: () => void;
  timelineIndex: ReturnType<typeof buildTimelineIndex>;
}) {
  return (
    <aside className="preview-column">
      <section className="panel preview-panel">
        <PreviewCanvas
          config={config}
          audioRef={audioRef}
          playbackTimeRef={playbackTimeRef}
          isPlaying={isPlaying}
          onReady={onReady}
          timelineIndex={timelineIndex}
        />
      </section>
    </aside>
  );
}
