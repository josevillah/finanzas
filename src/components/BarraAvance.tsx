import { cn } from "@/lib/cn";

export function BarraAvance({
  porcentaje,
  className,
  tono = "indigo",
}: {
  porcentaje: number;
  className?: string;
  tono?: "indigo" | "verde" | "amarillo" | "rojo";
}) {
  const ancho = Math.max(0, Math.min(100, porcentaje));

  const colores = {
    indigo: "bg-indigo-600",
    verde: "bg-emerald-500",
    amarillo: "bg-amber-500",
    rojo: "bg-rose-500",
  } as const;

  return (
    <div
      className={cn("h-2 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800", className)}
      role="progressbar"
      aria-valuenow={Math.round(ancho)}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div
        className={cn("h-full rounded-full transition-[width] duration-500", colores[tono])}
        style={{ width: `${ancho}%` }}
      />
    </div>
  );
}
