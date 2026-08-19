import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import { Boton } from "@/components/ui/Boton";
import { Entrada } from "@/components/ui/Campo";
import { Modal } from "@/components/ui/Modal";
import { mensajeDeError } from "@/lib/ipc";
import { CONFIRMACION_REINICIO, type ResultadoReinicio, type ResumenReinicio } from "@/types/dominio";

import { useReiniciarDatos, useResumenReinicio } from "../hooks-reinicio";

export function DialogoReinicio({
  abierto,
  onCerrar,
}: {
  abierto: boolean;
  onCerrar: () => void;
}) {
  const resumen = useResumenReinicio(abierto);
  const reiniciar = useReiniciarDatos();
  const navegar = useNavigate();

  const [texto, setTexto] = useState("");
  const [borrarServicios, setBorrarServicios] = useState(false);
  const [resultado, setResultado] = useState<ResultadoReinicio | null>(null);

  // Cada apertura parte de cero: ni el texto de confirmación ni el checkbox
  // deben quedar heredados de un intento anterior.
  useEffect(() => {
    if (!abierto) return;
    setTexto("");
    setBorrarServicios(false);
    setResultado(null);
    reiniciar.reset();
    // `reiniciar` cambia de identidad en cada render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [abierto]);

  // Sensible a mayúsculas y sin espacios de más.
  const confirmado = texto.trim() === CONFIRMACION_REINICIO;

  if (resultado) {
    return (
      <Modal
        abierto
        ancho="md"
        titulo="Datos reiniciados"
        onCerrar={() => {
          onCerrar();
          navegar("/mes");
        }}
        acciones={
          <Boton
            onClick={() => {
              onCerrar();
              navegar("/mes");
            }}
          >
            Ir al resumen
          </Boton>
        }
      >
        <div className="space-y-3 text-sm">
          <p>
            Se borraron <strong>{resultado.registros_borrados}</strong> registros. Las categorías
            y tus preferencias quedaron como estaban.
          </p>

          <div>
            <p className="font-medium">Tu respaldo quedó acá:</p>
            <p className="mt-1 break-all rounded-lg bg-slate-100 px-3 py-2 font-mono text-xs dark:bg-slate-800">
              {resultado.ruta_respaldo}
            </p>
            <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
              Es la única forma de recuperar lo borrado. No se elimina solo, y puedes restaurarlo
              desde la pantalla de Respaldo.
            </p>
          </div>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      abierto={abierto}
      ancho="md"
      titulo="Reiniciar todos mis datos"
      onCerrar={onCerrar}
      acciones={
        <>
          <Boton variante="secundario" onClick={onCerrar} disabled={reiniciar.isPending}>
            Cancelar
          </Boton>
          <Boton
            variante="peligro"
            disabled={!confirmado || reiniciar.isPending}
            onClick={() =>
              reiniciar.mutate(
                { confirmacion: texto.trim(), borrarServicios },
                { onSuccess: setResultado },
              )
            }
          >
            {reiniciar.isPending ? "Borrando…" : "Borrar mis datos"}
          </Boton>
        </>
      }
    >
      <div className="space-y-4 text-sm">
        <div className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
          <p className="font-medium">Esto no se puede deshacer desde la app.</p>
          <p className="mt-1 text-xs">
            Antes de borrar se guarda un respaldo automático, y esa copia es la única manera de
            volver atrás.
          </p>
        </div>

        {resumen.isPending ? (
          <p className="text-slate-500 dark:text-slate-400">Contando registros…</p>
        ) : resumen.data ? (
          <ListaDeBorrado resumen={resumen.data} borrarServicios={borrarServicios} />
        ) : null}

        <label className="flex items-start gap-2 rounded-lg bg-slate-50 px-3 py-2 dark:bg-slate-800/50">
          <input
            type="checkbox"
            className="mt-0.5 h-4 w-4 rounded border-slate-300 text-indigo-600"
            checked={borrarServicios}
            onChange={(e) => setBorrarServicios(e.target.checked)}
          />
          <span>
            Borrar también mis servicios recurrentes
            <span className="mt-0.5 block text-xs text-slate-500 dark:text-slate-400">
              Por omisión se conservan: no son datos financieros y reconfigurarlos es tedioso.
            </span>
          </span>
        </label>

        <div>
          <label className="block">
            <span className="mb-1.5 block font-medium">
              Escribe <code className="rounded bg-slate-100 px-1.5 py-0.5 dark:bg-slate-800">{CONFIRMACION_REINICIO}</code>{" "}
              para confirmar
            </span>
            <Entrada
              value={texto}
              onChange={(e) => setTexto(e.target.value)}
              placeholder={CONFIRMACION_REINICIO}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          {texto && !confirmado ? (
            <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
              Tiene que coincidir exactamente, en mayúsculas.
            </p>
          ) : null}
        </div>

        {reiniciar.error ? (
          <p className="rounded-lg bg-rose-50 px-3 py-2 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(reiniciar.error)}
          </p>
        ) : null}
      </div>
    </Modal>
  );
}

/** Los números reales frenan mejor que cualquier texto de advertencia. */
function ListaDeBorrado({
  resumen,
  borrarServicios,
}: {
  resumen: ResumenReinicio;
  borrarServicios: boolean;
}) {
  const seBorra: Array<[string, number]> = [
    ["Deudas", resumen.deudas],
    ["Cuotas", resumen.cuotas],
    ["Gastos e ingresos", resumen.movimientos],
    ["Presupuestos", resumen.presupuestos],
    ["Meses", resumen.periodos],
    ["Categorías creadas por ti", resumen.categorias_propias],
    ["Cuentas de ahorro", resumen.cuentas],
    ["Metas", resumen.metas],
  ];

  if (borrarServicios) seBorra.push(["Servicios recurrentes", resumen.servicios]);

  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <div>
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-rose-600 dark:text-rose-400">
          Se borra
        </p>
        <ul className="space-y-0.5">
          {seBorra.map(([nombre, cantidad]) => (
            <li key={nombre} className="flex justify-between gap-3">
              <span className={cantidad === 0 ? "text-slate-400" : ""}>{nombre}</span>
              <span className="tabular font-medium">{cantidad}</span>
            </li>
          ))}
        </ul>
      </div>

      <div>
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-emerald-600 dark:text-emerald-400">
          Se conserva
        </p>
        <ul className="space-y-0.5 text-slate-600 dark:text-slate-400">
          <li>Las 15 categorías de fábrica</li>
          {!borrarServicios ? <li>Tus {resumen.servicios} servicios recurrentes</li> : null}
          <li>Tus preferencias y el tema</li>
          <li>Los respaldos ya guardados</li>
        </ul>
      </div>

      <p className="text-xs text-slate-500 sm:col-span-2 dark:text-slate-400">
        Tu saldo inicial vuelve a $0: tendrás que ajustarlo de nuevo para cuadrar el disponible.
      </p>
    </div>
  );
}
