import { useMemo, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { Moneda } from "@/components/Moneda";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { SelectorMes, useMes } from "@/features/mes/MesContexto";
import { cn } from "@/lib/cn";
import { MESES_CORTOS } from "@/lib/fechas";
import { formatearCLP, formatearCompacto, formatearPorcentaje } from "@/lib/moneda";
import type { GastoPorCategoria, MesHormiga, SerieCategoria } from "@/types/dominio";

import { useEvolucionGastos, useReporteHormiga } from "../hooks";

const VENTANAS = [6, 12, 24];
const COLOR_POR_DEFECTO = "#94a3b8";

/** 'YYYY-MM' -> 'ago' o 'ago 26' cuando la ventana cruza más de un año. */
function etiquetaMes(clave: string, conAnio: boolean): string {
  const [anio, mes] = clave.split("-");
  const corto = MESES_CORTOS[Number(mes) - 1];
  return conAnio ? `${corto} ${anio.slice(2)}` : corto;
}

export function Reportes() {
  const { anio, mes } = useMes();
  const [meses, setMeses] = useState(12);

  const evolucion = useEvolucionGastos(anio, mes, meses);
  const hormiga = useReporteHormiga(anio, mes, meses);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Reportes</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Cómo se mueve tu gasto en el tiempo.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <SelectorMes />
          <div className="flex gap-1">
            {VENTANAS.map((v) => (
              <button
                key={v}
                type="button"
                onClick={() => setMeses(v)}
                className={
                  meses === v
                    ? "rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white"
                    : "rounded-lg px-3 py-1.5 text-xs font-medium text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
                }
              >
                {v} meses
              </button>
            ))}
          </div>
        </div>
      </header>

      <section className="space-y-3">
        <h2 className="font-medium">Evolución del gasto por categoría</h2>

        {evolucion.isPending ? (
          <Cargando />
        ) : evolucion.error ? (
          <ErrorCarga error={evolucion.error} onReintentar={evolucion.refetch} />
        ) : !evolucion.data?.total_ventana ? (
          <Vacio
            titulo="Sin gastos en esta ventana"
            descripcion="Registra gastos para ver cómo evolucionan mes a mes."
          />
        ) : (
          <EvolucionPanel
            series={evolucion.data.series}
            meses={evolucion.data.meses}
            totalVentana={evolucion.data.total_ventana}
          />
        )}
      </section>

      <section className="space-y-3">
        <h2 className="font-medium">Gastos hormiga</h2>

        {hormiga.isPending ? (
          <Cargando />
        ) : hormiga.error ? (
          <ErrorCarga error={hormiga.error} onReintentar={hormiga.refetch} />
        ) : !hormiga.data?.total_ventana ? (
          <Vacio
            titulo="Sin gastos hormiga registrados"
            descripcion="Usa Ctrl+Shift+G para capturarlos rápido y acá verás cómo se acumulan."
          />
        ) : (
          <HormigaPanel
            meses={hormiga.data.meses}
            mesActual={hormiga.data.mes_actual}
            promedioPrevios={hormiga.data.promedio_previos}
            variacionMesAnterior={hormiga.data.variacion_mes_anterior}
            variacionPromedio={hormiga.data.variacion_promedio}
            porCategoria={hormiga.data.por_categoria}
            totalVentana={hormiga.data.total_ventana}
          />
        )}
      </section>
    </div>
  );
}

// ── evolución ────────────────────────────────────────────────────────────────

function EvolucionPanel({
  series,
  meses,
  totalVentana,
}: {
  series: SerieCategoria[];
  meses: string[];
  totalVentana: number;
}) {
  const conAnio = new Set(meses.map((m) => m.slice(0, 4))).size > 1;

  // Recharts quiere una fila por mes con una columna por serie.
  const datos = useMemo(
    () =>
      meses.map((clave, idx) => {
        const fila: Record<string, string | number> = { clave, etiqueta: etiquetaMes(clave, conAnio) };
        for (const s of series) fila[s.categoria_nombre] = s.puntos[idx]?.total ?? 0;
        return fila;
      }),
    [meses, series, conAnio],
  );

  const promedioMensual = Math.round(totalVentana / Math.max(meses.length, 1));

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-3">
        <div className="tarjeta">
          <p className="etiqueta">Total del período</p>
          <p className="mt-1 text-xl font-semibold">
            <Moneda monto={totalVentana} />
          </p>
        </div>
        <div className="tarjeta">
          <p className="etiqueta">Promedio mensual</p>
          <p className="mt-1 text-xl font-semibold">
            <Moneda monto={promedioMensual} />
          </p>
        </div>
        <div className="tarjeta">
          <p className="etiqueta">Categoría más pesada</p>
          <p className="mt-1 truncate text-xl font-semibold">{series[0]?.categoria_nombre ?? "—"}</p>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            <Moneda monto={series[0]?.total ?? 0} /> en total
          </p>
        </div>
      </div>

      <div className="tarjeta">
        <div className="h-96 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={datos} margin={{ top: 8, right: 8, bottom: 8, left: 8 }}>
              <CartesianGrid
                strokeDasharray="3 3"
                vertical={false}
                className="stroke-slate-200 dark:stroke-slate-800"
              />
              <XAxis
                dataKey="etiqueta"
                tick={{ fontSize: 11 }}
                stroke="currentColor"
                className="text-slate-500"
              />
              <YAxis
                tickFormatter={formatearCompacto}
                tick={{ fontSize: 11 }}
                width={60}
                stroke="currentColor"
                className="text-slate-500"
              />
              <Tooltip content={<TooltipEvolucion />} cursor={{ fillOpacity: 0.08 }} />
              <Legend wrapperStyle={{ fontSize: 11 }} />
              {series.map((s, idx) => (
                <Bar
                  key={s.categoria_nombre}
                  dataKey={s.categoria_nombre}
                  stackId="gasto"
                  fill={s.color ?? COLOR_POR_DEFECTO}
                  // Solo la serie de más arriba lleva las esquinas redondeadas.
                  radius={idx === series.length - 1 ? [4, 4, 0, 0] : undefined}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div className="tarjeta p-0">
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
          {series.map((s) => (
            <li
              key={s.categoria_nombre}
              className="flex items-center justify-between gap-3 px-4 py-2.5 text-sm"
            >
              <span className="flex min-w-0 items-center gap-2">
                <span
                  aria-hidden
                  className="h-2.5 w-2.5 shrink-0 rounded-full"
                  style={{ backgroundColor: s.color ?? COLOR_POR_DEFECTO }}
                />
                <span className="truncate">{s.categoria_nombre}</span>
              </span>
              <span className="flex shrink-0 gap-5">
                <span className="text-slate-500 dark:text-slate-400">
                  <Moneda monto={s.promedio} atenuado /> /mes
                </span>
                <span className="w-28 text-right font-medium">
                  <Moneda monto={s.total} />
                </span>
              </span>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}

function TooltipEvolucion({
  active,
  payload,
  label,
}: {
  active?: boolean;
  payload?: Array<{ name: string; value: number; color: string }>;
  label?: string;
}) {
  if (!active || !payload?.length) return null;

  const total = payload.reduce((acc, p) => acc + (p.value ?? 0), 0);
  // Las series en cero solo agregan ruido al tooltip.
  const conMonto = payload.filter((p) => p.value > 0).reverse();

  return (
    <div className="max-w-64 rounded-lg border border-slate-200 bg-white px-3 py-2 text-xs shadow-lg dark:border-slate-700 dark:bg-slate-900">
      <p className="mb-1 font-medium">{label}</p>
      {conMonto.map((p) => (
        <p key={p.name} className="flex justify-between gap-4">
          <span className="flex min-w-0 items-center gap-1.5">
            <span
              aria-hidden
              className="h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: p.color }}
            />
            <span className="truncate">{p.name}</span>
          </span>
          <span className="tabular">{formatearCLP(p.value)}</span>
        </p>
      ))}
      <p className="mt-1 flex justify-between gap-4 border-t border-slate-200 pt-1 font-medium dark:border-slate-700">
        <span>Total</span>
        <span className="tabular">{formatearCLP(total)}</span>
      </p>
    </div>
  );
}

// ── hormiga ──────────────────────────────────────────────────────────────────

function HormigaPanel({
  meses,
  mesActual,
  promedioPrevios,
  variacionMesAnterior,
  variacionPromedio,
  porCategoria,
  totalVentana,
}: {
  meses: MesHormiga[];
  mesActual: MesHormiga | null;
  promedioPrevios: number;
  variacionMesAnterior: number | null;
  variacionPromedio: number | null;
  porCategoria: GastoPorCategoria[];
  totalVentana: number;
}) {
  const conAnio = new Set(meses.map((m) => m.clave.slice(0, 4))).size > 1;
  const datos = meses.map((m) => ({ ...m, etiqueta: etiquetaMes(m.clave, conAnio) }));

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div className="tarjeta">
          <p className="etiqueta">Este mes</p>
          <p className="mt-1 text-xl font-semibold">
            <Moneda monto={mesActual?.total ?? 0} />
          </p>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            {mesActual?.n_movimientos ?? 0} gasto{mesActual?.n_movimientos === 1 ? "" : "s"}
            {mesActual?.porcentaje !== null && mesActual?.porcentaje !== undefined
              ? ` · ${formatearPorcentaje(mesActual.porcentaje)} del gasto del mes`
              : ""}
          </p>
        </div>

        <Comparacion
          titulo="Contra el mes anterior"
          variacion={variacionMesAnterior}
          leyenda="menos que el mes pasado"
          leyendaSube="más que el mes pasado"
        />

        <Comparacion
          titulo="Contra el promedio"
          variacion={variacionPromedio}
          leyenda="bajo el promedio"
          leyendaSube="sobre el promedio"
          nota={`Promedio: ${formatearCLP(promedioPrevios)}`}
        />

        <div className="tarjeta">
          <p className="etiqueta">Acumulado del período</p>
          <p className="mt-1 text-xl font-semibold">
            <Moneda monto={totalVentana} />
          </p>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            en {meses.length} meses de hormigas
          </p>
        </div>
      </div>

      <div className="tarjeta">
        <div className="h-72 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={datos} margin={{ top: 8, right: 8, bottom: 8, left: 8 }}>
              <CartesianGrid
                strokeDasharray="3 3"
                vertical={false}
                className="stroke-slate-200 dark:stroke-slate-800"
              />
              <XAxis
                dataKey="etiqueta"
                tick={{ fontSize: 11 }}
                stroke="currentColor"
                className="text-slate-500"
              />
              <YAxis
                tickFormatter={formatearCompacto}
                tick={{ fontSize: 11 }}
                width={60}
                stroke="currentColor"
                className="text-slate-500"
              />
              <Tooltip content={<TooltipHormiga />} cursor={{ fillOpacity: 0.08 }} />
              {promedioPrevios > 0 ? (
                <ReferenceLine
                  y={promedioPrevios}
                  stroke="#f59e0b"
                  strokeDasharray="4 4"
                  label={{
                    value: "promedio",
                    position: "insideTopRight",
                    fontSize: 10,
                    fill: "#f59e0b",
                  }}
                />
              ) : null}
              <Bar dataKey="total" radius={[4, 4, 0, 0]}>
                {datos.map((m, idx) => (
                  <Cell
                    key={m.clave}
                    // El mes que se está viendo va destacado.
                    fill={idx === datos.length - 1 ? "#e11d48" : "#f97316"}
                  />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {porCategoria.length ? (
        <div className="tarjeta">
          <h3 className="mb-3 text-sm font-medium">En qué se fueron este mes</h3>
          <ul className="space-y-2">
            {porCategoria.map((c) => (
              <li key={c.categoria_id ?? "sin"} className="flex justify-between gap-3 text-sm">
                <span className="flex min-w-0 items-center gap-2">
                  <span
                    aria-hidden
                    className="h-2.5 w-2.5 shrink-0 rounded-full"
                    style={{ backgroundColor: c.color ?? COLOR_POR_DEFECTO }}
                  />
                  <span className="truncate">{c.categoria_nombre}</span>
                  <span className="text-xs text-slate-400">{c.n_movimientos}</span>
                </span>
                <Moneda monto={c.total} className="font-medium" />
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </>
  );
}

function Comparacion({
  titulo,
  variacion,
  leyenda,
  leyendaSube,
  nota,
}: {
  titulo: string;
  variacion: number | null;
  leyenda: string;
  leyendaSube: string;
  nota?: string;
}) {
  const sube = (variacion ?? 0) > 0;

  return (
    <div className="tarjeta">
      <p className="etiqueta">{titulo}</p>
      {variacion === null ? (
        <>
          <p className="mt-1 text-xl font-semibold text-slate-400">—</p>
          <p className="text-xs text-slate-500 dark:text-slate-400">Sin base para comparar</p>
        </>
      ) : (
        <>
          <p
            className={cn(
              "mt-1 text-xl font-semibold",
              // En gastos, subir es malo.
              sube ? "text-rose-600 dark:text-rose-400" : "text-emerald-600 dark:text-emerald-400",
            )}
          >
            {sube ? "+" : ""}
            {formatearPorcentaje(variacion)}
          </p>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            {sube ? leyendaSube : leyenda}
            {nota ? ` · ${nota}` : ""}
          </p>
        </>
      )}
    </div>
  );
}

function TooltipHormiga({
  active,
  payload,
}: {
  active?: boolean;
  payload?: Array<{ payload: MesHormiga & { etiqueta: string } }>;
}) {
  if (!active || !payload?.length) return null;

  const m = payload[0].payload;
  return (
    <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-xs shadow-lg dark:border-slate-700 dark:bg-slate-900">
      <p className="font-medium">{m.etiqueta}</p>
      <p className="mt-1 tabular">{formatearCLP(m.total)} en hormigas</p>
      <p className="text-slate-500 dark:text-slate-400">
        {m.n_movimientos} gasto{m.n_movimientos === 1 ? "" : "s"}
        {m.porcentaje !== null ? ` · ${formatearPorcentaje(m.porcentaje)} del mes` : ""}
      </p>
    </div>
  );
}
