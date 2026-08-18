import { Cargando, ErrorCarga } from "@/components/ui/Estados";

import { Ahorros } from "../componentes/Ahorros";
import { TarjetaSaldo } from "../componentes/TarjetaSaldo";
import { useResumenCuentas } from "../hooks";

/**
 * Cuánta plata hay y cuánta está apartada. No tiene selector de mes: el
 * patrimonio es acumulativo desde siempre, no de un período.
 */
export function Cuentas() {
  const { data, isPending, error, refetch } = useResumenCuentas();

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;

  return (
    <div className="space-y-8">
      <header>
        <h1 className="text-xl font-semibold tracking-tight">Cuentas</h1>
        <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
          Cuánta plata tienes hoy y cuánta tienes apartada.
        </p>
      </header>

      <TarjetaSaldo
        disponible={data.disponible}
        patrimonio={data.patrimonio}
        desglose={data.desglose}
      />

      <Ahorros
        ahorros={data.ahorros}
        disponible={data.disponible}
        total={data.total_ahorrado}
      />
    </div>
  );
}
