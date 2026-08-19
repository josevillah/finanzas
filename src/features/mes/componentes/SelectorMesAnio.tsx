import { useEffect, useMemo, useRef, useState } from "react";

import { cn } from "@/lib/cn";
import { MESES_CORTOS, capitalizar, formatearMesTitulo, mesAbsoluto, mesActual } from "@/lib/fechas";
import type { RangoMeses } from "@/types/dominio";

/**
 * Elige mes y año desde una grilla, en vez de ir de a un click con las flechas.
 *
 * El rango lo decide el backend: desde el mes más antiguo con datos (o 24 meses
 * atrás) hasta el mes actual —o más adelante, si quedó algún movimiento en un
 * mes futuro que haya que poder borrar—. Fuera de ahí solo se crearían
 * períodos vacíos.
 */
export function SelectorMesAnio({
  anio,
  mes,
  rango,
  onElegir,
}: {
  anio: number;
  mes: number;
  rango: RangoMeses | undefined;
  onElegir: (anio: number, mes: number) => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const [anioVisible, setAnioVisible] = useState(anio);
  const contenedor = useRef<HTMLDivElement>(null);

  // Al abrirlo siempre se parte mostrando el año del mes seleccionado.
  useEffect(() => {
    if (abierto) setAnioVisible(anio);
  }, [abierto, anio]);

  // Cerrar con Escape o al hacer clic fuera.
  useEffect(() => {
    if (!abierto) return;

    const alPresionar = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAbierto(false);
    };
    const alClic = (e: MouseEvent) => {
      if (!contenedor.current?.contains(e.target as Node)) setAbierto(false);
    };

    document.addEventListener("keydown", alPresionar);
    document.addEventListener("mousedown", alClic);
    return () => {
      document.removeEventListener("keydown", alPresionar);
      document.removeEventListener("mousedown", alClic);
    };
  }, [abierto]);

  const conDatos = useMemo(
    () => new Set((rango?.meses ?? []).map((m) => mesAbsoluto(m.anio, m.mes))),
    [rango],
  );

  const hoy = mesActual();
  const minAbs = rango ? mesAbsoluto(rango.desde_anio, rango.desde_mes) : mesAbsoluto(anio, mes);
  const maxAbs = rango ? mesAbsoluto(rango.hasta_anio, rango.hasta_mes) : mesAbsoluto(anio, mes);

  const anioMin = Math.floor((minAbs - 1) / 12);
  const anioMax = Math.floor((maxAbs - 1) / 12);

  return (
    <div ref={contenedor} className="relative">
      <button
        type="button"
        onClick={() => setAbierto((v) => !v)}
        aria-haspopup="dialog"
        aria-expanded={abierto}
        className={cn(
          "min-w-44 rounded-lg px-2 py-1 text-center text-sm font-medium transition-colors",
          "hover:bg-slate-100 dark:hover:bg-slate-800",
          "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600",
          abierto && "bg-slate-100 dark:bg-slate-800",
        )}
      >
        {formatearMesTitulo(anio, mes)}
        <span aria-hidden className="ml-1.5 text-xs text-slate-400">
          ▾
        </span>
      </button>

      {abierto ? (
        <div
          role="dialog"
          aria-label="Elegir mes y año"
          className={cn(
            "absolute left-1/2 top-full z-40 mt-1 w-72 -translate-x-1/2 rounded-xl border p-3 shadow-xl",
            "border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900",
          )}
        >
          <div className="mb-3 flex items-center justify-between">
            <button
              type="button"
              disabled={anioVisible <= anioMin}
              onClick={() => setAnioVisible((a) => a - 1)}
              aria-label="Año anterior"
              className="rounded-lg px-2 py-1 text-sm hover:bg-slate-100 disabled:opacity-30 dark:hover:bg-slate-800"
            >
              ←
            </button>
            <span className="text-sm font-semibold tabular">{anioVisible}</span>
            <button
              type="button"
              disabled={anioVisible >= anioMax}
              onClick={() => setAnioVisible((a) => a + 1)}
              aria-label="Año siguiente"
              className="rounded-lg px-2 py-1 text-sm hover:bg-slate-100 disabled:opacity-30 dark:hover:bg-slate-800"
            >
              →
            </button>
          </div>

          <div className="grid grid-cols-3 gap-1.5">
            {MESES_CORTOS.map((etiqueta, idx) => {
              const numero = idx + 1;
              const abs = mesAbsoluto(anioVisible, numero);

              const fuera = abs < minAbs || abs > maxAbs;
              const seleccionado = anioVisible === anio && numero === mes;
              const esHoy = anioVisible === hoy.anio && numero === hoy.mes;
              const tiene = conDatos.has(abs);

              return (
                <button
                  key={etiqueta}
                  type="button"
                  disabled={fuera}
                  title={
                    fuera
                      ? "Fuera del rango con datos"
                      : tiene
                        ? "Tiene movimientos registrados"
                        : "Sin datos todavía"
                  }
                  onClick={() => {
                    onElegir(anioVisible, numero);
                    setAbierto(false);
                  }}
                  className={cn(
                    "relative rounded-lg py-2 text-sm transition-colors",
                    "disabled:cursor-not-allowed disabled:opacity-30",
                    seleccionado
                      ? "bg-indigo-600 font-medium text-white"
                      : "hover:bg-slate-100 dark:hover:bg-slate-800",
                    !seleccionado && esHoy && "ring-1 ring-indigo-400",
                  )}
                >
                  {capitalizar(etiqueta)}
                  {/* El punto distingue los meses que tienen algo que mirar de
                      los que abrirían un período vacío. */}
                  {tiene ? (
                    <span
                      aria-hidden
                      className={cn(
                        "absolute bottom-1 left-1/2 h-1 w-1 -translate-x-1/2 rounded-full",
                        seleccionado ? "bg-white" : "bg-indigo-500",
                      )}
                    />
                  ) : null}
                </button>
              );
            })}
          </div>

          <p className="mt-3 flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
            <span aria-hidden className="h-1 w-1 rounded-full bg-indigo-500" />
            con datos registrados
          </p>
        </div>
      ) : null}
    </div>
  );
}
