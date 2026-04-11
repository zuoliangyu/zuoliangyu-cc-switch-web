import { Copy } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { SessionMessage } from "@/types";
import {
  formatTimestamp,
  getRoleLabel,
  getRoleTone,
  highlightText,
} from "./utils";

interface SessionMessageItemProps {
  message: SessionMessage;
  index: number;
  isActive: boolean;
  searchQuery?: string;
  setRef: (el: HTMLDivElement | null) => void;
  onCopy: (content: string) => void;
}

export function SessionMessageItem({
  message,
  isActive,
  searchQuery,
  setRef,
  onCopy,
}: SessionMessageItemProps) {
  const { t } = useTranslation();

  return (
    <div
      ref={setRef}
      className={cn(
        "group relative min-w-0 rounded-[22px] border px-4 py-3 transition-all",
        message.role.toLowerCase() === "user"
          ? "ml-6 bg-primary/6 border-primary/20 shadow-[0_10px_24px_hsl(var(--primary)/0.08)]"
          : message.role.toLowerCase() === "assistant"
            ? "mr-6 border-blue-500/20 bg-blue-500/5 shadow-[0_10px_24px_rgba(59,130,246,0.08)]"
            : "border-border/60 bg-background/65",
        isActive && "ring-2 ring-primary ring-offset-2 ring-offset-background",
      )}
    >
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="absolute top-2 right-2 size-7 rounded-xl opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100"
            onClick={() => onCopy(message.content)}
          >
            <Copy className="size-3" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {t("sessionManager.copyMessage", {
            defaultValue: "复制内容",
          })}
        </TooltipContent>
      </Tooltip>
      <div className="mb-2 flex items-center justify-between gap-3 pr-7 text-xs">
        <span
          className={cn(
            "rounded-full bg-background/80 px-2.5 py-1 font-semibold shadow-sm",
            getRoleTone(message.role),
          )}
        >
          {getRoleLabel(message.role, t)}
        </span>
        {message.ts && (
          <span className="text-muted-foreground">
            {formatTimestamp(message.ts)}
          </span>
        )}
      </div>
      <div className="min-w-0 whitespace-pre-wrap break-words text-sm leading-7 [overflow-wrap:anywhere]">
        {searchQuery
          ? highlightText(message.content, searchQuery)
          : message.content}
      </div>
    </div>
  );
}
