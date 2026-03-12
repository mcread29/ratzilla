import { Toaster as Sonner, ToasterProps } from "sonner";

function Toaster(props: ToasterProps) {
  return (
    <Sonner
      theme="dark"
      richColors
      closeButton
      toastOptions={{
        classNames: {
          toast: "!border-border !bg-popover !text-popover-foreground",
          title: "!text-sm !font-medium",
          description: "!text-xs !text-muted-foreground",
        },
      }}
      {...props}
    />
  );
}

export { Toaster };
