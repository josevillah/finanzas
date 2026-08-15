import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";
import type { AsignacionPresupuesto } from "@/types/dominio";

function useInvalidarPresupuesto() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["presupuesto"] });
  };
}

export function useResumenPresupuesto(anio: number, mes: number) {
  return useQuery({
    queryKey: ["presupuesto", anio, mes],
    queryFn: () => ipc.resumenPresupuesto(anio, mes),
  });
}

export function useAsignarPresupuesto() {
  const invalidar = useInvalidarPresupuesto();
  return useMutation({
    mutationFn: ({
      anio,
      mes,
      asignaciones,
    }: {
      anio: number;
      mes: number;
      asignaciones: AsignacionPresupuesto[];
    }) => ipc.asignarPresupuesto(anio, mes, asignaciones),
    onSuccess: invalidar,
  });
}

export function useCopiarPresupuesto() {
  const invalidar = useInvalidarPresupuesto();
  return useMutation({
    mutationFn: ipc.copiarPresupuesto,
    onSuccess: invalidar,
  });
}
