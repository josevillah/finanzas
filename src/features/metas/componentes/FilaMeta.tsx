import { BarraAvance } from "@/components/BarraAvance";
import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Insignia } from "@/components/ui/Insignia";
import { cn } from "@/lib/cn";
import { formatearFecha } from "@/lib/fechas";
import { formatearCLP, formatearPorcentaje } from "@/lib/moneda";
import type { EstadoMeta, MetaDetalle } from "@/types/dominio";

interface Props {
  meta: MetaDetalle;
  primera: boolean;
  ultima: boolean;
  /**
   * Solo se reordena cuando la lista visible incluye a las activas: son las
   * únicas que compiten por el saldo de una cuenta. Reordenar mirando solo las
   * cumplidas las mandaría adelante de las activas sin que se vea.
   */
  reordenable: boolean;
  ocupado: boolean;
  onSubir: () => void;
  onBajar: () => void;
  onEditar: () => void;
  onCambiarEstado: (estado: EstadoMeta) => void;
  onEliminar: () => void;
}

/** "5 meses" / "1 mes". */
function meses(n: number): string {
  return n === 1 ? "1 mes" : `${n} meses`;
}

export function FilaMeta({
  meta,
  primera,
  ultima,
  reordenable,
  ocupado,
  onSubir,
  onBajar,
  onEditar,
  onCambiarEstado,
  onEliminar,
}: Props) {
  const activa = meta.estado === "activa";
  const cumplida = meta.estado === "cumplida";
  const lograda = meta.tiene_progreso && meta.falta === 0;

  return (
    <li className="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-2">
          {/* Reordenar con botones y no arrastrando: funciona con teclado y no
              suma una dependencia solo para esto. */}
          <div className={cn("flex-col gap-0.5 pt-0.5", reordenable ? "flex" : "hidden")}>
            <button
              type="button"
              aria-label={`Subir «${meta.nombre}»`}
              onClick={onSubir}
              disabled={primera || ocupado}
              className="rounded px-1.5 text-xs text-slate-500 hover:bg-slate-100 disabled:opacity-30 dark:text-slate-400 dark:hover:bg-slate-800"
            >
              ▲
            </button>
            <button
              type="button"
              aria-label={`Bajar «${meta.nombre}»`}
              onClick={onBajar}
              disabled={ultima || ocupado}
              className="rounded px-1.5 text-xs text-slate-500 hover:bg-slate-100 disabled:opacity-30 dark:text-slate-400 dark:hover:bg-slate-800"
            >
              ▼
            </button>
          </div>

          <div className="min-w-0">
            <p className="flex flex-wrap items-center gap-2 text-sm font-medium">
              <span className="truncate">{meta.nombre}</span>
              {cumplida ? <Insignia tono="verde">Cumplida</Insignia> : null}
              {meta.estado === "archivada" ? <Insignia>Archivada</Insignia> : null}
              {activa && lograda ? <Insignia tono="verde">Ya la juntaste</Insignia> : null}
            </p>

            <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
              {meta.cuenta_nombre ? (
                <>Se financia desde «{meta.cuenta_nombre}»</>
              ) : (
                <>Sin cuenta vinculada: es una referencia de cuánto necesitas</>
              )}
              {meta.notas ? <> · {meta.notas}</> : null}
            </p>
          </div>
        </div>

        <div className="text-right">
          <Moneda monto={meta.monto_objetivo} className="text-lg font-semibold" />
          <p className="text-xs text-slate-500 dark:text-slate-400">objetivo</p>
        </div>
      </div>

      {meta.tiene_progreso ? (
        <div className="mt-3">
          <BarraAvance
            porcentaje={meta.progreso_pct}
            tono={meta.falta === 0 ? "verde" : "indigo"}
          />
          <div className="mt-1.5 flex flex-wrap justify-between gap-x-4 text-xs text-slate-600 dark:text-slate-400">
            <span>
              <Moneda monto={meta.acumulado} className="font-medium" /> acumulados ·{" "}
              {formatearPorcentaje(meta.progreso_pct, 0)}
            </span>
            <span>
              {meta.falta > 0 ? (
                <>
                  Faltan <Moneda monto={meta.falta} className="font-medium" />
                </>
              ) : (
                <>Cubierta por completo</>
              )}
            </span>
          </div>
        </div>
      ) : (
        <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
          {meta.estado === "archivada"
            ? "Archivada: no reserva ahorros."
            : "Sin avance que mostrar. Vincúlala a una cuenta de ahorro para verlo."}
        </p>
      )}

      {activa ? (
        <div className="mt-3 flex flex-wrap gap-x-6 gap-y-1 text-xs text-slate-600 dark:text-slate-400">
          {meta.fecha_pasada ? (
            <span>
              La fecha objetivo ({formatearFecha(meta.fecha_objetivo)}) ya pasó. Puedes moverla o
              seguir juntando.
            </span>
          ) : meta.ritmo_mensual !== null ? (
            <span>
              Para el {formatearFecha(meta.fecha_objetivo)}:{" "}
              <strong className="font-medium">{formatearCLP(meta.ritmo_mensual)}</strong> al mes
              durante {meses(meta.meses_restantes ?? 0)}.
            </span>
          ) : meta.fecha_objetivo && meta.falta === 0 ? (
            <span>Lista antes del {formatearFecha(meta.fecha_objetivo)}.</span>
          ) : null}

          {meta.falta > 0 && meta.meses_al_ritmo !== null ? (
            <span>
              Al balance de los últimos meses:{" "}
              <strong className="font-medium">{meses(meta.meses_al_ritmo)}</strong>.
            </span>
          ) : null}
        </div>
      ) : null}

      <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-slate-100 pt-3 dark:border-slate-800">
        {activa ? (
          <Boton
            tamano="sm"
            variante="secundario"
            onClick={() => onCambiarEstado("cumplida")}
            disabled={ocupado}
          >
            Marcar cumplida
          </Boton>
        ) : (
          <Boton
            tamano="sm"
            variante="secundario"
            onClick={() => onCambiarEstado("activa")}
            disabled={ocupado}
          >
            Volver a activarla
          </Boton>
        )}

        {meta.estado !== "archivada" ? (
          <Boton
            tamano="sm"
            variante="fantasma"
            onClick={() => onCambiarEstado("archivada")}
            disabled={ocupado}
          >
            Archivar
          </Boton>
        ) : null}

        <Boton tamano="sm" variante="fantasma" onClick={onEditar} disabled={ocupado}>
          Editar
        </Boton>

        <Boton
          tamano="sm"
          variante="peligro"
          onClick={onEliminar}
          disabled={ocupado}
          title="Archivarla la conserva como historial; eliminarla la borra para siempre."
        >
          Eliminar
        </Boton>
      </div>
    </li>
  );
}
