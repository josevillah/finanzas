import { useState } from "react";
import { NavLink, Outlet } from "react-router-dom";

import { Boton } from "@/components/ui/Boton";
import { AvisoActualizacion } from "@/features/actualizacion/componentes/AvisoActualizacion";
import { useCaptura } from "@/features/gastos/CapturaContexto";
import { AvisoRespaldo } from "@/features/respaldo/componentes/AvisoRespaldo";
import { cn } from "@/lib/cn";
import { alternarTema, temaActual, type Tema } from "@/lib/tema";

import { NAVEGACION, NAVEGACION_PLANA } from "./nav";

export function Shell() {
  const [tema, setTema] = useState<Tema>(() => temaActual());
  const { abrir } = useCaptura();

  return (
    <div className="flex min-h-screen">
      <aside className="hidden w-64 shrink-0 flex-col border-r border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900 md:flex">
        <div className="mb-5 px-2">
          <p className="text-lg font-semibold tracking-tight">Finanzas</p>
          <p className="text-xs text-slate-500 dark:text-slate-400">Control personal</p>
        </div>

        <Boton tamano="sm" className="mb-5" onClick={abrir} title="Ctrl+Shift+G">
          ⚡ Gasto rápido
        </Boton>

        <nav className="flex-1 space-y-5 overflow-y-auto">
          {NAVEGACION.map((grupo) => (
            <div key={grupo.titulo}>
              <p className="mb-1 px-3 text-[11px] font-semibold uppercase tracking-wide text-slate-400">
                {grupo.titulo}
              </p>
              <div className="space-y-0.5">
                {grupo.items.map((item) => (
                  <NavLink
                    key={item.ruta}
                    to={item.ruta}
                    title={item.descripcion}
                    className={({ isActive }) =>
                      cn(
                        "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                        isActive
                          ? "bg-indigo-50 font-medium text-indigo-700 dark:bg-indigo-950/60 dark:text-indigo-300"
                          : "text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800",
                      )
                    }
                  >
                    <span aria-hidden>{item.icono}</span>
                    {item.etiqueta}
                  </NavLink>
                ))}
              </div>
            </div>
          ))}
        </nav>

        <button
          type="button"
          onClick={() => setTema(alternarTema())}
          className="mt-4 flex items-center gap-3 rounded-lg px-3 py-2 text-sm text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
        >
          <span aria-hidden>{tema === "oscuro" ? "☀️" : "🌙"}</span>
          {tema === "oscuro" ? "Modo claro" : "Modo oscuro"}
        </button>
      </aside>

      {/* Navegación compacta para ventanas angostas. */}
      <nav className="fixed inset-x-0 bottom-0 z-40 flex overflow-x-auto border-t border-slate-200 bg-white md:hidden dark:border-slate-800 dark:bg-slate-900">
        {NAVEGACION_PLANA.map((item) => (
          <NavLink
            key={item.ruta}
            to={item.ruta}
            className={({ isActive }) =>
              cn(
                "flex min-w-16 flex-1 flex-col items-center gap-0.5 py-2 text-[10px]",
                isActive ? "text-indigo-600 dark:text-indigo-400" : "text-slate-500",
              )
            }
          >
            <span aria-hidden className="text-base">
              {item.icono}
            </span>
            {item.corto}
          </NavLink>
        ))}
      </nav>

      <main className="flex-1 overflow-x-hidden px-4 pb-20 pt-6 md:px-8 md:pb-8">
        <div className="mx-auto max-w-6xl">
          <AvisoActualizacion />
          <AvisoRespaldo />
          <Outlet />
        </div>
      </main>
    </div>
  );
}
