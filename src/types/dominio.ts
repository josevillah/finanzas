/**
 * Espejo de `src-tauri/src/modelos/`. Si cambia un struct en Rust, cambia acá.
 * Todos los montos son enteros de pesos chilenos; todas las fechas son
 * strings ISO 'YYYY-MM-DD'.
 */

export type TipoDeuda = "compra_cuotas" | "credito_consumo" | "avance" | "rotativo";
export type EstadoDeuda = "vigente" | "pagada" | "repactada";
export type EstadoCuota = "pendiente" | "pagada" | "atrasada";
export type Semaforo = "verde" | "amarillo" | "rojo" | "sin_datos";

export interface Deuda {
  id: number;
  descripcion: string;
  tipo: TipoDeuda;
  institucion: string | null;
  monto_original: number;
  /** Fracción mensual: 0.025 = 2,5% mensual. */
  tasa_mensual: number;
  n_cuotas: number;
  fecha_primera_cuota: string;
  estado: EstadoDeuda;
  notas: string | null;
}

export interface Cuota {
  id: number;
  deuda_id: number;
  numero: number;
  fecha_vencimiento: string;
  monto: number;
  capital: number;
  interes: number;
  estado: EstadoCuota;
  fecha_pago: string | null;
  monto_pagado: number | null;
}

/** Cuota calculada aún no guardada (vista previa del formulario). */
export interface CuotaCalculada {
  numero: number;
  fecha_vencimiento: string;
  monto: number;
  capital: number;
  interes: number;
}

/** Rust la serializa con `flatten`, así que los campos de Deuda vienen inline. */
export interface DeudaResumen extends Deuda {
  total_programado: number;
  monto_pagado: number;
  monto_pendiente: number;
  cuotas_pagadas: number;
  cuotas_totales: number;
  avance_pct: number;
  cuotas_atrasadas: number;
  proxima_cuota: Cuota | null;
}

export interface DeudaDetalle {
  resumen: DeudaResumen;
  cuotas: Cuota[];
}

export interface CuotaConDeuda extends Cuota {
  deuda_descripcion: string;
}

export interface MesCarga {
  anio: number;
  mes: number;
  /** 'YYYY-MM' */
  clave: string;
  total: number;
  total_pendiente: number;
  n_cuotas: number;
}

export interface CargaFinanciera {
  anio: number;
  mes: number;
  total_cuotas: number;
  sueldo_liquido: number;
  otros_ingresos: number;
  porcentaje: number | null;
  semaforo: Semaforo;
  n_cuotas: number;
}

export interface Liberacion {
  deuda_id: number;
  descripcion: string;
  fecha_ultima_cuota: string;
  monto_mensual_liberado: number;
  cuotas_restantes: number;
}

export interface FechaLibertad {
  fecha_ultima_cuota: string | null;
  meses_restantes: number | null;
  total_pendiente: number;
  liberaciones: Liberacion[];
}

// ── Fase 2: períodos, categorías, gastos y servicios ─────────────────────────

export type EstadoPeriodo = "abierto" | "cerrado";
export type TipoCategoria = "fijo" | "variable" | "hormiga";
export type TipoServicio = "basico" | "suscripcion";
export type TipoMovimiento = "ingreso" | "gasto";
export type MedioPago = "efectivo" | "debito" | "credito" | "transferencia";

export interface Periodo {
  id: number;
  anio: number;
  mes: number;
  sueldo_liquido: number;
  otros_ingresos: number;
  estado: EstadoPeriodo;
}

export interface GastoPorCategoria {
  categoria_id: number | null;
  categoria_nombre: string;
  categoria_tipo: TipoCategoria | null;
  color: string | null;
  total: number;
  n_movimientos: number;
}

/** Los campos de Periodo vienen inline (serde flatten). */
export interface ResumenPeriodo extends Periodo {
  total_ingresos: number;
  ingresos_extra: number;
  total_gastos: number;
  balance: number;
  total_cuotas: number;
  total_hormiga: number;
  n_movimientos: number;
  por_categoria: GastoPorCategoria[];
}

export interface Categoria {
  id: number;
  nombre: string;
  tipo: TipoCategoria;
  color: string | null;
  activa: boolean;
  /** Código estable del sistema; si viene, la categoría no se puede eliminar. */
  codigo: string | null;
}

export interface NuevaCategoria {
  nombre: string;
  tipo: TipoCategoria;
  color: string | null;
  activa: boolean;
}

export interface Servicio {
  id: number;
  nombre: string;
  categoria_id: number | null;
  monto_estimado: number;
  dia_vencimiento: number | null;
  tipo: TipoServicio;
  activo: boolean;
  /** Desde cuándo existe: no genera gastos en meses anteriores. */
  fecha_alta: string | null;
}

export interface NuevoServicio {
  nombre: string;
  categoria_id: number | null;
  monto_estimado: number;
  dia_vencimiento: number | null;
  tipo: TipoServicio;
  activo: boolean;
  /** Solo se respeta al crear. Vacío = hoy. */
  fecha_alta: string | null;
}

export interface ServicioConReal extends Servicio {
  categoria_nombre: string | null;
  monto_real: number;
  n_movimientos: number;
  /** Cuántos de esos gastos siguen siendo el estimado sin confirmar. */
  n_estimados: number;
  /** real - estimado. Positivo = te pasaste. */
  diferencia: number;
  fecha_vencimiento: string | null;
  /** El servicio ya existía en el mes que se está viendo. */
  corresponde_al_mes: boolean;
}

export interface ResumenServicios {
  anio: number;
  mes: number;
  total_estimado: number;
  total_real: number;
  diferencia: number;
  sin_registrar: number;
  por_confirmar: number;
  servicios: ServicioConReal[];
}

export interface Movimiento {
  id: number;
  periodo_id: number;
  fecha: string;
  monto: number;
  tipo: TipoMovimiento;
  categoria_id: number | null;
  servicio_id: number | null;
  /** Si viene, es el pago de una cuota y no se edita desde gastos. */
  cuota_id: number | null;
  medio_pago: MedioPago | null;
  descripcion: string | null;
  /** Lo generó el sistema con el estimado del servicio y falta confirmarlo. */
  es_estimado: boolean;
}

export interface NuevoMovimiento {
  fecha: string;
  monto: number;
  tipo: TipoMovimiento;
  categoria_id: number | null;
  servicio_id: number | null;
  medio_pago: MedioPago | null;
  descripcion: string | null;
}

export interface MovimientoDetalle extends Movimiento {
  categoria_nombre: string | null;
  categoria_color: string | null;
  categoria_tipo: TipoCategoria | null;
  servicio_nombre: string | null;
  deuda_descripcion: string | null;
}

export interface FiltroMovimientos {
  tipo?: TipoMovimiento | null;
  categoria_id?: number | null;
  busqueda?: string | null;
}

// ── Fase 3: presupuesto y reportes ───────────────────────────────────────────

export type EstadoPresupuesto = "sin_asignar" | "ok" | "alerta" | "excedido";

export interface LineaPresupuesto {
  categoria_id: number;
  categoria_nombre: string;
  categoria_tipo: TipoCategoria;
  color: string | null;
  monto_asignado: number;
  monto_gastado: number;
  /** asignado - gastado. Negativo = te pasaste. */
  disponible: number;
  porcentaje_usado: number | null;
  estado: EstadoPresupuesto;
  n_movimientos: number;
}

export interface ResumenPresupuesto {
  anio: number;
  mes: number;
  total_asignado: number;
  total_gastado: number;
  disponible: number;
  porcentaje_usado: number | null;
  gasto_sin_presupuestar: number;
  total_gastos_mes: number;
  total_ingresos: number;
  sin_asignar_del_ingreso: number;
  categorias_excedidas: number;
  /** El mes está cerrado: no acepta cambios de asignación. */
  periodo_cerrado: boolean;
  lineas: LineaPresupuesto[];
}

export interface AsignacionPresupuesto {
  categoria_id: number;
  monto_asignado: number;
}

export interface PuntoMes {
  anio: number;
  mes: number;
  /** 'YYYY-MM' */
  clave: string;
  total: number;
}

export interface SerieCategoria {
  categoria_id: number | null;
  categoria_nombre: string;
  color: string | null;
  total: number;
  promedio: number;
  puntos: PuntoMes[];
}

export interface EvolucionGastos {
  meses: string[];
  series: SerieCategoria[];
  total_por_mes: PuntoMes[];
  total_ventana: number;
}

export interface MesHormiga {
  anio: number;
  mes: number;
  clave: string;
  total: number;
  n_movimientos: number;
  total_gastos: number;
  porcentaje: number | null;
}

export interface ReporteHormiga {
  meses: MesHormiga[];
  mes_actual: MesHormiga | null;
  promedio_previos: number;
  variacion_mes_anterior: number | null;
  variacion_promedio: number | null;
  por_categoria: GastoPorCategoria[];
  total_ventana: number;
}

// ── Fase 4: respaldo y exportación ───────────────────────────────────────────

export interface EstadoRespaldo {
  /** ISO 'YYYY-MM-DD' del último respaldo. */
  ultimo_respaldo: string | null;
  dias_desde_ultimo: number | null;
  /** Nunca respaldaste, o pasaron más de 7 días. */
  requiere_recordatorio: boolean;
  ruta_db: string;
  tamano_bytes: number;
  version_esquema: number;
  total_registros: number;
  /** Copia local automática activada. */
  respaldo_automatico: boolean;
  carpeta_respaldos: string;
  /** Fecha ISO de la copia automática más reciente. */
  ultimo_automatico: string | null;
  copias_automaticas: number;
}

export interface ArchivoExportado {
  nombre: string;
  ruta: string;
  filas: number;
}

export interface ResultadoExportacion {
  archivos: ArchivoExportado[];
  total_filas: number;
}

export interface ResultadoRestauracion {
  ruta_respaldo_previo: string;
  version_restaurada: number;
  total_registros: number;
}

// ── Configuración de la app ──────────────────────────────────────────────────

export type AccionCierre = "preguntar" | "bandeja" | "salir";

export interface AjustesApp {
  accion_cierre: AccionCierre;
  autostart_activo: boolean;
  /** false si otra aplicación ya tenía tomada la combinación. */
  atajo_registrado: boolean;
  atajo: string;
}

export interface EstadoActualizacion {
  version_actual: string;
  version_disponible: string | null;
  /** Notas del release, en markdown crudo. */
  notas: string | null;
  lista_para_instalar: boolean;
}

export const ETIQUETAS_ACCION_CIERRE: Record<AccionCierre, string> = {
  preguntar: "Preguntarme cada vez",
  bandeja: "Dejarla corriendo en la bandeja",
  salir: "Cerrar la aplicación",
};

export const DETALLE_ACCION_CIERRE: Record<AccionCierre, string> = {
  preguntar: "Muestra este mismo diálogo cada vez que cierres con la X.",
  bandeja: "La ventana se esconde pero la app sigue viva y el atajo funciona.",
  salir: "La app se cierra del todo y el atajo deja de funcionar.",
};

export const ETIQUETAS_ESTADO_PRESUPUESTO: Record<EstadoPresupuesto, string> = {
  sin_asignar: "Sin presupuesto",
  ok: "Al día",
  alerta: "Cerca del límite",
  excedido: "Excedido",
};

export const ETIQUETAS_TIPO_CATEGORIA: Record<TipoCategoria, string> = {
  fijo: "Fijo",
  variable: "Variable",
  hormiga: "Hormiga",
};

export const ETIQUETAS_TIPO_SERVICIO: Record<TipoServicio, string> = {
  basico: "Servicio básico",
  suscripcion: "Suscripción",
};

export const ETIQUETAS_MEDIO_PAGO: Record<MedioPago, string> = {
  efectivo: "Efectivo",
  debito: "Débito",
  credito: "Crédito",
  transferencia: "Transferencia",
};

export const MEDIOS_PAGO: MedioPago[] = ["debito", "credito", "efectivo", "transferencia"];

/** Payload de creación/edición. */
export interface NuevaDeuda {
  descripcion: string;
  tipo: TipoDeuda;
  institucion: string | null;
  monto_original: number;
  tasa_mensual: number;
  n_cuotas: number;
  fecha_primera_cuota: string;
  notas: string | null;
}

export const ETIQUETAS_TIPO_DEUDA: Record<TipoDeuda, string> = {
  compra_cuotas: "Compra en cuotas",
  credito_consumo: "Crédito de consumo",
  avance: "Avance en efectivo",
  rotativo: "Línea / rotativo",
};

export const ETIQUETAS_ESTADO_DEUDA: Record<EstadoDeuda, string> = {
  vigente: "Vigente",
  pagada: "Pagada",
  repactada: "Repactada",
};

export const ETIQUETAS_ESTADO_CUOTA: Record<EstadoCuota, string> = {
  pendiente: "Pendiente",
  pagada: "Pagada",
  atrasada: "Atrasada",
};
