import { cn } from "@/lib/cn";

/**
 * Interruptor de encendido/apagado.
 *
 * El knob se coloca con flex y no en absolute: dentro de un `<button>` la
 * posición estática cae en el centro, así que un `absolute` sin `left` parte
 * desplazado y el desplazamiento se sale del track.
 */
export function Interruptor({
  activo,
  onCambio,
  etiqueta,
  ocupado,
}: {
  activo: boolean;
  onCambio: (activo: boolean) => void;
  /** Se usa como `aria-label`: el interruptor no tiene texto propio. */
  etiqueta: string;
  ocupado?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={activo}
      aria-label={etiqueta}
      disabled={ocupado}
      onClick={() => onCambio(!activo)}
      className={cn(
        "inline-flex h-6 w-11 shrink-0 items-center rounded-full p-0.5 transition-colors",
        "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600",
        "disabled:cursor-not-allowed disabled:opacity-60",
        activo ? "bg-indigo-600" : "bg-slate-300 dark:bg-slate-600",
      )}
    >
      <span
        className={cn(
          "h-5 w-5 rounded-full bg-white shadow transition-transform",
          activo ? "translate-x-5" : "translate-x-0",
        )}
      />
    </button>
  );
}
