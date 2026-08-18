import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { claves } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";

/** Conteos reales para mostrar antes de borrar. */
export function useResumenReinicio(habilitado: boolean) {
  return useQuery({
    queryKey: claves.resumenReinicio(),
    queryFn: () => ipc.resumenReinicio(),
    enabled: habilitado,
    // Se pide al abrir el diálogo: los números tienen que ser los de ahora.
    staleTime: 0,
  });
}

export function useReiniciarDatos() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ipc.reiniciarDatos,
    // Después de vaciar la base no queda nada válido en caché: se limpia
    // entera en vez de invalidar consulta por consulta.
    onSuccess: () => qc.clear(),
  });
}
