import { useEffect, useMemo, useState } from "react";

import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { AreaTexto, Campo, Entrada, Seleccion } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { hoyISO } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { parsearTasaPorcentaje } from "@/lib/moneda";
import {
  ETIQUETAS_TIPO_DEUDA,
  type Deuda,
  type NuevaDeuda,
  type TipoDeuda,
} from "@/types/dominio";

import { useSimulacion } from "../hooks";
import { VistaPreviaCuotas } from "./VistaPreviaCuotas";

interface Props {
  abierto: boolean;
  /** Si viene, el formulario edita en vez de crear. */
  deuda?: Deuda | null;
  onCerrar: () => void;
  onGuardar: (datos: NuevaDeuda) => void;
  guardando: boolean;
  error: unknown;
}

interface EstadoForm {
  descripcion: string;
  tipo: TipoDeuda;
  institucion: string;
  montoOriginal: number;
  tasaTexto: string;
  nCuotasTexto: string;
  fechaPrimeraCuota: string;
  notas: string;
}

function formVacio(): EstadoForm {
  return {
    descripcion: "",
    tipo: "compra_cuotas",
    institucion: "",
    montoOriginal: 0,
    tasaTexto: "",
    nCuotasTexto: "12",
    fechaPrimeraCuota: hoyISO(),
    notas: "",
  };
}

function formDesde(deuda: Deuda): EstadoForm {
  return {
    descripcion: deuda.descripcion,
    tipo: deuda.tipo,
    institucion: deuda.institucion ?? "",
    montoOriginal: deuda.monto_original,
    tasaTexto: deuda.tasa_mensual ? (deuda.tasa_mensual * 100).toString().replace(".", ",") : "",
    nCuotasTexto: String(deuda.n_cuotas),
    fechaPrimeraCuota: deuda.fecha_primera_cuota,
    notas: deuda.notas ?? "",
  };
}

export function FormularioDeuda({ abierto, deuda, onCerrar, onGuardar, guardando, error }: Props) {
  const [form, setForm] = useState<EstadoForm>(formVacio);

  useEffect(() => {
    if (!abierto) return;
    setForm(deuda ? formDesde(deuda) : formVacio());
  }, [abierto, deuda]);

  const nCuotas = Number.parseInt(form.nCuotasTexto, 10) || 0;
  const tasaMensual = parsearTasaPorcentaje(form.tasaTexto);

  const puedeSimular =
    form.montoOriginal > 0 && nCuotas >= 1 && /^\d{4}-\d{2}-\d{2}$/.test(form.fechaPrimeraCuota);

  const simulacion = useSimulacion({
    montoOriginal: form.montoOriginal,
    tasaMensual,
    nCuotas,
    fechaPrimeraCuota: form.fechaPrimeraCuota,
    habilitado: abierto && puedeSimular,
  });

  const errorLocal = useMemo(() => {
    if (!form.descripcion.trim()) return "Ponle una descripción a la deuda.";
    if (form.montoOriginal <= 0) return "El monto debe ser mayor a 0.";
    if (nCuotas < 1) return "Indica al menos 1 cuota.";
    return null;
  }, [form.descripcion, form.montoOriginal, nCuotas]);

  const enviar = () => {
    if (errorLocal) return;
    onGuardar({
      descripcion: form.descripcion.trim(),
      tipo: form.tipo,
      institucion: form.institucion.trim() || null,
      monto_original: form.montoOriginal,
      tasa_mensual: tasaMensual,
      n_cuotas: nCuotas,
      fecha_primera_cuota: form.fechaPrimeraCuota,
      notas: form.notas.trim() || null,
    });
  };

  return (
    <Modal
      abierto={abierto}
      ancho="xl"
      titulo={deuda ? "Editar deuda" : "Nueva deuda"}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton onClick={enviar} disabled={guardando || !!errorLocal}>
            {guardando ? "Guardando…" : deuda ? "Guardar cambios" : "Crear deuda"}
          </Boton>
        </>
      }
    >
      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          <Campo etiqueta="Descripción">
            <Entrada
              autoFocus
              placeholder="Ej: Notebook Falabella"
              value={form.descripcion}
              onChange={(e) => setForm({ ...form, descripcion: e.target.value })}
            />
          </Campo>

          <div className="grid gap-4 sm:grid-cols-2">
            <Campo etiqueta="Tipo">
              <Seleccion
                value={form.tipo}
                onChange={(e) => setForm({ ...form, tipo: e.target.value as TipoDeuda })}
              >
                {Object.entries(ETIQUETAS_TIPO_DEUDA).map(([valor, etiqueta]) => (
                  <option key={valor} value={valor}>
                    {etiqueta}
                  </option>
                ))}
              </Seleccion>
            </Campo>

            <Campo etiqueta="Institución" ayuda="Opcional">
              <Entrada
                placeholder="Ej: Banco de Chile"
                value={form.institucion}
                onChange={(e) => setForm({ ...form, institucion: e.target.value })}
              />
            </Campo>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <Campo etiqueta="Monto original">
              <MontoInput
                valor={form.montoOriginal}
                onCambio={(v) => setForm({ ...form, montoOriginal: v })}
              />
            </Campo>

            <Campo etiqueta="Número de cuotas">
              <Entrada
                inputMode="numeric"
                className="text-right tabular"
                value={form.nCuotasTexto}
                onChange={(e) =>
                  setForm({ ...form, nCuotasTexto: e.target.value.replace(/\D/g, "").slice(0, 3) })
                }
              />
            </Campo>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <Campo
              etiqueta="Tasa mensual (%)"
              ayuda="Déjala vacía si la compra es sin interés."
            >
              <Entrada
                inputMode="decimal"
                className="text-right tabular"
                placeholder="0"
                value={form.tasaTexto}
                onChange={(e) => setForm({ ...form, tasaTexto: e.target.value })}
              />
            </Campo>

            <Campo etiqueta="Primera cuota">
              <Entrada
                type="date"
                value={form.fechaPrimeraCuota}
                onChange={(e) => setForm({ ...form, fechaPrimeraCuota: e.target.value })}
              />
            </Campo>
          </div>

          <Campo etiqueta="Notas" ayuda="Opcional">
            <AreaTexto
              rows={2}
              value={form.notas}
              onChange={(e) => setForm({ ...form, notas: e.target.value })}
            />
          </Campo>
        </div>

        <div className="space-y-3">
          <h3 className="etiqueta">Simulación de cuotas</h3>
          <VistaPreviaCuotas
            cuotas={simulacion.data}
            cargando={simulacion.isFetching}
            error={simulacion.error}
            montoOriginal={form.montoOriginal}
          />

          {deuda ? (
            <p className="rounded-lg bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:bg-amber-950/40 dark:text-amber-300">
              Al guardar se regenera la tabla de cuotas completa. Solo se permite si ninguna cuota
              está pagada.
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
      </div>
    </Modal>
  );
}
