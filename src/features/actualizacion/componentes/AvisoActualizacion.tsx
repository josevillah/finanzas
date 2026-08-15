import { useState } from "react";

import { Boton } from "@/components/ui/Boton";
import { mensajeDeError } from "@/lib/ipc";

import { useEstadoActualizacion, useInstalarActualizacion } from "../hooks";

/**
 * Indicador discreto de que hay una versión nueva ya descargada.
 *
 * Deliberadamente no es un modal: el instalador reemplaza el ejecutable en
 * marcha, así que la app se cierra sí o sí. Interrumpir a alguien a mitad de
 * registrar un gasto para eso se siente como un crash.
 */
export function AvisoActualizacion() {
  const { data } = useEstadoActualizacion();
  const instalar = useInstalarActualizacion();

  const [verNotas, setVerNotas] = useState(false);
  const [oculto, setOculto] = useState(false);

  if (!data?.lista_para_instalar || oculto) return null;

  return (
    <div className="mb-5 rounded-xl border border-indigo-300 bg-indigo-50 px-4 py-3 text-sm text-indigo-900 dark:border-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-200">
      <div className="flex flex-wrap items-center gap-3">
        <span aria-hidden>⬆️</span>
        <span className="min-w-0 flex-1">
          La versión <strong>{data.version_disponible}</strong> está lista para instalarse. La app
          se va a cerrar y volver a abrir sola.
        </span>

        {data.notas ? (
          <button
            type="button"
            onClick={() => setVerNotas((v) => !v)}
            className="text-xs underline underline-offset-2"
          >
            {verNotas ? "Ocultar novedades" : "Ver novedades"}
          </button>
        ) : null}

        <Boton
          tamano="sm"
          disabled={instalar.isPending}
          onClick={() => instalar.mutate()}
        >
          {instalar.isPending ? "Instalando…" : "Instalar y reiniciar"}
        </Boton>
        <button
          type="button"
          onClick={() => setOculto(true)}
          aria-label="Ahora no"
          className="rounded-lg px-2 py-1 text-indigo-700 hover:bg-indigo-100 dark:text-indigo-300 dark:hover:bg-indigo-900/40"
        >
          ✕
        </button>
      </div>

      {verNotas && data.notas ? (
        <pre className="mt-3 max-h-52 overflow-y-auto whitespace-pre-wrap rounded-lg bg-white/60 px-3 py-2 font-sans text-xs dark:bg-slate-900/60">
          {data.notas}
        </pre>
      ) : null}

      {instalar.error ? (
        <p className="mt-2 rounded-lg bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(instalar.error)}
        </p>
      ) : null}
    </div>
  );
}
