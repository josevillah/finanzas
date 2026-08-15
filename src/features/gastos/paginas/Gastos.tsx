import { useState } from "react";

import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Entrada, Seleccion } from "@/components/ui/Campo";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Modal } from "@/components/ui/Modal";
import { useCategorias, useGenerarAlEntrarAlMes } from "@/features/catalogos/hooks";
import { SelectorMes, useMes } from "@/features/mes/MesContexto";
import {
  useActualizarMovimiento,
  useCambiarMonto,
  useEliminarMovimiento,
  useMovimientos,
  useRegistrarMovimiento,
  useResumenPeriodo,
} from "@/features/mes/hooks";
import { formatearFecha, hoyISO } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import {
  ETIQUETAS_MEDIO_PAGO,
  type FiltroMovimientos,
  type MovimientoDetalle,
  type NuevoMovimiento,
  type TipoMovimiento,
} from "@/types/dominio";

import { useCaptura } from "../CapturaContexto";
import { DialogoCambiarPrecio } from "../componentes/DialogoCambiarPrecio";
import { FormularioMovimiento } from "../componentes/FormularioMovimiento";

export function Gastos() {
  const { anio, mes, esMesActual } = useMes();
  const { abrir: abrirCaptura } = useCaptura();

  const [filtro, setFiltro] = useState<FiltroMovimientos>({});
  const [formAbierto, setFormAbierto] = useState(false);
  const [editando, setEditando] = useState<MovimientoDetalle | null>(null);
  const [porBorrar, setPorBorrar] = useState<MovimientoDetalle | null>(null);
  const [cambiandoPrecio, setCambiandoPrecio] = useState<MovimientoDetalle | null>(null);

  // Los servicios recurrentes cargan su gasto del mes al entrar.
  useGenerarAlEntrarAlMes(anio, mes);

  const movimientos = useMovimientos(anio, mes, filtro);
  const resumen = useResumenPeriodo(anio, mes);
  const categorias = useCategorias(true);

  const registrar = useRegistrarMovimiento();
  const actualizar = useActualizarMovimiento();
  const cambiarPrecio = useCambiarMonto();
  const eliminar = useEliminarMovimiento();

  const cerrado = resumen.data?.estado === "cerrado";

  const abrirNuevo = () => {
    registrar.reset();
    setEditando(null);
    setFormAbierto(true);
  };

  const abrirEdicion = (m: MovimientoDetalle) => {
    actualizar.reset();
    setEditando(m);
    setFormAbierto(true);
  };

  const guardar = (datos: NuevoMovimiento) => {
    const alCerrar = { onSuccess: () => setFormAbierto(false) };
    if (editando) {
      actualizar.mutate({ id: editando.id, datos }, alCerrar);
    } else {
      registrar.mutate(datos, alCerrar);
    }
  };

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-semibold">
            Gastos e ingresos
            {cerrado ? <Insignia tono="neutro">Mes cerrado</Insignia> : null}
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Todo lo que se movió en el mes.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <SelectorMes />
          <Boton variante="secundario" onClick={abrirCaptura} title="Ctrl+Shift+G">
            ⚡ Gasto rápido
          </Boton>
          <Boton onClick={abrirNuevo} disabled={cerrado}>
            + Registrar
          </Boton>
        </div>
      </header>

      {resumen.data ? (
        <div className="grid gap-3 sm:grid-cols-3">
          <Tarjeta titulo="Gastos del mes" monto={resumen.data.total_gastos} />
          <Tarjeta titulo="Ingresos del mes" monto={resumen.data.total_ingresos} />
          <Tarjeta
            titulo="Balance"
            monto={resumen.data.balance}
            tono={resumen.data.balance >= 0 ? "verde" : "rojo"}
          />
        </div>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <Seleccion
          className="w-auto"
          value={filtro.tipo ?? ""}
          onChange={(e) =>
            setFiltro({ ...filtro, tipo: (e.target.value || null) as TipoMovimiento | null })
          }
        >
          <option value="">Todo</option>
          <option value="gasto">Solo gastos</option>
          <option value="ingreso">Solo ingresos</option>
        </Seleccion>

        <Seleccion
          className="w-auto"
          value={filtro.categoria_id ?? ""}
          onChange={(e) =>
            setFiltro({ ...filtro, categoria_id: e.target.value ? Number(e.target.value) : null })
          }
        >
          <option value="">Todas las categorías</option>
          {(categorias.data ?? []).map((c) => (
            <option key={c.id} value={c.id}>
              {c.nombre}
            </option>
          ))}
        </Seleccion>

        <Entrada
          className="w-auto min-w-52 flex-1"
          placeholder="Buscar en la descripción…"
          value={filtro.busqueda ?? ""}
          onChange={(e) => setFiltro({ ...filtro, busqueda: e.target.value || null })}
        />
      </div>

      {movimientos.isPending ? (
        <Cargando />
      ) : movimientos.error ? (
        <ErrorCarga error={movimientos.error} onReintentar={movimientos.refetch} />
      ) : !movimientos.data?.length ? (
        <Vacio
          titulo="Sin movimientos"
          descripcion="No hay nada registrado con estos filtros en el mes seleccionado."
          accion={
            !cerrado ? <Boton onClick={abrirNuevo}>Registrar el primero</Boton> : undefined
          }
        />
      ) : (
        <div className="tarjeta p-0">
          <ul className="divide-y divide-slate-100 dark:divide-slate-800">
            {movimientos.data.map((m) => (
              <Fila
                key={m.id}
                movimiento={m}
                bloqueado={cerrado}
                onEditar={() => abrirEdicion(m)}
                onCambiarPrecio={() => {
                  cambiarPrecio.reset();
                  setCambiandoPrecio(m);
                }}
                onBorrar={() => {
                  eliminar.reset();
                  setPorBorrar(m);
                }}
              />
            ))}
          </ul>
        </div>
      )}

      <FormularioMovimiento
        abierto={formAbierto}
        movimiento={editando}
        // En el mes en curso lo natural es hoy; en otro mes, su primer día.
        fechaPorDefecto={esMesActual ? hoyISO() : `${anio}-${String(mes).padStart(2, "0")}-01`}
        onCerrar={() => setFormAbierto(false)}
        onGuardar={guardar}
        guardando={registrar.isPending || actualizar.isPending}
        error={editando ? actualizar.error : registrar.error}
      />

      <DialogoCambiarPrecio
        movimiento={cambiandoPrecio}
        onCerrar={() => setCambiandoPrecio(null)}
        onConfirmar={(datos) =>
          cambiarPrecio.mutate(datos, { onSuccess: () => setCambiandoPrecio(null) })
        }
        guardando={cambiarPrecio.isPending}
        error={cambiarPrecio.error}
      />

      <Modal
        abierto={!!porBorrar}
        ancho="md"
        titulo="Eliminar movimiento"
        onCerrar={() => setPorBorrar(null)}
        acciones={
          <>
            <Boton variante="secundario" onClick={() => setPorBorrar(null)}>
              Cancelar
            </Boton>
            <Boton
              variante="peligro"
              disabled={eliminar.isPending}
              onClick={() =>
                porBorrar &&
                eliminar.mutate(porBorrar.id, { onSuccess: () => setPorBorrar(null) })
              }
            >
              {eliminar.isPending ? "Eliminando…" : "Eliminar"}
            </Boton>
          </>
        }
      >
        <p className="text-sm">
          Se eliminará el movimiento de{" "}
          <strong>
            <Moneda monto={porBorrar?.monto ?? 0} />
          </strong>{" "}
          del {formatearFecha(porBorrar?.fecha ?? null)}.
        </p>
        {eliminar.error ? (
          <p className="mt-3 rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
            {mensajeDeError(eliminar.error)}
          </p>
        ) : null}
      </Modal>
    </div>
  );
}

function Tarjeta({
  titulo,
  monto,
  tono,
}: {
  titulo: string;
  monto: number;
  tono?: "verde" | "rojo";
}) {
  return (
    <div className="tarjeta">
      <p className="etiqueta">{titulo}</p>
      <p
        className={
          tono === "verde"
            ? "mt-1 text-xl font-semibold text-emerald-600 dark:text-emerald-400"
            : tono === "rojo"
              ? "mt-1 text-xl font-semibold text-rose-600 dark:text-rose-400"
              : "mt-1 text-xl font-semibold"
        }
      >
        <Moneda monto={monto} />
      </p>
    </div>
  );
}

function Fila({
  movimiento,
  bloqueado,
  onEditar,
  onCambiarPrecio,
  onBorrar,
}: {
  movimiento: MovimientoDetalle;
  bloqueado: boolean;
  onEditar: () => void;
  onCambiarPrecio: () => void;
  onBorrar: () => void;
}) {
  const esCuota = movimiento.cuota_id !== null;
  const esIngreso = movimiento.tipo === "ingreso";

  return (
    <li className="flex items-center justify-between gap-3 px-4 py-3">
      <div className="flex min-w-0 items-center gap-3">
        <span
          aria-hidden
          className="h-2.5 w-2.5 shrink-0 rounded-full"
          style={{ backgroundColor: movimiento.categoria_color ?? "#94a3b8" }}
        />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">
            {movimiento.descripcion || movimiento.categoria_nombre || "Sin descripción"}
          </p>
          <p className="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-slate-500 dark:text-slate-400">
            <span>{formatearFecha(movimiento.fecha)}</span>
            {movimiento.categoria_nombre ? <span>· {movimiento.categoria_nombre}</span> : null}
            {movimiento.medio_pago ? (
              <span>· {ETIQUETAS_MEDIO_PAGO[movimiento.medio_pago]}</span>
            ) : null}
            {movimiento.servicio_nombre ? <span>· {movimiento.servicio_nombre}</span> : null}
            {movimiento.deuda_descripcion ? <span>· {movimiento.deuda_descripcion}</span> : null}
          </p>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-3">
        {esCuota ? <Insignia tono="indigo">Cuota</Insignia> : null}
        {movimiento.es_estimado ? <Insignia tono="amarillo">Estimado</Insignia> : null}

        <Moneda
          monto={movimiento.monto}
          className={
            esIngreso
              ? "font-medium text-emerald-600 dark:text-emerald-400"
              : "font-medium"
          }
        />

        {esCuota ? (
          <span className="w-56 text-right text-xs text-slate-400">desde la deuda</span>
        ) : (
          <span className="flex w-56 justify-end gap-1">
            <Boton
              variante={movimiento.es_estimado ? "secundario" : "fantasma"}
              tamano="sm"
              onClick={onCambiarPrecio}
              disabled={bloqueado}
            >
              Cambiar precio
            </Boton>
            <Boton variante="fantasma" tamano="sm" onClick={onEditar} disabled={bloqueado}>
              Editar
            </Boton>
            <Boton variante="fantasma" tamano="sm" onClick={onBorrar} disabled={bloqueado}>
              ✕
            </Boton>
          </span>
        )}
      </div>
    </li>
  );
}
