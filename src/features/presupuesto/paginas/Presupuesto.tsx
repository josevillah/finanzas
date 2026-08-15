import { useEffect, useMemo, useState } from "react";

import { BarraAvance } from "@/components/BarraAvance";
import { Moneda } from "@/components/Moneda";
import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { Insignia, type TonoInsignia } from "@/components/ui/Insignia";
import { SelectorMes, useMes } from "@/features/mes/MesContexto";
import { cn } from "@/lib/cn";
import { formatearMesLargo } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { formatearCLP, formatearPorcentaje } from "@/lib/moneda";
import {
  ETIQUETAS_ESTADO_PRESUPUESTO,
  ETIQUETAS_TIPO_CATEGORIA,
  type AsignacionPresupuesto,
  type EstadoPresupuesto,
  type LineaPresupuesto,
} from "@/types/dominio";

import { useAsignarPresupuesto, useCopiarPresupuesto, useResumenPresupuesto } from "../hooks";

const TONO: Record<EstadoPresupuesto, TonoInsignia> = {
  sin_asignar: "neutro",
  ok: "verde",
  alerta: "amarillo",
  excedido: "rojo",
};

const TONO_BARRA: Record<EstadoPresupuesto, "indigo" | "verde" | "amarillo" | "rojo"> = {
  sin_asignar: "indigo",
  ok: "verde",
  alerta: "amarillo",
  excedido: "rojo",
};

export function Presupuesto() {
  const { anio, mes } = useMes();
  const { data, isPending, error, refetch } = useResumenPresupuesto(anio, mes);

  const asignar = useAsignarPresupuesto();
  const copiar = useCopiarPresupuesto();

  /** Montos editados en pantalla, indexados por categoría. */
  const [borrador, setBorrador] = useState<Record<number, number>>({});

  // Al cambiar de mes o al recargar, el borrador parte de lo guardado.
  useEffect(() => {
    if (!data) return;
    setBorrador(Object.fromEntries(data.lineas.map((l) => [l.categoria_id, l.monto_asignado])));
  }, [data]);

  const cambios: AsignacionPresupuesto[] = useMemo(() => {
    if (!data) return [];
    return data.lineas
      .filter((l) => (borrador[l.categoria_id] ?? l.monto_asignado) !== l.monto_asignado)
      .map((l) => ({
        categoria_id: l.categoria_id,
        monto_asignado: borrador[l.categoria_id] ?? 0,
      }));
  }, [data, borrador]);

  const totalBorrador = useMemo(
    () => Object.values(borrador).reduce((acc, m) => acc + (m || 0), 0),
    [borrador],
  );

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;
  if (!data) return null;

  const mesAnterior = (() => {
    const total = anio * 12 + (mes - 1) - 1;
    return { anio: Math.floor(total / 12), mes: (total % 12) + 1 };
  })();

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-semibold">
            Presupuesto
            {data.periodo_cerrado ? <Insignia tono="neutro">Mes cerrado</Insignia> : null}
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            {data.periodo_cerrado
              ? "Este mes está cerrado: se puede mirar, pero no reasignar."
              : "Cuánto asignaste a cada categoría y cuánto llevas gastado."}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <SelectorMes />
          <Boton
            variante="secundario"
            disabled={copiar.isPending || data.periodo_cerrado}
            title={`Trae las asignaciones de ${formatearMesLargo(mesAnterior.anio, mesAnterior.mes)}`}
            onClick={() =>
              copiar.mutate({
                desdeAnio: mesAnterior.anio,
                desdeMes: mesAnterior.mes,
                haciaAnio: anio,
                haciaMes: mes,
              })
            }
          >
            {copiar.isPending ? "Copiando…" : "Copiar del mes anterior"}
          </Boton>
        </div>
      </header>

      {copiar.error ? (
        <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(copiar.error)}
        </p>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Tarjeta titulo="Presupuestado" monto={data.total_asignado} />
        <Tarjeta titulo="Gastado" monto={data.total_gastado} />
        <Tarjeta
          titulo="Disponible"
          monto={data.disponible}
          tono={data.disponible >= 0 ? "verde" : "rojo"}
          nota={
            data.porcentaje_usado !== null
              ? `${formatearPorcentaje(data.porcentaje_usado)} consumido`
              : undefined
          }
        />
        <Tarjeta
          titulo="Fuera del presupuesto"
          monto={data.gasto_sin_presupuestar}
          nota="Gasto en categorías sin asignación"
        />
      </div>

      {data.total_asignado > 0 ? (
        <div className="tarjeta space-y-2">
          <div className="flex flex-wrap justify-between gap-2 text-sm">
            <span className="font-medium">Avance del mes</span>
            <span className="text-slate-500 dark:text-slate-400">
              <Moneda monto={data.total_gastado} /> de{" "}
              <Moneda monto={data.total_asignado} /> asignados
              {data.categorias_excedidas > 0 ? (
                <>
                  {" · "}
                  <span className="text-rose-600 dark:text-rose-400">
                    {data.categorias_excedidas} categoría
                    {data.categorias_excedidas === 1 ? "" : "s"} excedida
                    {data.categorias_excedidas === 1 ? "" : "s"}
                  </span>
                </>
              ) : null}
            </span>
          </div>
          <BarraAvance
            porcentaje={data.porcentaje_usado ?? 0}
            tono={
              (data.porcentaje_usado ?? 0) > 100
                ? "rojo"
                : (data.porcentaje_usado ?? 0) >= 80
                  ? "amarillo"
                  : "verde"
            }
          />
          {data.total_ingresos > 0 ? (
            <p className="text-xs text-slate-500 dark:text-slate-400">
              Sobre ingresos de <Moneda monto={data.total_ingresos} />, te quedan{" "}
              <Moneda monto={data.sin_asignar_del_ingreso} /> sin asignar a ninguna categoría.
            </p>
          ) : null}
        </div>
      ) : null}

      {!data.lineas.length ? (
        <Vacio
          titulo="No hay categorías activas"
          descripcion="Crea categorías para poder asignarles presupuesto."
        />
      ) : (
        <div className="tarjeta p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500 dark:border-slate-800 dark:text-slate-400">
                  <th className="px-4 py-2 font-medium">Categoría</th>
                  <th className="px-3 py-2 text-right font-medium">Presupuesto</th>
                  <th className="px-3 py-2 text-right font-medium">Gastado</th>
                  <th className="px-3 py-2 text-right font-medium">Disponible</th>
                  <th className="px-4 py-2 font-medium">Avance</th>
                </tr>
              </thead>
              <tbody>
                {data.lineas.map((l) => (
                  <Fila
                    key={l.categoria_id}
                    linea={l}
                    valor={borrador[l.categoria_id] ?? l.monto_asignado}
                    bloqueado={data.periodo_cerrado}
                    onCambio={(v) =>
                      setBorrador((b) => ({ ...b, [l.categoria_id]: v }))
                    }
                  />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* La barra de guardado aparece solo cuando hay algo distinto que guardar. */}
      {cambios.length ? (
        <div className="sticky bottom-4 z-30 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-indigo-200 bg-white px-4 py-3 shadow-lg dark:border-indigo-900 dark:bg-slate-900">
          <span className="text-sm">
            {cambios.length} categoría{cambios.length === 1 ? "" : "s"} modificada
            {cambios.length === 1 ? "" : "s"} · nuevo total{" "}
            <strong className="tabular">{formatearCLP(totalBorrador)}</strong>
          </span>
          <span className="flex gap-2">
            <Boton
              variante="secundario"
              onClick={() =>
                setBorrador(
                  Object.fromEntries(data.lineas.map((l) => [l.categoria_id, l.monto_asignado])),
                )
              }
            >
              Descartar
            </Boton>
            <Boton
              disabled={asignar.isPending}
              onClick={() => asignar.mutate({ anio, mes, asignaciones: cambios })}
            >
              {asignar.isPending ? "Guardando…" : "Guardar presupuesto"}
            </Boton>
          </span>
        </div>
      ) : null}

      {asignar.error ? (
        <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(asignar.error)}
        </p>
      ) : null}

      <p className="text-xs text-slate-400">
        Un presupuesto en $0 quita la categoría del control. Los pagos de cuotas entran en la
        categoría "Deudas y créditos", así que presupuestarla incluye tus cuotas del mes.
      </p>
    </div>
  );
}

function Tarjeta({
  titulo,
  monto,
  nota,
  tono,
}: {
  titulo: string;
  monto: number;
  nota?: string;
  tono?: "verde" | "rojo";
}) {
  return (
    <div className="tarjeta">
      <p className="etiqueta">{titulo}</p>
      <p
        className={cn(
          "mt-1 text-xl font-semibold",
          tono === "verde" && "text-emerald-600 dark:text-emerald-400",
          tono === "rojo" && "text-rose-600 dark:text-rose-400",
        )}
      >
        <Moneda monto={monto} />
      </p>
      {nota ? <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{nota}</p> : null}
    </div>
  );
}

function Fila({
  linea,
  valor,
  bloqueado,
  onCambio,
}: {
  linea: LineaPresupuesto;
  valor: number;
  bloqueado: boolean;
  onCambio: (valor: number) => void;
}) {
  const disponible = valor - linea.monto_gastado;
  const porcentaje = valor > 0 ? (linea.monto_gastado / valor) * 100 : null;

  // El estado se recalcula con el borrador para que la fila reaccione mientras
  // se escribe, sin esperar el guardado.
  const estado: EstadoPresupuesto =
    porcentaje === null
      ? "sin_asignar"
      : porcentaje > 100
        ? "excedido"
        : porcentaje >= 80
          ? "alerta"
          : "ok";

  return (
    <tr className="border-b border-slate-100 last:border-0 dark:border-slate-800/70">
      <td className="px-4 py-2.5">
        <span className="flex items-center gap-2">
          <span
            aria-hidden
            className="h-2.5 w-2.5 shrink-0 rounded-full"
            style={{ backgroundColor: linea.color ?? "#94a3b8" }}
          />
          <span className="truncate">{linea.categoria_nombre}</span>
          <span className="text-xs text-slate-400">
            {ETIQUETAS_TIPO_CATEGORIA[linea.categoria_tipo]}
          </span>
        </span>
      </td>

      <td className="px-3 py-2 text-right">
        <div className="ml-auto w-36">
          <MontoInput valor={valor} onCambio={onCambio} disabled={bloqueado} />
        </div>
      </td>

      <td className="px-3 py-2.5 text-right">
        <Moneda monto={linea.monto_gastado} atenuado={linea.monto_gastado === 0} />
        {linea.n_movimientos > 0 ? (
          <span className="ml-2 text-xs text-slate-400">{linea.n_movimientos}</span>
        ) : null}
      </td>

      <td className="px-3 py-2.5 text-right">
        {valor > 0 ? (
          <span
            className={cn(
              "tabular font-medium",
              disponible < 0 && "text-rose-600 dark:text-rose-400",
            )}
          >
            {formatearCLP(disponible)}
          </span>
        ) : (
          <span className="text-slate-400">—</span>
        )}
      </td>

      <td className="w-56 px-4 py-2.5">
        {porcentaje === null ? (
          <Insignia tono={TONO[estado]}>{ETIQUETAS_ESTADO_PRESUPUESTO[estado]}</Insignia>
        ) : (
          <div className="space-y-1">
            <BarraAvance porcentaje={porcentaje} tono={TONO_BARRA[estado]} />
            <span className="text-xs text-slate-500 dark:text-slate-400">
              {formatearPorcentaje(porcentaje)}
            </span>
          </div>
        )}
      </td>
    </tr>
  );
}
