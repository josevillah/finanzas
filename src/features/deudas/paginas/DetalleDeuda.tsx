import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { BarraAvance } from "@/components/BarraAvance";
import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Modal } from "@/components/ui/Modal";
import { formatearFecha } from "@/lib/fechas";
import { formatearPorcentaje, formatearTasa } from "@/lib/moneda";
import {
  ETIQUETAS_ESTADO_DEUDA,
  ETIQUETAS_TIPO_DEUDA,
  type Cuota,
  type NuevaDeuda,
} from "@/types/dominio";

import { DialogoPagarCuota } from "../componentes/DialogoPagarCuota";
import { FormularioDeuda } from "../componentes/FormularioDeuda";
import { TablaAmortizacion } from "../componentes/TablaAmortizacion";
import {
  useActualizarDeuda,
  useCambiarEstadoDeuda,
  useDeshacerPago,
  useDeuda,
  useEliminarDeuda,
  usePagarCuota,
} from "../hooks";

export function DetalleDeuda() {
  const { id } = useParams<{ id: string }>();
  const deudaId = Number(id);
  const navegar = useNavigate();

  const { data, isPending, error, refetch } = useDeuda(deudaId);

  const [cuotaAPagar, setCuotaAPagar] = useState<Cuota | null>(null);
  const [editando, setEditando] = useState(false);
  const [confirmandoBorrado, setConfirmandoBorrado] = useState(false);

  const pagar = usePagarCuota();
  const deshacer = useDeshacerPago();
  const actualizar = useActualizarDeuda();
  const eliminar = useEliminarDeuda();
  const cambiarEstado = useCambiarEstadoDeuda();

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;
  if (!data) return null;

  const { resumen, cuotas } = data;
  const deuda = resumen;
  const conInteres = deuda.tasa_mensual > 0;
  const esCobro = deuda.direccion === "tercero";

  const guardarEdicion = (datos: NuevaDeuda) => {
    actualizar.mutate({ id: deudaId, datos }, { onSuccess: () => setEditando(false) });
  };

  const confirmarPago = (pago: { cuota_id: number; fecha_pago: string; monto_pagado: number }) => {
    pagar.mutate(pago, { onSuccess: () => setCuotaAPagar(null) });
  };

  return (
    <div className="space-y-6">
      <div>
        <Link
          to="/deudas"
          className="text-sm text-indigo-600 hover:underline dark:text-indigo-400"
        >
          ← Volver a deudas
        </Link>
      </div>

      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="text-xl font-semibold">{deuda.descripcion}</h1>
            <Insignia tono={deuda.estado === "pagada" ? "verde" : deuda.estado === "vigente" ? "indigo" : "neutro"}>
              {ETIQUETAS_ESTADO_DEUDA[deuda.estado]}
            </Insignia>
          </div>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            {ETIQUETAS_TIPO_DEUDA[deuda.tipo]}
            {deuda.institucion ? ` · ${deuda.institucion}` : ""} · {formatearTasa(deuda.tasa_mensual)}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          <Boton
            variante="secundario"
            onClick={() => {
              actualizar.reset();
              setEditando(true);
            }}
          >
            Editar
          </Boton>
          {deuda.estado === "vigente" ? (
            <Boton
              variante="secundario"
              onClick={() => cambiarEstado.mutate({ id: deudaId, estado: "repactada" })}
              disabled={cambiarEstado.isPending}
            >
              Marcar repactada
            </Boton>
          ) : null}
          <Boton variante="peligro" onClick={() => setConfirmandoBorrado(true)}>
            Eliminar
          </Boton>
        </div>
      </header>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metrica titulo="Monto original" valor={<Moneda monto={deuda.monto_original} />} />
        <Metrica
          titulo="Total a pagar"
          valor={<Moneda monto={deuda.total_programado} />}
          nota={
            conInteres
              ? `Interés total ${formatearPorcentaje(
                  ((deuda.total_programado - deuda.monto_original) / deuda.monto_original) * 100,
                )}`
              : "Sin interés"
          }
        />
        <Metrica
          titulo="Pagado"
          valor={<Moneda monto={deuda.monto_pagado} />}
          nota={`${deuda.cuotas_pagadas} de ${deuda.cuotas_totales} cuotas`}
        />
        <Metrica
          titulo="Pendiente"
          valor={<Moneda monto={deuda.monto_pendiente} />}
          nota={
            deuda.proxima_cuota
              ? `Próxima el ${formatearFecha(deuda.proxima_cuota.fecha_vencimiento)}`
              : "Sin cuotas pendientes"
          }
        />
      </div>

      <div className="tarjeta space-y-2">
        <div className="flex justify-between text-sm">
          <span className="font-medium">Avance</span>
          <span className="tabular">{formatearPorcentaje(deuda.avance_pct)}</span>
        </div>
        <BarraAvance
          porcentaje={deuda.avance_pct}
          tono={deuda.estado === "pagada" ? "verde" : "indigo"}
        />
      </div>

      {deuda.notas ? (
        <div className="tarjeta">
          <p className="etiqueta">Notas</p>
          <p className="mt-1 whitespace-pre-line text-sm">{deuda.notas}</p>
        </div>
      ) : null}

      <div className="tarjeta">
        <h2 className="mb-3 font-medium">Tabla de amortización</h2>
        <TablaAmortizacion
          cuotas={cuotas}
          conInteres={conInteres}
          onPagar={(c) => {
            pagar.reset();
            setCuotaAPagar(c);
          }}
          onDeshacer={(c) => deshacer.mutate(c.id)}
          ocupado={pagar.isPending || deshacer.isPending}
        />
      </div>

      <DialogoPagarCuota
        cuota={cuotaAPagar}
        esCobro={esCobro}
        onCerrar={() => setCuotaAPagar(null)}
        onConfirmar={confirmarPago}
        guardando={pagar.isPending}
        error={pagar.error}
      />

      <FormularioDeuda
        abierto={editando}
        deuda={deuda}
        onCerrar={() => setEditando(false)}
        onGuardar={guardarEdicion}
        guardando={actualizar.isPending}
        error={actualizar.error}
      />

      <Modal
        abierto={confirmandoBorrado}
        ancho="md"
        titulo="Eliminar deuda"
        onCerrar={() => setConfirmandoBorrado(false)}
        acciones={
          <>
            <Boton variante="secundario" onClick={() => setConfirmandoBorrado(false)}>
              Cancelar
            </Boton>
            <Boton
              variante="peligro"
              disabled={eliminar.isPending}
              onClick={() =>
                eliminar.mutate(deudaId, { onSuccess: () => navegar("/deudas", { replace: true }) })
              }
            >
              {eliminar.isPending ? "Eliminando…" : "Eliminar definitivamente"}
            </Boton>
          </>
        }
      >
        <p className="text-sm">
          Se eliminarán también sus {deuda.cuotas_totales} cuotas, incluido el historial de pagos.
          Esta acción no se puede deshacer.
        </p>
      </Modal>
    </div>
  );
}

function Metrica({
  titulo,
  valor,
  nota,
}: {
  titulo: string;
  valor: React.ReactNode;
  nota?: string;
}) {
  return (
    <div className="tarjeta">
      <p className="etiqueta">{titulo}</p>
      <p className="mt-1 text-xl font-semibold">{valor}</p>
      {nota ? <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{nota}</p> : null}
    </div>
  );
}
