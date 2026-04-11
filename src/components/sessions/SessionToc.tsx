import { List, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

interface TocItem {
  index: number;
  preview: string;
  ts?: number;
}

interface SessionTocSidebarProps {
  items: TocItem[];
  onItemClick: (index: number) => void;
}

export function SessionTocSidebar({
  items,
  onItemClick,
}: SessionTocSidebarProps) {
  const { t } = useTranslation();
  if (items.length <= 2) return null;

  return (
    <div className="hidden w-72 shrink-0 border-l border-border-default bg-background/30 xl:block">
      <div className="border-b border-border-default p-4">
        <div className="text-[11px] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
          {t("sessionManager.tocLabel", {
            defaultValue: "Conversation Map",
          })}
        </div>
        <div className="mt-2 flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-background/80 shadow-sm">
            <List className="size-4 text-primary" />
          </div>
          <div>
            <div className="text-sm font-semibold text-foreground">
              {t("sessionManager.tocTitle")}
            </div>
            <div className="text-xs text-muted-foreground">{items.length}</div>
          </div>
        </div>
      </div>
      <ScrollArea className="h-[calc(100%-89px)]">
        <div className="space-y-2 p-3">
          {items.map((item, tocIndex) => (
            <button
              key={item.index}
              type="button"
              onClick={() => onItemClick(item.index)}
              className={cn(
                "flex w-full items-start gap-3 rounded-[18px] border border-transparent px-3 py-3 text-left text-xs transition-all",
                "bg-background/55 text-muted-foreground hover:border-border-hover hover:bg-background/80 hover:text-foreground",
              )}
            >
              <span className="theme-chip-primary flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold">
                {tocIndex + 1}
              </span>
              <span className="line-clamp-2 pt-0.5 leading-6">
                {item.preview}
              </span>
            </button>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}

interface SessionTocDialogProps {
  items: TocItem[];
  onItemClick: (index: number) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SessionTocDialog({
  items,
  onItemClick,
  open,
  onOpenChange,
}: SessionTocDialogProps) {
  const { t } = useTranslation();
  if (items.length <= 2) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger asChild>
        <Button
          size="icon"
          className="fixed bottom-20 right-4 z-30 size-11 rounded-full shadow-lg xl:hidden"
        >
          <List className="size-4" />
        </Button>
      </DialogTrigger>
      <DialogContent
        className="max-h-[70vh] max-w-md gap-0 p-0"
        zIndex="alert"
        onInteractOutside={() => onOpenChange(false)}
        onEscapeKeyDown={() => onOpenChange(false)}
      >
        <DialogHeader className="relative border-b border-border-default px-4 py-4">
          <DialogTitle className="flex items-center gap-2 text-base font-semibold">
            <span className="flex h-9 w-9 items-center justify-center rounded-xl bg-background/80 shadow-sm">
              <List className="size-4 text-primary" />
            </span>
            <span>{t("sessionManager.tocTitle")}</span>
          </DialogTitle>
          <DialogClose
            className="absolute right-3 top-1/2 -translate-y-1/2 rounded-full p-1.5 hover:bg-muted transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2"
            aria-label={t("common.close")}
          >
            <X className="size-4 text-muted-foreground" />
          </DialogClose>
        </DialogHeader>
        <div className="max-h-[calc(70vh-88px)] overflow-y-auto">
          <div className="space-y-2 p-3 pb-4">
            {items.map((item, tocIndex) => (
              <button
                key={item.index}
                type="button"
                onClick={() => onItemClick(item.index)}
                className={cn(
                  "flex w-full items-start gap-3 rounded-[18px] border border-transparent px-3 py-3 text-left text-sm transition-all",
                  "bg-background/60 text-foreground hover:border-border-hover hover:bg-primary/10",
                  "focus:outline-none focus:ring-2 focus:ring-primary focus:ring-inset",
                )}
              >
                <span className="theme-chip-primary flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-xs font-semibold">
                  {tocIndex + 1}
                </span>
                <span className="line-clamp-2 pt-0.5 leading-6">
                  {item.preview}
                </span>
              </button>
            ))}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
