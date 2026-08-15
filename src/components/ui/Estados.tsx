import type { ReactNode } from "react";

import { mensajeDeError } from "@/lib/ipc";

export function Cargando({ texto = "Cargando…" }: { texto?: string }) {
  return (
    <div className="flex items-center justify-center gap-3 py-16 text-sm text-slate-500 dark:text-slate-400">
      <span className="h-4 w-4 animate-spin rounded-full border-2 border-slate-300 border-t-indigo-600" />
      {texto}
    </div>
  );
}

export function ErrorCarga({ error, onReintentar }: { error: unknown; onReintentar?: () => void }) {
  return (
    <div className="rounded-xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
      <p className="font-medium">No se pudo cargar la información.</p>
      <p className="mt-1">{mensajeDeError(error)}</p>
      {onReintentar ? (
        <button
          type="button"
          onClick={onReintentar}
          className="mt-3 rounded-lg border border-rose-300 px-3 py-1.5 text-xs font-medium hover:bg-rose-100 dark:border-rose-800 dark:hover:bg-rose-900/40"
        >
          Reintentar
        </button>
      ) : null}
    </div>
  );
}

export function Vacio({
  titulo,
  descripcion,
  accion,
}: {
  titulo: string;
  descripcion?: string;
  accion?: ReactNode;
}) {
  return (
    <div className="rounded-xl border border-dashed border-slate-300 py-14 text-center dark:border-slate-700">
      <p className="text-sm font-medium text-slate-700 dark:text-slate-300">{titulo}</p>
      {descripcion ? (
        <p className="mx-auto mt-1 max-w-md text-sm text-slate-500 dark:text-slate-400">
          {descripcion}
        </p>
      ) : null}
      {accion ? <div className="mt-5">{accion}</div> : null}
    </div>
  );
}
