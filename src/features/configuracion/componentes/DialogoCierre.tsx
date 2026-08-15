import { useEffect, useState } from "react";

import { Boton } from "@/components/ui/Boton";
import { Modal } from "@/components/ui/Modal";
import { mensajeDeError } from "@/lib/ipc";

import { useResolverCierre } from "../hooks";

/**
 * Qué hacer al cerrar con la X. Se muestra cuando la preferencia está en
 * "preguntar"; Rust ya interceptó el cierre antes de pedirlo.
 */
export function DialogoCierre({
  abierto,
  onCerrar,
}: {
  abierto: boolean;
  onCerrar: () => void;
}) {
  const [recordar, setRecordar] = useState(false);
  const resolver = useResolverCierre();

  // Cada vez que reaparece, el checkbox parte limpio: recordar es una decisión
  // deliberada, no algo que quede pegado de la vez anterior.
  useEffect(() => {
    if (abierto) {
      setRecordar(false);
      resolver.reset();
    }
    // `resolver` cambia de identidad en cada render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abierto]);

  const decidir = (accion: "bandeja" | "salir") => {
    // Con éxito la ventana se oculta o la app se cierra: no hace falta cerrar
    // el modal a mano, pero se hace igual por si la acción fue "bandeja" y la
    // ventana vuelve a mostrarse después.
    resolver.mutate({ accion, recordar }, { onSuccess: onCerrar });
  };

  return (
    <Modal
      abierto={abierto}
      ancho="md"
      titulo="¿Cerrar Finanzas?"
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={resolver.isPending}>
            Cancelar
          </Boton>
          <Boton
            variante="peligro"
            onClick={() => decidir("salir")}
            disabled={resolver.isPending}
          >
            Salir de la app
          </Boton>
          <Boton onClick={() => decidir("bandeja")} disabled={resolver.isPending}>
            Dejarla en segundo plano
          </Boton>
        </>
      }
    >
      <div className="space-y-4 text-sm">
        <p>
          Si la dejas en segundo plano, la ventana se esconde pero la app sigue corriendo: el
          atajo <kbd className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-xs dark:bg-slate-800">Ctrl+Shift+G</kbd>{" "}
          para anotar un gasto rápido sigue funcionando, y la vuelves a abrir desde el ícono
          junto al reloj.
        </p>
        <p className="text-slate-500 dark:text-slate-400">
          Si sales, el atajo deja de funcionar hasta que abras la app de nuevo.
        </p>

        <label className="flex items-start gap-2 rounded-lg bg-slate-50 px-3 py-2 dark:bg-slate-800/50">
          <input
            type="checkbox"
            className="mt-0.5 h-4 w-4 rounded border-slate-300 text-indigo-600"
            checked={recordar}
            onChange={(e) => setRecordar(e.target.checked)}
          />
          <span>
            Recordar mi elección y no volver a preguntar.
            <span className="mt-0.5 block text-xs text-slate-500 dark:text-slate-400">
              Se puede cambiar después en Configuración.
            </span>
          </span>
        </label>

        {resolver.error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(resolver.error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}
