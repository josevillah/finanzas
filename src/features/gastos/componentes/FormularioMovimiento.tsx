import { useEffect, useMemo, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { AreaTexto, Campo, Entrada, Seleccion } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { useCategorias, useServicios } from "@/features/catalogos/hooks";
import { hoyISO } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import {
  ETIQUETAS_MEDIO_PAGO,
  ETIQUETAS_TIPO_CATEGORIA,
  MEDIOS_PAGO,
  type MedioPago,
  type MovimientoDetalle,
  type NuevoMovimiento,
  type TipoCategoria,
  type TipoMovimiento,
} from "@/types/dominio";

interface Props {
  abierto: boolean;
  /** Si viene, el formulario edita en vez de crear. */
  movimiento?: MovimientoDetalle | null;
  /** Fecha por omisión al crear: primer día del mes que se está viendo. */
  fechaPorDefecto?: string;
  onCerrar: () => void;
  onGuardar: (datos: NuevoMovimiento) => void;
  guardando: boolean;
  error: unknown;
}

interface EstadoForm {
  fecha: string;
  monto: number;
  tipo: TipoMovimiento;
  categoriaId: string;
  servicioId: string;
  medioPago: string;
  descripcion: string;
}

function vacio(fecha: string): EstadoForm {
  return {
    fecha,
    monto: 0,
    tipo: "gasto",
    categoriaId: "",
    servicioId: "",
    medioPago: "debito",
    descripcion: "",
  };
}

export function FormularioMovimiento({
  abierto,
  movimiento,
  fechaPorDefecto,
  onCerrar,
  onGuardar,
  guardando,
  error,
}: Props) {
  const [form, setForm] = useState<EstadoForm>(() => vacio(hoyISO()));

  const categorias = useCategorias(true);
  const servicios = useServicios(true);

  useEffect(() => {
    if (!abierto) return;

    setForm(
      movimiento
        ? {
            fecha: movimiento.fecha,
            monto: movimiento.monto,
            tipo: movimiento.tipo,
            categoriaId: movimiento.categoria_id?.toString() ?? "",
            servicioId: movimiento.servicio_id?.toString() ?? "",
            medioPago: movimiento.medio_pago ?? "",
            descripcion: movimiento.descripcion ?? "",
          }
        : vacio(fechaPorDefecto ?? hoyISO()),
    );
  }, [abierto, movimiento, fechaPorDefecto]);

  const errorLocal = useMemo(() => {
    if (form.monto <= 0) return "El monto debe ser mayor a 0.";
    if (!/^\d{4}-\d{2}-\d{2}$/.test(form.fecha)) return "Revisa la fecha.";
    return null;
  }, [form.monto, form.fecha]);

  // Las categorías se agrupan por tipo para no perderse en una lista larga, y
  // se ofrecen solo las que corresponden: las de ingreso no sirven para un
  // gasto, ni al revés.
  const tiposVisibles: TipoCategoria[] =
    form.tipo === "ingreso" ? ["ingreso"] : ["fijo", "variable", "hormiga"];

  const porTipo = useMemo(() => {
    const grupos: Record<string, Array<{ id: number; nombre: string }>> = {
      fijo: [],
      variable: [],
      hormiga: [],
      ingreso: [],
    };
    for (const c of categorias.data ?? []) {
      grupos[c.tipo]?.push({ id: c.id, nombre: c.nombre });
    }
    return grupos;
  }, [categorias.data]);

  const enviar = () => {
    if (errorLocal) return;
    onGuardar({
      fecha: form.fecha,
      monto: form.monto,
      tipo: form.tipo,
      categoria_id: form.categoriaId ? Number(form.categoriaId) : null,
      // Un ingreso nunca se asocia a un servicio.
      servicio_id: form.tipo === "gasto" && form.servicioId ? Number(form.servicioId) : null,
      medio_pago: form.medioPago ? (form.medioPago as MedioPago) : null,
      descripcion: form.descripcion.trim() || null,
    });
  };

  return (
    <Modal
      abierto={abierto}
      titulo={movimiento ? "Editar movimiento" : "Nuevo movimiento"}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton onClick={enviar} disabled={guardando || !!errorLocal}>
            {guardando ? "Guardando…" : movimiento ? "Guardar cambios" : "Registrar"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex gap-1 rounded-lg bg-slate-100 p-1 dark:bg-slate-800">
          {(["gasto", "ingreso"] as TipoMovimiento[]).map((t) => (
            <button
              key={t}
              type="button"
              // Cambiar de gasto a ingreso invalida la categoría elegida.
              onClick={() => setForm({ ...form, tipo: t, servicioId: "", categoriaId: "" })}
              className={
                form.tipo === t
                  ? "flex-1 rounded-md bg-white px-3 py-1.5 text-sm font-medium shadow-sm dark:bg-slate-950"
                  : "flex-1 rounded-md px-3 py-1.5 text-sm text-slate-600 dark:text-slate-400"
              }
            >
              {t === "gasto" ? "Gasto" : "Ingreso"}
            </button>
          ))}
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <Campo etiqueta="Monto">
            <MontoInput valor={form.monto} onCambio={(v) => setForm({ ...form, monto: v })} autoFocus />
          </Campo>

          <Campo etiqueta="Fecha">
            <Entrada
              type="date"
              value={form.fecha}
              onChange={(e) => setForm({ ...form, fecha: e.target.value })}
            />
          </Campo>
        </div>

        <Campo etiqueta="Categoría" ayuda="Opcional, pero sin ella no aparece en el desglose.">
          <Seleccion
            value={form.categoriaId}
            onChange={(e) => setForm({ ...form, categoriaId: e.target.value })}
          >
            <option value="">Sin categoría</option>
            {tiposVisibles.map((tipo) =>
              porTipo[tipo]?.length ? (
                <optgroup key={tipo} label={ETIQUETAS_TIPO_CATEGORIA[tipo]}>
                  {porTipo[tipo].map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.nombre}
                    </option>
                  ))}
                </optgroup>
              ) : null,
            )}
          </Seleccion>
        </Campo>

        {form.tipo === "gasto" ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <Campo etiqueta="Servicio" ayuda="Enlaza el gasto para comparar contra lo estimado.">
              <Seleccion
                value={form.servicioId}
                onChange={(e) => setForm({ ...form, servicioId: e.target.value })}
              >
                <option value="">Ninguno</option>
                {(servicios.data ?? []).map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.nombre}
                  </option>
                ))}
              </Seleccion>
            </Campo>

            <Campo etiqueta="Medio de pago">
              <Seleccion
                value={form.medioPago}
                onChange={(e) => setForm({ ...form, medioPago: e.target.value })}
              >
                <option value="">Sin especificar</option>
                {MEDIOS_PAGO.map((m) => (
                  <option key={m} value={m}>
                    {ETIQUETAS_MEDIO_PAGO[m]}
                  </option>
                ))}
              </Seleccion>
            </Campo>
          </div>
        ) : null}

        <Campo etiqueta="Descripción" ayuda="Opcional">
          <AreaTexto
            rows={2}
            value={form.descripcion}
            onChange={(e) => setForm({ ...form, descripcion: e.target.value })}
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
