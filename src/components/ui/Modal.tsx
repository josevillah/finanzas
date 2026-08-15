import { useEffect, type ReactNode } from "react";

import { cn } from "@/lib/cn";

interface Props {
  abierto: boolean;
  titulo: string;
  onCerrar: () => void;
  children: ReactNode;
  /** Botones del pie. */
  acciones?: ReactNode;
  ancho?: "md" | "lg" | "xl";
}

const ANCHOS = { md: "max-w-md", lg: "max-w-2xl", xl: "max-w-4xl" } as const;

export function Modal({ abierto, titulo, onCerrar, children, acciones, ancho = "lg" }: Props) {
  // Escape cierra, y mientras el modal está abierto el fondo no hace scroll.
  useEffect(() => {
    if (!abierto) return;

    const alPresionar = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCerrar();
    };
    document.addEventListener("keydown", alPresionar);
    const overflowPrevio = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", alPresionar);
      document.body.style.overflow = overflowPrevio;
    };
  }, [abierto, onCerrar]);

  if (!abierto) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-slate-900/50 p-4 backdrop-blur-sm sm:p-8">
      <div
        className={cn(
          "w-full rounded-xl border border-slate-200 bg-white shadow-xl",
          "dark:border-slate-800 dark:bg-slate-900",
          ANCHOS[ancho],
        )}
        role="dialog"
        aria-modal="true"
        aria-label={titulo}
      >
        <header className="flex items-center justify-between border-b border-slate-200 px-5 py-4 dark:border-slate-800">
          <h2 className="text-base font-semibold">{titulo}</h2>
          <button
            type="button"
            onClick={onCerrar}
            aria-label="Cerrar"
            className="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            ✕
          </button>
        </header>

        <div className="px-5 py-4">{children}</div>

        {acciones ? (
          <footer className="flex justify-end gap-2 border-t border-slate-200 px-5 py-4 dark:border-slate-800">
            {acciones}
          </footer>
        ) : null}
      </div>
    </div>
  );
}
