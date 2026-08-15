import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import * as ipc from "@/lib/ipc";
import type {
  EstadoPeriodo,
  FiltroMovimientos,
  MedioPago,
  NuevoMovimiento,
} from "@/types/dominio";

/**
 * Un movimiento afecta al resumen del mes y a la comparación de servicios.
 * Los pagos de cuota además tocan las vistas de deudas, por eso el conjunto
 * completo se invalida desde un solo lugar.
 */
export function useInvalidarMes() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["movimientos"] });
    qc.invalidateQueries({ queryKey: ["resumen-periodo"] });
    qc.invalidateQueries({ queryKey: ["resumen-servicios"] });
    qc.invalidateQueries({ queryKey: ["periodo"] });
    qc.invalidateQueries({ queryKey: ["periodos"] });
    qc.invalidateQueries({ queryKey: ["carga-financiera"] });
    // El presupuesto y los reportes se leen contra los mismos movimientos.
    qc.invalidateQueries({ queryKey: ["presupuesto"] });
    qc.invalidateQueries({ queryKey: ["evolucion-gastos"] });
    qc.invalidateQueries({ queryKey: ["reporte-hormiga"] });
  };
}

// ── lecturas ─────────────────────────────────────────────────────────────────

export function useResumenPeriodo(anio: number, mes: number) {
  return useQuery({
    queryKey: ["resumen-periodo", anio, mes],
    queryFn: () => ipc.resumenPeriodo(anio, mes),
  });
}

export function usePeriodos() {
  return useQuery({
    queryKey: ["periodos"],
    queryFn: () => ipc.listarPeriodos(),
  });
}

export function useMovimientos(anio: number, mes: number, filtro: FiltroMovimientos) {
  return useQuery({
    queryKey: ["movimientos", anio, mes, filtro],
    queryFn: () => ipc.listarMovimientos(anio, mes, filtro),
  });
}

// ── escrituras ───────────────────────────────────────────────────────────────

export function useRegistrarMovimiento() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: (datos: NuevoMovimiento) => ipc.registrarMovimiento(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarMovimiento() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevoMovimiento }) =>
      ipc.actualizarMovimiento(id, datos),
    onSuccess: invalidar,
  });
}

/** Camino corto del botón "Cambiar precio". */
export function useCambiarMonto() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: ({ id, monto }: { id: number; monto: number }) =>
      ipc.cambiarMontoMovimiento(id, monto),
    onSuccess: invalidar,
  });
}

export function useEliminarMovimiento() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarMovimiento(id),
    onSuccess: invalidar,
  });
}

export function useCapturaRapida() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: (datos: {
      monto: number;
      categoriaId: number;
      medioPago?: MedioPago | null;
      descripcion?: string | null;
    }) => ipc.capturaRapida(datos),
    onSuccess: invalidar,
  });
}

export function useCambiarEstadoPeriodo() {
  const invalidar = useInvalidarMes();
  return useMutation({
    mutationFn: ({ anio, mes, estado }: { anio: number; mes: number; estado: EstadoPeriodo }) =>
      ipc.cambiarEstadoPeriodo(anio, mes, estado),
    onSuccess: invalidar,
  });
}
