import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";

import { cn } from "@/lib/cn";

const BASE_CONTROL = cn(
  "w-full rounded-lg border px-3 py-2 text-sm transition-colors",
  "border-slate-300 bg-white text-slate-900 placeholder:text-slate-400",
  "focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/30",
  "dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100 dark:placeholder:text-slate-500",
);

interface CampoProps {
  etiqueta: string;
  ayuda?: string;
  error?: string | null;
  children: ReactNode;
  className?: string;
}

export function Campo({ etiqueta, ayuda, error, children, className }: CampoProps) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1.5 block text-sm font-medium text-slate-700 dark:text-slate-300">
        {etiqueta}
      </span>
      {children}
      {error ? (
        <span className="mt-1 block text-xs text-rose-600 dark:text-rose-400">{error}</span>
      ) : ayuda ? (
        <span className="mt-1 block text-xs text-slate-500 dark:text-slate-400">{ayuda}</span>
      ) : null}
    </label>
  );
}

export function Entrada({ className, ...resto }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn(BASE_CONTROL, className)} {...resto} />;
}

export function Seleccion({ className, ...resto }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={cn(BASE_CONTROL, className)} {...resto} />;
}

export function AreaTexto({ className, ...resto }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={cn(BASE_CONTROL, "resize-y", className)} {...resto} />;
}
