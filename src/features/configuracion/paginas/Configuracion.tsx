import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Interruptor } from "@/components/ui/Interruptor";
import { cn } from "@/lib/cn";
import { mensajeDeError } from "@/lib/ipc";
import {
  DETALLE_ACCION_CIERRE,
  ETIQUETAS_ACCION_CIERRE,
  type AccionCierre,
} from "@/types/dominio";

import { useAjustes, useFijarAccionCierre, useFijarAutostart } from "../hooks";

const ACCIONES: AccionCierre[] = ["preguntar", "bandeja", "salir"];

export function Configuracion() {
  const { data, isPending, error, refetch } = useAjustes();

  const fijarCierre = useFijarAccionCierre();
  const fijarAutostart = useFijarAutostart();

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;
  if (!data) return null;

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-xl font-semibold">Configuración</h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">
          Cómo se comporta la aplicación en tu computador.
        </p>
      </header>

      <div className="tarjeta space-y-4">
        <div>
          <h2 className="font-medium">Al cerrar la ventana</h2>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Qué pasa cuando haces clic en la X.
          </p>
        </div>

        <div className="space-y-2">
          {ACCIONES.map((accion) => {
            const activa = data.accion_cierre === accion;
            return (
              <label
                key={accion}
                className={cn(
                  "flex cursor-pointer items-start gap-3 rounded-xl border px-4 py-3 transition-colors",
                  activa
                    ? "border-indigo-500 bg-indigo-50 dark:bg-indigo-950/40"
                    : "border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800/60",
                )}
              >
                <input
                  type="radio"
                  name="accion-cierre"
                  className="mt-1 h-4 w-4 border-slate-300 text-indigo-600"
                  checked={activa}
                  disabled={fijarCierre.isPending}
                  onChange={() => fijarCierre.mutate(accion)}
                />
                <span className="text-sm">
                  <span className="font-medium">{ETIQUETAS_ACCION_CIERRE[accion]}</span>
                  <span className="mt-0.5 block text-xs text-slate-500 dark:text-slate-400">
                    {DETALLE_ACCION_CIERRE[accion]}
                  </span>
                </span>
              </label>
            );
          })}
        </div>

        {fijarCierre.error ? <MensajeError error={fijarCierre.error} /> : null}
      </div>

      <div className="tarjeta space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 className="font-medium">Iniciar con Windows</h2>
            <p className="text-sm text-slate-500 dark:text-slate-400">
              La app arranca sola al encender el computador, directo a la bandeja y sin abrir la
              ventana. Es lo que mantiene vivo el atajo de gasto rápido después de cada reinicio.
            </p>
          </div>

          <Interruptor
            activo={data.autostart_activo}
            ocupado={fijarAutostart.isPending}
            onCambio={(v) => fijarAutostart.mutate(v)}
            etiqueta="Iniciar con Windows"
          />
        </div>

        {fijarAutostart.error ? <MensajeError error={fijarAutostart.error} /> : null}
      </div>

      <div className="tarjeta space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="font-medium">Atajo de gasto rápido</h2>
            <p className="text-sm text-slate-500 dark:text-slate-400">
              Abre la captura rápida desde cualquier parte del sistema.
            </p>
          </div>

          <span className="flex items-center gap-3">
            <kbd className="rounded-lg border border-slate-300 bg-slate-50 px-2.5 py-1 font-mono text-sm dark:border-slate-600 dark:bg-slate-800">
              {data.atajo}
            </kbd>
            <Insignia tono={data.atajo_registrado ? "verde" : "rojo"}>
              {data.atajo_registrado ? "Activo" : "No disponible"}
            </Insignia>
          </span>
        </div>

        {!data.atajo_registrado ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            Otra aplicación ya tiene tomada esa combinación, así que el atajo no funciona. La
            captura rápida sigue disponible desde el botón "Gasto rápido" del menú lateral y desde
            el ícono junto al reloj. Si cierras la otra aplicación y reinicias Finanzas, el atajo
            se recupera.
          </p>
        ) : null}
      </div>
    </div>
  );
}

function MensajeError({ error }: { error: unknown }) {
  return (
    <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
      {mensajeDeError(error)}
    </p>
  );
}
