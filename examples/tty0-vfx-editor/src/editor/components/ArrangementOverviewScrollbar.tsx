type ScrollbarDrag =
  | { kind: "move"; pointerId: number; startClientX: number; startStartBeat: number; spanBeats: number }
  | { kind: "resize-left"; pointerId: number; startClientX: number; startStartBeat: number; endBeat: number }
  | { kind: "resize-right"; pointerId: number; startClientX: number; startBeat: number; startEndBeat: number };

export function ArrangementOverviewScrollbar({
  visibleTimelineWidth,
  overviewPxPerBeat,
  visibleBeatSpan,
  visibleStartBeat,
  visibleEndBeat,
  totalTimelineBeats,
  scrollbarThumbWidth,
  scrollbarThumbLeft,
  scrollbarDragRef,
  viewportFromOverview,
  applyViewport,
}: {
  visibleTimelineWidth: number;
  overviewPxPerBeat: number;
  visibleBeatSpan: number;
  visibleStartBeat: number;
  visibleEndBeat: number;
  totalTimelineBeats: number;
  scrollbarThumbWidth: number;
  scrollbarThumbLeft: number;
  scrollbarDragRef: { current: ScrollbarDrag | null };
  viewportFromOverview: (offset: number) => number;
  applyViewport: (startBeat: number, endBeat: number) => void;
}) {
  return (
    <div className="arrangement-top-scrollbar">
      <div
        className="arrangement-top-scrollbar-track"
        onPointerDown={(event) => {
          if (event.target !== event.currentTarget) {
            return;
          }
          const rect = event.currentTarget.getBoundingClientRect();
          const centerBeat = viewportFromOverview(event.clientX - rect.left);
          const nextStartBeat = centerBeat - visibleBeatSpan / 2;
          applyViewport(nextStartBeat, nextStartBeat + visibleBeatSpan);
        }}
      >
        <div
          className="arrangement-top-scrollbar-thumb"
          style={{ width: scrollbarThumbWidth, transform: `translateX(${scrollbarThumbLeft}px)` }}
          onPointerDown={(event) => {
            event.preventDefault();
            event.stopPropagation();
            scrollbarDragRef.current = {
              kind: "move",
              pointerId: event.pointerId,
              startClientX: event.clientX,
              startStartBeat: visibleStartBeat,
              spanBeats: visibleBeatSpan,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
          }}
          onPointerMove={(event) => {
            const drag = scrollbarDragRef.current;
            if (!drag || drag.pointerId !== event.pointerId || drag.kind !== "move" || overviewPxPerBeat <= 0) {
              return;
            }
            const deltaBeats = (event.clientX - drag.startClientX) / overviewPxPerBeat;
            const nextStartBeat = drag.startStartBeat + deltaBeats;
            applyViewport(nextStartBeat, nextStartBeat + drag.spanBeats);
          }}
          onPointerUp={(event) => {
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
              event.currentTarget.releasePointerCapture(event.pointerId);
            }
            scrollbarDragRef.current = null;
          }}
          onPointerCancel={() => {
            scrollbarDragRef.current = null;
          }}
          onLostPointerCapture={() => {
            scrollbarDragRef.current = null;
          }}
        >
          <div
            className="arrangement-top-scrollbar-handle left"
            onPointerDown={(event) => {
              event.preventDefault();
              event.stopPropagation();
              scrollbarDragRef.current = {
                kind: "resize-left",
                pointerId: event.pointerId,
                startClientX: event.clientX,
                startStartBeat: visibleStartBeat,
                endBeat: visibleEndBeat,
              };
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerMove={(event) => {
              const drag = scrollbarDragRef.current;
              if (!drag || drag.pointerId !== event.pointerId || drag.kind !== "resize-left" || overviewPxPerBeat <= 0) {
                return;
              }
              const deltaBeats = (event.clientX - drag.startClientX) / overviewPxPerBeat;
              applyViewport(drag.startStartBeat + deltaBeats, drag.endBeat);
            }}
            onPointerUp={(event) => {
              if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
              }
              scrollbarDragRef.current = null;
            }}
            onPointerCancel={() => {
              scrollbarDragRef.current = null;
            }}
            onLostPointerCapture={() => {
              scrollbarDragRef.current = null;
            }}
          />
          <div
            className="arrangement-top-scrollbar-handle right"
            onPointerDown={(event) => {
              event.preventDefault();
              event.stopPropagation();
              scrollbarDragRef.current = {
                kind: "resize-right",
                pointerId: event.pointerId,
                startClientX: event.clientX,
                startBeat: visibleStartBeat,
                startEndBeat: visibleEndBeat,
              };
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerMove={(event) => {
              const drag = scrollbarDragRef.current;
              if (!drag || drag.pointerId !== event.pointerId || drag.kind !== "resize-right" || overviewPxPerBeat <= 0) {
                return;
              }
              const deltaBeats = (event.clientX - drag.startClientX) / overviewPxPerBeat;
              applyViewport(drag.startBeat, drag.startEndBeat + deltaBeats);
            }}
            onPointerUp={(event) => {
              if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
              }
              scrollbarDragRef.current = null;
            }}
            onPointerCancel={() => {
              scrollbarDragRef.current = null;
            }}
            onLostPointerCapture={() => {
              scrollbarDragRef.current = null;
            }}
          />
        </div>
      </div>
    </div>
  );
}
