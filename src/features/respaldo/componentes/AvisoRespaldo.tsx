import { useState } from "react";
import { Link, useLocation } from "react-router-dom";

import { useEstadoRespaldo } from "../hooks";

/**
 * Recordatorio de respaldo. Aparece cuando nunca se respaldó o pasaron más de
 * 7 días, y se puede cerrar hasta la próxima vez que se abra la app: es un
 * aviso, no un bloqueo.
 */
export function AvisoRespaldo() {
  const { data } = useEstadoRespaldo();
  const [oculto, setOculto] = useState(false);
  const { pathname } = useLocation();

  // En la propia pantalla de respaldo el aviso ya está en el contenido.
  if (!data?.requiere_recordatorio || oculto || pathname === "/respaldo") return null;

  return (
    <div className="mb-5 flex flex-wrap items-center gap-3 rounded-xl border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-200">
      <span aria-hidden>💾</span>
      <span className="min-w-0 flex-1">
        {data.ultimo_respaldo
          ? `Pasaron ${data.dias_desde_ultimo} días desde tu último respaldo.`
          : "Todavía no has respaldado tus datos."}{" "}
        Están solo en este computador.
      </span>

      <Link
        to="/respaldo"
        className="rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-500"
      >
        Respaldar ahora
      </Link>
      <button
        type="button"
        onClick={() => setOculto(true)}
        aria-label="Ocultar el aviso"
        className="rounded-lg px-2 py-1 text-amber-700 hover:bg-amber-100 dark:text-amber-300 dark:hover:bg-amber-900/40"
      >
        ✕
      </button>
    </div>
  );
}
