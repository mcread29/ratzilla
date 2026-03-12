import * as React from "react";
import { GripVertical } from "lucide-react";
import { Group, Panel, Separator } from "react-resizable-panels";
import { cn } from "@/lib/utils";

function ResizablePanelGroup({ className, ...props }: React.ComponentProps<typeof Group>) {
  return <Group data-slot="resizable-panel-group" className={cn("flex h-full w-full data-[orientation=vertical]:flex-col", className)} {...props} />;
}

function ResizablePanel(props: React.ComponentProps<typeof Panel>) {
  return <Panel data-slot="resizable-panel" {...props} />;
}

function ResizableHandle({ className, withHandle, ...props }: React.ComponentProps<typeof Separator> & { withHandle?: boolean }) {
  return (
    <Separator data-slot="resizable-handle" className={cn("resizable-shell-handle", className)} {...props}>
      {withHandle ? <GripVertical className="size-4 rotate-90 text-muted-foreground" /> : null}
    </Separator>
  );
}

export { ResizableHandle, ResizablePanel, ResizablePanelGroup };
