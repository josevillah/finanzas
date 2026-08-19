import { useState } from "react";

import { Moneda } from "@/components/Moneda";
import { MontoInput } from "@/components/MontoInput";
import { Boton } from "@/components/ui/Boton";
import { Campo, Entrada } from "@/components/ui/Campo";
import { Vacio } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Modal } from "@/components/ui/Modal";
import { mensajeDeError } from "@/lib/ipc";
import type { Cuenta, CuentaConNotas } from "@/types/dominio";

import { useActualizarCuenta, useApartar, useCrearCuenta, useEliminarCuenta, useRetirar } from "../hooks";
import { NotasCuenta } from "./NotasCuenta";

interface Props {
  ahorros: CuentaConNotas[];
  disponible: number;
  total: number;
}

type Movimiento = { cuenta: Cuenta; direccion: "apartar" | "retirar" };

/**
 * Cuentas de ahorro. La plata entra y sale solo apartando o retirando, así el
 * patrimonio nunca cambia por crear, archivar o borrar una cuenta.
 */
export function Ahorros({ ahorros, disponible, total }: Props) {
  const [creando, setCreando] = useState(false);
  const [moviendo, setMoviendo] = useState<Movimiento | null>(null);
  const [renombrando, setRenombrando] = useState<Cuenta | null>(null);
  // Una sola cuenta desplegada a la vez: son pocas y así la lista no se dispara.
  const [abierta, setAbierta] = useState<number | null>(null);

  const eliminar = useEliminarCuenta();
  const actualizar = useActualizarCuenta();
  const error = eliminar.error ?? actualizar.error;

  return (
    <section>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold">Ahorros</h2>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Plata que apartas para no gastarla. Sale del disponible, pero sigue siendo tuya.
          </p>
        </div>

        <div className="flex items-center gap-3">
          {ahorros.length > 0 ? (
            <span className="text-sm">
              Total <Moneda monto={total} className="font-semibold" />
            </span>
          ) : null}
          <Boton tamano="sm" variante="secundario" onClick={() => setCreando(true)}>
            Nueva cuenta
          </Boton>
        </div>
      </div>

      {error ? (
        <p className="mb-3 rounded-lg border border-rose-200 bg-rose-50 p-3 text-sm text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
          {mensajeDeError(error)}
        </p>
      ) : null}

      {ahorros.length === 0 ? (
        <Vacio
          titulo="Sin cuentas de ahorro"
          descripcion="Crea una y aparta plata desde tu disponible: el patrimonio no cambia, solo queda separada."
        />
      ) : (
        <ul className="space-y-2">
          {ahorros.map((c) => (
            <li
              key={c.id}
              className="rounded-xl border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900"
            >
              <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
                <button
                  type="button"
                  aria-expanded={abierta === c.id}
                  onClick={() => setAbierta((id) => (id === c.id ? null : c.id))}
                  className="-m-1 flex min-w-0 items-center gap-2 rounded-lg p-1 text-left hover:bg-slate-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:hover:bg-slate-800/50"
                >
                  <span aria-hidden className="text-xs text-slate-400">
                    {abierta === c.id ? "▾" : "▸"}
                  </span>
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">
                      {c.nombre}
                      {!c.activa ? (
                        <Insignia tono="neutro" className="ml-2">
                          Archivada
                        </Insignia>
                      ) : null}
                      {c.notas.length > 0 ? (
                        <Insignia tono="indigo" className="ml-2">
                          {c.notas.length} {c.notas.length === 1 ? "nota" : "notas"}
                        </Insignia>
                      ) : null}
                    </span>
                    <Moneda monto={c.saldo} className="text-lg font-semibold" />
                  </span>
                </button>

                <div className="flex flex-wrap items-center gap-1.5">
                  <Boton
                    tamano="sm"
                    variante="secundario"
                    onClick={() => setMoviendo({ cuenta: c, direccion: "apartar" })}
                    disabled={disponible <= 0}
                    title={disponible <= 0 ? "No tienes disponible para apartar" : undefined}
                  >
                    Apartar
                  </Boton>
                  <Boton
                    tamano="sm"
                    variante="secundario"
                    onClick={() => setMoviendo({ cuenta: c, direccion: "retirar" })}
                    disabled={c.saldo === 0}
                  >
                    Retirar
                  </Boton>
                  <Boton tamano="sm" variante="fantasma" onClick={() => setRenombrando(c)}>
                    Renombrar
                  </Boton>
                  <Boton
                    tamano="sm"
                    variante="fantasma"
                    onClick={() =>
                      actualizar.mutate({ id: c.id, nombre: c.nombre, activa: !c.activa })
                    }
                    disabled={c.activa && c.saldo > 0}
                    title={
                      c.activa && c.saldo > 0 ? "Retira la plata antes de archivarla" : undefined
                    }
                  >
                    {c.activa ? "Archivar" : "Reactivar"}
                  </Boton>
                  <Boton
                    tamano="sm"
                    variante="peligro"
                    onClick={() => eliminar.mutate(c.id)}
                    disabled={c.saldo > 0}
                    title={
                      c.saldo > 0
                        ? "Retira la plata antes de eliminarla"
                        : c.notas.length > 0
                          ? "También se borrarán sus notas"
                          : undefined
                    }
                  >
                    Eliminar
                  </Boton>
                </div>
              </div>

              {abierta === c.id ? <NotasCuenta cuenta={c} /> : null}
            </li>
          ))}
        </ul>
      )}

      <DialogoNueva abierto={creando} onCerrar={() => setCreando(false)} />
      <DialogoMover
        movimiento={moviendo}
        disponible={disponible}
        onCerrar={() => setMoviendo(null)}
      />
      <DialogoRenombrar cuenta={renombrando} onCerrar={() => setRenombrando(null)} />
    </section>
  );
}

function DialogoNueva({ abierto, onCerrar }: { abierto: boolean; onCerrar: () => void }) {
  const [nombre, setNombre] = useState("");
  const crear = useCrearCuenta();

  const guardar = async () => {
    try {
      await crear.mutateAsync({ nombre });
      setNombre("");
      onCerrar();
    } catch {
      // El error se muestra en el propio diálogo.
    }
  };

  return (
    <Modal
      abierto={abierto}
      titulo="Nueva cuenta de ahorro"
      ancho="md"
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar}>
            Cancelar
          </Boton>
          <Boton onClick={guardar} disabled={!nombre.trim() || crear.isPending}>
            Crear
          </Boton>
        </>
      }
    >
      <Campo etiqueta="Nombre" ayuda="La cuenta nace vacía. Después apartas plata en ella.">
        <Entrada
          value={nombre}
          autoFocus
          placeholder="Vacaciones, emergencias…"
          onChange={(e) => setNombre(e.target.value)}
        />
      </Campo>

      {crear.isError ? (
        <p className="mt-3 text-sm text-rose-600 dark:text-rose-400">{mensajeDeError(crear.error)}</p>
      ) : null}
    </Modal>
  );
}

function DialogoMover({
  movimiento,
  disponible,
  onCerrar,
}: {
  movimiento: Movimiento | null;
  disponible: number;
  onCerrar: () => void;
}) {
  const [monto, setMonto] = useState(0);
  const apartar = useApartar();
  const retirar = useRetirar();

  const esApartar = movimiento?.direccion === "apartar";
  const mutacion = esApartar ? apartar : retirar;
  const tope = esApartar ? disponible : (movimiento?.cuenta.saldo ?? 0);

  const guardar = async () => {
    if (!movimiento) return;
    try {
      await mutacion.mutateAsync({ id: movimiento.cuenta.id, monto });
      setMonto(0);
      onCerrar();
    } catch {
      // El error se muestra en el propio diálogo.
    }
  };

  return (
    <Modal
      abierto={movimiento !== null}
      ancho="md"
      titulo={
        movimiento
          ? esApartar
            ? `Apartar en «${movimiento.cuenta.nombre}»`
            : `Retirar de «${movimiento.cuenta.nombre}»`
          : ""
      }
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar}>
            Cancelar
          </Boton>
          <Boton onClick={guardar} disabled={monto <= 0 || monto > tope || mutacion.isPending}>
            {esApartar ? "Apartar" : "Retirar"}
          </Boton>
        </>
      }
    >
      <Campo
        etiqueta="Monto"
        ayuda={
          esApartar
            ? "Sale de tu disponible. Tu patrimonio no cambia."
            : "Vuelve a tu disponible. Tu patrimonio no cambia."
        }
      >
        <MontoInput valor={monto} onCambio={setMonto} autoFocus />
      </Campo>

      <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
        Máximo <Moneda monto={tope} />
      </p>

      {mutacion.isError ? (
        <p className="mt-3 text-sm text-rose-600 dark:text-rose-400">
          {mensajeDeError(mutacion.error)}
        </p>
      ) : null}
    </Modal>
  );
}

function DialogoRenombrar({ cuenta, onCerrar }: { cuenta: Cuenta | null; onCerrar: () => void }) {
  const [nombre, setNombre] = useState("");
  const actualizar = useActualizarCuenta();

  // El nombre visible arranca en el de la cuenta que se acaba de abrir.
  const [ultimaId, setUltimaId] = useState<number | null>(null);
  if (cuenta && cuenta.id !== ultimaId) {
    setUltimaId(cuenta.id);
    setNombre(cuenta.nombre);
  }

  const guardar = async () => {
    if (!cuenta) return;
    try {
      await actualizar.mutateAsync({ id: cuenta.id, nombre, activa: cuenta.activa });
      onCerrar();
    } catch {
      // El error se muestra en el propio diálogo.
    }
  };

  return (
    <Modal
      abierto={cuenta !== null}
      titulo="Renombrar cuenta"
      ancho="md"
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar}>
            Cancelar
          </Boton>
          <Boton onClick={guardar} disabled={!nombre.trim() || actualizar.isPending}>
            Guardar
          </Boton>
        </>
      }
    >
      <Campo etiqueta="Nombre">
        <Entrada value={nombre} autoFocus onChange={(e) => setNombre(e.target.value)} />
      </Campo>

      {actualizar.isError ? (
        <p className="mt-3 text-sm text-rose-600 dark:text-rose-400">
          {mensajeDeError(actualizar.error)}
        </p>
      ) : null}
    </Modal>
  );
}
