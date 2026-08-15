import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";

export function useEstadoRespaldo() {
  return useQuery({
    queryKey: ["estado-respaldo"],
    queryFn: () => ipc.estadoRespaldo(),
    // El recordatorio se muestra en toda la app, así que conviene fresco.
    staleTime: 60_000,
  });
}

export function useRespaldar() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (destino: string) => ipc.respaldarBase(destino),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["estado-respaldo"] }),
  });
}

export function useFijarRespaldoAutomatico() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (activo: boolean) => ipc.fijarRespaldoAutomatico(activo),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["estado-respaldo"] }),
  });
}

export function useExportarJson() {
  return useMutation({ mutationFn: (destino: string) => ipc.exportarJson(destino) });
}

export function useExportarCsv() {
  return useMutation({ mutationFn: (directorio: string) => ipc.exportarCsv(directorio) });
}

/**
 * Restaurar reemplaza toda la base, así que después no queda nada válido en
 * caché: se limpia entera en vez de invalidar consulta por consulta.
 */
export function useRestaurar() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (origen: string) => ipc.restaurarBase(origen),
    onSuccess: () => qc.clear(),
  });
}
