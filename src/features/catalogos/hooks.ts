import { useEffect } from "react";

import { useMutation, useQuery } from "@tanstack/react-query";

import { claves, useInvalidar, type EventoDominio } from "@/lib/consultas";
import { mesAbsoluto, mesActual } from "@/lib/fechas";
import * as ipc from "@/lib/ipc";
import type { NuevaCategoria, NuevoServicio } from "@/types/dominio";

/**
 * Meses cuya generación de gastos de servicios ya se disparó en esta sesión.
 * Es a nivel de módulo para que dos pantallas del mismo mes no repitan la
 * llamada. Se limpia al tocar los servicios, porque uno nuevo sí debe generar.
 */
const mesesGenerados = new Set<string>();

/**
 * Servicios y categorías terminan afectando los gastos del mes, y por ahí el
 * presupuesto y los reportes. Qué se invalida exactamente está en
 * `RELACIONES`, en `lib/consultas.ts`.
 */
function useInvalidarCatalogos(evento: Extract<EventoDominio, "servicio" | "categoria">) {
  const invalidar = useInvalidar();

  return () => {
    // Un servicio nuevo o reactivado tiene que poder generar su gasto aunque
    // el mes ya se haya visitado. Sin esto, el gasto no aparece hasta que se
    // reinicia la app.
    mesesGenerados.clear();
    invalidar(evento);
  };
}

// ── categorías ───────────────────────────────────────────────────────────────

export function useCategorias(soloActivas = false) {
  return useQuery({
    queryKey: claves.categorias(soloActivas),
    // Cambian poco y las usan casi todas las pantallas.
    staleTime: 5 * 60_000,
    queryFn: () => ipc.listarCategorias(soloActivas),
  });
}

export function useCrearCategoria() {
  const invalidar = useInvalidarCatalogos("categoria");
  return useMutation({
    mutationFn: (datos: NuevaCategoria) => ipc.crearCategoria(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarCategoria() {
  const invalidar = useInvalidarCatalogos("categoria");
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevaCategoria }) =>
      ipc.actualizarCategoria(id, datos),
    onSuccess: invalidar,
  });
}

export function useEliminarCategoria() {
  const invalidar = useInvalidarCatalogos("categoria");
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarCategoria(id),
    onSuccess: invalidar,
  });
}

// ── servicios ────────────────────────────────────────────────────────────────

export function useServicios(soloActivos = false) {
  return useQuery({
    queryKey: claves.servicios(soloActivos),
    queryFn: () => ipc.listarServicios(soloActivos),
  });
}

export function useResumenServicios(anio: number, mes: number) {
  return useQuery({
    queryKey: claves.resumenServicios(anio, mes),
    queryFn: () => ipc.resumenServicios(anio, mes),
  });
}

export function useCrearServicio() {
  const invalidar = useInvalidarCatalogos("servicio");
  return useMutation({
    mutationFn: (datos: NuevoServicio) => ipc.crearServicio(datos),
    onSuccess: invalidar,
  });
}

export function useActualizarServicio() {
  const invalidar = useInvalidarCatalogos("servicio");
  return useMutation({
    mutationFn: ({ id, datos }: { id: number; datos: NuevoServicio }) =>
      ipc.actualizarServicio(id, datos),
    onSuccess: invalidar,
  });
}

export function useEliminarServicio() {
  const invalidar = useInvalidarCatalogos("servicio");
  return useMutation({
    mutationFn: (id: number) => ipc.eliminarServicio(id),
    onSuccess: invalidar,
  });
}

export function useGenerarGastosServicios() {
  const invalidar = useInvalidarCatalogos("servicio");
  return useMutation({
    mutationFn: ({ anio, mes }: { anio: number; mes: number }) =>
      ipc.generarGastosServicios(anio, mes),
    onSuccess: invalidar,
  });
}

export function useActivarServicioEnMes() {
  const invalidar = useInvalidarCatalogos("servicio");
  return useMutation({
    mutationFn: ipc.activarServicioEnMes,
    onSuccess: invalidar,
  });
}

/**
 * Materializa los gastos de los servicios del mes que se está viendo, una vez
 * por mes y por sesión. El comando es idempotente y no toca meses anteriores
 * al alta de cada servicio, así que abrir un mes viejo no lo contamina.
 *
 * Un mes futuro no genera nada: sería una proyección descontando disponible.
 * El backend es el que manda —también corta ahí—; esto solo evita el viaje.
 */
export function useGenerarAlEntrarAlMes(anio: number, mes: number) {
  const generar = useGenerarGastosServicios();

  useEffect(() => {
    const hoy = mesActual();
    if (mesAbsoluto(anio, mes) > mesAbsoluto(hoy.anio, hoy.mes)) return;

    const clave = `${anio}-${mes}`;
    if (mesesGenerados.has(clave)) return;

    mesesGenerados.add(clave);
    generar.mutate({ anio, mes });
    // `generar` cambia de identidad en cada render de react-query; el Set es
    // lo que evita repetir la llamada.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [anio, mes]);
}
