import { useState } from "react";

import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { cn } from "@/lib/cn";
import { mensajeDeError } from "@/lib/ipc";
import type { EstadoMeta, MetaDetalle, NuevaMeta } from "@/types/dominio";

import { FilaMeta } from "../componentes/FilaMeta";
import { FormularioMeta } from "../componentes/FormularioMeta";
import { TotalesMetas } from "../componentes/TotalesMetas";
import {
  useActualizarMeta,
  useCambiarEstadoMeta,
  useCrearMeta,
  useEliminarMeta,
  useReordenarMetas,
  useResumenMetas,
  type FiltroMetas,
} from "../hooks";

const FILTROS: { valor: FiltroMetas; etiqueta: string }[] = [
  { valor: "activa", etiqueta: "Activas" },
  { valor: "cumplida", etiqueta: "Cumplidas" },
  { valor: "archivada", etiqueta: "Archivadas" },
  { valor: "todas", etiqueta: "Todas" },
];

const VACIOS: Record<FiltroMetas, { titulo: string; descripcion: string }> = {
  activa: {
    titulo: "Sin metas activas",
    descripcion:
      "Anota lo que quieres comprar o juntar. Si la vinculas a una cuenta de ahorro, verás cuánto llevas y cuánto falta.",
  },
  cumplida: {
    titulo: "Todavía no cumples ninguna",
    descripcion: "Las metas cumplidas se quedan acá como historial, no se borran.",
  },
  archivada: {
    titulo: "Sin metas archivadas",
    descripcion: "Archivar una meta la saca del camino sin perder el registro.",
  },
  todas: {
    titulo: "Sin metas",
    descripcion: "Crea la primera para empezar a seguirle el rastro.",
  },
};

/**
 * Objetivos de compra o ahorro, en orden de prioridad.
 *
 * No tiene selector de mes: una meta no pertenece a un período. Y no mueve
 * plata: lo que muestra sale de los ahorros que ya existen.
 */
export function Metas() {
  const [filtro, setFiltro] = useState<FiltroMetas>("activa");
  const [editando, setEditando] = useState<MetaDetalle | null>(null);
  const [creando, setCreando] = useState(false);

  const { data, isPending, error, refetch } = useResumenMetas(filtro);

  const crear = useCrearMeta();
  const actualizar = useActualizarMeta();
  const cambiarEstado = useCambiarEstadoMeta();
  const eliminar = useEliminarMeta();
  const reordenar = useReordenarMetas();

  const ocupado =
    cambiarEstado.isPending || eliminar.isPending || reordenar.isPending;

  const errorAccion =
    cambiarEstado.error ?? eliminar.error ?? reordenar.error ?? null;

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;

  const metas = data.metas;

  // Reordenar solo tiene sentido —y solo es seguro— cuando las activas están a
  // la vista: son las únicas que se reparten el saldo de una cuenta.
  const reordenable = filtro === "activa" || filtro === "todas";

  /** Mueve una meta un lugar y manda la lista completa en el nuevo orden. */
  function mover(indice: number, delta: number) {
    const orden = metas.map((m) => m.id);
    const destino = indice + delta;
    if (destino < 0 || destino >= orden.length) return;

    [orden[indice], orden[destino]] = [orden[destino], orden[indice]];
    reordenar.mutate(orden);
  }

  function guardar(datos: NuevaMeta) {
    if (editando) {
      actualizar.mutate(
        { id: editando.id, datos },
        { onSuccess: () => setEditando(null) },
      );
    } else {
      crear.mutate(datos, { onSuccess: () => setCreando(false) });
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Metas</h1>
          <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
            Lo que quieres comprar o juntar, en orden de prioridad. Las metas no mueven plata: no
            cambian tu disponible ni tu patrimonio.
          </p>
        </div>

        <Boton onClick={() => setCreando(true)}>Nueva meta</Boton>
      </header>

      <TotalesMetas resumen={data} />

      <div className="flex flex-wrap items-center gap-2">
        {FILTROS.map((f) => (
          <button
            key={f.valor}
            type="button"
            onClick={() => setFiltro(f.valor)}
            className={cn(
              "rounded-full px-3 py-1 text-xs font-medium transition-colors",
              filtro === f.valor
                ? "bg-indigo-600 text-white"
                : "border border-slate-300 text-slate-600 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800",
            )}
          >
            {f.etiqueta}
            {f.valor === "activa" ? ` (${data.n_activas})` : null}
            {f.valor === "cumplida" ? ` (${data.n_cumplidas})` : null}
            {f.valor === "archivada" ? ` (${data.n_archivadas})` : null}
          </button>
        ))}
      </div>

      {errorAccion ? (
        <p className="rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(errorAccion)}
        </p>
      ) : null}

      {metas.length === 0 ? (
        <Vacio
          titulo={VACIOS[filtro].titulo}
          descripcion={VACIOS[filtro].descripcion}
          accion={
            filtro === "activa" || filtro === "todas" ? (
              <Boton onClick={() => setCreando(true)}>Crear una meta</Boton>
            ) : null
          }
        />
      ) : (
        <ul className="space-y-3">
          {metas.map((meta, i) => (
            <FilaMeta
              key={meta.id}
              meta={meta}
              primera={i === 0}
              ultima={i === metas.length - 1}
              reordenable={reordenable}
              ocupado={ocupado}
              onSubir={() => mover(i, -1)}
              onBajar={() => mover(i, 1)}
              onEditar={() => setEditando(meta)}
              onCambiarEstado={(estado: EstadoMeta) =>
                cambiarEstado.mutate({ id: meta.id, estado })
              }
              onEliminar={() => eliminar.mutate(meta.id)}
            />
          ))}
        </ul>
      )}

      {reordenable && metas.length > 1 ? (
        <p className="text-xs text-slate-500 dark:text-slate-400">
          El orden manda: cuando varias metas comparten una cuenta de ahorro, la de más arriba
          consume el saldo primero. Muévelas con ▲ y ▼.
        </p>
      ) : null}

      <FormularioMeta
        abierto={creando || editando !== null}
        meta={editando}
        onCerrar={() => {
          setCreando(false);
          setEditando(null);
        }}
        onGuardar={guardar}
        guardando={crear.isPending || actualizar.isPending}
        error={crear.error ?? actualizar.error}
      />
    </div>
  );
}
