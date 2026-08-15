import { Link } from "react-router-dom";

import { BarraAvance } from "@/components/BarraAvance";
import { Moneda } from "@/components/Moneda";
import {
  DETALLE_SEMAFORO,
  PuntoSemaforo,
  TEXTO_SEMAFORO,
  TONO_SEMAFORO,
} from "@/components/SemaforoCarga";
import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { EditorIngresos } from "@/features/mes/componentes/EditorIngresos";
import { SelectorMes, useMes } from "@/features/mes/MesContexto";
import { formatearFecha, formatearMesLargo } from "@/lib/fechas";
import { formatearPorcentaje } from "@/lib/moneda";

import { useCargaFinanciera, useCuotasMes } from "../hooks";

export function CargaFinanciera() {
  const { anio, mes } = useMes();

  const carga = useCargaFinanciera(anio, mes);
  const cuotas = useCuotasMes(anio, mes);

  if (carga.isPending) return <Cargando />;
  if (carga.error) return <ErrorCarga error={carga.error} onReintentar={carga.refetch} />;
  if (!carga.data) return null;

  const c = carga.data;
  const tono = TONO_SEMAFORO[c.semaforo];

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Carga financiera</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Qué parte de tu sueldo líquido se va en cuotas.
          </p>
        </div>
        <SelectorMes />
      </header>

      <div className="tarjeta">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <p className="etiqueta">Cuotas del mes sobre sueldo líquido</p>
            <p className="mt-1 flex items-baseline gap-3">
              <span className="text-4xl font-semibold tabular">
                {formatearPorcentaje(c.porcentaje)}
              </span>
              <Insignia tono={tono}>
                <PuntoSemaforo semaforo={c.semaforo} className="mr-1.5" />
                {TEXTO_SEMAFORO[c.semaforo]}
              </Insignia>
            </p>
            <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
              {DETALLE_SEMAFORO[c.semaforo]}
            </p>
          </div>

          <div className="text-right">
            <p className="text-sm text-slate-500 dark:text-slate-400">
              <Moneda monto={c.total_cuotas} /> en {c.n_cuotas} cuota
              {c.n_cuotas === 1 ? "" : "s"}
            </p>
            <p className="text-sm text-slate-500 dark:text-slate-400">
              de <Moneda monto={c.sueldo_liquido} /> líquidos
            </p>
          </div>
        </div>

        <div className="mt-4 space-y-1">
          <BarraAvance porcentaje={c.porcentaje ?? 0} tono={tono === "indigo" ? "indigo" : tono} />
          {/* Referencias fijas para leer la barra sin tener que recordar los cortes. */}
          <div className="flex justify-between text-[11px] text-slate-400">
            <span>0%</span>
            <span>15% · sano</span>
            <span>25% · límite</span>
            <span>100%</span>
          </div>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="tarjeta lg:col-span-1">
          <EditorIngresos anio={anio} mes={mes} />

          {c.otros_ingresos > 0 && c.total_cuotas > 0 ? (
            <p className="mt-4 border-t border-slate-200 pt-4 text-xs text-slate-500 dark:border-slate-800 dark:text-slate-400">
              Sobre el ingreso total (
              <Moneda monto={c.sueldo_liquido + c.otros_ingresos} />) la carga sería{" "}
              {formatearPorcentaje(
                (c.total_cuotas / (c.sueldo_liquido + c.otros_ingresos)) * 100,
              )}
              .
            </p>
          ) : null}
        </div>

        <div className="tarjeta lg:col-span-2">
          <h2 className="mb-3 font-medium">Cuotas que vencen este mes</h2>

          {cuotas.isPending ? (
            <Cargando texto="Cargando cuotas…" />
          ) : !cuotas.data?.length ? (
            <p className="py-6 text-center text-sm text-slate-500 dark:text-slate-400">
              No hay cuotas con vencimiento en {formatearMesLargo(anio, mes)}.
            </p>
          ) : (
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {cuotas.data.map((cu) => (
                <li key={cu.id} className="flex items-center justify-between gap-3 py-2.5">
                  <div className="min-w-0">
                    <Link
                      to={`/deudas/${cu.deuda_id}`}
                      className="truncate text-sm font-medium hover:text-indigo-600 dark:hover:text-indigo-400"
                    >
                      {cu.deuda_descripcion}
                    </Link>
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      Cuota {cu.numero} · vence {formatearFecha(cu.fecha_vencimiento)}
                    </p>
                  </div>
                  <div className="flex items-center gap-3">
                    {cu.estado === "atrasada" ? (
                      <Insignia tono="rojo">Atrasada</Insignia>
                    ) : cu.estado === "pagada" ? (
                      <Insignia tono="verde">Pagada</Insignia>
                    ) : null}
                    <Moneda
                      monto={cu.monto}
                      className={
                        cu.estado === "pagada"
                          ? "font-medium line-through opacity-60"
                          : "font-medium"
                      }
                    />
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
