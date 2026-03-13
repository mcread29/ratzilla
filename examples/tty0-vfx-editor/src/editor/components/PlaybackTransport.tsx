import { usePlaybackDisplayTime } from "../hooks/usePlaybackDisplayTime";
import { formatDuration, formatMeasurePosition } from "../utils/formatting";
import { TransportIcon } from "./icons";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function PlaybackTransport({
  audioRef,
  beatsPerMeasure,
  dirty,
  effectiveAudioUrl,
  isPlaying,
  leadInBars,
  onSeekToTime,
  onStop,
  onTogglePlayback,
  playPending,
  playbackTimeRef,
  timelineBpm,
  totalDurationSeconds,
}: {
  audioRef: { current: HTMLAudioElement | null };
  beatsPerMeasure: number;
  dirty: boolean;
  effectiveAudioUrl: string | null;
  isPlaying: boolean;
  leadInBars: number;
  onSeekToTime: (time: number) => void;
  onStop: () => void;
  onTogglePlayback: () => void;
  playPending: boolean;
  playbackTimeRef: { current: number };
  timelineBpm: number;
  totalDurationSeconds: number;
}) {
  const [displayTime, setDisplayTime] = usePlaybackDisplayTime(audioRef, playbackTimeRef, isPlaying);
  const displayBeat = Math.max(0, displayTime) * timelineBpm / 60;

  return (
    <div className="timeline-footer">
      <input
        className="timeline-scrubber"
        type="range"
        min={0}
        max={Math.max(totalDurationSeconds, 0.01)}
        step={0.01}
        value={Math.min(displayTime, totalDurationSeconds)}
        onChange={(event) => {
          const nextTime = Number(event.target.value);
          setDisplayTime(nextTime);
          onSeekToTime(nextTime);
        }}
      />
      <div className="timeline-transport">
        <div className="transport-actions">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                className="icon-button"
                onClick={onTogglePlayback}
                disabled={!effectiveAudioUrl}
                type="button"
                variant="toolbar"
                size="icon"
                aria-label={playPending ? "Preparing playback" : isPlaying ? "Pause playback" : "Play playback"}
              >
                <TransportIcon name={playPending ? "loading" : isPlaying ? "pause" : "play"} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{playPending ? "Preparing playback" : isPlaying ? "Pause playback" : "Play playback"}</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                className="icon-button"
                onClick={() => {
                  setDisplayTime(0);
                  onStop();
                }}
                disabled={!effectiveAudioUrl}
                type="button"
                variant="toolbar"
                size="icon"
                aria-label="Stop playback"
              >
                <TransportIcon name="stop" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Stop playback</TooltipContent>
          </Tooltip>
        </div>
        <div className="transport-stat">
          <span>Measure</span>
          <strong>{formatMeasurePosition(displayBeat, beatsPerMeasure)}</strong>
        </div>
        <div className="transport-stat">
          <span>Time</span>
          <strong>
            {formatDuration(displayTime)} / {formatDuration(totalDurationSeconds)}
          </strong>
        </div>
        <div className="transport-stat">
          <span>Lead-In</span>
          <strong>{leadInBars} bar{leadInBars === 1 ? "" : "s"}</strong>
        </div>
        <div className="transport-stat">
          <span>State</span>
          <strong>{dirty ? "Unsaved" : "Saved"}</strong>
        </div>
      </div>
    </div>
  );
}
