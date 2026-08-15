import { useEffect, useState } from "react";

import { Moneda } from "@/components/Moneda";
import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo } from "@/components/ui/Campo";
import { MESES } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { useGuardarIngresos, usePeriodo } from "@/features/deudas/hooks";

/**
 * Sueldo líquido y otros ingresos del mes. Se usa tanto en el resumen del
 * período como en la carga financiera, que necesita el sueldo para el semáforo.
 */
export function EditorIngresos({
  anio,
  mes,
  bloqueado,
}: {
  anio: number;
  mes: number;
  bloqueado?: boolean;
}) {
  const periodo = usePeriodo(anio, mes);
  const guardar = useGuardarIngresos();

  const [sueldo, setSueldo] = useState(0);
  const [otros, setOtros] = useState(0);

  // El período llega después del primer render: hay que sincronizar los campos.
  useEffect(() => {
    if (periodo.data) {
      setSueldo(periodo.data.sueldo_liquido);
      setOtros(periodo.data.otros_ingresos);
    }
  }, [periodo.data]);

  const sinCambios =
    periodo.data?.sueldo_liquido === sueldo && periodo.data?.otros_ingresos === otros;

  return (
    <div className="space-y-4">
      <h2 className="font-medium">Ingresos de {MESES[mes - 1]}</h2>

      <Campo etiqueta="Sueldo líquido">
        <MontoInput valor={sueldo} onCambio={setSueldo} disabled={bloqueado} />
      </Campo>

      <Campo etiqueta="Otros ingresos" ayuda="Arriendos, freelance, bonos.">
        <MontoInput valor={otros} onCambio={setOtros} disabled={bloqueado} />
      </Campo>

      <div className="flex items-center justify-between gap-3">
        <span className="text-xs text-slate-500 dark:text-slate-400">
          Total <Moneda monto={sueldo + otros} />
        </span>
        <Boton
          disabled={guardar.isPending || sinCambios || bloqueado}
          onClick={() =>
            guardar.mutate({ anio, mes, sueldoLiquido: sueldo, otrosIngresos: otros })
          }
        >
          {guardar.isPending ? "Guardando…" : sinCambios ? "Sin cambios" : "Guardar"}
        </Boton>
      </div>

      {guardar.error ? (
        <p className="rounded-lg bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(guardar.error)}
        </p>
      ) : null}
    </div>
  );
}
