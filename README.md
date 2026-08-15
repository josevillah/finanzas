# Finanzas

Gestor de finanzas personales de escritorio. Un solo usuario, datos 100% locales
en SQLite, sin servidor ni cuenta en la nube.

## Stack

- **Tauri v2** — Rust + WebView del sistema, instalador `.msi` / `.exe`
- **React 18 + TypeScript + Tailwind v4** — solo presentación
- **SQLite** vía `rusqlite` (compilado dentro del binario, sin dependencias del sistema)

## Reglas del proyecto

1. **Los montos son `INTEGER`** (pesos chilenos sin decimales) en toda la base y
   en todo Rust. El formato `$1.234.567` existe solo en la capa de presentación
   (`src/lib/moneda.ts`).
2. **Las cuotas son filas reales** en la tabla `cuotas`, materializadas al crear
   la deuda dentro de la misma transacción. Nunca se calculan al vuelo.
3. **Toda la lógica de negocio vive en Rust** (`src-tauri/src/dominio/`). El
   frontend solo pinta lo que devuelven los comandos.
4. **Fechas en ISO** `YYYY-MM-DD`, como `TEXT` en SQLite.

## Requisitos

- Node.js 20+
- Rust (estable, toolchain MSVC) y Visual Studio Build Tools con el workload
  "Desarrollo para escritorio con C++"

## Comandos

```bash
npm install
```

```bash
npm run dev
```

Levanta Vite y abre la ventana de Tauri con recarga en caliente.

```bash
npm run build
```

Genera el instalador en `src-tauri/target/release/bundle/`.

```bash
npm run typecheck
```

```bash
cd src-tauri && cargo test
```

## Dónde quedan los datos

`%APPDATA%\cl.local.finanzas\finanzas.db`

Las migraciones corren solas al abrir la app, cada una en su propia transacción,
y el esquema se versiona con `PRAGMA user_version`. Para agregar una: crea el
`.sql` en `src-tauri/migrations/` y súmalo al arreglo `MIGRACIONES` de
`src-tauri/src/db/migraciones.rs`.

## Estructura

```
src/                      frontend (presentación)
  lib/                    formato de moneda y fechas, wrappers de invoke, tema
  types/dominio.ts        espejo TypeScript de los structs de Rust
  components/             UI genérica y componentes de dominio
  layout/                 shell con navegación
  features/deudas/        Fase 1: deudas, cuotas y análisis
  features/mes/           Fase 2: período seleccionado, resumen e ingresos
  features/gastos/        Fase 2: movimientos y captura rápida
  features/catalogos/     Fase 2: categorías y servicios recurrentes
  features/presupuesto/   Fase 3: asignación y control por categoría
  features/reportes/      Fase 3: evolución del gasto y gastos hormiga
  features/respaldo/      Fase 4: respaldo, exportación y restauración

src-tauri/
  migrations/*.sql        esquema versionado
  src/dominio/            lógica pura: dinero, fechas, amortización
  src/modelos/            structs de tabla + DTOs
  src/repos/              todo el SQL
  src/comandos/           comandos de Tauri (capa delgada)
  tests/                  tests de integración
```

## Estado

**Fase 1 completa** — módulo de deudas y cuotas:

- CRUD de deudas con generación automática de cuotas y vista previa antes de guardar
- Listado con pagado / pendiente / % de avance y cuotas atrasadas
- Detalle con tabla de amortización completa
- Marcar cuota pagada con fecha y monto real (puede diferir del programado), y deshacer
- Calendario de carga: barras de los próximos 12 / 24 / 36 meses
- Carga financiera: cuotas del mes sobre sueldo líquido, con semáforo
  (verde <15%, amarillo 15–25%, rojo >25%)
- Fecha de libertad: mes de la última cuota y cuánto se libera al terminar cada deuda

**Fase 2 completa** — períodos, ingresos y gastos:

- Resumen del mes: ingresos, gastos, balance y desglose por categoría
- Registro de gastos e ingresos con categoría, servicio y medio de pago
- Captura rápida de gastos hormiga: monto + categoría en botones grandes, con
  atajo global **Ctrl+Shift+G** que abre la app aunque esté minimizada
- CRUD de categorías y de servicios recurrentes
- Los servicios activos **cargan su gasto del mes automáticamente** con el monto
  estimado, marcado como `es_estimado`. El botón "Cambiar precio" lo ajusta al
  monto real de la boleta y lo da por confirmado
- Comparación estimado vs. real por servicio, distinguiendo lo confirmado de lo
  que sigue siendo el estimado
- Cierre y reapertura del mes: un período cerrado no acepta cambios
- Pagar una cuota genera su gasto del mes en `movimientos`, enlazado por
  `cuota_id`; deshacer el pago lo borra

**Fase 3 completa** — presupuesto y reportes:

- Asignación de presupuesto por categoría y por período, editable en la misma
  tabla que muestra lo gastado; un monto de $0 saca la categoría del control
- Copiar el presupuesto del mes anterior
- Semáforo por categoría: al día (<80% consumido), cerca del límite (80–100%),
  excedido (>100%)
- Evolución del gasto por categoría, en ventanas de 6 / 12 / 24 meses. Las
  categorías más pesadas se grafican aparte y el resto se agrupa en "Otras"
- Reporte de gastos hormiga mes a mes, con la variación contra el mes anterior
  y contra el promedio de los meses previos

**Fase 4 completa** — respaldo y exportación:

- Respaldo de la base a la ruta que elijas, usando la **API de respaldo de
  SQLite** (no una copia de archivo: con WAL activo eso puede dejar fuera las
  últimas transacciones)
- Restauración desde un `.db`, con validación previa del archivo y copia de
  seguridad automática de lo que había antes de sobrescribir
- Exportación a JSON (un archivo) y a CSV (uno por tabla, UTF-8 con BOM para
  que Excel abra bien las tildes)
- Recordatorio si pasaron más de 7 días desde el último respaldo, visible en
  toda la app

Las cuatro fases del MVP están entregadas.

**Cerrar a segundo plano y bandeja del sistema:**

- Cerrar con la X pregunta si dejar la app corriendo en la bandeja o salir, con
  opción de recordar la elección. La preferencia (`accion_cierre`) vive en la
  tabla `configuracion` y la lee Rust, no el frontend
- Ícono en la bandeja con menú Abrir / Gasto rápido / Salir; el click izquierdo
  muestra la ventana
- Inicio automático con Windows, arrancando directo a la bandeja
- Instancia única: abrir la app estando ya en la bandeja le da el foco a la que
  existe en vez de levantar un segundo proceso sobre la misma base
- La pantalla de Preferencias permite cambiar la acción de cierre después de
  haberla recordado, y avisa si el atajo global quedó tomado por otra aplicación

### Detalles de cálculo

- **Sin interés:** división entera; el residuo se suma a la última cuota para que
  la suma calce exacto con el monto original.
- **Con interés:** cuota francesa `P·i / (1 − (1+i)^−n)`, redondeada a peso. Se
  desglosa cuota a cuota y la última absorbe el saldo, de modo que
  `suma(capital) == monto_original`.
- **Vencimientos:** mes a mes desde la primera cuota, recortando al último día
  del mes cuando el día no existe (31-ene → 28-feb), y siempre calculados desde
  la fecha original para no quedarse pegado en el día recortado.

### Generación del gasto de un servicio

Cada servicio guarda su `fecha_alta`. La generación **nunca retrocede**: solo
crea el gasto en meses cuyo último día es posterior o igual al alta, así abrir
un mes o un año anterior no lo contamina. La edición de un servicio no toca su
`fecha_alta` por la misma razón.

El gasto nace con `es_estimado = 1` y el monto estimado del servicio; el día se
toma de `dia_vencimiento` recortado al mes (el 31 pasa a ser 28 en febrero).
Cambiarle el precio, o editarlo, lo pasa a `es_estimado = 0`. Un índice único
parcial impide que un servicio genere dos estimados en el mismo período, y si ya
registraste el gasto a mano, no se genera nada.

### Protección de los datos al actualizar

Tres capas, pensadas para instalaciones en equipos que el desarrollador no
controla y sin copia en la nube:

1. **Se rechaza una base más nueva que el binario.** Si `user_version` supera la
   última migración conocida, la app muestra un error y no arranca. El caso
   real: se instala una versión que migra el esquema, algo falla, se vuelve a la
   anterior, y esa versión no entiende las tablas nuevas. Sin el chequeo abriría
   igual y escribiría datos inconsistentes en silencio.
2. **Respaldo antes de migrar.** Si hay migraciones pendientes, la base se copia
   a `finanzas-pre-v{N}.db` antes de tocarla. Una base recién creada se salta:
   no hay nada que perder.
3. **Copia automática local**, una por día en
   `%APPDATA%\cl.local.finanzas\respaldos\`, conservando las últimas 5. Se
   dispara al arrancar (si no hay copia de hoy) y al salir (rescribiendo la del
   día con el trabajo de la jornada). Se puede desactivar desde Respaldo.

La copia automática **no silencia el recordatorio de respaldo manual**: vive en
el mismo disco, así que no protege contra que el disco falle. El recordatorio
existe para que la copia salga del computador.

El orden de arranque es `abrir → verificar → respaldar → migrar`, y está en
`db::iniciar`. Invertir cualquier paso deja al usuario sin red.

### Respaldo y restauración

Ambos usan `Connection::backup` / `Connection::restore` de SQLite, que copian
las páginas de la base en caliente y respetan el WAL. Restaurar valida primero
el archivo en solo lectura (que tenga las 7 tablas y que su `user_version` no
sea mayor que la que entiende esta versión de la app), después deja una copia
de la base actual en `finanzas-antes-de-restaurar-YYYY-MM-DD.db` junto al
archivo de datos, y recién ahí sobrescribe. Si el respaldo viene de un esquema
anterior, las migraciones pendientes corren enseguida.

La exportación a JSON y CSV es de solo lectura: sirve para llevarse los datos a
otra herramienta, no para restaurar.

### Enlace cuota → movimiento

El pago de una cuota crea un gasto en `movimientos` con `cuota_id` apuntando a
la cuota, imputado a la categoría marcada con el código `deudas`. Un índice
único parcial sobre `cuota_id` garantiza que sea uno solo, y volver a pagar con
otra fecha lo mueve de período en vez de duplicarlo. Esos movimientos no se
editan ni se borran desde la pantalla de gastos: se deshacen desde la deuda.
