import { useEffect, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { formatearFecha, formatearMesLargo } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import type { ServicioConReal } from "@/types/dominio";

interface Props {
  servicio: ServicioConReal | null;
  anio: number;
  mes: number;
  onCerrar: () => void;
  onConfirmar: (datos: { servicioId: number; anio: number; mes: number; monto: number }) => void;
  guardando: boolean;
  error: unknown;
}

/**
 * Registra a mano el gasto de un servicio en un mes anterior a su alta.
 *
 * El monto se pide en vez de tomarlo del estimado sin preguntar: es un mes que
 * ya pasó, así que el usuario sabe lo que pagó de verdad.
 */
export function DialogoActivarServicio({
  servicio,
  anio,
  mes,
  onCerrar,
  onConfirmar,
  guardando,
  error,
}: Props) {
  const [monto, setMonto] = useState(0);

  useEffect(() => {
    if (servicio) setMonto(servicio.monto_estimado);
  }, [servicio]);

  if (!servicio) return null;

  return (
    <Modal
      abierto
      ancho="md"
      titulo={`Activar ${servicio.nombre} en ${formatearMesLargo(anio, mes)}`}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton
            disabled={guardando || monto <= 0}
            onClick={() => onConfirmar({ servicioId: servicio.id, anio, mes, monto })}
          >
            {guardando ? "Registrando…" : "Registrar el gasto"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4 text-sm">
        <p className="text-slate-600 dark:text-slate-400">
          Este servicio se dio de alta después, así que la app no le generó gasto en este mes.
          Acá lo registras a mano.
        </p>

        <Campo
          etiqueta="Monto pagado ese mes"
          ayuda="Viene precargado con el estimado actual, pero puedes ajustarlo a lo que pagaste."
        >
          <MontoInput valor={monto} onCambio={setMonto} autoFocus />
        </Campo>

        {servicio.fecha_vencimiento ? (
          <p className="text-xs text-slate-500 dark:text-slate-400">
            El gasto queda con fecha {formatearFecha(servicio.fecha_vencimiento)}, según el día de
            vencimiento del servicio.
          </p>
        ) : null}

        <p className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
          Vale solo para este mes. No cambia la fecha de alta del servicio ni hace que aparezca
          solo en otros meses anteriores: cada uno se activa por separado.
        </p>

        {error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}
