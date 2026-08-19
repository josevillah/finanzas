export interface ItemNav {
  ruta: string;
  etiqueta: string;
  /** Nombre corto para la barra inferior en ventanas angostas. */
  corto: string;
  icono: string;
  descripcion: string;
}

export interface GrupoNav {
  titulo: string;
  items: ItemNav[];
}

export const NAVEGACION: GrupoNav[] = [
  // Grupo propio y no dentro de "Mes": los saldos son de ahora, no de un
  // período, y no cambian al moverse de mes.
  {
    titulo: "Saldos",
    items: [
      {
        ruta: "/cuentas",
        etiqueta: "Cuentas",
        corto: "Cuentas",
        icono: "🏦",
        descripcion: "Disponible, ahorros y patrimonio",
      },
    ],
  },
  // Junto a Saldos y no dentro de "Mes": una meta no pertenece a un período,
  // y su avance sale de los ahorros.
  {
    titulo: "Metas",
    items: [
      {
        ruta: "/metas",
        etiqueta: "Metas",
        corto: "Metas",
        icono: "🏆",
        descripcion: "Objetivos de compra o ahorro",
      },
    ],
  },
  {
    titulo: "Mes",
    items: [
      {
        ruta: "/mes",
        etiqueta: "Resumen del mes",
        corto: "Mes",
        icono: "🗓️",
        descripcion: "Ingresos, gastos y balance",
      },
      {
        ruta: "/gastos",
        etiqueta: "Gastos e ingresos",
        corto: "Gastos",
        icono: "🧾",
        descripcion: "Registro de movimientos",
      },
      {
        ruta: "/servicios",
        etiqueta: "Servicios",
        corto: "Servicios",
        icono: "💡",
        descripcion: "Recurrentes: estimado vs. real",
      },
      {
        ruta: "/presupuesto",
        etiqueta: "Presupuesto",
        corto: "Presup.",
        icono: "🎯",
        descripcion: "Asignado vs. gastado por categoría",
      },
      {
        ruta: "/reportes",
        etiqueta: "Reportes",
        corto: "Reportes",
        icono: "📈",
        descripcion: "Evolución del gasto y hormigas",
      },
    ],
  },
  {
    titulo: "Deudas",
    items: [
      {
        ruta: "/deudas",
        etiqueta: "Deudas",
        corto: "Deudas",
        icono: "📋",
        descripcion: "Listado y detalle de cada deuda",
      },
      {
        ruta: "/calendario",
        etiqueta: "Calendario de carga",
        corto: "Calendario",
        icono: "📊",
        descripcion: "Cuotas comprometidas mes a mes",
      },
      {
        ruta: "/carga",
        etiqueta: "Carga financiera",
        corto: "Carga",
        icono: "🚦",
        descripcion: "Cuotas del mes sobre el sueldo",
      },
      {
        ruta: "/libertad",
        etiqueta: "Fecha de libertad",
        corto: "Libertad",
        icono: "🏁",
        descripcion: "Cuándo terminas de pagar",
      },
    ],
  },
  {
    titulo: "Configuración",
    items: [
      {
        ruta: "/categorias",
        etiqueta: "Categorías",
        corto: "Categorías",
        icono: "🏷️",
        descripcion: "Cómo se clasifican los gastos",
      },
      {
        ruta: "/respaldo",
        etiqueta: "Respaldo",
        corto: "Respaldo",
        icono: "💾",
        descripcion: "Respaldar, exportar y restaurar",
      },
      {
        ruta: "/configuracion",
        etiqueta: "Preferencias",
        corto: "Config.",
        icono: "⚙️",
        descripcion: "Cierre, inicio automático y atajo",
      },
    ],
  },
];

/** Todos los ítems en una lista plana, para la navegación compacta. */
export const NAVEGACION_PLANA: ItemNav[] = NAVEGACION.flatMap((g) => g.items);
