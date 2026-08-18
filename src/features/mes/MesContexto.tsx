import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import { Boton } from "@/components/ui/Boton";
import { mesAbsoluto, mesActual } from "@/lib/fechas";

import { SelectorMesAnio } from "./componentes/SelectorMesAnio";
import { useMesesDisponibles } from "./hooks";

interface ValorMes {
  anio: number;
  mes: number;
  /** Avanza o retrocede meses respecto del seleccionado. */
  irMes: (delta: number) => void;
  fijarMes: (anio: number, mes: number) => void;
  volverAHoy: () => void;
  esMesActual: boolean;
}

const Contexto = createContext<ValorMes | null>(null);

/**
 * El mes seleccionado se comparte entre todas las pantallas del período:
 * si cambias de mes en Gastos, Resumen y Servicios te siguen.
 */
export function MesProvider({ children }: { children: ReactNode }) {
  const [seleccion, setSeleccion] = useState(mesActual);

  const irMes = useCallback((delta: number) => {
    setSeleccion(({ anio, mes }) => {
      // Meses absolutos desde el año 0, para no pelear con el borde de diciembre.
      const total = anio * 12 + (mes - 1) + delta;
      return { anio: Math.floor(total / 12), mes: (total % 12) + 1 };
    });
  }, []);

  const valor = useMemo<ValorMes>(() => {
    const hoy = mesActual();
    return {
      anio: seleccion.anio,
      mes: seleccion.mes,
      irMes,
      fijarMes: (anio, mes) => setSeleccion({ anio, mes }),
      volverAHoy: () => setSeleccion(hoy),
      esMesActual: seleccion.anio === hoy.anio && seleccion.mes === hoy.mes,
    };
  }, [seleccion, irMes]);

  return <Contexto.Provider value={valor}>{children}</Contexto.Provider>;
}

export function useMes(): ValorMes {
  const valor = useContext(Contexto);
  if (!valor) throw new Error("useMes debe usarse dentro de <MesProvider>");
  return valor;
}

/** Control de mes, compartido por todas las pantallas del período. */
export function SelectorMes() {
  const { anio, mes, irMes, fijarMes, volverAHoy, esMesActual } = useMes();
  const { data: rango } = useMesesDisponibles();

  // Mientras el rango no llega, las flechas no se bloquean: es peor dejar el
  // control muerto un instante que permitir un mes de más.
  const actual = mesAbsoluto(anio, mes);
  const puedeRetroceder = !rango || actual > mesAbsoluto(rango.desde_anio, rango.desde_mes);
  const puedeAvanzar = !rango || actual < mesAbsoluto(rango.hasta_anio, rango.hasta_mes);

  return (
    <div className="flex items-center gap-2">
      <Boton
        variante="secundario"
        tamano="sm"
        onClick={() => irMes(-1)}
        disabled={!puedeRetroceder}
        aria-label="Mes anterior"
      >
        ←
      </Boton>

      <SelectorMesAnio anio={anio} mes={mes} rango={rango} onElegir={fijarMes} />

      <Boton
        variante="secundario"
        tamano="sm"
        onClick={() => irMes(1)}
        disabled={!puedeAvanzar}
        aria-label="Mes siguiente"
      >
        →
      </Boton>

      {!esMesActual ? (
        <Boton variante="fantasma" tamano="sm" onClick={volverAHoy}>
          Hoy
        </Boton>
      ) : null}
    </div>
  );
}
