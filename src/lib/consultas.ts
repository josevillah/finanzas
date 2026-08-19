/**
 * Claves de React Query y qué queda obsoleto con cada cambio.
 *
 * Está todo junto a propósito: el bug que motivó este archivo era que las
 * mutaciones de servicios refrescaban sus propias vistas pero no el
 * presupuesto ni los reportes, que dependen de los gastos que esos servicios
 * generan. Con las invalidaciones repartidas en tres archivos, la omisión no
 * se veía. Acá la relación entre lo que cambia y lo que hay que refrescar se
 * revisa de una sola lectura.
 */

import { useQueryClient } from "@tanstack/react-query";

/** Primer elemento de cada query key. */
export const RAICES = [
  "actualizacion",
  "ajustes",
  "calendario",
  "carga-financiera",
  "categorias",
  "cuentas",
  "cuotas-mes",
  "deuda",
  "deudas",
  "terceros",
  "estado-respaldo",
  "evolucion-gastos",
  "fecha-libertad",
  "movimientos",
  "periodo",
  "meses-disponibles",
  "metas",
  "periodos",
  "presupuesto",
  "reporte-hormiga",
  "resumen-periodo",
  "resumen-reinicio",
  "resumen-servicios",
  "servicios",
  "simulacion",
] as const;

/** Una raíz mal escrita no compila, en vez de fallar en silencio. */
export type Raiz = (typeof RAICES)[number];

// ── claves ───────────────────────────────────────────────────────────────────

/** Única fuente de las query keys: ningún componente arma la suya. */
export const claves = {
  deudas: (estado?: string | null) => ["deudas", estado ?? "todas"] as const,
  deuda: (id: number) => ["deuda", id] as const,
  terceros: () => ["terceros"] as const,
  calendario: (meses: number) => ["calendario", meses] as const,
  cargaFinanciera: (anio: number, mes: number) => ["carga-financiera", anio, mes] as const,
  cuentas: () => ["cuentas"] as const,
  metas: (filtro: string) => ["metas", filtro] as const,
  cuotasMes: (anio: number, mes: number) => ["cuotas-mes", anio, mes] as const,
  fechaLibertad: () => ["fecha-libertad"] as const,
  simulacion: (monto: number, tasa: number, cuotas: number, fecha: string) =>
    ["simulacion", monto, tasa, cuotas, fecha] as const,

  periodo: (anio: number, mes: number) => ["periodo", anio, mes] as const,
  mesesDisponibles: () => ["meses-disponibles"] as const,
  resumenPeriodo: (anio: number, mes: number) => ["resumen-periodo", anio, mes] as const,
  movimientos: (anio: number, mes: number, filtro: unknown) =>
    ["movimientos", anio, mes, filtro] as const,

  categorias: (soloActivas: boolean) => ["categorias", soloActivas] as const,
  servicios: (soloActivos: boolean) => ["servicios", soloActivos] as const,
  resumenServicios: (anio: number, mes: number) => ["resumen-servicios", anio, mes] as const,

  presupuesto: (anio: number, mes: number) => ["presupuesto", anio, mes] as const,
  evolucionGastos: (anio: number, mes: number, meses: number) =>
    ["evolucion-gastos", anio, mes, meses] as const,
  reporteHormiga: (anio: number, mes: number, meses: number) =>
    ["reporte-hormiga", anio, mes, meses] as const,

  estadoRespaldo: () => ["estado-respaldo"] as const,
  resumenReinicio: () => ["resumen-reinicio"] as const,
  ajustes: () => ["ajustes"] as const,
  actualizacion: () => ["actualizacion"] as const,
};

// ── qué depende de qué ───────────────────────────────────────────────────────

/**
 * Vistas que leen movimientos. Cualquier cosa que cree, edite o borre uno
 * las deja obsoletas: el resumen del mes, el presupuesto por categoría y los
 * dos reportes salen todos de la misma tabla.
 */
const POR_MOVIMIENTO = [
  "movimientos",
  "resumen-periodo",
  "resumen-servicios",
  "presupuesto",
  "evolucion-gastos",
  "reporte-hormiga",
  "meses-disponibles",
  // Las metas proyectan contra el balance promedio de los últimos meses, que
  // sale de estos mismos movimientos.
  "metas",
  // El disponible se calcula desde los movimientos: cada gasto o ingreso lo
  // cambia.
  "cuentas",
] as const satisfies readonly Raiz[];

/** Vistas sobre deudas y cuotas. */
const POR_DEUDA = [
  "deudas",
  "deuda",
  "terceros",
  "calendario",
  "carga-financiera",
  "cuotas-mes",
  "fecha-libertad",
] as const satisfies readonly Raiz[];

export type EventoDominio =
  | "servicio"
  | "categoria"
  | "movimiento"
  | "cuota"
  | "deuda"
  | "periodo"
  | "presupuesto"
  | "cuenta"
  | "meta";

export const RELACIONES: Record<EventoDominio, readonly Raiz[]> = {
  // Un servicio activo materializa su gasto del mes, así que arrastra todo lo
  // que depende de movimientos.
  servicio: ["servicios", ...POR_MOVIMIENTO],

  // Renombrar o desactivar una categoría cambia cómo se ven los gastos, el
  // desglose del presupuesto y las series de los reportes.
  categoria: ["categorias", "servicios", ...POR_MOVIMIENTO],

  movimiento: POR_MOVIMIENTO,

  // Pagar o deshacer una cuota toca la deuda y además crea o borra el gasto
  // del mes en la categoría de deudas.
  cuota: [...POR_DEUDA, ...POR_MOVIMIENTO],

  deuda: POR_DEUDA,

  // El sueldo del período alimenta el semáforo de carga, el "sin asignar" del
  // presupuesto y —porque vive en `periodos` y no en `movimientos`— también el
  // disponible de Cuentas.
  periodo: [
    "periodo",
    "meses-disponibles",
    "resumen-periodo",
    "carga-financiera",
    "presupuesto",
    "cuentas",
    // El sueldo entra en el balance del mes, y de ahí sale la proyección de
    // las metas.
    "metas",
  ],

  presupuesto: ["presupuesto"],

  // Apartar plata o ajustar el saldo inicial no alimenta ningún otro cálculo:
  // el presupuesto y los reportes salen de los movimientos, que no se tocan.
  // La dependencia va al revés, y está en POR_MOVIMIENTO.
  //
  // Las metas sí: su avance es el saldo de la cuenta a la que apuntan.
  cuenta: ["cuentas", "metas"],

  // Una meta no mueve plata: no invalida saldos ni nada del mes. Es el único
  // evento del dominio que solo se refresca a sí mismo.
  meta: ["metas"],
};

/**
 * Invalida lo que corresponda a uno o varios eventos de dominio.
 *
 *     const invalidar = useInvalidar();
 *     invalidar("servicio");
 */
export function useInvalidar() {
  const qc = useQueryClient();

  return (...eventos: EventoDominio[]) => {
    const raices = new Set<Raiz>(eventos.flatMap((evento) => RELACIONES[evento]));
    for (const raiz of raices) {
      qc.invalidateQueries({ queryKey: [raiz] });
    }
  };
}

/**
 * Queries que a propósito no dependen de ningún evento de dominio:
 *
 * - `simulacion`: se calcula desde sus propios parámetros, que ya son parte de
 *   la clave. No hay nada que invalidar.
 * - `estado-respaldo`, `ajustes`, `actualizacion`: las refresca su propia
 *   mutación, y no las toca ningún cambio de datos financieros.
 * - `resumen-reinicio`: se pide con `staleTime: 0` cada vez que se abre el
 *   diálogo, porque los números tienen que ser los del momento.
 *
 * El reinicio de datos no usa esta tabla: vacía la base entera, así que llama
 * a `queryClient.clear()`.
 */
