import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";

interface GlassDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children: React.ReactNode;
}

export function GlassDialog({
  open,
  onOpenChange,
  title,
  description,
  children,
}: GlassDialogProps) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="bg-surface-variant/80 backdrop-blur-[20px] border-l-0 rounded-l-xl overflow-y-auto">
        <SheetHeader>
          <SheetTitle className="font-display text-xl text-on-surface">
            {title}
          </SheetTitle>
          {description && (
            <SheetDescription className="text-on-surface-variant">
              {description}
            </SheetDescription>
          )}
        </SheetHeader>
        <div className="mt-6">{children}</div>
      </SheetContent>
    </Sheet>
  );
}
