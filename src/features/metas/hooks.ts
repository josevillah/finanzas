import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";
import type { EstadoMeta, NuevaMeta } from "@/types/dominio";

/** Lo que puede pedir el filtro de la pantalla. */
export type FiltroMetas = EstadoMeta | "todas";

export function useResumenMetas(filtro: FiltroMetas) {
  return useQuery({
    queryKey: claves.metas(filtro),
    queryFn: () => ipc.resumenMetas(filtro),
  });
}

/**
 * Todas las mutaciones de metas invalidan lo mismo, y solo lo suyo: una meta
 * no mueve plata. Qué exactamente, en `lib/consultas.ts`.
 */
function useInvalidarMetas() {
  const invalidar = useInvalidar();
  return () => invalidar("meta");
}

export function useCrearMeta() {
  const invalidar = useInvalidarMetas();
  return useMutation({
    mutationFn: (datos: NuevaMeta) => ipc.crearMeta(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarMeta() {
  const invalidar = useInvalidarMetas();
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevaMeta }) =>
      ipc.actualizarMeta(id, datos),
    onSuccess: invalidar,
  });
}

export function useCambiarEstadoMeta() {
  const invalidar = useInvalidarMetas();
  return useMutation({
    mutationFn: ({ id, estado }: { id: number; estado: EstadoMeta }) =>
      ipc.cambiarEstadoMeta(id, estado),
    onSuccess: invalidar,
  });
}

export function useEliminarMeta() {
  const invalidar = useInvalidarMetas();
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarMeta(id),
    onSuccess: invalidar,
  });
}

export function useReordenarMetas() {
  const invalidar = useInvalidarMetas();
  return useMutation({
    mutationFn: (ids: number[]) => ipc.reordenarMetas(ids),
    onSuccess: invalidar,
  });
}
