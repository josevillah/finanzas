import { useState } from "react";

import { Moneda } from "@/components/Moneda";
import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Entrada } from "@/components/ui/Campo";
import { mensajeDeError } from "@/lib/ipc";
import type { CuentaConNotas, NotaAhorro } from "@/types/dominio";

import { useActualizarNota, useCrearNota, useEliminarNota } from "../hooks";

/**
 * Las notas de propósito de una cuenta: en qué está pensada la plata que hay
 * adentro.
 *
 * Son una anotación y nada más. No mueven plata, no cambian el disponible y su
 * suma puede no calzar con el saldo: cuando eso pasa se avisa, pero no se
 * bloquea nada. El usuario ajusta cuando quiera.
 */
export function NotasCuenta({ cuenta }: { cuenta: CuentaConNotas }) {
  const [agregando, setAgregando] = useState(false);
  const eliminar = useEliminarNota();

  return (
    <div className="border-t border-slate-200 px-4 py-3 dark:border-slate-800">
      <Cuadratura cuenta={cuenta} />

      {cuenta.notas.length === 0 && !agregando ? (
        <p className="text-sm text-slate-500 dark:text-slate-400">
          Sin notas todavía. Puedes anotar para qué es la plata de esta cuenta —por ejemplo
          «libros» y «videojuegos»— sin que eso mueva ni un peso.
        </p>
      ) : (
        <ul className="space-y-1">
          {cuenta.notas.map((nota) => (
            <Fila key={nota.id} nota={nota} onEliminar={() => eliminar.mutate(nota.id)} />
          ))}
        </ul>
      )}

      {agregando ? (
        <FilaNueva cuentaId={cuenta.id} onListo={() => setAgregando(false)} />
      ) : (
        <div className="mt-2">
          <Boton tamano="sm" variante="secundario" onClick={() => setAgregando(true)}>
            Agregar nota
          </Boton>
        </div>
      )}

      {eliminar.isError ? (
        <p className="mt-2 text-xs text-rose-600 dark:text-rose-400">
          {mensajeDeError(eliminar.error)}
        </p>
      ) : null}

      <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
        Las notas son solo referenciales: no mueven plata ni afectan tu disponible.
      </p>
    </div>
  );
}

/**
 * El aviso de si las notas calzan con el saldo. Nunca bloquea: apartar y
 * retirar no tocan las notas, así que quedar descuadrado es normal y se
 * arregla cuando el usuario quiere.
 */
function Cuadratura({ cuenta }: { cuenta: CuentaConNotas }) {
  if (cuenta.notas.length === 0 || cuenta.sin_asignar === 0) return null;

  if (cuenta.sin_asignar < 0) {
    return (
      <p className="mb-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-300">
        Tus notas suman <Moneda monto={cuenta.total_notas} className="font-medium" /> y la cuenta
        tiene <Moneda monto={cuenta.saldo} className="font-medium" />. Ajústalas cuando quieras.
      </p>
    );
  }

  return (
    <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
      Hay <Moneda monto={cuenta.sin_asignar} className="font-medium" /> sin asignar.
    </p>
  );
}

function Fila({ nota, onEliminar }: { nota: NotaAhorro; onEliminar: () => void }) {
  const [editando, setEditando] = useState(false);

  if (editando) {
    return (
      <li>
        <Editor
          nombreInicial={nota.nombre}
          montoInicial={nota.monto}
          etiquetaGuardar="Guardar"
          notaId={nota.id}
          onListo={() => setEditando(false)}
        />
      </li>
    );
  }

  return (
    <li className="flex items-center justify-between gap-3 rounded-lg px-2 py-1.5 hover:bg-slate-50 dark:hover:bg-slate-800/50">
      <span className="min-w-0 truncate text-sm">{nota.nombre}</span>

      <span className="flex shrink-0 items-center gap-1.5">
        <Moneda monto={nota.monto} className="text-sm font-medium" />
        <Boton tamano="sm" variante="fantasma" onClick={() => setEditando(true)}>
          Editar
        </Boton>
        <Boton tamano="sm" variante="fantasma" onClick={onEliminar}>
          Borrar
        </Boton>
      </span>
    </li>
  );
}

function FilaNueva({ cuentaId, onListo }: { cuentaId: number; onListo: () => void }) {
  return (
    <div className="mt-2">
      <Editor
        nombreInicial=""
        montoInicial={0}
        etiquetaGuardar="Agregar"
        cuentaId={cuentaId}
        onListo={onListo}
      />
    </div>
  );
}

/**
 * Edición en línea con Guardar y Cancelar explícitos, como el editor del saldo
 * inicial. El guardado automático dejaría sin lugar donde mostrar el rechazo
 * cuando las notas se pasan del saldo.
 *
 * Con `notaId` edita; con `cuentaId` crea. Nunca las dos.
 */
function Editor({
  nombreInicial,
  montoInicial,
  etiquetaGuardar,
  notaId,
  cuentaId,
  onListo,
}: {
  nombreInicial: string;
  montoInicial: number;
  etiquetaGuardar: string;
  notaId?: number;
  cuentaId?: number;
  onListo: () => void;
}) {
  const [nombre, setNombre] = useState(nombreInicial);
  const [monto, setMonto] = useState(montoInicial);

  const crear = useCrearNota();
  const actualizar = useActualizarNota();
  const mutacion = notaId === undefined ? crear : actualizar;

  const guardar = async () => {
    try {
      if (notaId !== undefined) {
        await actualizar.mutateAsync({ id: notaId, nombre, monto });
      } else if (cuentaId !== undefined) {
        await crear.mutateAsync({ cuenta_id: cuentaId, nombre, monto });
      }
      onListo();
    } catch {
      // El error queda a la vista acá abajo; el editor sigue abierto para
      // corregir el monto sin volver a escribir todo.
    }
  };

  return (
    <div className="rounded-lg bg-slate-50 p-2 dark:bg-slate-800/50">
      <div className="flex flex-wrap items-center gap-2">
        <Entrada
          value={nombre}
          autoFocus
          placeholder="Libros, videojuegos…"
          className="w-44"
          onChange={(e) => setNombre(e.target.value)}
        />
        <div className="w-32">
          <MontoInput valor={monto} onCambio={setMonto} />
        </div>
        <Boton tamano="sm" onClick={guardar} disabled={!nombre.trim() || mutacion.isPending}>
          {etiquetaGuardar}
        </Boton>
        <Boton tamano="sm" variante="fantasma" onClick={onListo}>
          Cancelar
        </Boton>
      </div>

      {mutacion.isError ? (
        <p className="mt-2 text-xs text-rose-600 dark:text-rose-400">
          {mensajeDeError(mutacion.error)}
        </p>
      ) : null}
    </div>
  );
}
