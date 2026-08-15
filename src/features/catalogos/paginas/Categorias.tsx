import { useEffect, useMemo, useState } from "react";

import { Boton } from "@/components/ui/Boton";
import { Campo, Entrada, Seleccion } from "@/components/ui/Campo";
import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Modal } from "@/components/ui/Modal";
import { mensajeDeError } from "@/lib/ipc";
import {
  ETIQUETAS_TIPO_CATEGORIA,
  type Categoria,
  type NuevaCategoria,
  type TipoCategoria,
} from "@/types/dominio";

import {
  useActualizarCategoria,
  useCategorias,
  useCrearCategoria,
  useEliminarCategoria,
} from "../hooks";

const TIPOS: TipoCategoria[] = ["fijo", "variable", "hormiga"];

const DESCRIPCION_TIPO: Record<TipoCategoria, string> = {
  fijo: "Sale todos los meses sí o sí.",
  variable: "Necesario, pero el monto cambia.",
  hormiga: "Gasto chico y frecuente. Es lo que se captura con Ctrl+Shift+G.",
};

const PALETA = [
  "#6366f1", "#0ea5e9", "#14b8a6", "#22c55e", "#84cc16",
  "#f59e0b", "#f97316", "#ef4444", "#e11d48", "#d946ef",
  "#8b5cf6", "#64748b",
];

export function Categorias() {
  const { data, isPending, error, refetch } = useCategorias(false);

  const [formAbierto, setFormAbierto] = useState(false);
  const [editando, setEditando] = useState<Categoria | null>(null);
  const [porBorrar, setPorBorrar] = useState<Categoria | null>(null);

  const crear = useCrearCategoria();
  const actualizar = useActualizarCategoria();
  const eliminar = useEliminarCategoria();

  const porTipo = useMemo(() => {
    const grupos: Record<TipoCategoria, Categoria[]> = { fijo: [], variable: [], hormiga: [] };
    for (const c of data ?? []) grupos[c.tipo].push(c);
    return grupos;
  }, [data]);

  const guardar = (datos: NuevaCategoria) => {
    const alCerrar = { onSuccess: () => setFormAbierto(false) };
    if (editando) {
      actualizar.mutate({ id: editando.id, datos }, alCerrar);
    } else {
      crear.mutate(datos, alCerrar);
    }
  };

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Categorías</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Cómo se clasifica cada gasto. Las de tipo hormiga son las que salen en la captura rápida.
          </p>
        </div>
        <Boton
          onClick={() => {
            crear.reset();
            setEditando(null);
            setFormAbierto(true);
          }}
        >
          + Nueva categoría
        </Boton>
      </header>

      <div className="grid gap-4 lg:grid-cols-3">
        {TIPOS.map((tipo) => (
          <div key={tipo} className="tarjeta">
            <h2 className="font-medium">{ETIQUETAS_TIPO_CATEGORIA[tipo]}</h2>
            <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
              {DESCRIPCION_TIPO[tipo]}
            </p>

            {!porTipo[tipo].length ? (
              <p className="py-4 text-sm text-slate-400">Ninguna todavía.</p>
            ) : (
              <ul className="divide-y divide-slate-100 dark:divide-slate-800">
                {porTipo[tipo].map((c) => (
                  <li key={c.id} className="flex items-center justify-between gap-2 py-2">
                    <span className="flex min-w-0 items-center gap-2">
                      <span
                        aria-hidden
                        className="h-3 w-3 shrink-0 rounded-full"
                        style={{ backgroundColor: c.color ?? "#94a3b8" }}
                      />
                      <span
                        className={
                          c.activa
                            ? "truncate text-sm"
                            : "truncate text-sm text-slate-400 line-through"
                        }
                      >
                        {c.nombre}
                      </span>
                      {c.codigo ? <Insignia tono="indigo">sistema</Insignia> : null}
                    </span>

                    <span className="flex shrink-0 gap-1">
                      <Boton
                        variante="fantasma"
                        tamano="sm"
                        onClick={() => {
                          actualizar.reset();
                          setEditando(c);
                          setFormAbierto(true);
                        }}
                      >
                        Editar
                      </Boton>
                      {!c.codigo ? (
                        <Boton
                          variante="fantasma"
                          tamano="sm"
                          onClick={() => {
                            eliminar.reset();
                            setPorBorrar(c);
                          }}
                        >
                          ✕
                        </Boton>
                      ) : null}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>

      <FormularioCategoria
        abierto={formAbierto}
        categoria={editando}
        onCerrar={() => setFormAbierto(false)}
        onGuardar={guardar}
        guardando={crear.isPending || actualizar.isPending}
        error={editando ? actualizar.error : crear.error}
      />

      <Modal
        abierto={!!porBorrar}
        ancho="md"
        titulo="Eliminar categoría"
        onCerrar={() => setPorBorrar(null)}
        acciones={
          <>
            <Boton variante="secundario" onClick={() => setPorBorrar(null)}>
              Cancelar
            </Boton>
            <Boton
              variante="peligro"
              disabled={eliminar.isPending}
              onClick={() =>
                porBorrar && eliminar.mutate(porBorrar.id, { onSuccess: () => setPorBorrar(null) })
              }
            >
              {eliminar.isPending ? "Eliminando…" : "Eliminar"}
            </Boton>
          </>
        }
      >
        <p className="text-sm">
          Se eliminará <strong>{porBorrar?.nombre}</strong>. Si ya tiene gastos o servicios
          asociados no se podrá borrar: en ese caso desactívala desde Editar.
        </p>
        {eliminar.error ? (
          <p className="mt-3 rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(eliminar.error)}
          </p>
        ) : null}
      </Modal>
    </div>
  );
}

function FormularioCategoria({
  abierto,
  categoria,
  onCerrar,
  onGuardar,
  guardando,
  error,
}: {
  abierto: boolean;
  categoria?: Categoria | null;
  onCerrar: () => void;
  onGuardar: (datos: NuevaCategoria) => void;
  guardando: boolean;
  error: unknown;
}) {
  const [nombre, setNombre] = useState("");
  const [tipo, setTipo] = useState<TipoCategoria>("variable");
  const [color, setColor] = useState<string>(PALETA[0]);
  const [activa, setActiva] = useState(true);

  useEffect(() => {
    if (!abierto) return;
    setNombre(categoria?.nombre ?? "");
    setTipo(categoria?.tipo ?? "variable");
    setColor(categoria?.color ?? PALETA[0]);
    setActiva(categoria?.activa ?? true);
  }, [abierto, categoria]);

  return (
    <Modal
      abierto={abierto}
      ancho="md"
      titulo={categoria ? "Editar categoría" : "Nueva categoría"}
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={guardando}>
            Cancelar
          </Boton>
          <Boton
            disabled={guardando || !nombre.trim()}
            onClick={() => onGuardar({ nombre: nombre.trim(), tipo, color, activa })}
          >
            {guardando ? "Guardando…" : categoria ? "Guardar cambios" : "Crear"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4">
        <Campo etiqueta="Nombre">
          <Entrada
            autoFocus
            value={nombre}
            onChange={(e) => setNombre(e.target.value)}
            placeholder="Ej: Mascotas"
          />
        </Campo>

        <Campo etiqueta="Tipo" ayuda={DESCRIPCION_TIPO[tipo]}>
          <Seleccion value={tipo} onChange={(e) => setTipo(e.target.value as TipoCategoria)}>
            {TIPOS.map((t) => (
              <option key={t} value={t}>
                {ETIQUETAS_TIPO_CATEGORIA[t]}
              </option>
            ))}
          </Seleccion>
        </Campo>

        <div>
          <span className="mb-1.5 block text-sm font-medium text-slate-700 dark:text-slate-300">
            Color
          </span>
          <div className="flex flex-wrap gap-2">
            {PALETA.map((c) => (
              <button
                key={c}
                type="button"
                aria-label={`Color ${c}`}
                onClick={() => setColor(c)}
                style={{ backgroundColor: c }}
                className={
                  color === c
                    ? "h-7 w-7 rounded-full ring-2 ring-slate-900 ring-offset-2 dark:ring-white dark:ring-offset-slate-900"
                    : "h-7 w-7 rounded-full"
                }
              />
            ))}
          </div>
        </div>

        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-slate-300 text-indigo-600"
            checked={activa}
            onChange={(e) => setActiva(e.target.checked)}
          />
          Activa (aparece al registrar gastos)
        </label>

        {categoria?.codigo ? (
          <p className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
            Esta categoría la usa el sistema para imputar los pagos de cuotas. Puedes renombrarla,
            pero no eliminarla.
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
