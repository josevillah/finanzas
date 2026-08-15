import { cn } from "@/lib/cn";
import type { Semaforo } from "@/types/dominio";

export const TEXTO_SEMAFORO: Record<Semaforo, string> = {
  verde: "Carga sana",
  amarillo: "Carga ajustada",
  rojo: "Carga alta",
  sin_datos: "Falta el sueldo líquido",
};

export const DETALLE_SEMAFORO: Record<Semaforo, string> = {
  verde: "Menos del 15% del sueldo líquido se va en cuotas.",
  amarillo: "Entre 15% y 25% del sueldo líquido se va en cuotas.",
  rojo: "Más del 25% del sueldo líquido se va en cuotas.",
  sin_datos: "Registra tu sueldo líquido del mes para calcular el porcentaje.",
};

const COLOR_PUNTO: Record<Semaforo, string> = {
  verde: "bg-emerald-500",
  amarillo: "bg-amber-500",
  rojo: "bg-rose-500",
  sin_datos: "bg-slate-400",
};

export const TONO_SEMAFORO: Record<Semaforo, "verde" | "amarillo" | "rojo" | "indigo"> = {
  verde: "verde",
  amarillo: "amarillo",
  rojo: "rojo",
  sin_datos: "indigo",
};

export function PuntoSemaforo({ semaforo, className }: { semaforo: Semaforo; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn("inline-block h-2.5 w-2.5 rounded-full", COLOR_PUNTO[semaforo], className)}
    />
  );
}
