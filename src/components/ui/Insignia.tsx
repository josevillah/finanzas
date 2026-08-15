import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

export type TonoInsignia = "neutro" | "verde" | "amarillo" | "rojo" | "indigo";

const TONOS: Record<TonoInsignia, string> = {
  neutro: "bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300",
  verde: "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300",
  amarillo: "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300",
  rojo: "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-300",
  indigo: "bg-indigo-100 text-indigo-800 dark:bg-indigo-950 dark:text-indigo-300",
};

export function Insignia({
  tono = "neutro",
  children,
  className,
}: {
  tono?: TonoInsignia;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
        TONOS[tono],
        className,
      )}
    >
      {children}
    </span>
  );
}
