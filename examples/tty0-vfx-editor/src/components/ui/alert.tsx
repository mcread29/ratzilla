import * as React from "react";
import { AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";

export function Alert({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      role="alert"
      className={cn(
        "grid grid-cols-[auto_1fr] items-start gap-x-3 gap-y-1 rounded-lg border border-border/80 bg-card/80 px-3 py-2 text-sm text-foreground",
        className,
      )}
      {...props}
    />
  );
}

export function AlertIcon({ className }: { className?: string }) {
  return <AlertTriangle className={cn("mt-0.5 size-4 text-amber-300", className)} />;
}

export function AlertTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h5 className={cn("font-medium leading-none", className)} {...props} />;
}

export function AlertDescription({ className, ...props }: React.HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("text-xs leading-relaxed text-muted-foreground", className)} {...props} />;
}
