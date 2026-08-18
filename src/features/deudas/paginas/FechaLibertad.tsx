import { Link } from "react-router-dom";

import { Moneda } from "@/components/Moneda";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { describirMeses, formatearFecha, formatearMesTitulo, partirISO } from "@/lib/fechas";

import { useFechaLibertad } from "../hooks";

export function FechaLibertad() {
  const { data, isPending, error, refetch } = useFechaLibertad();

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;
  if (!data) return null;

  const partes = data.fecha_ultima_cuota ? partirISO(data.fecha_ultima_cuota) : null;

  if (!data.fecha_ultima_cuota || !partes) {
    return (
      <div className="space-y-6">
        <Encabezado />
        <Vacio
          titulo="No tienes cuotas pendientes"
          descripcion="Ya estás libre de deudas vigentes. Cuando registres una nueva, acá verás cuándo termina."
        />
      </div>
    );
  }

  // Lo que se libera acumulado: al terminar cada deuda se suma su cuota mensual.
  let acumulado = 0;

  return (
    <div className="space-y-6">
      <Encabezado />

      <div className="tarjeta bg-gradient-to-br from-indigo-600 to-violet-600 text-white dark:from-indigo-700 dark:to-violet-800">
        <p className="text-xs font-medium uppercase tracking-wide text-indigo-200">
          Última cuota vigente
        </p>
        <p className="mt-1 text-3xl font-semibold">
          {formatearMesTitulo(partes.anio, partes.mes)}
        </p>
        <p className="mt-2 text-sm text-indigo-100">
          Te quedan {describirMeses(data.meses_restantes)} y{" "}
          <strong className="tabular">
            <Moneda monto={data.total_pendiente} className="text-white" />
          </strong>{" "}
          por pagar en {data.liberaciones.length} deuda
          {data.liberaciones.length === 1 ? "" : "s"}.
        </p>
      </div>

      <div className="tarjeta">
        <h2 className="mb-1 font-medium">Cuánto se libera y cuándo</h2>
        <p className="mb-4 text-sm text-slate-500 dark:text-slate-400">
          Ordenado por la deuda que termina antes. La columna acumulada es el alivio mensual total
          una vez que esa deuda y todas las anteriores estén pagadas.
        </p>

        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500 dark:border-slate-800 dark:text-slate-400">
                <th className="py-2 pr-3 font-medium">Deuda</th>
                <th className="py-2 pr-3 font-medium">Termina</th>
                <th className="py-2 pr-3 text-right font-medium">Cuotas</th>
                <th className="py-2 pr-3 text-right font-medium">Libera al mes</th>
                <th className="py-2 text-right font-medium">Acumulado</th>
              </tr>
            </thead>
            <tbody>
              {data.liberaciones.map((l) => {
                acumulado += l.monto_mensual_liberado;
                return (
                  <tr
                    key={l.deuda_id}
                    className="border-b border-slate-100 last:border-0 dark:border-slate-800/70"
                  >
                    <td className="py-2.5 pr-3">
                      <Link
                        to={`/deudas/${l.deuda_id}`}
                        className="font-medium hover:text-indigo-600 dark:hover:text-indigo-400"
                      >
                        {l.descripcion}
                      </Link>
                    </td>
                    <td className="py-2.5 pr-3 whitespace-nowrap">
                      {formatearFecha(l.fecha_ultima_cuota)}
                    </td>
                    <td className="py-2.5 pr-3 text-right tabular text-slate-500 dark:text-slate-400">
                      {l.cuotas_restantes}
                    </td>
                    <td className="py-2.5 pr-3 text-right">
                      <Moneda monto={l.monto_mensual_liberado} />
                    </td>
                    <td className="py-2.5 text-right font-medium text-emerald-600 dark:text-emerald-400">
                      <Moneda monto={acumulado} className="text-emerald-600 dark:text-emerald-400" />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

function Encabezado() {
  return (
    <header>
      <h1 className="text-xl font-semibold">Fecha de libertad</h1>
      <p className="text-sm text-slate-500 dark:text-slate-400">
        Cuándo terminas de pagar y cuánta plata mensual recuperas en el camino.
      </p>
    </header>
  );
}
