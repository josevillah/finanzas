import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";
import type { DireccionDeuda, EstadoDeuda, NuevaDeuda } from "@/types/dominio";

// ── lecturas ─────────────────────────────────────────────────────────────────

export function useDeudas(filtroEstado?: EstadoDeuda | null, direccion?: DireccionDeuda | null) {
  return useQuery({
    queryKey: [...claves.deudas(filtroEstado), direccion ?? "todas"],
    queryFn: () => ipc.listarDeudas(filtroEstado, direccion),
  });
}

/** Resumen por persona de lo que me deben. */
export function useResumenTerceros() {
  return useQuery({
    queryKey: claves.terceros(),
    queryFn: () => ipc.resumenTerceros(),
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
    queryKey: claves.cuotasMes(anio, mes),
    queryFn: () => ipc.listarCuotasMes(anio, mes),
  });
}

export function usePeriodo(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.periodo(anio, mes),
    queryFn: () => ipc.obtenerPeriodo(anio, mes),
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
    queryKey: claves.simulacion(
      datos.montoOriginal,
      datos.tasaMensual,
      datos.nCuotas,
      datos.fechaPrimeraCuota,
    ),
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

// ── escrituras ───────────────────────────────────────────────────────────────

/**
 * Crear, editar o borrar una deuda no toca los gastos del mes: las cuotas
 * nacen pendientes. Lo que sí genera movimientos es pagarlas.
 */
function useInvalidarDeuda() {
  const invalidar = useInvalidar();
  return () => invalidar("deuda");
}

export function useCrearDeuda() {
  const invalidar = useInvalidarDeuda();
  return useMutation({
    mutationFn: (datos: NuevaDeuda) => ipc.crearDeuda(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarDeuda() {
  const invalidar = useInvalidarDeuda();
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevaDeuda }) =>
      ipc.actualizarDeuda(id, datos),
    onSuccess: invalidar,
  });
}

export function useEliminarDeuda() {
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarDeuda(id),
    // Borrar una deuda arrastra sus cuotas pagadas, y con ellas los gastos que
    // habían generado en los meses.
    onSuccess: () => invalidar("cuota"),
  });
}

export function useCambiarEstadoDeuda() {
  const invalidar = useInvalidarDeuda();
  return useMutation({
    mutationFn: ({ id, estado }: { id: number; estado: EstadoDeuda }) =>
      ipc.cambiarEstadoDeuda(id, estado),
    onSuccess: invalidar,
  });
}

export function usePagarCuota() {
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: (pago: { cuota_id: number; fecha_pago: string | null; monto_pagado: number }) =>
      ipc.pagarCuota(pago),
    // Pagar crea el gasto del mes: además de la deuda, cambia el presupuesto
    // de "Deudas y créditos" y las series de los reportes.
    onSuccess: () => invalidar("cuota"),
  });
}

export function useDeshacerPago() {
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: (cuotaId: number) => ipc.deshacerPagoCuota(cuotaId),
    onSuccess: () => invalidar("cuota"),
  });
}

export function useGuardarIngresos() {
  const invalidar = useInvalidar();
  return useMutation({
    mutationFn: ipc.guardarIngresosPeriodo,
    // El sueldo alimenta el semáforo de carga y el "sin asignar" del
    // presupuesto.
    onSuccess: () => invalidar("periodo"),
  });
}
