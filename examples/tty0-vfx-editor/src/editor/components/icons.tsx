import { ShapeInteractionMode, TimelineTool } from "../editor-types";

export function TransportIcon({ name }: { name: "loading" | "pause" | "play" | "stop" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "play" ? <path d="M6 4.5 15 10 6 15.5Z" fill="currentColor" stroke="none" /> : null}
      {name === "pause" ? (
        <>
          <path d="M6.5 4.5v11" {...commonProps} />
          <path d="M13.5 4.5v11" {...commonProps} />
        </>
      ) : null}
      {name === "stop" ? <rect x="5.5" y="5.5" width="9" height="9" rx="1.5" fill="currentColor" /> : null}
      {name === "loading" ? (
        <>
          <path d="M10 3.5a6.5 6.5 0 1 1-4.6 1.9" {...commonProps} />
          <path d="M5.4 5.4 4 2.8l2.8 1.4" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function ClipActionIcon({ name }: { name: "add" | "copy" | "delete" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "add" ? (
        <>
          <path d="M10 4.5v11" {...commonProps} />
          <path d="M4.5 10h11" {...commonProps} />
        </>
      ) : null}
      {name === "copy" ? (
        <>
          <rect x="7" y="5" width="8" height="10" rx="1.5" {...commonProps} />
          <path d="M5 12.5H4.5A1.5 1.5 0 0 1 3 11V6.5A1.5 1.5 0 0 1 4.5 5H9" {...commonProps} />
        </>
      ) : null}
      {name === "delete" ? (
        <>
          <path d="M5.5 6.5h9" {...commonProps} />
          <path d="M8 3.8h4" {...commonProps} />
          <path d="M7 6.5v8" {...commonProps} />
          <path d="M10 6.5v8" {...commonProps} />
          <path d="M13 6.5v8" {...commonProps} />
          <path d="M6.5 6.5 7 15a1.5 1.5 0 0 0 1.5 1.4h3a1.5 1.5 0 0 0 1.5-1.4l.5-8.5" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function TimelineActionIcon({ name }: { name: "copy" | "delete" | "place" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "place" ? (
        <>
          <path d="M4.5 10h6" {...commonProps} />
          <path d="M10 6.5 15.5 10 10 13.5" {...commonProps} />
          <path d="M4.5 5.5v9" {...commonProps} />
        </>
      ) : null}
      {name === "copy" ? (
        <>
          <rect x="7" y="5" width="8" height="10" rx="1.5" {...commonProps} />
          <path d="M5 12.5H4.5A1.5 1.5 0 0 1 3 11V6.5A1.5 1.5 0 0 1 4.5 5H9" {...commonProps} />
        </>
      ) : null}
      {name === "delete" ? (
        <>
          <path d="M5.5 6.5h9" {...commonProps} />
          <path d="M8 3.8h4" {...commonProps} />
          <path d="M7 6.5v8" {...commonProps} />
          <path d="M10 6.5v8" {...commonProps} />
          <path d="M13 6.5v8" {...commonProps} />
          <path d="M6.5 6.5 7 15a1.5 1.5 0 0 0 1.5 1.4h3a1.5 1.5 0 0 0 1.5-1.4l.5-8.5" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function TimelineToolIcon({ name }: { name: TimelineTool }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "select" ? <path d="M4.5 3.5 12 11H8.6l2.2 5-2 0.9-2.2-5L4.5 14Z" fill="currentColor" stroke="none" /> : null}
      {name === "pencil" ? (
        <>
          <path d="M4.5 15.5 6 11.8 13.9 3.9a1.3 1.3 0 0 1 1.8 0l0.4 0.4a1.3 1.3 0 0 1 0 1.8L8.2 14l-3.7 1.5Z" {...commonProps} />
          <path d="M12.8 5 15 7.2" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function TimelineFieldIcon({ name }: { name: "bpm" | "measures" | "time" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.7,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "bpm" ? (
        <>
          <path d="M5 15.5V8.5a1 1 0 0 1 1-1h8.5" {...commonProps} />
          <path d="M10 10 13.2 6.8" {...commonProps} />
          <path d="M5.5 15.5h9" {...commonProps} />
        </>
      ) : null}
      {name === "measures" ? (
        <>
          <rect x="4.5" y="5" width="11" height="10" rx="0.5" {...commonProps} />
          <path d="M8.2 5v10" {...commonProps} />
          <path d="M11.8 5v10" {...commonProps} />
        </>
      ) : null}
      {name === "time" ? (
        <>
          <circle cx="10" cy="10" r="5.5" {...commonProps} />
          <path d="M10 7.3v3.1l2.2 1.5" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function ClipFieldIcon({ name }: { name: "name" | "length" | "min" | "max" }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.7,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "name" ? (
        <>
          <path d="M4.5 5.5h11" {...commonProps} />
          <path d="M10 5.5v9" {...commonProps} />
          <path d="M6.5 14.5h7" {...commonProps} />
        </>
      ) : null}
      {name === "length" ? (
        <>
          <path d="M4.5 10h11" {...commonProps} />
          <path d="M7.2 7.2 4.5 10l2.7 2.8" {...commonProps} />
          <path d="M12.8 7.2 15.5 10l-2.7 2.8" {...commonProps} />
        </>
      ) : null}
      {name === "min" ? (
        <>
          <path d="M10 4.5v11" {...commonProps} />
          <path d="M6.8 12.3 10 15.5l3.2-3.2" {...commonProps} />
          <path d="M5 15.5h10" {...commonProps} />
        </>
      ) : null}
      {name === "max" ? (
        <>
          <path d="M10 15.5v-11" {...commonProps} />
          <path d="M6.8 7.7 10 4.5l3.2 3.2" {...commonProps} />
          <path d="M5 4.5h10" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}

export function ShapeModeIcon({ name }: { name: ShapeInteractionMode }) {
  const commonProps = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.7,
  };

  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      {name === "add" ? (
        <>
          <path d="M10 4.5v11" {...commonProps} />
          <path d="M4.5 10h11" {...commonProps} />
        </>
      ) : null}
      {name === "move" ? (
        <>
          <path d="M10 3.8v12.4" {...commonProps} />
          <path d="M3.8 10h12.4" {...commonProps} />
          <path d="M7.8 6.2 10 4l2.2 2.2" {...commonProps} />
          <path d="M7.8 13.8 10 16l2.2-2.2" {...commonProps} />
          <path d="M6.2 7.8 4 10l2.2 2.2" {...commonProps} />
          <path d="M13.8 7.8 16 10l-2.2 2.2" {...commonProps} />
        </>
      ) : null}
      {name === "delete" ? (
        <>
          <path d="M5.5 5.5 14.5 14.5" {...commonProps} />
          <path d="M14.5 5.5 5.5 14.5" {...commonProps} />
        </>
      ) : null}
    </svg>
  );
}
