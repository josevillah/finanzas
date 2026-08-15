import { useEffect, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { formatearFecha } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { formatearCLP } from "@/lib/moneda";
import type { MovimientoDetalle } from "@/types/dominio";

interface Props {
  movimiento: MovimientoDetalle | null;
  onCerrar: () => void;
  onConfirmar: (datos: { id: number; monto: number }) => void;
  guardando: boolean;
  error: unknown;
}

/**
 * Ajuste rápido del monto, sin pasar por el formulario completo. Es el caso
 * de "llegó la boleta y vino distinta al estimado".
 */
export function DialogoCambiarPrecio({
  movimiento,
  onCerrar,
  onConfirmar,
  guardando,
  error,
}: Props) {
  const [monto, setMonto] = useState(0);

  useEffect(() => {
    if (movimiento) setMonto(movimiento.monto);
  }, [movimiento]);

  if (!movimiento) return null;

  const diferencia = monto - movimiento.monto;

  return (
    <Modal
      abierto
      ancho="md"
      titulo="Cambiar precio"
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton
            disabled={guardando || monto <= 0}
            onClick={() => onConfirmar({ id: movimiento.id, monto })}
          >
            {guardando ? "Guardando…" : "Guardar precio"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <p className="text-sm text-slate-600 dark:text-slate-400">
          {movimiento.descripcion || movimiento.categoria_nombre || "Movimiento"} ·{" "}
          {formatearFecha(movimiento.fecha)}
          {movimiento.es_estimado ? (
            <>
              {" "}
              · monto estimado{" "}
              <strong className="tabular text-slate-900 dark:text-slate-100">
                {formatearCLP(movimiento.monto)}
              </strong>
            </>
          ) : null}
        </p>

        <Campo
          etiqueta="Monto real"
          ayuda={
            diferencia === 0
              ? "Igual al que está registrado."
              : diferencia > 0
                ? `${formatearCLP(diferencia)} más que lo registrado.`
                : `${formatearCLP(Math.abs(diferencia))} menos que lo registrado.`
          }
        >
          <MontoInput valor={monto} onCambio={setMonto} autoFocus />
        </Campo>

        {movimiento.es_estimado ? (
          <p className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
            Al guardar, este gasto deja de contar como estimado y pasa a ser el monto real del
            servicio.
          </p>
        ) : null}

        {error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}
