import { useEffect, useMemo, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { AreaTexto, Campo, Entrada, Seleccion } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { useResumenCuentas } from "@/features/cuentas/hooks";
import { mensajeDeError } from "@/lib/ipc";
import type { MetaDetalle, NuevaMeta } from "@/types/dominio";

interface Props {
  abierto: boolean;
  meta?: MetaDetalle | null;
  onCerrar: () => void;
  onGuardar: (datos: NuevaMeta) => void;
  guardando: boolean;
  error: unknown;
}

interface EstadoForm {
  nombre: string;
  montoObjetivo: number;
  cuentaId: string;
  fechaObjetivo: string;
  notas: string;
}

const VACIO: EstadoForm = {
  nombre: "",
  montoObjetivo: 0,
  cuentaId: "",
  fechaObjetivo: "",
  notas: "",
};

export function FormularioMeta({
  abierto,
  meta,
  onCerrar,
  onGuardar,
  guardando,
  error,
}: Props) {
  const [form, setForm] = useState<EstadoForm>(VACIO);
  const cuentas = useResumenCuentas();

  useEffect(() => {
    if (!abierto) return;

    setForm(
      meta
        ? {
            nombre: meta.nombre,
            montoObjetivo: meta.monto_objetivo,
            cuentaId: meta.cuenta_id?.toString() ?? "",
            fechaObjetivo: meta.fecha_objetivo ?? "",
            notas: meta.notas ?? "",
          }
        : VACIO,
    );
  }, [abierto, meta]);

  const errorLocal = useMemo(() => {
    if (!form.nombre.trim()) return "Ponle un nombre a la meta.";
    if (form.montoObjetivo <= 0) return "El objetivo tiene que ser mayor a $0.";
    return null;
  }, [form.nombre, form.montoObjetivo]);

  const ahorros = cuentas.data?.ahorros ?? [];

  return (
    <Modal
      abierto={abierto}
      titulo={meta ? "Editar meta" : "Nueva meta"}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton
            disabled={guardando || !!errorLocal}
            onClick={() =>
              onGuardar({
                nombre: form.nombre.trim(),
                monto_objetivo: form.montoObjetivo,
                cuenta_id: form.cuentaId ? Number(form.cuentaId) : null,
                fecha_objetivo: form.fechaObjetivo || null,
                notas: form.notas.trim() || null,
              })
            }
          >
            {guardando ? "Guardando…" : meta ? "Guardar cambios" : "Crear meta"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <Campo etiqueta="Nombre">
          <Entrada
            autoFocus
            placeholder="Ej: Viaje a Japón, notebook nuevo"
            value={form.nombre}
            onChange={(e) => setForm({ ...form, nombre: e.target.value })}
          />
        </Campo>

        <div className="grid gap-4 sm:grid-cols-2">
          <Campo etiqueta="Cuánto necesito">
            <MontoInput
              valor={form.montoObjetivo}
              onCambio={(v) => setForm({ ...form, montoObjetivo: v })}
            />
          </Campo>

          <Campo
            etiqueta="Fecha objetivo"
            ayuda="Opcional. Con fecha se calcula cuánto apartar por mes."
          >
            <Entrada
              type="date"
              value={form.fechaObjetivo}
              onChange={(e) => setForm({ ...form, fechaObjetivo: e.target.value })}
            />
          </Campo>
        </div>

        <Campo
          etiqueta="Cuenta de ahorro"
          ayuda={
            form.cuentaId
              ? "El avance sale del saldo de esta cuenta. Si varias metas la comparten, la más prioritaria se sirve primero."
              : "Sin cuenta la meta es solo referencia de cuánto necesitas: no muestra avance."
          }
        >
          <Seleccion
            value={form.cuentaId}
            onChange={(e) => setForm({ ...form, cuentaId: e.target.value })}
          >
            <option value="">Sin cuenta vinculada</option>
            {ahorros.map((c) => (
              <option key={c.id} value={c.id}>
                {c.nombre}
              </option>
            ))}
          </Seleccion>
        </Campo>

        {ahorros.length === 0 ? (
          <p className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
            Todavía no tienes cuentas de ahorro. Puedes crear la meta igual y vincularla después.
          </p>
        ) : null}

        <Campo etiqueta="Notas" ayuda="Opcional.">
          <AreaTexto
            rows={2}
            value={form.notas}
            onChange={(e) => setForm({ ...form, notas: e.target.value })}
          />
        </Campo>

        {errorLocal ? (
          <p className="text-xs text-rose-600 dark:text-rose-400">{errorLocal}</p>
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
