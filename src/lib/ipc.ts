/**
 * Única puerta de entrada al backend. Ningún componente llama a `invoke`
 * directamente: así los nombres de comando y sus tipos viven en un solo lugar.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  AccionCierre,
  AjustesApp,
  AsignacionPresupuesto,
  CargaFinanciera,
  Categoria,
  EvolucionGastos,
  Cuota,
  CuotaCalculada,
  CuotaConDeuda,
  DeudaDetalle,
  DeudaResumen,
  EstadoDeuda,
  EstadoPeriodo,
  EstadoRespaldo,
  FechaLibertad,
  FiltroMovimientos,
  MedioPago,
  MesCarga,
  MovimientoDetalle,
  NuevaCategoria,
  NuevaDeuda,
  NuevoMovimiento,
  NuevoServicio,
  Periodo,
  ReporteHormiga,
  ResultadoExportacion,
  ResultadoRestauracion,
  ResumenPeriodo,
  ResumenPresupuesto,
  ResumenServicios,
  Servicio,
} from "@/types/dominio";

// ── deudas ───────────────────────────────────────────────────────────────────

export function simularCuotas(datos: {
  montoOriginal: number;
  tasaMensual: number;
  nCuotas: number;
  fechaPrimeraCuota: string;
}): Promise<CuotaCalculada[]> {
  return invoke("simular_cuotas", {
    montoOriginal: datos.montoOriginal,
    tasaMensual: datos.tasaMensual,
    nCuotas: datos.nCuotas,
    fechaPrimeraCuota: datos.fechaPrimeraCuota,
  });
}

export function crearDeuda(datos: NuevaDeuda): Promise<number> {
  return invoke("crear_deuda", { datos });
}

export function actualizarDeuda(id: number, datos: NuevaDeuda): Promise<void> {
  return invoke("actualizar_deuda", { id, datos });
}

export function eliminarDeuda(id: number): Promise<void> {
  return invoke("eliminar_deuda", { id });
}

export function cambiarEstadoDeuda(id: number, nuevoEstado: EstadoDeuda): Promise<void> {
  return invoke("cambiar_estado_deuda", { id, nuevoEstado });
}

export function listarDeudas(filtroEstado?: EstadoDeuda | null): Promise<DeudaResumen[]> {
  return invoke("listar_deudas", { filtroEstado: filtroEstado ?? null });
}

export function obtenerDeuda(id: number): Promise<DeudaDetalle> {
  return invoke("obtener_deuda", { id });
}

// ── cuotas ───────────────────────────────────────────────────────────────────

export function pagarCuota(pago: {
  cuota_id: number;
  fecha_pago: string | null;
  monto_pagado: number;
}): Promise<void> {
  return invoke("pagar_cuota", { pago });
}

export function deshacerPagoCuota(cuotaId: number): Promise<void> {
  return invoke("deshacer_pago_cuota", { cuotaId });
}

export function listarCuotasDeuda(deudaId: number): Promise<Cuota[]> {
  return invoke("listar_cuotas_deuda", { deudaId });
}

export function listarCuotasMes(anio: number, mes: number): Promise<CuotaConDeuda[]> {
  return invoke("listar_cuotas_mes", { anio, mes });
}

// ── análisis ─────────────────────────────────────────────────────────────────

export function calendarioCarga(meses = 24): Promise<MesCarga[]> {
  return invoke("calendario_carga", { meses });
}

export function cargaFinanciera(anio?: number, mes?: number): Promise<CargaFinanciera> {
  return invoke("carga_financiera", { anio: anio ?? null, mes: mes ?? null });
}

export function fechaLibertad(): Promise<FechaLibertad> {
  return invoke("fecha_libertad");
}

// ── períodos ─────────────────────────────────────────────────────────────────

export function obtenerPeriodo(anio: number, mes: number): Promise<Periodo> {
  return invoke("obtener_periodo", { anio, mes });
}

export function guardarIngresosPeriodo(datos: {
  anio: number;
  mes: number;
  sueldoLiquido: number;
  otrosIngresos: number;
}): Promise<Periodo> {
  return invoke("guardar_ingresos_periodo", {
    anio: datos.anio,
    mes: datos.mes,
    sueldoLiquido: datos.sueldoLiquido,
    otrosIngresos: datos.otrosIngresos,
  });
}

export function listarPeriodos(): Promise<Periodo[]> {
  return invoke("listar_periodos");
}

export function resumenPeriodo(anio: number, mes: number): Promise<ResumenPeriodo> {
  return invoke("resumen_periodo", { anio, mes });
}

export function cambiarEstadoPeriodo(
  anio: number,
  mes: number,
  nuevoEstado: EstadoPeriodo,
): Promise<Periodo> {
  return invoke("cambiar_estado_periodo", { anio, mes, nuevoEstado });
}

// ── movimientos ──────────────────────────────────────────────────────────────

export function registrarMovimiento(datos: NuevoMovimiento): Promise<number> {
  return invoke("registrar_movimiento", { datos });
}

export function actualizarMovimiento(id: number, datos: NuevoMovimiento): Promise<void> {
  return invoke("actualizar_movimiento", { id, datos });
}

export function cambiarMontoMovimiento(id: number, monto: number): Promise<void> {
  return invoke("cambiar_monto_movimiento", { id, monto });
}

export function eliminarMovimiento(id: number): Promise<void> {
  return invoke("eliminar_movimiento", { id });
}

export function listarMovimientos(
  anio: number,
  mes: number,
  filtro?: FiltroMovimientos,
): Promise<MovimientoDetalle[]> {
  return invoke("listar_movimientos", { anio, mes, filtro: filtro ?? null });
}

export function capturaRapida(datos: {
  monto: number;
  categoriaId: number;
  medioPago?: MedioPago | null;
  descripcion?: string | null;
}): Promise<number> {
  return invoke("captura_rapida", {
    monto: datos.monto,
    categoriaId: datos.categoriaId,
    medioPago: datos.medioPago ?? null,
    descripcion: datos.descripcion ?? null,
  });
}

// ── categorías ───────────────────────────────────────────────────────────────

export function listarCategorias(soloActivas = false): Promise<Categoria[]> {
  return invoke("listar_categorias", { soloActivas });
}

export function crearCategoria(datos: NuevaCategoria): Promise<number> {
  return invoke("crear_categoria", { datos });
}

export function actualizarCategoria(id: number, datos: NuevaCategoria): Promise<void> {
  return invoke("actualizar_categoria", { id, datos });
}

export function eliminarCategoria(id: number): Promise<void> {
  return invoke("eliminar_categoria", { id });
}

// ── servicios ────────────────────────────────────────────────────────────────

export function listarServicios(soloActivos = false): Promise<Servicio[]> {
  return invoke("listar_servicios", { soloActivos });
}

export function crearServicio(datos: NuevoServicio): Promise<number> {
  return invoke("crear_servicio", { datos });
}

export function actualizarServicio(id: number, datos: NuevoServicio): Promise<void> {
  return invoke("actualizar_servicio", { id, datos });
}

export function eliminarServicio(id: number): Promise<void> {
  return invoke("eliminar_servicio", { id });
}

export function resumenServicios(anio: number, mes: number): Promise<ResumenServicios> {
  return invoke("resumen_servicios", { anio, mes });
}

/** Crea el gasto del mes de cada servicio que aún no lo tenga. Devuelve cuántos. */
export function generarGastosServicios(anio: number, mes: number): Promise<number> {
  return invoke("generar_gastos_servicios", { anio, mes });
}

// ── presupuesto ──────────────────────────────────────────────────────────────

export function resumenPresupuesto(anio: number, mes: number): Promise<ResumenPresupuesto> {
  return invoke("resumen_presupuesto", { anio, mes });
}

export function asignarPresupuesto(
  anio: number,
  mes: number,
  asignaciones: AsignacionPresupuesto[],
): Promise<void> {
  return invoke("asignar_presupuesto", { anio, mes, asignaciones });
}

/** Devuelve cuántas líneas copió. */
export function copiarPresupuesto(datos: {
  desdeAnio: number;
  desdeMes: number;
  haciaAnio: number;
  haciaMes: number;
}): Promise<number> {
  return invoke("copiar_presupuesto", {
    desdeAnio: datos.desdeAnio,
    desdeMes: datos.desdeMes,
    haciaAnio: datos.haciaAnio,
    haciaMes: datos.haciaMes,
  });
}

// ── reportes ─────────────────────────────────────────────────────────────────

export function evolucionGastos(
  anio: number,
  mes: number,
  meses = 12,
): Promise<EvolucionGastos> {
  return invoke("evolucion_gastos", { anio, mes, meses });
}

export function reporteHormiga(anio: number, mes: number, meses = 12): Promise<ReporteHormiga> {
  return invoke("reporte_hormiga", { anio, mes, meses });
}

// ── respaldo y exportación ───────────────────────────────────────────────────

export function estadoRespaldo(): Promise<EstadoRespaldo> {
  return invoke("estado_respaldo");
}

/** Devuelve la ruta final del archivo. */
export function respaldarBase(destino: string): Promise<string> {
  return invoke("respaldar_base", { destino });
}

export function restaurarBase(origen: string): Promise<ResultadoRestauracion> {
  return invoke("restaurar_base", { origen });
}

export function exportarJson(destino: string): Promise<ResultadoExportacion> {
  return invoke("exportar_json", { destino });
}

/** Escribe un .csv por tabla dentro del directorio. */
export function exportarCsv(directorio: string): Promise<ResultadoExportacion> {
  return invoke("exportar_csv", { directorio });
}

export function fijarRespaldoAutomatico(activo: boolean): Promise<void> {
  return invoke("fijar_respaldo_automatico", { activo });
}

// ── configuración de la app ──────────────────────────────────────────────────

export function obtenerAjustes(): Promise<AjustesApp> {
  return invoke("obtener_ajustes");
}

export function fijarAccionCierre(accion: AccionCierre): Promise<void> {
  return invoke("fijar_accion_cierre", { accion });
}

/** Devuelve el estado real después del cambio, según lo que diga Windows. */
export function fijarAutostart(activo: boolean): Promise<boolean> {
  return invoke("fijar_autostart", { activo });
}

/** Resuelve el diálogo de cierre. `accion` solo puede ser bandeja o salir. */
export function resolverCierre(
  accion: Exclude<AccionCierre, "preguntar">,
  recordar: boolean,
): Promise<void> {
  return invoke("resolver_cierre", { accion, recordar });
}

/** Los comandos rechazan con un string; esto lo normaliza para mostrarlo. */
export function mensajeDeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Ocurrió un error inesperado.";
}
