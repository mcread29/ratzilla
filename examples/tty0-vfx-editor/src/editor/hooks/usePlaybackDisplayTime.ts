import { useEffect, useState } from "react";

export function usePlaybackDisplayTime(
  audioRef: { current: HTMLAudioElement | null },
  playbackTimeRef: { current: number },
  isPlaying: boolean,
) {
  void audioRef;
  const [displayTime, setDisplayTime] = useState(playbackTimeRef.current);

  useEffect(() => {
    setDisplayTime(playbackTimeRef.current);
  }, [isPlaying, playbackTimeRef]);

  useEffect(() => {
    let frame = 0;
    let lastPublished = -1;
    const tick = () => {
      const nextTime = playbackTimeRef.current;
      if (Math.abs(nextTime - lastPublished) >= 1 / 30) {
        lastPublished = nextTime;
        setDisplayTime(nextTime);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [isPlaying, playbackTimeRef]);

  return [displayTime, setDisplayTime] as const;
}
