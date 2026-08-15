import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";
import type { AccionCierre } from "@/types/dominio";

export function useAjustes() {
  return useQuery({
    queryKey: ["ajustes"],
    queryFn: () => ipc.obtenerAjustes(),
  });
}

function useInvalidarAjustes() {
  const qc = useQueryClient();
  return () => qc.invalidateQueries({ queryKey: ["ajustes"] });
}

export function useFijarAccionCierre() {
  const invalidar = useInvalidarAjustes();
  return useMutation({
    mutationFn: (accion: AccionCierre) => ipc.fijarAccionCierre(accion),
    onSuccess: invalidar,
  });
}

export function useFijarAutostart() {
  const invalidar = useInvalidarAjustes();
  return useMutation({
    mutationFn: (activo: boolean) => ipc.fijarAutostart(activo),
    onSuccess: invalidar,
  });
}

export function useResolverCierre() {
  return useMutation({
    mutationFn: ({
      accion,
      recordar,
    }: {
      accion: Exclude<AccionCierre, "preguntar">;
      recordar: boolean;
    }) => ipc.resolverCierre(accion, recordar),
  });
}
