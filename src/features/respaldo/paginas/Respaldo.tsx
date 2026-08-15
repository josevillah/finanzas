import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";

import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { Interruptor } from "@/components/ui/Interruptor";
import { Modal } from "@/components/ui/Modal";
import { formatearFecha, hoyISO } from "@/lib/fechas";
import { mensajeDeError } from "@/lib/ipc";
import type { ResultadoExportacion, ResultadoRestauracion } from "@/types/dominio";

import {
  useEstadoRespaldo,
  useExportarCsv,
  useExportarJson,
  useFijarRespaldoAutomatico,
  useRespaldar,
  useRestaurar,
} from "../hooks";

function formatearTamano(bytes: number): string {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1).replace(".", ",")} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${bytes} bytes`;
}

export function Respaldo() {
  const estado = useEstadoRespaldo();

  const respaldar = useRespaldar();
  const fijarAutomatico = useFijarRespaldoAutomatico();
  const exportarJson = useExportarJson();
  const exportarCsv = useExportarCsv();
  const restaurar = useRestaurar();

  const [exportacion, setExportacion] = useState<ResultadoExportacion | null>(null);
  const [restauracion, setRestauracion] = useState<ResultadoRestauracion | null>(null);
  const [rutaRespaldo, setRutaRespaldo] = useState<string | null>(null);
  const [confirmarRestauracion, setConfirmarRestauracion] = useState<string | null>(null);

  if (estado.isPending) return <Cargando />;
  if (estado.error) return <ErrorCarga error={estado.error} onReintentar={estado.refetch} />;
  if (!estado.data) return null;

  const d = estado.data;

  const elegirYRespaldar = async () => {
    const destino = await save({
      title: "Guardar respaldo",
      defaultPath: `finanzas-respaldo-${hoyISO()}.db`,
      filters: [{ name: "Base de datos", extensions: ["db"] }],
    });
    if (!destino) return;

    setRutaRespaldo(null);
    respaldar.mutate(destino, { onSuccess: setRutaRespaldo });
  };

  const elegirYExportarJson = async () => {
    const destino = await save({
      title: "Exportar a JSON",
      defaultPath: `finanzas-${hoyISO()}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!destino) return;

    setExportacion(null);
    exportarJson.mutate(destino, { onSuccess: setExportacion });
  };

  const elegirYExportarCsv = async () => {
    const directorio = await open({
      title: "Elige la carpeta para los CSV",
      directory: true,
      multiple: false,
    });
    if (typeof directorio !== "string") return;

    setExportacion(null);
    exportarCsv.mutate(directorio, { onSuccess: setExportacion });
  };

  const elegirRestauracion = async () => {
    const origen = await open({
      title: "Elige el respaldo a restaurar",
      multiple: false,
      filters: [{ name: "Base de datos", extensions: ["db"] }],
    });
    if (typeof origen !== "string") return;

    restaurar.reset();
    setConfirmarRestauracion(origen);
  };

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-xl font-semibold">Respaldo y exportación</h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">
          Tus datos viven solo en este computador. El respaldo es tu única red de seguridad.
        </p>
      </header>

      {d.requiere_recordatorio ? (
        <div className="rounded-xl border border-amber-300 bg-amber-50 p-4 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-200">
          <p className="font-medium">
            {d.ultimo_respaldo
              ? `Pasaron ${d.dias_desde_ultimo} días desde tu último respaldo.`
              : "Todavía no has respaldado nunca."}
          </p>
          <p className="mt-1">
            Si este computador falla, no hay copia en la nube desde donde recuperar tus datos.
          </p>
        </div>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-3">
        <div className="tarjeta">
          <p className="etiqueta">Último respaldo</p>
          <p className="mt-1 text-lg font-semibold">
            {d.ultimo_respaldo ? formatearFecha(d.ultimo_respaldo) : "Nunca"}
          </p>
          {d.dias_desde_ultimo !== null ? (
            <p className="text-xs text-slate-500 dark:text-slate-400">
              hace {d.dias_desde_ultimo} día{d.dias_desde_ultimo === 1 ? "" : "s"}
            </p>
          ) : null}
        </div>

        <div className="tarjeta">
          <p className="etiqueta">Registros guardados</p>
          <p className="mt-1 text-lg font-semibold tabular">{d.total_registros}</p>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            {formatearTamano(d.tamano_bytes)} en disco
          </p>
        </div>

        <div className="tarjeta">
          <p className="etiqueta">Versión del esquema</p>
          <p className="mt-1 text-lg font-semibold tabular">{d.version_esquema}</p>
          <p className="truncate text-xs text-slate-500 dark:text-slate-400" title={d.ruta_db}>
            {d.ruta_db}
          </p>
        </div>
      </div>

      <div className="tarjeta space-y-3">
        <div>
          <h2 className="font-medium">Respaldar la base</h2>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Copia fiel de todo, en un archivo <code>.db</code> que después puedes restaurar. Es lo
            que conviene guardar en un pendrive o en tu nube.
          </p>
        </div>

        <Boton onClick={elegirYRespaldar} disabled={respaldar.isPending}>
          {respaldar.isPending ? "Respaldando…" : "Guardar respaldo…"}
        </Boton>

        {rutaRespaldo ? (
          <p className="rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300">
            Respaldo guardado en <code className="break-all">{rutaRespaldo}</code>
          </p>
        ) : null}

        {respaldar.error ? <MensajeError error={respaldar.error} /> : null}
      </div>

      <div className="tarjeta space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 className="font-medium">Copia automática local</h2>
            <p className="text-sm text-slate-500 dark:text-slate-400">
              La app guarda sola una copia por día en tu computador y conserva las últimas 5. Es
              una red contra un error al actualizar o un borrado accidental, no contra que se dañe
              el disco: para eso sigue haciendo falta el respaldo de arriba.
            </p>
          </div>

          <Interruptor
            activo={d.respaldo_automatico}
            ocupado={fijarAutomatico.isPending}
            etiqueta="Copia automática local"
            onCambio={(v) => fijarAutomatico.mutate(v)}
          />
        </div>

        {d.respaldo_automatico ? (
          <p className="text-xs text-slate-500 dark:text-slate-400">
            {d.copias_automaticas > 0 ? (
              <>
                {d.copias_automaticas} copia{d.copias_automaticas === 1 ? "" : "s"} guardada
                {d.copias_automaticas === 1 ? "" : "s"}
                {d.ultimo_automatico ? `, la última del ${formatearFecha(d.ultimo_automatico)}` : ""}
                .{" "}
              </>
            ) : (
              "Todavía no hay copias; se genera una al abrir o cerrar la app. "
            )}
            <code className="break-all">{d.carpeta_respaldos}</code>
          </p>
        ) : null}

        {fijarAutomatico.error ? <MensajeError error={fijarAutomatico.error} /> : null}
      </div>

      <div className="tarjeta space-y-3">
        <div>
          <h2 className="font-medium">Exportar</h2>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Para abrir tus datos en otra parte. La exportación no sirve para restaurar: para eso
            usa el respaldo <code>.db</code>.
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          <Boton variante="secundario" onClick={elegirYExportarJson} disabled={exportarJson.isPending}>
            {exportarJson.isPending ? "Exportando…" : "Exportar a JSON…"}
          </Boton>
          <Boton variante="secundario" onClick={elegirYExportarCsv} disabled={exportarCsv.isPending}>
            {exportarCsv.isPending ? "Exportando…" : "Exportar a CSV…"}
          </Boton>
        </div>

        <p className="text-xs text-slate-500 dark:text-slate-400">
          El CSV genera un archivo por tabla dentro de la carpeta que elijas, en UTF-8 con BOM para
          que Excel abra bien las tildes.
        </p>

        {exportacion ? (
          <div className="rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300">
            <p className="font-medium">
              {exportacion.archivos.length} archivo
              {exportacion.archivos.length === 1 ? "" : "s"} · {exportacion.total_filas} registros
            </p>
            <ul className="mt-1 space-y-0.5 text-xs">
              {exportacion.archivos.map((a) => (
                <li key={a.ruta} className="flex justify-between gap-3">
                  <span className="truncate">{a.nombre}</span>
                  <span className="shrink-0 tabular">{a.filas}</span>
                </li>
              ))}
            </ul>
          </div>
        ) : null}

        {exportarJson.error ? <MensajeError error={exportarJson.error} /> : null}
        {exportarCsv.error ? <MensajeError error={exportarCsv.error} /> : null}
      </div>

      <div className="tarjeta space-y-3 border-rose-200 dark:border-rose-900">
        <div>
          <h2 className="flex items-center gap-2 font-medium">
            Restaurar un respaldo
            <Insignia tono="rojo">Reemplaza todo</Insignia>
          </h2>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Deja la base tal como estaba en el respaldo. Todo lo que hayas registrado después se
            pierde, salvo por la copia de seguridad que la app guarda automáticamente antes de
            sobrescribir.
          </p>
        </div>

        <Boton variante="peligro" onClick={elegirRestauracion} disabled={restaurar.isPending}>
          {restaurar.isPending ? "Restaurando…" : "Elegir respaldo…"}
        </Boton>

        {restauracion ? (
          <div className="rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300">
            <p className="font-medium">
              Restaurado: {restauracion.total_registros} registros.
            </p>
            <p className="mt-1 text-xs">
              Lo que había quedó guardado en{" "}
              <code className="break-all">{restauracion.ruta_respaldo_previo}</code>
            </p>
          </div>
        ) : null}

        {restaurar.error ? <MensajeError error={restaurar.error} /> : null}
      </div>

      <Modal
        abierto={!!confirmarRestauracion}
        ancho="md"
        titulo="Restaurar respaldo"
        onCerrar={() => setConfirmarRestauracion(null)}
        acciones={
          <>
            <Boton variante="secundario" onClick={() => setConfirmarRestauracion(null)}>
              Cancelar
            </Boton>
            <Boton
              variante="peligro"
              disabled={restaurar.isPending}
              onClick={() =>
                confirmarRestauracion &&
                restaurar.mutate(confirmarRestauracion, {
                  onSuccess: (r) => {
                    setRestauracion(r);
                    setConfirmarRestauracion(null);
                  },
                })
              }
            >
              {restaurar.isPending ? "Restaurando…" : "Sí, reemplazar mis datos"}
            </Boton>
          </>
        }
      >
        <div className="space-y-3 text-sm">
          <p>
            Se va a reemplazar toda tu base actual —{" "}
            <strong>{d.total_registros} registros</strong> — por el contenido de:
          </p>
          <p className="break-all rounded-lg bg-slate-100 px-3 py-2 font-mono text-xs dark:bg-slate-800">
            {confirmarRestauracion}
          </p>
          <p className="text-slate-500 dark:text-slate-400">
            Antes de sobrescribir, la app guarda una copia de lo que hay hoy junto al archivo de
            datos, así que esto se puede deshacer.
          </p>
        </div>
      </Modal>
    </div>
  );
}

function MensajeError({ error }: { error: unknown }) {
  return (
    <p className="rounded-lg bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950/40 dark:text-rose-300">
      {mensajeDeError(error)}
    </p>
  );
}
