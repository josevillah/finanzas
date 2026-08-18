import { useQuery } from "@tanstack/react-query";

import { claves } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";

export function useEvolucionGastos(anio: number, mes: number, meses: number) {
  return useQuery({
    queryKey: claves.evolucionGastos(anio, mes, meses),
    queryFn: () => ipc.evolucionGastos(anio, mes, meses),
  });
}

export function useReporteHormiga(anio: number, mes: number, meses: number) {
  return useQuery({
    queryKey: claves.reporteHormiga(anio, mes, meses),
    queryFn: () => ipc.reporteHormiga(anio, mes, meses),
  });
}
