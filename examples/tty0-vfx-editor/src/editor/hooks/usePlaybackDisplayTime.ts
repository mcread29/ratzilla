import { useEffect, useState } from "react";

export function usePlaybackDisplayTime(
  audioRef: { current: HTMLAudioElement | null },
  playbackTimeRef: { current: number },
  isPlaying: boolean,
) {
  const [displayTime, setDisplayTime] = useState(playbackTimeRef.current);

  useEffect(() => {
    const audio = audioRef.current;
    setDisplayTime(audio ? audio.currentTime : playbackTimeRef.current);
  }, [audioRef, isPlaying, playbackTimeRef]);

  useEffect(() => {
    let frame = 0;
    let lastPublished = -1;
    const tick = () => {
      const audio = audioRef.current;
      const nextTime = audio ? audio.currentTime : playbackTimeRef.current;
      if (Math.abs(nextTime - lastPublished) >= 1 / 30) {
        lastPublished = nextTime;
        setDisplayTime(nextTime);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [audioRef, isPlaying, playbackTimeRef]);

  return [displayTime, setDisplayTime] as const;
}
