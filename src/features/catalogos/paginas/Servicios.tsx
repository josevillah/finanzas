import { useState } from "react";

import { Moneda } from "@/components/Moneda";
import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga, Vacio } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Modal } from "@/components/ui/Modal";
import { SelectorMes, useMes } from "@/features/mes/MesContexto";
import { cn } from "@/lib/cn";
import { formatearFecha } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import { formatearCLP } from "@/lib/moneda";
import {
  ETIQUETAS_TIPO_SERVICIO,
  type NuevoServicio,
  type Servicio,
  type ServicioConReal,
} from "@/types/dominio";

import { DialogoActivarServicio } from "../componentes/DialogoActivarServicio";
import { FormularioServicio } from "../componentes/FormularioServicio";
import {
  useActivarServicioEnMes,
  useActualizarServicio,
  useCrearServicio,
  useEliminarServicio,
  useGenerarAlEntrarAlMes,
  useGenerarGastosServicios,
  useResumenServicios,
  useServicios,
} from "../hooks";

export function Servicios() {
  const { anio, mes } = useMes();

  // Al entrar, los servicios activos cargan su gasto del mes si les falta.
  useGenerarAlEntrarAlMes(anio, mes);

  const resumen = useResumenServicios(anio, mes);
  const generar = useGenerarGastosServicios();
  // El resumen solo trae los activos; para el CRUD necesitamos también los apagados.
  const todos = useServicios(false);

  const [formAbierto, setFormAbierto] = useState(false);
  const [editando, setEditando] = useState<Servicio | null>(null);
  const [porBorrar, setPorBorrar] = useState<Servicio | null>(null);
  const [porActivar, setPorActivar] = useState<ServicioConReal | null>(null);

  const crear = useCrearServicio();
  const actualizar = useActualizarServicio();
  const eliminar = useEliminarServicio();
  const activar = useActivarServicioEnMes();

  const guardar = (datos: NuevoServicio) => {
    const alCerrar = { onSuccess: () => setFormAbierto(false) };
    if (editando) {
      actualizar.mutate({ id: editando.id, datos }, alCerrar);
    } else {
      crear.mutate(datos, alCerrar);
    }
  };

  const inactivos = (todos.data ?? []).filter((s) => !s.activo);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">Servicios recurrentes</h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Lo que estimaste versus lo que realmente llegó.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <SelectorMes />
          {resumen.data?.sin_registrar ? (
            <Boton
              variante="secundario"
              disabled={generar.isPending}
              onClick={() => generar.mutate({ anio, mes })}
            >
              {generar.isPending ? "Generando…" : "Cargar gastos del mes"}
            </Boton>
          ) : null}
          <Boton
            onClick={() => {
              crear.reset();
              setEditando(null);
              setFormAbierto(true);
            }}
          >
            + Nuevo servicio
          </Boton>
        </div>
      </header>

      {resumen.isPending ? (
        <Cargando />
      ) : resumen.error ? (
        <ErrorCarga error={resumen.error} onReintentar={resumen.refetch} />
      ) : !resumen.data?.servicios.length ? (
        <Vacio
          titulo="No tienes servicios activos"
          descripcion="Registra la luz, el agua, el internet o tus suscripciones para proyectar el gasto del mes y compararlo con lo real."
          accion={<Boton onClick={() => setFormAbierto(true)}>Agregar el primero</Boton>}
        />
      ) : (
        <>
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="tarjeta">
              <p className="etiqueta">Estimado del mes</p>
              <p className="mt-1 text-xl font-semibold">
                <Moneda monto={resumen.data.total_estimado} />
              </p>
            </div>
            <div className="tarjeta">
              <p className="etiqueta">Cargado al mes</p>
              <p className="mt-1 text-xl font-semibold">
                <Moneda monto={resumen.data.total_real} />
              </p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {resumen.data.sin_registrar > 0
                  ? `${resumen.data.sin_registrar} sin cargar todavía`
                  : resumen.data.por_confirmar > 0
                    ? `${resumen.data.por_confirmar} con el monto estimado, por confirmar`
                    : "Todos con su monto real"}
              </p>
            </div>
            <div className="tarjeta">
              <p className="etiqueta">Diferencia</p>
              <p
                className={cn(
                  "mt-1 text-xl font-semibold",
                  resumen.data.diferencia > 0 && "text-rose-600 dark:text-rose-400",
                  resumen.data.diferencia < 0 && "text-emerald-600 dark:text-emerald-400",
                )}
              >
                {resumen.data.diferencia > 0 ? "+" : ""}
                {formatearCLP(resumen.data.diferencia)}
              </p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {resumen.data.diferencia > 0
                  ? "por sobre lo estimado"
                  : resumen.data.diferencia < 0
                    ? "por debajo de lo estimado"
                    : "clavado"}
              </p>
            </div>
          </div>

          {resumen.data.servicios.some((s) => !s.incluido_en_el_mes) ? (
            <p className="rounded-lg bg-slate-100 px-3 py-2 text-xs text-slate-600 dark:bg-slate-800/60 dark:text-slate-400">
              Algunos servicios se dieron de alta después de este mes, así que no cuentan acá ni
              generan gasto. Aparecen abajo en gris; si ya los pagabas por entonces, puedes
              registrarlos con "Activar en este mes".
            </p>
          ) : null}

          <div className="tarjeta p-0">
            <ul className="divide-y divide-slate-100 dark:divide-slate-800">
              {resumen.data.servicios.map((s) => (
                <FilaServicio
                  key={s.id}
                  servicio={s}
                  periodoCerrado={resumen.data.periodo_cerrado}
                  onActivar={() => {
                    activar.reset();
                    setPorActivar(s);
                  }}
                  onEditar={() => {
                    actualizar.reset();
                    setEditando(s);
                    setFormAbierto(true);
                  }}
                  onBorrar={() => {
                    eliminar.reset();
                    setPorBorrar(s);
                  }}
                />
              ))}
            </ul>
          </div>
        </>
      )}

      {inactivos.length ? (
        <div className="tarjeta">
          <h2 className="mb-3 text-sm font-medium text-slate-500 dark:text-slate-400">
            Inactivos ({inactivos.length})
          </h2>
          <ul className="space-y-2">
            {inactivos.map((s) => (
              <li key={s.id} className="flex items-center justify-between gap-3 text-sm">
                <span className="text-slate-500 dark:text-slate-400">{s.nombre}</span>
                <Boton
                  variante="fantasma"
                  tamano="sm"
                  onClick={() => {
                    actualizar.reset();
                    setEditando(s);
                    setFormAbierto(true);
                  }}
                >
                  Reactivar
                </Boton>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      <DialogoActivarServicio
        servicio={porActivar}
        anio={anio}
        mes={mes}
        onCerrar={() => setPorActivar(null)}
        onConfirmar={(datos) =>
          activar.mutate(datos, { onSuccess: () => setPorActivar(null) })
        }
        guardando={activar.isPending}
        error={activar.error}
      />

      <FormularioServicio
        abierto={formAbierto}
        servicio={editando}
        onCerrar={() => setFormAbierto(false)}
        onGuardar={guardar}
        guardando={crear.isPending || actualizar.isPending}
        error={editando ? actualizar.error : crear.error}
      />

      <Modal
        abierto={!!porBorrar}
        ancho="md"
        titulo="Eliminar servicio"
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
                porBorrar && eliminar.mutate(porBorrar.id, { onSuccess: () => setPorBorrar(null) })
              }
            >
              {eliminar.isPending ? "Eliminando…" : "Eliminar"}
            </Boton>
          </>
        }
      >
        <p className="text-sm">
          Se eliminará <strong>{porBorrar?.nombre}</strong>. Si ya tiene gastos registrados no se
          podrá borrar: en ese caso desactívalo.
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

function FilaServicio({
  servicio,
  periodoCerrado,
  onEditar,
  onActivar,
  onBorrar,
}: {
  servicio: ServicioConReal;
  periodoCerrado: boolean;
  onEditar: () => void;
  onActivar: () => void;
  onBorrar: () => void;
}) {
  const sinRegistrar = servicio.n_movimientos === 0;
  const porConfirmar = servicio.n_estimados > 0;
  // Activarlo a mano no mueve su alta, pero sí lo hace contar para el mes.
  const fueraDelMes = !servicio.incluido_en_el_mes;

  return (
    <li
      className={cn(
        "flex flex-wrap items-center justify-between gap-3 px-4 py-3",
        fueraDelMes && "opacity-50",
      )}
    >
      <div className="min-w-0">
        <p className="flex items-center gap-2 text-sm font-medium">
          <span className="truncate">{servicio.nombre}</span>
          {fueraDelMes ? (
            <Insignia tono="neutro">Aún no existía</Insignia>
          ) : porConfirmar ? (
            <Insignia tono="amarillo">Por confirmar</Insignia>
          ) : sinRegistrar ? (
            <Insignia tono="amarillo">Sin cargar</Insignia>
          ) : null}
        </p>
        <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
          {ETIQUETAS_TIPO_SERVICIO[servicio.tipo]}
          {servicio.categoria_nombre ? ` · ${servicio.categoria_nombre}` : ""}
          {servicio.fecha_vencimiento
            ? ` · vence ${formatearFecha(servicio.fecha_vencimiento)}`
            : ""}
        </p>
      </div>

      <div className="flex items-center gap-5 text-sm">
        <div className="text-right">
          <p className="etiqueta">Estimado</p>
          <Moneda monto={servicio.monto_estimado} atenuado />
        </div>

        <div className="text-right">
          <p className="etiqueta">Real</p>
          {sinRegistrar ? (
            <span className="text-slate-400">—</span>
          ) : (
            <Moneda
              monto={servicio.monto_real}
              className={porConfirmar ? "font-medium text-amber-600 dark:text-amber-400" : "font-medium"}
            />
          )}
        </div>

        <div className="w-24 text-right">
          <p className="etiqueta">Dif.</p>
          {sinRegistrar || porConfirmar ? (
            <span className="text-slate-400">—</span>
          ) : (
            <span
              className={cn(
                "tabular font-medium",
                servicio.diferencia > 0 && "text-rose-600 dark:text-rose-400",
                servicio.diferencia < 0 && "text-emerald-600 dark:text-emerald-400",
              )}
            >
              {servicio.diferencia > 0 ? "+" : ""}
              {formatearCLP(servicio.diferencia)}
            </span>
          )}
        </div>

        <span className="flex gap-1">
          {fueraDelMes ? (
            <Boton
              variante="secundario"
              tamano="sm"
              onClick={onActivar}
              disabled={periodoCerrado}
              title={
                periodoCerrado
                  ? "El mes está cerrado: reábrelo para poder registrar gastos"
                  : "Registrar a mano el gasto de este servicio en el mes"
              }
            >
              Activar en este mes
            </Boton>
          ) : null}
          <Boton variante="fantasma" tamano="sm" onClick={onEditar}>
            Editar
          </Boton>
          <Boton variante="fantasma" tamano="sm" onClick={onBorrar}>
            ✕
          </Boton>
        </span>
      </div>
    </li>
  );
}
