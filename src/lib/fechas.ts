/**
 * Todas las fechas viajan como string ISO 'YYYY-MM-DD'. Se parsean a mano
 * para no toparse con el corrimiento de zona horaria de `new Date("2026-09-05")`.
 */

export const MESES = [
  "enero", "febrero", "marzo", "abril", "mayo", "junio",
  "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
];

export const MESES_CORTOS = [
  "ene", "feb", "mar", "abr", "may", "jun",
  "jul", "ago", "sep", "oct", "nov", "dic",
];

export interface PartesFecha {
  anio: number;
  mes: number; // 1-12
  dia: number;
}

export function partirISO(iso: string): PartesFecha | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso ?? "");
  if (!m) return null;
  return { anio: Number(m[1]), mes: Number(m[2]), dia: Number(m[3]) };
}

/** "2026-09-05" -> "05 sep 2026". */
export function formatearFecha(iso: string | null): string {
  const p = iso ? partirISO(iso) : null;
  if (!p) return "—";
  return `${String(p.dia).padStart(2, "0")} ${MESES_CORTOS[p.mes - 1]} ${p.anio}`;
}

/** "2026-09-05" -> "sep 2026". */
export function formatearMesCorto(anio: number, mes: number): string {
  return `${MESES_CORTOS[mes - 1]} ${anio}`;
}

/** "2026-09-05" -> "septiembre de 2026". Para usar dentro de una frase. */
export function formatearMesLargo(anio: number, mes: number): string {
  return `${MESES[mes - 1]} de ${anio}`;
}

/**
 * Mayúscula solo en la primera letra, dejando el resto intacto.
 *
 * No se usa `text-transform: capitalize` de CSS porque capitaliza **cada**
 * palabra: "agosto de 2026" salía como "Agosto De 2026".
 */
export function capitalizar(texto: string): string {
  if (!texto) return texto;
  return texto.charAt(0).toLocaleUpperCase("es-CL") + texto.slice(1);
}

/** "2026-09-05" -> "Septiembre de 2026". Para títulos y encabezados. */
export function formatearMesTitulo(anio: number, mes: number): string {
  return capitalizar(formatearMesLargo(anio, mes));
}

/**
 * Índice absoluto del mes, para comparar y ordenar sin pelear con el borde de
 * diciembre. Espejo de `fechas::mes_absoluto` en Rust.
 */
export function mesAbsoluto(anio: number, mes: number): number {
  return anio * 12 + mes;
}

export function hoyISO(): string {
  const d = new Date();
  const mes = String(d.getMonth() + 1).padStart(2, "0");
  const dia = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mes}-${dia}`;
}

export function mesActual(): { anio: number; mes: number } {
  const d = new Date();
  return { anio: d.getFullYear(), mes: d.getMonth() + 1 };
}

/** ¿La fecha ISO ya pasó? Comparación lexicográfica: sirve con formato ISO. */
export function estaVencida(iso: string): boolean {
  return iso < hoyISO();
}

/** Convierte 14 meses en "1 año y 2 meses". */
export function describirMeses(meses: number | null): string {
  if (meses === null || meses === undefined) return "—";
  if (meses <= 0) return "este mes";

  const anios = Math.floor(meses / 12);
  const resto = meses % 12;

  const partes: string[] = [];
  if (anios > 0) partes.push(`${anios} ${anios === 1 ? "año" : "años"}`);
  if (resto > 0) partes.push(`${resto} ${resto === 1 ? "mes" : "meses"}`);

  return partes.join(" y ") || "este mes";
}

/**
 * Rango legible entre dos claves 'YYYY-MM', para decir qué período cubre un
 * reporte: "marzo a agosto de 2026", o "septiembre de 2025 a agosto de 2026"
 * cuando la ventana cruza el año.
 *
 * Un solo mes se dice entero: "agosto de 2026".
 */
export function describirRangoDeMeses(desde: string, hasta: string): string {
  const a = /^(\d{4})-(\d{2})$/.exec(desde ?? "");
  const b = /^(\d{4})-(\d{2})$/.exec(hasta ?? "");
  if (!a || !b) return "";

  const [anioA, mesA] = [Number(a[1]), Number(a[2])];
  const [anioB, mesB] = [Number(b[1]), Number(b[2])];

  if (anioA === anioB && mesA === mesB) return formatearMesLargo(anioB, mesB);
  if (anioA === anioB) return `${MESES[mesA - 1]} a ${formatearMesLargo(anioB, mesB)}`;

  return `${formatearMesLargo(anioA, mesA)} a ${formatearMesLargo(anioB, mesB)}`;
}
