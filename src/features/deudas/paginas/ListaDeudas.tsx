import { useState } from "react";
import { Link } from "react-router-dom";

import { BarraAvance } from "@/components/BarraAvance";
import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Insignia } from "@/components/ui/Insignia";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { formatearFecha } from "@/lib/fechas";
import { formatearPorcentaje } from "@/lib/moneda";
import {
  ETIQUETAS_ESTADO_DEUDA,
  ETIQUETAS_TIPO_DEUDA,
  type DeudaResumen,
  type EstadoDeuda,
  type NuevaDeuda,
} from "@/types/dominio";

import { FormularioDeuda } from "../componentes/FormularioDeuda";
import { useCrearDeuda, useDeudas } from "../hooks";

const FILTROS: Array<{ valor: EstadoDeuda | null; etiqueta: string }> = [
  { valor: "vigente", etiqueta: "Vigentes" },
  { valor: "pagada", etiqueta: "Pagadas" },
  { valor: "repactada", etiqueta: "Repactadas" },
  { valor: null, etiqueta: "Todas" },
];

export function ListaDeudas() {
  const [filtro, setFiltro] = useState<EstadoDeuda | null>("vigente");
  const [formAbierto, setFormAbierto] = useState(false);

  const { data: deudas, isPending, error, refetch } = useDeudas(filtro);
  const crear = useCrearDeuda();

  const guardar = (datos: NuevaDeuda) => {
    crear.mutate(datos, { onSuccess: () => setFormAbierto(false) });
  };

  const abrirForm = () => {
    crear.reset();
    setFormAbierto(true);
  };

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Deudas</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Todo lo que debes, con su avance real.
          </p>
        </div>
        <Boton onClick={abrirForm}>+ Nueva deuda</Boton>
      </header>

      <div className="flex flex-wrap gap-1">
        {FILTROS.map((f) => (
          <button
            key={f.etiqueta}
            type="button"
            onClick={() => setFiltro(f.valor)}
            className={
              filtro === f.valor
                ? "rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white"
                : "rounded-lg px-3 py-1.5 text-xs font-medium text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            }
          >
            {f.etiqueta}
          </button>
        ))}
      </div>

      {isPending ? (
        <Cargando />
      ) : error ? (
        <ErrorCarga error={error} onReintentar={refetch} />
      ) : !deudas?.length ? (
        <Vacio
          titulo="No hay deudas en este filtro"
          descripcion="Cuando registres una deuda, sus cuotas se generan automáticamente y aparecen acá."
          accion={<Boton onClick={abrirForm}>Registrar la primera deuda</Boton>}
        />
      ) : (
        <>
          <ResumenGlobal deudas={deudas} />
          <div className="space-y-3">
            {deudas.map((d) => (
              <TarjetaDeuda key={d.id} deuda={d} />
            ))}
          </div>
        </>
      )}

      <FormularioDeuda
        abierto={formAbierto}
        onCerrar={() => setFormAbierto(false)}
        onGuardar={guardar}
        guardando={crear.isPending}
        error={crear.error}
      />
    </div>
  );
}

function ResumenGlobal({ deudas }: { deudas: DeudaResumen[] }) {
  const pendiente = deudas.reduce((acc, d) => acc + d.monto_pendiente, 0);
  const pagado = deudas.reduce((acc, d) => acc + d.monto_pagado, 0);
  const atrasadas = deudas.reduce((acc, d) => acc + d.cuotas_atrasadas, 0);

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <div className="tarjeta">
        <p className="etiqueta">Falta por pagar</p>
        <p className="mt-1 text-2xl font-semibold">
          <Moneda monto={pendiente} />
        </p>
      </div>
      <div className="tarjeta">
        <p className="etiqueta">Ya pagado</p>
        <p className="mt-1 text-2xl font-semibold text-emerald-600 dark:text-emerald-400">
          <Moneda monto={pagado} />
        </p>
      </div>
      <div className="tarjeta">
        <p className="etiqueta">Cuotas atrasadas</p>
        <p
          className={
            atrasadas > 0
              ? "mt-1 text-2xl font-semibold text-rose-600 dark:text-rose-400"
              : "mt-1 text-2xl font-semibold"
          }
        >
          {atrasadas}
        </p>
      </div>
    </div>
  );
}

function TarjetaDeuda({ deuda }: { deuda: DeudaResumen }) {
  return (
    <Link
      to={`/deudas/${deuda.id}`}
      className="tarjeta block transition-colors hover:border-indigo-300 dark:hover:border-indigo-800"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="truncate font-medium">{deuda.descripcion}</h2>
            {deuda.estado !== "vigente" ? (
              <Insignia tono={deuda.estado === "pagada" ? "verde" : "neutro"}>
                {ETIQUETAS_ESTADO_DEUDA[deuda.estado]}
              </Insignia>
            ) : null}
            {deuda.cuotas_atrasadas > 0 ? (
              <Insignia tono="rojo">
                {deuda.cuotas_atrasadas} atrasada{deuda.cuotas_atrasadas > 1 ? "s" : ""}
              </Insignia>
            ) : null}
          </div>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
            {ETIQUETAS_TIPO_DEUDA[deuda.tipo]}
            {deuda.institucion ? ` · ${deuda.institucion}` : ""}
            {deuda.proxima_cuota
              ? ` · próxima el ${formatearFecha(deuda.proxima_cuota.fecha_vencimiento)}`
              : ""}
          </p>
        </div>

        <div className="text-right">
          <p className="text-lg font-semibold">
            <Moneda monto={deuda.monto_pendiente} />
          </p>
          <p className="text-xs text-slate-500 dark:text-slate-400">pendiente</p>
        </div>
      </div>

      <div className="mt-4 space-y-1.5">
        <BarraAvance porcentaje={deuda.avance_pct} tono={deuda.estado === "pagada" ? "verde" : "indigo"} />
        <div className="flex justify-between text-xs text-slate-500 dark:text-slate-400">
          <span>
            {deuda.cuotas_pagadas} de {deuda.cuotas_totales} cuotas ·{" "}
            {formatearPorcentaje(deuda.avance_pct)} pagado
          </span>
          <span className="tabular">
            <Moneda monto={deuda.monto_pagado} atenuado /> de{" "}
            <Moneda monto={deuda.total_programado} atenuado />
          </span>
        </div>
      </div>
    </Link>
  );
}
