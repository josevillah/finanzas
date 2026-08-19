import { Moneda } from "@/components/Moneda";
import { BarraAvance } from "@/components/BarraAvance";
import { Boton } from "@/components/ui/Boton";
import { Cargando, ErrorCarga } from "@/components/ui/Estados";
import { Insignia } from "@/components/ui/Insignia";
import { useGenerarAlEntrarAlMes } from "@/features/catalogos/hooks";
import { cn } from "@/lib/cn";
import { formatearPorcentaje } from "@/lib/moneda";
import { ETIQUETAS_TIPO_CATEGORIA, type GastoPorCategoria } from "@/types/dominio";

import { EditorIngresos } from "../componentes/EditorIngresos";
import { useCambiarEstadoPeriodo, useResumenPeriodo } from "../hooks";
import { SelectorMes, useMes } from "../MesContexto";

export function ResumenMes() {
  const { anio, mes } = useMes();

  // Los servicios recurrentes cargan su gasto del mes al entrar.
  useGenerarAlEntrarAlMes(anio, mes);

  const { data, isPending, error, refetch } = useResumenPeriodo(anio, mes);
  const cambiarEstado = useCambiarEstadoPeriodo();

  if (isPending) return <Cargando />;
  if (error) return <ErrorCarga error={error} onReintentar={refetch} />;
  if (!data) return null;

  const cerrado = data.estado === "cerrado";
  const maximoCategoria = Math.max(...data.por_categoria.map((c) => c.total), 0);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="flex items-center gap-2 text-xl font-semibold">
            Resumen del mes
            {cerrado ? <Insignia tono="neutro">Cerrado</Insignia> : null}
          </h1>
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Lo que entró, lo que salió y en qué se fue.
          </p>
        </div>
        <SelectorMes />
      </header>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metrica titulo="Ingresos" valor={data.total_ingresos} tono="verde" />
        <Metrica titulo="Gastos" valor={data.total_gastos} tono="rojo" />
        <Metrica
          titulo="Balance"
          valor={data.balance}
          tono={data.balance >= 0 ? "verde" : "rojo"}
          nota={
            data.total_ingresos > 0
              ? `${formatearPorcentaje((data.balance / data.total_ingresos) * 100)} de tus ingresos`
              : undefined
          }
        />
        <Metrica
          titulo="Gastos hormiga"
          valor={data.total_hormiga}
          nota={
            data.total_gastos > 0
              ? `${formatearPorcentaje((data.total_hormiga / data.total_gastos) * 100)} del gasto total`
              : "Sin gastos registrados"
          }
        />
      </div>

      {/* Sin apartados no hay nada que aclarar: no se pinta la tarjeta y la
          grilla queda pegada a lo que sigue, sin hueco. */}
      {data.apartado_neto !== 0 ? (
        <ContextoAhorro
          balance={data.balance}
          apartadoNeto={data.apartado_neto}
          libre={data.libre}
        />
      ) : null}

      <div className="grid gap-4 lg:grid-cols-3">
        <div className="tarjeta lg:col-span-1">
          <EditorIngresos anio={anio} mes={mes} bloqueado={cerrado} />

          <div className="mt-5 border-t border-slate-200 pt-4 dark:border-slate-800">
            <p className="text-xs text-slate-500 dark:text-slate-400">
              {cerrado
                ? "Este mes está cerrado: no acepta cambios en sus movimientos."
                : "Cerrar el mes lo congela: nadie puede agregar ni editar movimientos."}
            </p>
            <Boton
              variante="secundario"
              className="mt-3 w-full"
              disabled={cambiarEstado.isPending}
              onClick={() =>
                cambiarEstado.mutate({
                  anio,
                  mes,
                  estado: cerrado ? "abierto" : "cerrado",
                })
              }
            >
              {cerrado ? "Reabrir el mes" : "Cerrar el mes"}
            </Boton>
          </div>
        </div>

        <div className="tarjeta lg:col-span-2">
          <div className="mb-4 flex items-baseline justify-between">
            <h2 className="font-medium">En qué se fue</h2>
            <span className="text-xs text-slate-500 dark:text-slate-400">
              {data.n_movimientos} movimiento{data.n_movimientos === 1 ? "" : "s"}
              {data.total_cuotas > 0 ? " · incluye pagos de cuotas" : ""}
            </span>
          </div>

          {!data.por_categoria.length ? (
            <p className="py-10 text-center text-sm text-slate-500 dark:text-slate-400">
              Todavía no registras gastos este mes.
            </p>
          ) : (
            <ul className="space-y-3">
              {data.por_categoria.map((c) => (
                <FilaCategoria key={c.categoria_id ?? "sin"} categoria={c} maximo={maximoCategoria} />
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Explica un balance que se lee mal.
 *
 * El balance es correcto —ingresos menos gastos— pero se interpreta como "lo
 * que me queda libre", y parte de eso puede haberse ido a un ahorro el mismo
 * mes. Va en su propia tarjeta ancha y no dentro de la de Balance: la frase no
 * entra en un cuarto de fila sin partirse, y acá además hay lugar para decir
 * por qué el balance no baja al apartar.
 *
 * Cuando el neto es negativo se sacó plata de los ahorros. Ahí no se muestra
 * "libre": sería un número mayor que el balance y se leería como si hubiera
 * aparecido plata, cuando lo que pasó es que salió de una reserva.
 */
function ContextoAhorro({
  balance,
  apartadoNeto,
  libre,
}: {
  balance: number;
  apartadoNeto: number;
  libre: number;
}) {
  const retiro = apartadoNeto < 0;

  return (
    <section className="tarjeta flex flex-wrap items-center justify-between gap-x-8 gap-y-3">
      <div className="min-w-0 max-w-3xl">
        <p className="etiqueta">{retiro ? "Retiros de tus ahorros" : "Apartado este mes"}</p>
        <p className="mt-1 text-sm text-slate-600 dark:text-slate-300">
          {retiro ? (
            <>
              Este mes sacaste <Moneda monto={-apartadoNeto} className="font-medium" /> de tus
              ahorros. No aparece en el balance porque no es un ingreso: es plata que ya era tuya.
            </>
          ) : (
            <>
              De tu balance de <Moneda monto={balance} className="font-medium" /> apartaste{" "}
              <Moneda monto={apartadoNeto} className="font-medium" /> a tus ahorros. El balance no
              baja por eso: apartar no es un gasto, la plata sigue siendo tuya y solo cambió de
              bolsillo.
            </>
          )}
        </p>
      </div>

      {!retiro ? (
        // `ml-auto` para que al envolverse en ventanas angostas siga pegado al
        // borde derecho, en vez de quedar a la izquierda con el texto alineado
        // a la derecha.
        <div className="ml-auto shrink-0 text-right">
          <p className="etiqueta">Libre</p>
          <p className="mt-1 text-2xl font-semibold">
            <Moneda monto={libre} />
          </p>
        </div>
      ) : null}
    </section>
  );
}

function Metrica({
  titulo,
  valor,
  nota,
  tono,
}: {
  titulo: string;
  valor: number;
  nota?: string;
  tono?: "verde" | "rojo";
}) {
  return (
    <div className="tarjeta">
      <p className="etiqueta">{titulo}</p>
      <p
        className={cn(
          "mt-1 text-2xl font-semibold",
          tono === "verde" && "text-emerald-600 dark:text-emerald-400",
          tono === "rojo" && "text-rose-600 dark:text-rose-400",
        )}
      >
        <Moneda monto={valor} />
      </p>
      {nota ? <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{nota}</p> : null}
    </div>
  );
}

function FilaCategoria({
  categoria,
  maximo,
}: {
  categoria: GastoPorCategoria;
  maximo: number;
}) {
  const proporcion = maximo > 0 ? (categoria.total / maximo) * 100 : 0;

  return (
    <li className="space-y-1.5">
      <div className="flex items-center justify-between gap-3 text-sm">
        <span className="flex min-w-0 items-center gap-2">
          <span
            aria-hidden
            className="h-2.5 w-2.5 shrink-0 rounded-full"
            style={{ backgroundColor: categoria.color ?? "#94a3b8" }}
          />
          <span className="truncate">{categoria.categoria_nombre}</span>
          {categoria.categoria_tipo ? (
            <span className="shrink-0 text-xs text-slate-400">
              {ETIQUETAS_TIPO_CATEGORIA[categoria.categoria_tipo]}
            </span>
          ) : null}
        </span>
        <span className="shrink-0 tabular">
          <Moneda monto={categoria.total} />
          <span className="ml-2 text-xs text-slate-400">{categoria.n_movimientos}</span>
        </span>
      </div>
      <BarraAvance porcentaje={proporcion} />
    </li>
  );
}
