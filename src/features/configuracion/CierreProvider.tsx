import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState, type ReactNode } from "react";

import { DialogoCierre } from "./componentes/DialogoCierre";

/** Mismo nombre que la constante `EVENTO_SOLICITAR_CIERRE` en Rust. */
const EVENTO = "solicitar-cierre";

/**
 * Escucha el pedido de cierre que envía Rust cuando la preferencia está en
 * "preguntar", y monta el diálogo una sola vez para toda la app.
 *
 * Rust ya bloqueó el cierre antes de emitir el evento: si el usuario cancela,
 * simplemente no pasa nada y la ventana sigue abierta.
 */
export function CierreProvider({ children }: { children: ReactNode }) {
  const [abierto, setAbierto] = useState(false);
  const abrir = useCallback(() => setAbierto(true), []);

  useEffect(() => {
    // Fuera de Tauri (por ejemplo `vite` a secas) no hay puente de eventos.
    const desuscribir = listen(EVENTO, abrir).catch(() => null);

    return () => {
      desuscribir.then((fn) => fn?.()).catch(() => undefined);
    };
  }, [abrir]);

  return (
    <>
      {children}
      <DialogoCierre abierto={abierto} onCerrar={() => setAbierto(false)} />
    </>
  );
}
