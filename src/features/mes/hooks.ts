import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";
import type {
  EstadoPeriodo,
  FiltroMovimientos,
  MedioPago,
  NuevoMovimiento,
} from "@/types/dominio";

/**
 * Qué se invalida con cada cosa está en `RELACIONES`, en `lib/consultas.ts`.
 * Acá solo se declara qué pasó.
 */
export function useInvalidarMes() {
  const invalidar = useInvalidar();
  return () => invalidar("movimiento", "periodo");
}

// ── lecturas ─────────────────────────────────────────────────────────────────

export function useResumenPeriodo(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.resumenPeriodo(anio, mes),
    queryFn: () => ipc.resumenPeriodo(anio, mes),
  });
}

/** Rango navegable y qué meses tienen contenido. Alimenta el selector. */
export function useMesesDisponibles() {
  return useQuery({
    queryKey: claves.mesesDisponibles(),
    queryFn: () => ipc.mesesDisponibles(),
  });
}

export function useMovimientos(anio: number, mes: number, filtro: FiltroMovimientos) {
  return useQuery({
    queryKey: claves.movimientos(anio, mes, filtro),
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
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: ({ anio, mes, estado }: { anio: number; mes: number; estado: EstadoPeriodo }) =>
      ipc.cambiarEstadoPeriodo(anio, mes, estado),
    // Cerrar o reabrir un mes no cambia sus movimientos, solo si se aceptan.
    onSuccess: () => invalidar("periodo"),
  });
}
