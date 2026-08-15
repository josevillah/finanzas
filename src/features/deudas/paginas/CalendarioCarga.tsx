import { useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { Moneda } from "@/components/Moneda";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { MESES_CORTOS } from "@/lib/fechas";
import { formatearCLP, formatearCompacto } from "@/lib/moneda";
import type { MesCarga } from "@/types/dominio";

import { useCalendarioCarga } from "../hooks";

const RANGOS = [12, 24, 36];

export function CalendarioCarga() {
  const [meses, setMeses] = useState(24);
  const { data, isPending, error, refetch } = useCalendarioCarga(meses);

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;

  const datos = data ?? [];
  const hayCarga = datos.some((m) => m.total > 0);
  const maximo = Math.max(...datos.map((m) => m.total), 0);
  const promedio = hayCarga
    ? Math.round(datos.reduce((acc, m) => acc + m.total, 0) / datos.length)
    : 0;
  const mesPeak = datos.find((m) => m.total === maximo);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Calendario de carga</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Cuánto tienes comprometido en cuotas, mes a mes.
          </p>
        </div>

        <div className="flex gap-1">
          {RANGOS.map((r) => (
            <button
              key={r}
              type="button"
              onClick={() => setMeses(r)}
              className={
                meses === r
                  ? "rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white"
                  : "rounded-lg px-3 py-1.5 text-xs font-medium text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
              }
            >
              {r} meses
            </button>
          ))}
        </div>
      </header>

      {!hayCarga ? (
        <Vacio
          titulo="No hay cuotas comprometidas"
          descripcion="Cuando registres deudas vigentes, acá verás la carga mensual de los próximos meses."
        />
      ) : (
        <>
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="tarjeta">
              <p className="etiqueta">Mes más pesado</p>
              <p className="mt-1 text-xl font-semibold">
                <Moneda monto={maximo} />
              </p>
              {mesPeak ? (
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {MESES_CORTOS[mesPeak.mes - 1]} {mesPeak.anio}
                </p>
              ) : null}
            </div>
            <div className="tarjeta">
              <p className="etiqueta">Promedio mensual</p>
              <p className="mt-1 text-xl font-semibold">
                <Moneda monto={promedio} />
              </p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                sobre los próximos {meses} meses
              </p>
            </div>
            <div className="tarjeta">
              <p className="etiqueta">Total del período</p>
              <p className="mt-1 text-xl font-semibold">
                <Moneda monto={datos.reduce((acc, m) => acc + m.total, 0)} />
              </p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {datos.reduce((acc, m) => acc + m.n_cuotas, 0)} cuotas
              </p>
            </div>
          </div>

          <div className="tarjeta">
            <div className="h-80 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={datos} margin={{ top: 8, right: 8, bottom: 8, left: 8 }}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-slate-200 dark:stroke-slate-800" vertical={false} />
                  <XAxis
                    dataKey="clave"
                    tickFormatter={(clave: string) => {
                      const [, mes] = clave.split("-");
                      return MESES_CORTOS[Number(mes) - 1];
                    }}
                    tick={{ fontSize: 11 }}
                    interval={meses > 24 ? 2 : meses > 12 ? 1 : 0}
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
                  <Tooltip content={<TooltipCarga />} cursor={{ fillOpacity: 0.08 }} />
                  <Bar dataKey="total" radius={[4, 4, 0, 0]}>
                    {datos.map((m) => (
                      <Cell
                        key={m.clave}
                        // El mes más pesado se destaca para que salte a la vista.
                        fill={m.total === maximo && maximo > 0 ? "#e11d48" : "#6366f1"}
                      />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function TooltipCarga({ active, payload }: { active?: boolean; payload?: Array<{ payload: MesCarga }> }) {
  if (!active || !payload?.length) return null;

  const m = payload[0].payload;
  return (
    <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-xs shadow-lg dark:border-slate-700 dark:bg-slate-900">
      <p className="font-medium">
        {MESES_CORTOS[m.mes - 1]} {m.anio}
      </p>
      <p className="mt-1 tabular">{formatearCLP(m.total)} comprometido</p>
      {m.total_pendiente !== m.total ? (
        <p className="tabular text-slate-500 dark:text-slate-400">
          {formatearCLP(m.total_pendiente)} sin pagar
        </p>
      ) : null}
      <p className="text-slate-500 dark:text-slate-400">
        {m.n_cuotas} cuota{m.n_cuotas === 1 ? "" : "s"}
      </p>
    </div>
  );
}
