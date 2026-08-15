import { useEffect, useState } from "react";

import { Entrada } from "@/components/ui/Campo";
import { formatearMiles, parsearMonto } from "@/lib/moneda";

interface Props {
  valor: number;
  onCambio: (valor: number) => void;
  placeholder?: string;
  id?: string;
  disabled?: boolean;
  autoFocus?: boolean;
}

/**
 * Campo de monto en pesos. El usuario puede escribir con o sin separadores;
 * se muestra siempre normalizado ("1.234.567") y hacia afuera entrega un
 * entero.
 */
export function MontoInput({ valor, onCambio, placeholder, id, disabled, autoFocus }: Props) {
  const [texto, setTexto] = useState(() => (valor ? formatearMiles(valor) : ""));

  // Si el valor cambia desde afuera (reset del formulario, edición), se
  // resincroniza el texto visible.
  useEffect(() => {
    const actual = parsearMonto(texto);
    if (actual !== valor) setTexto(valor ? formatearMiles(valor) : "");
    // Solo depende de `valor`: reaccionar a `texto` reformatearía mientras se escribe.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [valor]);

  return (
    <div className="relative">
      <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-sm text-slate-400">
        $
      </span>
      <Entrada
        id={id}
        className="pl-7 text-right tabular"
        inputMode="numeric"
        autoComplete="off"
        placeholder={placeholder ?? "0"}
        disabled={disabled}
        autoFocus={autoFocus}
        value={texto}
        onChange={(e) => {
          const entero = parsearMonto(e.target.value);
          setTexto(entero ? formatearMiles(entero) : "");
          onCambio(entero);
        }}
      />
    </div>
  );
}
