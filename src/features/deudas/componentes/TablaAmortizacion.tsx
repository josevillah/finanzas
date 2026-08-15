import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Insignia, type TonoInsignia } from "@/components/ui/Insignia";
import { formatearFecha } from "@/lib/fechas";
import { ETIQUETAS_ESTADO_CUOTA, type Cuota, type EstadoCuota } from "@/types/dominio";

const TONO_CUOTA: Record<EstadoCuota, TonoInsignia> = {
  pendiente: "neutro",
  pagada: "verde",
  atrasada: "rojo",
};

interface Props {
  cuotas: Cuota[];
  conInteres: boolean;
  onPagar: (cuota: Cuota) => void;
  onDeshacer: (cuota: Cuota) => void;
  ocupado?: boolean;
}

export function TablaAmortizacion({ cuotas, conInteres, onPagar, onDeshacer, ocupado }: Props) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500 dark:border-slate-800 dark:text-slate-400">
            <th className="py-2 pr-3 font-medium">N°</th>
            <th className="py-2 pr-3 font-medium">Vence</th>
            <th className="py-2 pr-3 text-right font-medium">Cuota</th>
            {conInteres ? (
              <>
                <th className="py-2 pr-3 text-right font-medium">Capital</th>
                <th className="py-2 pr-3 text-right font-medium">Interés</th>
              </>
            ) : null}
            <th className="py-2 pr-3 font-medium">Estado</th>
            <th className="py-2 pr-3 text-right font-medium">Pagado</th>
            <th className="py-2 text-right font-medium">Acción</th>
          </tr>
        </thead>

        <tbody>
          {cuotas.map((c) => (
            <tr
              key={c.id}
              className="border-b border-slate-100 last:border-0 hover:bg-slate-50 dark:border-slate-800/70 dark:hover:bg-slate-800/40"
            >
              <td className="py-2 pr-3 tabular text-slate-500 dark:text-slate-400">{c.numero}</td>
              <td className="py-2 pr-3 whitespace-nowrap">{formatearFecha(c.fecha_vencimiento)}</td>
              <td className="py-2 pr-3 text-right">
                <Moneda monto={c.monto} />
              </td>

              {conInteres ? (
                <>
                  <td className="py-2 pr-3 text-right">
                    <Moneda monto={c.capital} atenuado />
                  </td>
                  <td className="py-2 pr-3 text-right">
                    <Moneda monto={c.interes} atenuado />
                  </td>
                </>
              ) : null}

              <td className="py-2 pr-3">
                <Insignia tono={TONO_CUOTA[c.estado]}>{ETIQUETAS_ESTADO_CUOTA[c.estado]}</Insignia>
              </td>

              <td className="py-2 pr-3 text-right">
                {c.estado === "pagada" ? (
                  <span className="whitespace-nowrap">
                    <Moneda monto={c.monto_pagado ?? c.monto} />
                    <span className="ml-2 text-xs text-slate-400">
                      {formatearFecha(c.fecha_pago)}
                    </span>
                  </span>
                ) : (
                  <span className="text-slate-400">—</span>
                )}
              </td>

              <td className="py-2 text-right">
                {c.estado === "pagada" ? (
                  <Boton
                    variante="fantasma"
                    tamano="sm"
                    disabled={ocupado}
                    onClick={() => onDeshacer(c)}
                  >
                    Deshacer
                  </Boton>
                ) : (
                  <Boton
                    variante="secundario"
                    tamano="sm"
                    disabled={ocupado}
                    onClick={() => onPagar(c)}
                  >
                    Pagar
                  </Boton>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
