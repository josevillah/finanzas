import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar } from "@/lib/consultas";
import * as ipc from "@/lib/ipc";
import type { NuevaCuenta } from "@/types/dominio";

export function useResumenCuentas() {
  return useQuery({
    queryKey: claves.cuentas(),
    queryFn: () => ipc.resumenCuentas(),
  });
}

/**
 * Todas las mutaciones de cuentas invalidan lo mismo. Qué exactamente está en
 * `RELACIONES`, en `lib/consultas.ts`.
 */
function useInvalidarCuentas() {
  const invalidar = useInvalidar();
  return () => invalidar("cuenta");
}

export function useFijarSaldoInicial() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: (saldo: number) => ipc.fijarSaldoInicial(saldo),
    onSuccess: invalidar,
  });
}

export function useApartar() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: ({ id, monto }: { id: number; monto: number }) => ipc.apartar(id, monto),
    onSuccess: invalidar,
  });
}

export function useRetirar() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: ({ id, monto }: { id: number; monto: number }) => ipc.retirar(id, monto),
    onSuccess: invalidar,
  });
}

export function useCrearCuenta() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: (datos: NuevaCuenta) => ipc.crearCuenta(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarCuenta() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: ({ id, nombre, activa }: { id: number; nombre: string; activa: boolean }) =>
      ipc.actualizarCuenta(id, nombre, activa),
    onSuccess: invalidar,
  });
}


export function useEliminarCuenta() {
  const invalidar = useInvalidarCuentas();
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarCuenta(id),
    onSuccess: invalidar,
  });
}
