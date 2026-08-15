import { useQuery } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";

export function useEvolucionGastos(anio: number, mes: number, meses: number) {
  return useQuery({
    queryKey: ["evolucion-gastos", anio, mes, meses],
    queryFn: () => ipc.evolucionGastos(anio, mes, meses),
  });
}

export function useReporteHormiga(anio: number, mes: number, meses: number) {
  return useQuery({
    queryKey: ["reporte-hormiga", anio, mes, meses],
    queryFn: () => ipc.reporteHormiga(anio, mes, meses),
  });
}
