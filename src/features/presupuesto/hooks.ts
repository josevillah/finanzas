import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";
import type { AsignacionPresupuesto } from "@/types/dominio";

export function useResumenPresupuesto(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.presupuesto(anio, mes),
    queryFn: () => ipc.resumenPresupuesto(anio, mes),
  });
}

export function useAsignarPresupuesto() {
  const invalidar = useInvalidar();
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
    // Asignar no mueve gastos: solo cambia el monto contra el que se comparan.
    onSuccess: () => invalidar("presupuesto"),
  });
}

export function useCopiarPresupuesto() {
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: ipc.copiarPresupuesto,
    onSuccess: () => invalidar("presupuesto"),
  });
}
