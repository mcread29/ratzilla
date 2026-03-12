import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-[color,box-shadow,background-color,border-color] outline-none disabled:pointer-events-none disabled:opacity-45 [&_svg]:pointer-events-none [&_svg]:shrink-0 focus-visible:ring-2 focus-visible:ring-ring/70 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
  {
    variants: {
      variant: {
        default:
          "border border-transparent bg-primary text-primary-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.08)] hover:bg-primary/92",
        secondary: "border border-border/80 bg-secondary text-secondary-foreground hover:bg-secondary/88",
        outline: "border border-border bg-transparent text-foreground hover:bg-accent/70 hover:text-accent-foreground",
        ghost: "border border-transparent bg-transparent text-muted-foreground hover:bg-accent/60 hover:text-accent-foreground",
        destructive: "border border-destructive/70 bg-destructive/90 text-destructive-foreground hover:bg-destructive",
        toolbar:
          "border border-border/80 bg-[color:var(--editor-surface)] text-foreground hover:border-editor-cyan/55 hover:bg-accent/50 data-[state=on]:border-editor-cyan data-[state=on]:bg-accent/70 data-[state=on]:text-accent-foreground",
      },
      size: {
        default: "h-9 px-3.5 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        icon: "size-9 rounded-full",
        compact: "h-8 px-2.5 text-xs",
        toolbar: "size-9 rounded-md",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

const Button = React.forwardRef<
  HTMLButtonElement,
  React.ComponentProps<"button"> &
    VariantProps<typeof buttonVariants> & {
      asChild?: boolean;
    }
>(({ className, variant, size, asChild = false, ...props }, ref) => {
  const Comp = asChild ? Slot : "button";

  return <Comp ref={ref} data-slot="button" className={cn(buttonVariants({ variant, size, className }))} {...props} />;
});

Button.displayName = "Button";

export { Button, buttonVariants };
