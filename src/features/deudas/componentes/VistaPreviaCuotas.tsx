import { Moneda } from "@/components/Moneda";
import { formatearFecha } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import type { CuotaCalculada } from "@/types/dominio";

interface Props {
  cuotas: CuotaCalculada[] | undefined;
  cargando: boolean;
  error: unknown;
  montoOriginal: number;
}

/** Resumen de lo que se va a guardar, calculado por el backend. */
export function VistaPreviaCuotas({ cuotas, cargando, error, montoOriginal }: Props) {
  if (error) {
    return (
      <div className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
        {mensajeDeError(error)}
      </div>
    );
  }

  if (cargando) {
    return (
      <p className="text-sm text-slate-500 dark:text-slate-400">Calculando cuotas…</p>
    );
  }

  if (!cuotas?.length) {
    return (
      <p className="text-sm text-slate-500 dark:text-slate-400">
        Completa monto, número de cuotas y fecha para ver la simulación.
      </p>
    );
  }

  const total = cuotas.reduce((acc, c) => acc + c.monto, 0);
  const interes = cuotas.reduce((acc, c) => acc + c.interes, 0);
  const primera = cuotas[0];
  const ultima = cuotas[cuotas.length - 1];

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <Dato titulo="Cuota" valor={<Moneda monto={primera.monto} />} />
        <Dato titulo="Total a pagar" valor={<Moneda monto={total} />} />
        <Dato
          titulo="Interés total"
          valor={interes > 0 ? <Moneda monto={interes} /> : <span className="text-slate-400">Sin interés</span>}
        />
        <Dato titulo="Última cuota" valor={<span>{formatearFecha(ultima.fecha_vencimiento)}</span>} />
      </div>

      {total !== montoOriginal && interes === 0 ? (
        <p className="text-xs text-amber-700 dark:text-amber-400">
          El reparto ajusta la última cuota para que la suma calce exactamente con el monto.
        </p>
      ) : null}

      <div className="max-h-52 overflow-y-auto rounded-lg border border-slate-200 dark:border-slate-800">
        <table className="w-full text-xs">
          <tbody>
            {cuotas.map((c) => (
              <tr
                key={c.numero}
                className="border-b border-slate-100 last:border-0 dark:border-slate-800/70"
              >
                <td className="px-3 py-1.5 tabular text-slate-500 dark:text-slate-400">{c.numero}</td>
                <td className="px-3 py-1.5 whitespace-nowrap">
                  {formatearFecha(c.fecha_vencimiento)}
                </td>
                <td className="px-3 py-1.5 text-right">
                  <Moneda monto={c.monto} />
                </td>
                {interes > 0 ? (
                  <td className="px-3 py-1.5 text-right text-slate-500 dark:text-slate-400">
                    int. <Moneda monto={c.interes} />
                  </td>
                ) : null}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Dato({ titulo, valor }: { titulo: string; valor: React.ReactNode }) {
  return (
    <div className="rounded-lg bg-slate-50 px-3 py-2 dark:bg-slate-800/50">
      <p className="etiqueta">{titulo}</p>
      <p className="mt-0.5 text-sm font-semibold">{valor}</p>
    </div>
  );
}
