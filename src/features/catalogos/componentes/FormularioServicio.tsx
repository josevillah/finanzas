import { useEffect, useMemo, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo, Entrada, Seleccion } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { mensajeDeError } from "@/lib/ipc";
import {
  ETIQUETAS_TIPO_SERVICIO,
  type NuevoServicio,
  type Servicio,
  type TipoServicio,
} from "@/types/dominio";

import { useCategorias } from "../hooks";

interface Props {
  abierto: boolean;
  servicio?: Servicio | null;
  onCerrar: () => void;
  onGuardar: (datos: NuevoServicio) => void;
  guardando: boolean;
  error: unknown;
}

interface EstadoForm {
  nombre: string;
  categoriaId: string;
  montoEstimado: number;
  diaTexto: string;
  tipo: TipoServicio;
  activo: boolean;
}

const VACIO: EstadoForm = {
  nombre: "",
  categoriaId: "",
  montoEstimado: 0,
  diaTexto: "",
  tipo: "basico",
  activo: true,
};

export function FormularioServicio({
  abierto,
  servicio,
  onCerrar,
  onGuardar,
  guardando,
  error,
}: Props) {
  const [form, setForm] = useState<EstadoForm>(VACIO);
  const categorias = useCategorias(true);

  useEffect(() => {
    if (!abierto) return;

    setForm(
      servicio
        ? {
            nombre: servicio.nombre,
            categoriaId: servicio.categoria_id?.toString() ?? "",
            montoEstimado: servicio.monto_estimado,
            diaTexto: servicio.dia_vencimiento?.toString() ?? "",
            tipo: servicio.tipo,
            activo: servicio.activo,
          }
        : VACIO,
    );
  }, [abierto, servicio]);

  const dia = form.diaTexto ? Number(form.diaTexto) : null;

  const errorLocal = useMemo(() => {
    if (!form.nombre.trim()) return "Ponle un nombre al servicio.";
    if (dia !== null && (dia < 1 || dia > 31)) return "El día debe estar entre 1 y 31.";
    return null;
  }, [form.nombre, dia]);

  return (
    <Modal
      abierto={abierto}
      titulo={servicio ? "Editar servicio" : "Nuevo servicio"}
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
                categoria_id: form.categoriaId ? Number(form.categoriaId) : null,
                monto_estimado: form.montoEstimado,
                dia_vencimiento: dia,
                tipo: form.tipo,
                activo: form.activo,
                // El alta la pone el backend con la fecha de hoy y no se
                // toca al editar: mover el alta hacia atrás haría aparecer
                // gastos en meses en que el servicio no existía.
                fecha_alta: null,
              })
            }
          >
            {guardando ? "Guardando…" : servicio ? "Guardar cambios" : "Crear"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <Campo etiqueta="Nombre">
          <Entrada
            autoFocus
            placeholder="Ej: Enel, Netflix, Mundo Internet"
            value={form.nombre}
            onChange={(e) => setForm({ ...form, nombre: e.target.value })}
          />
        </Campo>

        <div className="grid gap-4 sm:grid-cols-2">
          <Campo etiqueta="Tipo">
            <Seleccion
              value={form.tipo}
              onChange={(e) => setForm({ ...form, tipo: e.target.value as TipoServicio })}
            >
              {Object.entries(ETIQUETAS_TIPO_SERVICIO).map(([valor, etiqueta]) => (
                <option key={valor} value={valor}>
                  {etiqueta}
                </option>
              ))}
            </Seleccion>
          </Campo>

          <Campo etiqueta="Categoría">
            <Seleccion
              value={form.categoriaId}
              onChange={(e) => setForm({ ...form, categoriaId: e.target.value })}
            >
              <option value="">Sin categoría</option>
              {(categorias.data ?? []).map((c) => (
                <option key={c.id} value={c.id}>
                  {c.nombre}
                </option>
              ))}
            </Seleccion>
          </Campo>
        </div>

        <div className="grid gap-4 sm:grid-cols-2">
          <Campo
            etiqueta="Monto estimado"
            ayuda={
              form.tipo === "basico"
                ? "Se carga como gasto del mes y lo ajustas cuando llegue la boleta."
                : "El cobro mensual de la suscripción."
            }
          >
            <MontoInput
              valor={form.montoEstimado}
              onCambio={(v) => setForm({ ...form, montoEstimado: v })}
            />
          </Campo>

          <Campo etiqueta="Día de vencimiento" ayuda="Opcional. En meses cortos se ajusta solo.">
            <Entrada
              inputMode="numeric"
              className="text-right tabular"
              placeholder="—"
              value={form.diaTexto}
              onChange={(e) =>
                setForm({ ...form, diaTexto: e.target.value.replace(/\D/g, "").slice(0, 2) })
              }
            />
          </Campo>
        </div>

        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-slate-300 text-indigo-600"
            checked={form.activo}
            onChange={(e) => setForm({ ...form, activo: e.target.checked })}
          />
          Activo (genera su gasto todos los meses)
        </label>

        {!servicio ? (
          <p className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
            El gasto empieza a generarse desde este mes en adelante. Los meses anteriores no se
            tocan.
          </p>
        ) : null}

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
