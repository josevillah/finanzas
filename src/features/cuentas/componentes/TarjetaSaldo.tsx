import { useState } from "react";

import { Moneda } from "@/components/Moneda";
import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { cn } from "@/lib/cn";
import { mensajeDeError } from "@/lib/ipc";
import type { DesgloseSaldo } from "@/types/dominio";

import { useFijarSaldoInicial } from "../hooks";

interface Props {
  disponible: number;
  patrimonio: number;
  desglose: DesgloseSaldo;
}

/**
 * El disponible y de dónde sale.
 *
 * El desglose no es decoración: un número calculado que el usuario no puede
 * desarmar es un número que no puede verificar. Sin ver los términos no tiene
 * forma de saber si está bien ni qué ajustar cuando no calza.
 */
export function TarjetaSaldo({ disponible, patrimonio, desglose }: Props) {
  const ingresos = desglose.ingresos_declarados + desglose.ingresos_registrados;

  return (
    <section className="rounded-xl border border-slate-200 bg-white p-5 dark:border-slate-800 dark:bg-slate-900">
      <p className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
        Disponible
      </p>
      <Moneda
        monto={disponible}
        className={cn(
          "text-4xl font-semibold",
          disponible < 0 && "text-rose-600 dark:text-rose-400",
        )}
      />
      <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
        Lo que te queda para gastar, sin contar lo que tienes apartado en ahorros.
      </p>

      <dl className="mt-5 space-y-1.5 border-t border-slate-200 pt-4 text-sm dark:border-slate-800">
        <Linea etiqueta="Saldo inicial" monto={desglose.saldo_inicial} />
        <Linea etiqueta="Ingresos" monto={ingresos} signo="+" />
        <Linea etiqueta="Gastos" monto={-desglose.gastos} signo="−" />
        {desglose.apartado > 0 ? (
          <Linea etiqueta="Apartado en ahorros" monto={-desglose.apartado} signo="−" />
        ) : null}

        <div className="flex items-baseline justify-between gap-4 border-t border-slate-200 pt-1.5 font-medium dark:border-slate-800">
          <dt>Disponible</dt>
          <dd>
            <Moneda monto={disponible} />
          </dd>
        </div>
      </dl>

      {desglose.gastos_estimados > 0 ? (
        <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
          De los gastos, <Moneda monto={desglose.gastos_estimados} className="font-medium" /> son
          servicios estimados que aún no confirmas. Ya están descontados, así que a principio de
          mes el disponible puede verse más bajo que tu banco.
        </p>
      ) : null}

      {desglose.apartado > 0 ? (
        <p className="mt-4 flex items-baseline justify-between gap-4 border-t border-slate-200 pt-3 text-sm dark:border-slate-800">
          <span className="text-slate-500 dark:text-slate-400">
            Patrimonio total{" "}
            <span className="text-xs">(disponible + ahorros, sin descontar deudas)</span>
          </span>
          <Moneda monto={patrimonio} className="font-semibold" />
        </p>
      ) : null}

      <EditorSaldoInicial actual={desglose.saldo_inicial} />
    </section>
  );
}

function Linea({
  etiqueta,
  monto,
  signo,
}: {
  etiqueta: string;
  monto: number;
  signo?: "+" | "−";
}) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="text-slate-500 dark:text-slate-400">{etiqueta}</dt>
      <dd className="tabular">
        {signo ? <span className="mr-0.5 text-slate-400">{signo}</span> : null}
        <Moneda monto={Math.abs(monto)} atenuado={monto === 0} />
      </dd>
    </div>
  );
}

/**
 * La única perilla para cuadrar con la realidad: se sube o se baja hasta que
 * el disponible calce con el banco, y después no se toca más.
 */
function EditorSaldoInicial({ actual }: { actual: number }) {
  const [editando, setEditando] = useState(false);
  const [valor, setValor] = useState(actual);
  const fijar = useFijarSaldoInicial();

  const abrir = () => {
    setValor(actual);
    fijar.reset();
    setEditando(true);
  };

  const guardar = async () => {
    try {
      await fijar.mutateAsync(valor);
      setEditando(false);
    } catch {
      // El mensaje se muestra abajo; el editor sigue abierto para corregir.
    }
  };

  return (
    <div className="mt-4 rounded-lg bg-slate-50 p-3 dark:bg-slate-800/50">
      <p className="text-xs text-slate-600 dark:text-slate-400">
        ¿El disponible no coincide con lo que tienes de verdad? Ajusta el{" "}
        <strong className="font-medium">saldo inicial</strong> —lo que tenías antes de empezar a
        usar la app— hasta que calce. Es un solo número y se toca una vez.
      </p>

      {editando ? (
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <div className="w-44">
            <MontoInput valor={valor} onCambio={setValor} autoFocus permiteNegativo />
          </div>
          <Boton tamano="sm" onClick={guardar} disabled={fijar.isPending}>
            Guardar
          </Boton>
          <Boton tamano="sm" variante="fantasma" onClick={() => setEditando(false)}>
            Cancelar
          </Boton>
        </div>
      ) : (
        <div className="mt-2 flex flex-wrap items-center gap-3">
          <Moneda monto={actual} className="font-semibold" />
          <Boton tamano="sm" variante="secundario" onClick={abrir}>
            Ajustar
          </Boton>
        </div>
      )}

      {fijar.isError ? (
        <p className="mt-2 text-xs text-rose-600 dark:text-rose-400">
          {mensajeDeError(fijar.error)}
        </p>
      ) : null}
    </div>
  );
}
