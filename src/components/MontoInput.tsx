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
  /**
   * Deja escribir montos negativos. Solo lo usa el saldo inicial, que puede
   * serlo si el usuario empezó con la cuenta en rojo.
   */
  permiteNegativo?: boolean;
}

/**
 * Campo de monto en pesos. El usuario puede escribir con o sin separadores;
 * se muestra siempre normalizado ("1.234.567") y hacia afuera entrega un
 * entero.
 */
export function MontoInput({
  valor,
  onCambio,
  placeholder,
  id,
  disabled,
  autoFocus,
  permiteNegativo,
}: Props) {
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
          const bruto = e.target.value;
          const entero = parsearMonto(bruto);

          // Un "-" solo todavía no es un número, pero hay que dejarlo escrito:
          // si se borrara, el usuario nunca podría terminar de escribir el
          // negativo.
          const menosEnCurso = permiteNegativo && bruto.trim().startsWith("-");
          setTexto(entero ? formatearMiles(entero) : menosEnCurso ? "-" : "");

          onCambio(entero);
        }}
      />
    </div>
  );
}
