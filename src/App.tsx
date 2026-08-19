import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { Categorias } from "@/features/catalogos/paginas/Categorias";
import { CierreProvider } from "@/features/configuracion/CierreProvider";
import { Cuentas } from "@/features/cuentas/paginas/Cuentas";
import { Metas } from "@/features/metas/paginas/Metas";
import { Configuracion } from "@/features/configuracion/paginas/Configuracion";
import { Servicios } from "@/features/catalogos/paginas/Servicios";
import { CalendarioCarga } from "@/features/deudas/paginas/CalendarioCarga";
import { CargaFinanciera } from "@/features/deudas/paginas/CargaFinanciera";
import { DetalleDeuda } from "@/features/deudas/paginas/DetalleDeuda";
import { FechaLibertad } from "@/features/deudas/paginas/FechaLibertad";
import { ListaDeudas } from "@/features/deudas/paginas/ListaDeudas";
import { CapturaProvider } from "@/features/gastos/CapturaContexto";
import { Gastos } from "@/features/gastos/paginas/Gastos";
import { MesProvider } from "@/features/mes/MesContexto";
import { ResumenMes } from "@/features/mes/paginas/ResumenMes";
import { Presupuesto } from "@/features/presupuesto/paginas/Presupuesto";
import { Reportes } from "@/features/reportes/paginas/Reportes";
import { Respaldo } from "@/features/respaldo/paginas/Respaldo";
import { Shell } from "@/layout/Shell";

// Los datos son locales: no hay latencia que justifique refetch agresivo.
const cliente = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      refetchOnWindowFocus: false,
      retry: false,
    },
  },
});

export default function App() {
  return (
    <QueryClientProvider client={cliente}>
      <MesProvider>
        <CierreProvider>
          <CapturaProvider>
            {/* HashRouter: con el protocolo propio de Tauri las rutas profundas
                no dependen de la configuración del servidor. */}
            <HashRouter>
              <Routes>
                <Route element={<Shell />}>
                  <Route index element={<Navigate to="/mes" replace />} />

                  <Route path="/cuentas" element={<Cuentas />} />
                  <Route path="/metas" element={<Metas />} />

                  <Route path="/mes" element={<ResumenMes />} />
                  <Route path="/gastos" element={<Gastos />} />
                  <Route path="/servicios" element={<Servicios />} />
                  <Route path="/presupuesto" element={<Presupuesto />} />
                  <Route path="/reportes" element={<Reportes />} />

                  <Route path="/deudas" element={<ListaDeudas />} />
                  <Route path="/deudas/:id" element={<DetalleDeuda />} />
                  <Route path="/calendario" element={<CalendarioCarga />} />
                  <Route path="/carga" element={<CargaFinanciera />} />
                  <Route path="/libertad" element={<FechaLibertad />} />

                  <Route path="/categorias" element={<Categorias />} />
                  <Route path="/respaldo" element={<Respaldo />} />
                  <Route path="/configuracion" element={<Configuracion />} />

                  <Route path="*" element={<Navigate to="/mes" replace />} />
                </Route>
              </Routes>
            </HashRouter>
          </CapturaProvider>
        </CierreProvider>
      </MesProvider>
    </QueryClientProvider>
  );
}
