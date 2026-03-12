import { TrackVisualizerConfig } from "../../types";
import { buildTimelineIndex } from "../../vfx";
import { PreviewCanvas } from "./PreviewCanvas";
import { cn } from "@/lib/utils";

export function PreviewPanel({
  config,
  audioRef,
  playbackTimeRef,
  isPlaying,
  onReady,
  timelineIndex,
  className,
  panelClassName,
}: {
  config: TrackVisualizerConfig;
  audioRef: { current: HTMLAudioElement | null };
  playbackTimeRef: { current: number };
  isPlaying: boolean;
  onReady: () => void;
  timelineIndex: ReturnType<typeof buildTimelineIndex>;
  className?: string;
  panelClassName?: string;
}) {
  return (
    <aside className={cn("preview-column", className)}>
      <section className={cn("panel preview-panel", panelClassName)}>
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
