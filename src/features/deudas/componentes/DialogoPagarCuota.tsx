import { useEffect, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo, Entrada } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { formatearFecha, hoyISO } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { formatearCLP } from "@/lib/moneda";
import type { Cuota } from "@/types/dominio";

interface Props {
  cuota: Cuota | null;
  onCerrar: () => void;
  onConfirmar: (datos: { cuota_id: number; fecha_pago: string; monto_pagado: number }) => void;
  guardando: boolean;
  error: unknown;
}

/** Permite registrar un monto distinto al programado (pago parcial o recargo). */
export function DialogoPagarCuota({ cuota, onCerrar, onConfirmar, guardando, error }: Props) {
  const [fecha, setFecha] = useState(hoyISO());
  const [monto, setMonto] = useState(0);

  // Al abrirse con otra cuota, precarga el monto programado y la fecha de hoy.
  useEffect(() => {
    if (cuota) {
      setMonto(cuota.monto);
      setFecha(hoyISO());
    }
  }, [cuota]);

  if (!cuota) return null;

  const diferencia = monto - cuota.monto;

  return (
    <Modal
      abierto
      ancho="md"
      titulo={`Pagar cuota ${cuota.numero}`}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton
            onClick={() => onConfirmar({ cuota_id: cuota.id, fecha_pago: fecha, monto_pagado: monto })}
            disabled={guardando}
          >
            {guardando ? "Guardando…" : "Registrar pago"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <p className="text-sm text-slate-600 dark:text-slate-400">
          Vence el {formatearFecha(cuota.fecha_vencimiento)} · monto programado{" "}
          <strong className="tabular text-slate-900 dark:text-slate-100">
            {formatearCLP(cuota.monto)}
          </strong>
        </p>

        <Campo etiqueta="Fecha de pago">
          <Entrada type="date" value={fecha} onChange={(e) => setFecha(e.target.value)} />
        </Campo>

        <Campo
          etiqueta="Monto realmente pagado"
          ayuda={
            diferencia === 0
              ? "Coincide con el monto programado."
              : diferencia > 0
                ? `Pagaste ${formatearCLP(diferencia)} más que la cuota.`
                : `Pagaste ${formatearCLP(Math.abs(diferencia))} menos que la cuota.`
          }
        >
          <MontoInput valor={monto} onCambio={setMonto} autoFocus />
        </Campo>

        {error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}
