/** Formato chileno: punto como separador de miles, sin decimales. */

const FORMATO = new Intl.NumberFormat("es-CL", { maximumFractionDigits: 0 });

/** 1234567 -> "$1.234.567". Los negativos quedan "-$1.234". */
export function formatearCLP(monto: number): string {
  const n = Math.round(monto || 0);
  const signo = n < 0 ? "-" : "";
  return `${signo}$${FORMATO.format(Math.abs(n))}`;
}

/** 1234567 -> "1.234.567" (sin signo peso, para inputs y ejes de gráficos). */
export function formatearMiles(monto: number): string {
  return FORMATO.format(Math.round(monto || 0));
}

/** 1234567 -> "$1,2M" — para etiquetas de gráficos donde no cabe el número. */
export function formatearCompacto(monto: number): string {
  const n = Math.abs(Math.round(monto || 0));
  const signo = monto < 0 ? "-" : "";
  if (n >= 1_000_000) return `${signo}$${(n / 1_000_000).toFixed(1).replace(".", ",")}M`;
  if (n >= 1_000) return `${signo}$${Math.round(n / 1_000)}K`;
  return `${signo}$${n}`;
}

/**
 * Acepta lo que sea que escriba el usuario ("1.234.567", "1234567", "$1 234 567")
 * y devuelve el entero. Ignora todo lo que no sea dígito, salvo un signo menos
 * inicial.
 */
export function parsearMonto(texto: string): number {
  if (!texto) return 0;
  const negativo = texto.trim().startsWith("-");
  const digitos = texto.replace(/\D/g, "");
  if (!digitos) return 0;
  const valor = Number.parseInt(digitos, 10);
  return negativo ? -valor : valor;
}

/** 18.437 -> "18,4%". */
export function formatearPorcentaje(pct: number | null, decimales = 1): string {
  if (pct === null || pct === undefined || !Number.isFinite(pct)) return "—";
  return `${pct.toFixed(decimales).replace(".", ",")}%`;
}

/** 0.021 -> "2,1%" (tasa mensual guardada como fracción). */
export function formatearTasa(tasa: number): string {
  if (!tasa) return "Sin interés";
  return `${(tasa * 100).toFixed(2).replace(".", ",")}% mensual`;
}

/** "2,5" o "2.5" -> 0.025. El usuario escribe porcentaje, la base guarda fracción. */
export function parsearTasaPorcentaje(texto: string): number {
  if (!texto?.trim()) return 0;
  const normalizado = texto.replace(",", ".").replace(/[^\d.]/g, "");
  const valor = Number.parseFloat(normalizado);
  return Number.isFinite(valor) ? valor / 100 : 0;
}
