import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";
import type { EstadoDeuda, NuevaDeuda } from "@/types/dominio";

/** Claves de caché en un solo lugar, para invalidar sin adivinar. */
export const claves = {
  deudas: (estado?: EstadoDeuda | null) => ["deudas", estado ?? "todas"] as const,
  deuda: (id: number) => ["deuda", id] as const,
  calendario: (meses: number) => ["calendario", meses] as const,
  cargaFinanciera: (anio: number, mes: number) => ["carga-financiera", anio, mes] as const,
  fechaLibertad: () => ["fecha-libertad"] as const,
  periodo: (anio: number, mes: number) => ["periodo", anio, mes] as const,
};

/**
 * Cualquier cambio en deudas o cuotas mueve las tres vistas de análisis,
 * así que se invalidan juntas.
 */
function useInvalidarTodo() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["deudas"] });
    qc.invalidateQueries({ queryKey: ["deuda"] });
    qc.invalidateQueries({ queryKey: ["calendario"] });
    qc.invalidateQueries({ queryKey: ["carga-financiera"] });
    qc.invalidateQueries({ queryKey: ["cuotas-mes"] });
    qc.invalidateQueries({ queryKey: ["fecha-libertad"] });
    // Pagar o deshacer una cuota crea o borra un gasto del mes.
    qc.invalidateQueries({ queryKey: ["movimientos"] });
    qc.invalidateQueries({ queryKey: ["resumen-periodo"] });
  };
}

// ── lecturas ─────────────────────────────────────────────────────────────────

export function useDeudas(filtroEstado?: EstadoDeuda | null) {
  return useQuery({
    queryKey: claves.deudas(filtroEstado),
    queryFn: () => ipc.listarDeudas(filtroEstado),
  });
}

export function useDeuda(id: number) {
  return useQuery({
    queryKey: claves.deuda(id),
    queryFn: () => ipc.obtenerDeuda(id),
    enabled: Number.isFinite(id) && id > 0,
  });
}

export function useCalendarioCarga(meses = 24) {
  return useQuery({
    queryKey: claves.calendario(meses),
    queryFn: () => ipc.calendarioCarga(meses),
  });
}

export function useCargaFinanciera(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.cargaFinanciera(anio, mes),
    queryFn: () => ipc.cargaFinanciera(anio, mes),
  });
}

export function useFechaLibertad() {
  return useQuery({
    queryKey: claves.fechaLibertad(),
    queryFn: () => ipc.fechaLibertad(),
  });
}

export function useCuotasMes(anio: number, mes: number) {
  return useQuery({
    queryKey: ["cuotas-mes", anio, mes],
    queryFn: () => ipc.listarCuotasMes(anio, mes),
  });
}

export function usePeriodo(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.periodo(anio, mes),
    queryFn: () => ipc.obtenerPeriodo(anio, mes),
  });
}

// ── escrituras ───────────────────────────────────────────────────────────────

export function useCrearDeuda() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: (datos: NuevaDeuda) => ipc.crearDeuda(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarDeuda() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevaDeuda }) =>
      ipc.actualizarDeuda(id, datos),
    onSuccess: invalidar,
  });
}

export function useEliminarDeuda() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarDeuda(id),
    onSuccess: invalidar,
  });
}

export function useCambiarEstadoDeuda() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: ({ id, estado }: { id: number; estado: EstadoDeuda }) =>
      ipc.cambiarEstadoDeuda(id, estado),
    onSuccess: invalidar,
  });
}

export function usePagarCuota() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: (pago: { cuota_id: number; fecha_pago: string | null; monto_pagado: number }) =>
      ipc.pagarCuota(pago),
    onSuccess: invalidar,
  });
}

export function useDeshacerPago() {
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: (cuotaId: number) => ipc.deshacerPagoCuota(cuotaId),
    onSuccess: invalidar,
  });
}

export function useGuardarIngresos() {
  const qc = useQueryClient();
  const invalidar = useInvalidarTodo();
  return useMutation({
    mutationFn: ipc.guardarIngresosPeriodo,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["periodo"] });
      invalidar();
    },
  });
}

/** Vista previa de cuotas antes de guardar la deuda. */
export function useSimulacion(datos: {
  montoOriginal: number;
  tasaMensual: number;
  nCuotas: number;
  fechaPrimeraCuota: string;
  habilitado: boolean;
}) {
  return useQuery({
    queryKey: [
      "simulacion",
      datos.montoOriginal,
      datos.tasaMensual,
      datos.nCuotas,
      datos.fechaPrimeraCuota,
    ],
    queryFn: () =>
      ipc.simularCuotas({
        montoOriginal: datos.montoOriginal,
        tasaMensual: datos.tasaMensual,
        nCuotas: datos.nCuotas,
        fechaPrimeraCuota: datos.fechaPrimeraCuota,
      }),
    enabled: datos.habilitado,
    retry: false,
  });
}
