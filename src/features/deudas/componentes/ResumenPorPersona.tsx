import { Moneda } from "@/components/Moneda";
import { Insignia } from "@/components/ui/Insignia";
import { formatearFecha } from "@/lib/fechas";
import type { ResumenTerceros } from "@/types/dominio";

/**
 * Cuánto me debe cada persona, sumando todas sus deudas.
 *
 * Es el equivalente de "Mis deudas" para el otro lado, pero agrupado por quien
 * debe en vez de por deuda: lo que uno quiere saber es a quién cobrarle.
 */
export function ResumenPorPersona({ resumen }: { resumen: ResumenTerceros }) {
  if (!resumen.deudores.length) return null;

  return (
    <div className="space-y-3">
      <div className="grid gap-3 sm:grid-cols-3">
        <div className="tarjeta">
          <p className="etiqueta">Te deben en total</p>
          <p className="mt-1 text-2xl font-semibold">
            <Moneda monto={resumen.total_pendiente} />
          </p>
        </div>

        <div className="tarjeta">
          <p className="etiqueta">Ya te pagaron</p>
          <p className="mt-1 text-2xl font-semibold text-emerald-600 dark:text-emerald-400">
            <Moneda monto={resumen.total_cobrado} />
          </p>
        </div>

        <div className="tarjeta">
          <p className="etiqueta">Cuotas atrasadas</p>
          <p
            className={
              resumen.cuotas_atrasadas > 0
                ? "mt-1 text-2xl font-semibold text-rose-600 dark:text-rose-400"
                : "mt-1 text-2xl font-semibold"
            }
          >
            {resumen.cuotas_atrasadas}
          </p>
        </div>
      </div>

      <div className="tarjeta p-0">
        <ul className="divide-y divide-slate-100 dark:divide-slate-800">
          {resumen.deudores.map((d) => (
            <li
              key={d.deudor}
              className="flex flex-wrap items-center justify-between gap-3 px-4 py-3"
            >
              <div className="min-w-0">
                <p className="flex items-center gap-2 text-sm font-medium">
                  {d.deudor}
                  {d.cuotas_atrasadas > 0 ? (
                    <Insignia tono="rojo">
                      {d.cuotas_atrasadas} atrasada{d.cuotas_atrasadas > 1 ? "s" : ""}
                    </Insignia>
                  ) : null}
                </p>
                <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
                  {d.n_deudas} deuda{d.n_deudas === 1 ? "" : "s"} · {d.cuotas_pendientes} cuota
                  {d.cuotas_pendientes === 1 ? "" : "s"} por cobrar
                  {d.proxima_fecha ? ` · próxima el ${formatearFecha(d.proxima_fecha)}` : ""}
                </p>
              </div>

              <div className="text-right">
                <p className="text-lg font-semibold">
                  <Moneda monto={d.total_pendiente} />
                </p>
                <p className="text-xs text-slate-500 dark:text-slate-400">por cobrar</p>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
