import { cn } from "@/lib/cn";
import { formatearCLP } from "@/lib/moneda";

/** Muestra un monto entero como "$1.234.567", con cifras tabulares. */
export function Moneda({
  monto,
  className,
  atenuado,
}: {
  monto: number | null | undefined;
  className?: string;
  atenuado?: boolean;
}) {
  if (monto === null || monto === undefined) {
    return <span className={cn("text-slate-400", className)}>—</span>;
  }

  return (
    <span
      className={cn(
        "tabular",
        atenuado && "text-slate-500 dark:text-slate-400",
        className,
      )}
    >
      {formatearCLP(monto)}
    </span>
  );
}
