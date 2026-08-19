import { Moneda } from "@/components/Moneda";
import { formatearCLP } from "@/lib/moneda";
import type { ResumenMetas } from "@/types/dominio";

function Dato({
  etiqueta,
  monto,
  detalle,
  destacado,
}: {
  etiqueta: string;
  monto: number;
  detalle?: string;
  destacado?: boolean;
}) {
  return (
    <div>
      <p className="text-xs text-slate-500 dark:text-slate-400">{etiqueta}</p>
      <Moneda
        monto={monto}
        className={destacado ? "text-xl font-semibold" : "text-lg font-medium"}
      />
      {detalle ? (
        <p className="text-xs text-slate-500 dark:text-slate-400">{detalle}</p>
      ) : null}
    </div>
  );
}

/** "5 meses" / "1 mes". */
function meses(n: number): string {
  return n === 1 ? "1 mes" : `${n} meses`;
}

/**
 * Los objetivos activos contra la plata que existe. Es la pregunta que el
 * conjunto tiene que responder: si todo esto junto es alcanzable o no.
 */
export function TotalesMetas({ resumen }: { resumen: ResumenMetas }) {
  const {
    total_objetivo,
    total_acumulado,
    total_falta,
    total_ahorrado,
    ahorro_sin_meta,
    balance_promedio,
    meses_al_ritmo,
    meses_considerados,
    n_activas,
  } = resumen;

  return (
    <section className="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Dato
          etiqueta={n_activas === 1 ? "1 meta activa" : `${n_activas} metas activas`}
          monto={total_objetivo}
          detalle="sumando sus objetivos"
          destacado
        />
        <Dato etiqueta="Ya reunido" monto={total_acumulado} detalle="en las cuentas vinculadas" />
        <Dato etiqueta="Falta" monto={total_falta} destacado />
        <Dato
          etiqueta="Total ahorrado"
          monto={total_ahorrado}
          detalle={
            ahorro_sin_meta > 0
              ? `sin comprometer: ${formatearCLP(ahorro_sin_meta)}`
              : "todo comprometido con alguna meta"
          }
        />
      </div>

      <p className="mt-4 border-t border-slate-100 pt-3 text-sm text-slate-600 dark:border-slate-800 dark:text-slate-400">
        {n_activas === 0 ? (
          <>No tienes metas activas. Las cumplidas y archivadas quedan como historial.</>
        ) : total_falta === 0 ? (
          <>Ya tienes apartado todo lo que piden tus metas activas.</>
        ) : balance_promedio === null ? (
          <>
            Todavía no hay meses cerrados con movimientos para estimar cuánto tomaría llegar.
            Después de un par de meses de uso aparece la proyección.
          </>
        ) : balance_promedio <= 0 ? (
          <>
            En los últimos {meses(meses_considerados)} el mes cerró con un balance promedio de{" "}
            <Moneda monto={balance_promedio} className="font-medium" />, así que{" "}
            <strong className="font-medium">al ritmo actual no se alcanza</strong>: no está
            sobrando plata para apartar.
          </>
        ) : (
          <>
            Con un balance promedio de{" "}
            <Moneda monto={balance_promedio} className="font-medium" /> al mes (últimos{" "}
            {meses(meses_considerados)}), juntar lo que falta tomaría{" "}
            <strong className="font-medium">
              {meses_al_ritmo === null ? "—" : meses(meses_al_ritmo)}
            </strong>{" "}
            destinando todo el excedente a estas metas.
          </>
        )}
      </p>
    </section>
  );
}
