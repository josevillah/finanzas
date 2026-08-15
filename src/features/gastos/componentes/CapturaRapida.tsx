import { useEffect, useMemo, useRef, useState } from "react";

import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Modal } from "@/components/ui/Modal";
import { useCategorias } from "@/features/catalogos/hooks";
import { useCapturaRapida } from "@/features/mes/hooks";
import { cn } from "@/lib/cn";
import { mensajeDeError } from "@/lib/ipc";
import { formatearMiles, parsearMonto } from "@/lib/moneda";

const CLAVE_ULTIMA = "captura-rapida:ultima-categoria";

/**
 * Captura de gasto hormiga en el mínimo de clics.
 *
 * Tocar una categoría la selecciona; si ya hay monto escrito, además guarda de
 * inmediato. Enter guarda con la seleccionada. Así se puede cambiar de
 * categoría sin tener que registrar un gasto para lograrlo.
 */
export function CapturaRapida({ abierto, onCerrar }: { abierto: boolean; onCerrar: () => void }) {
  const categorias = useCategorias(true);
  const guardar = useCapturaRapida();

  const [texto, setTexto] = useState("");
  const [verTodas, setVerTodas] = useState(false);
  const [aviso, setAviso] = useState<string | null>(null);
  const [guardado, setGuardado] = useState<{ monto: number; categoria: string } | null>(null);
  const campoMonto = useRef<HTMLInputElement>(null);

  const [seleccionada, setSeleccionada] = useState<number | null>(() => {
    const guardada = Number(localStorage.getItem(CLAVE_ULTIMA));
    return Number.isFinite(guardada) && guardada > 0 ? guardada : null;
  });

  const monto = parsearMonto(texto);

  const visibles = useMemo(() => {
    const todas = categorias.data ?? [];
    // Por omisión solo las hormiga: es para lo que existe esta pantalla.
    return verTodas ? todas : todas.filter((c) => c.tipo === "hormiga");
  }, [categorias.data, verTodas]);

  const activa = useMemo(
    () => visibles.find((c) => c.id === seleccionada) ?? null,
    [visibles, seleccionada],
  );

  useEffect(() => {
    if (!abierto) return;
    setTexto("");
    setAviso(null);
    setGuardado(null);
    guardar.reset();
    // El foco al abrir es lo que hace que el flujo sea "teclear y listo".
    const t = setTimeout(() => campoMonto.current?.focus(), 50);
    return () => clearTimeout(t);
    // `guardar` cambia de identidad en cada render; solo interesa la apertura.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abierto]);

  const registrar = (categoriaId: number, nombre: string) => {
    if (monto <= 0) {
      setAviso("Escribe el monto y vuelve a tocar la categoría.");
      campoMonto.current?.focus();
      return;
    }

    guardar.mutate(
      { monto, categoriaId },
      {
        onSuccess: () => {
          localStorage.setItem(CLAVE_ULTIMA, String(categoriaId));
          setGuardado({ monto, categoria: nombre });
          setAviso(null);
          setTexto("");
          setTimeout(() => campoMonto.current?.focus(), 50);
        },
      },
    );
  };

  return (
    <Modal
      abierto={abierto}
      ancho="md"
      titulo="Gasto rápido"
      onCerrar={onCerrar}
      acciones={
        <>
          <span className="mr-auto text-xs text-slate-500 dark:text-slate-400">
            Ctrl+Shift+G abre esto desde cualquier parte
          </span>
          <Boton variante="secundario" onClick={onCerrar}>
            Cerrar
          </Boton>
        </>
      }
    >
      <div className="space-y-5">
        <div>
          <div className="relative">
            <span className="pointer-events-none absolute left-4 top-1/2 -translate-y-1/2 text-2xl text-slate-400">
              $
            </span>
            <input
              ref={campoMonto}
              inputMode="numeric"
              autoComplete="off"
              placeholder="0"
              value={texto}
              onChange={(e) => {
                const entero = parsearMonto(e.target.value);
                setTexto(entero ? formatearMiles(entero) : "");
                if (entero > 0) setAviso(null);
              }}
              onKeyDown={(e) => {
                if (e.key !== "Enter") return;
                e.preventDefault();

                if (!activa) {
                  setAviso("Elige una categoría abajo.");
                  return;
                }
                registrar(activa.id, activa.nombre);
              }}
              className={cn(
                "w-full rounded-xl border px-4 py-4 pl-10 text-right text-3xl font-semibold tabular",
                "border-slate-300 bg-white text-slate-900 placeholder:text-slate-300",
                "focus:border-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/30",
                "dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100 dark:placeholder:text-slate-700",
              )}
            />
          </div>

          <p className="mt-2 text-center text-xs text-slate-500 dark:text-slate-400">
            {activa ? (
              <>
                Enter lo guarda en <strong>{activa.nombre}</strong>
              </>
            ) : (
              "Elige una categoría para poder guardar con Enter"
            )}
          </p>
        </div>

        {categorias.isPending ? (
          <p className="py-6 text-center text-sm text-slate-500">Cargando categorías…</p>
        ) : !visibles.length ? (
          <p className="py-6 text-center text-sm text-slate-500 dark:text-slate-400">
            No tienes categorías {verTodas ? "activas" : "de tipo hormiga"}. Créalas en Categorías.
          </p>
        ) : (
          <div className="grid grid-cols-2 gap-2">
            {visibles.map((c) => {
              const esActiva = c.id === seleccionada;
              return (
                <button
                  key={c.id}
                  type="button"
                  aria-pressed={esActiva}
                  disabled={guardar.isPending}
                  onClick={() => {
                    // Tocar la categoría siempre la selecciona. Guardar solo
                    // cuando ya hay monto: si no, se queda esperándolo.
                    setSeleccionada(c.id);
                    setAviso(null);
                    if (monto > 0) {
                      registrar(c.id, c.nombre);
                    } else {
                      campoMonto.current?.focus();
                    }
                  }}
                  className={cn(
                    "flex min-h-16 items-center gap-3 rounded-xl border px-4 py-3 text-left text-sm font-medium transition-colors",
                    "hover:border-indigo-400 hover:bg-indigo-50",
                    "disabled:opacity-50",
                    "dark:hover:border-indigo-600 dark:hover:bg-indigo-950/40",
                    esActiva
                      ? "border-indigo-500 bg-indigo-50 ring-2 ring-indigo-500/30 dark:bg-indigo-950/50"
                      : "border-slate-200 dark:border-slate-700",
                  )}
                >
                  <span
                    aria-hidden
                    className="h-3 w-3 shrink-0 rounded-full"
                    style={{ backgroundColor: c.color ?? "#94a3b8" }}
                  />
                  <span className="truncate">{c.nombre}</span>
                </button>
              );
            })}
          </div>
        )}

        <button
          type="button"
          onClick={() => setVerTodas((v) => !v)}
          className="w-full text-center text-xs text-indigo-600 hover:underline dark:text-indigo-400"
        >
          {verTodas ? "Mostrar solo gastos hormiga" : "Mostrar todas las categorías"}
        </button>

        {aviso ? (
          <p className="rounded-lg bg-amber-50 px-3 py-2 text-center text-sm text-amber-800 dark:bg-amber-950/40 dark:text-amber-300">
            {aviso}
          </p>
        ) : null}

        {guardado ? (
          <p className="rounded-lg bg-emerald-50 px-3 py-2 text-center text-sm text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300">
            Guardado <Moneda monto={guardado.monto} /> en {guardado.categoria}. Puedes registrar otro.
          </p>
        ) : null}

        {guardar.error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(guardar.error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}
