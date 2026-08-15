import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { CapturaRapida } from "./componentes/CapturaRapida";

/** Mismo nombre que la constante `EVENTO_CAPTURA_RAPIDA` en Rust. */
const EVENTO = "abrir-captura-rapida";

const Contexto = createContext<{ abrir: () => void } | null>(null);

/**
 * Monta la captura rápida una sola vez para toda la app y la conecta a sus
 * dos disparadores: el atajo global del sistema (Ctrl+Shift+G, que llega como
 * evento desde Rust) y el mismo atajo dentro de la ventana, por si el registro
 * global falló porque otra aplicación ya lo tenía tomado.
 */
export function CapturaProvider({ children }: { children: ReactNode }) {
  const [abierta, setAbierta] = useState(false);
  const abrir = useCallback(() => setAbierta(true), []);

  useEffect(() => {
    // Fuera de Tauri (por ejemplo `vite` a secas) no hay puente de eventos:
    // el atajo dentro de la ventana sigue funcionando igual.
    const desuscribir = listen(EVENTO, abrir).catch(() => null);

    return () => {
      desuscribir.then((fn) => fn?.()).catch(() => undefined);
    };
  }, [abrir]);

  useEffect(() => {
    const alPresionar = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.code === "KeyG") {
        e.preventDefault();
        abrir();
      }
    };
    window.addEventListener("keydown", alPresionar);
    return () => window.removeEventListener("keydown", alPresionar);
  }, [abrir]);

  const valor = useMemo(() => ({ abrir }), [abrir]);

  return (
    <Contexto.Provider value={valor}>
      {children}
      <CapturaRapida abierto={abierta} onCerrar={() => setAbierta(false)} />
    </Contexto.Provider>
  );
}

export function useCaptura() {
  const valor = useContext(Contexto);
  if (!valor) throw new Error("useCaptura debe usarse dentro de <CapturaProvider>");
  return valor;
}
