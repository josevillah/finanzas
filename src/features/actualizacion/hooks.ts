import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { claves } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";

/** Mismo nombre que la constante `EVENTO_ACTUALIZACION_LISTA` en Rust. */
const EVENTO = "actualizacion-lista";

export function useEstadoActualizacion() {
  const qc = useQueryClient();

  // El chequeo del arranque corre en segundo plano: cuando termina de bajar,
  // Rust avisa y recién ahí hay algo nuevo que mostrar.
  useEffect(() => {
    const desuscribir = listen(EVENTO, () => {
      qc.invalidateQueries({ queryKey: claves.actualizacion() });
    }).catch(() => null);

    return () => {
      desuscribir.then((fn) => fn?.()).catch(() => undefined);
    };
  }, [qc]);

  return useQuery({
    queryKey: claves.actualizacion(),
    queryFn: () => ipc.estadoActualizacion(),
    staleTime: 60_000,
  });
}

export function useBuscarActualizacion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.buscarActualizacion(),
    onSuccess: () => qc.invalidateQueries({ queryKey: claves.actualizacion() }),
  });
}

export function useInstalarActualizacion() {
  return useMutation({ mutationFn: () => ipc.instalarActualizacion() });
}
