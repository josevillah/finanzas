# Finanzas — contexto para retomar el proyecto

Gestor de finanzas personales de escritorio. Uso personal, un solo usuario,
datos 100% locales. Chile: CLP y es-CL. Ubicación: `C:\Programacion\finanzas`.

**Estado: las 4 fases del MVP están entregadas y verificadas** (2026-08-15).

---

## Stack

- **Tauri v2** (Rust + WebView del sistema), instalador `.msi` / `.exe`
- **React 18 + TypeScript + Tailwind v4** (config en CSS, plugin de Vite; no hay
  `tailwind.config.ts` ni `postcss.config.js`)
- **SQLite** vía `rusqlite` con features `bundled`, `chrono`, `backup`
- `react-router-dom` (HashRouter), `@tanstack/react-query`, `recharts`
- Plugins Tauri: `global-shortcut`, `dialog`

Requiere Rust estable (toolchain MSVC) y VS Build Tools con el workload de C++.

---

## Las 4 reglas no negociables

Vienen del brief original. Están cubiertas por tests; no romperlas.

1. **Los montos son `INTEGER`** (pesos chilenos sin decimales) en toda la base y
   en todo Rust. `f64` solo aparece como cálculo intermedio de intereses. El
   formato `$1.234.567` existe únicamente en `src/lib/moneda.ts`.
2. **Las cuotas son filas reales** en la tabla `cuotas`, materializadas al crear
   la deuda dentro de la misma transacción. Nunca se calculan al vuelo.
3. **Toda la lógica de negocio vive en Rust.** El frontend solo presenta.
4. **Fechas ISO `YYYY-MM-DD`** como `TEXT` en SQLite.

---

## Arquitectura

```
src/                       frontend, solo presentación
  lib/                     moneda, fechas, wrappers de invoke, tema, cn
  types/dominio.ts         espejo TypeScript de los structs de Rust
  components/              UI genérica y de dominio
  layout/                  Shell con navegación en 3 grupos
  features/
    deudas/                Fase 1
    mes/                   Fase 2: contexto del mes, resumen, ingresos
    gastos/                Fase 2: movimientos y captura rápida
    catalogos/             Fase 2: categorías y servicios
    presupuesto/           Fase 3
    reportes/              Fase 3
    respaldo/              Fase 4

src-tauri/
  migrations/*.sql         11 migraciones, embebidas con include_str!
  src/dominio/             lógica pura: dinero, fechas, amortización, csv
  src/modelos/             structs de tabla + DTOs (serde, snake_case)
  src/repos/               TODO el SQL, nada de SQL fuera de acá
  src/comandos/            45 comandos de Tauri, capa delgada
  tests/                   12 archivos de integración contra SQLite en memoria
```

**Capas:** `comandos` valida entrada y abre transacciones → `repos` ejecuta SQL →
`dominio` hace los cálculos. `dominio` no conoce SQL ni Tauri, por eso es el
lugar donde vive lo testeable de verdad.

Las funciones de repo reciben `&Connection`, que también acepta `&Transaction`
vía Deref: el manejo de transacciones queda en la capa de comandos.

**Estado compartido:** `Mutex<Connection>` en el `State` de Tauri. Un solo
usuario, un solo escritor; no hace falta un pool.

**Migraciones:** runner propio con `PRAGMA user_version`, cada una en su propia
transacción, idempotente, corre en cada arranque. Para agregar una: crear el
`.sql` en `src-tauri/migrations/` y sumarla al arreglo `MIGRACIONES` de
`src-tauri/src/db/migraciones.rs`.

**Base de datos:** `%APPDATA%\cl.local.finanzas\finanzas.db`, con
`journal_mode = WAL` y `foreign_keys = ON`.

---

## Qué hace cada fase

### Fase 1 — Deudas y cuotas
- CRUD de deudas con generación automática de cuotas y vista previa antes de guardar
- Listado con pagado / pendiente / % de avance y cuotas atrasadas
- Detalle con tabla de amortización completa
- Marcar cuota pagada con fecha y monto real (puede diferir del programado), y deshacer
- Calendario de carga: barras de los próximos 12 / 24 / 36 meses
- Carga financiera: cuotas del mes sobre sueldo líquido, con semáforo
  (verde <15%, amarillo 15–25%, rojo >25%)
- Fecha de libertad: mes de la última cuota y cuánto se libera al terminar cada deuda

### Fase 2 — Períodos, ingresos y gastos
- Resumen del mes: ingresos, gastos, balance, desglose por categoría
- Registro de gastos e ingresos con categoría, servicio y medio de pago
- Captura rápida de gastos hormiga con atajo global **Ctrl+Shift+G**
- CRUD de categorías y de servicios recurrentes
- Cierre y reapertura del mes (un mes cerrado no acepta cambios)
- Pagar una cuota genera su gasto del mes, enlazado por `cuota_id`

### Fase 3 — Presupuesto y reportes
- Asignación por categoría y período, editable en la misma tabla que muestra lo gastado
- Copiar el presupuesto del mes anterior
- Semáforo por categoría: al día (<80%), cerca del límite (80–100%), excedido (>100%)
- Evolución del gasto por categoría en ventanas de 6 / 12 / 24 meses
- Reporte de gastos hormiga con variación contra el mes anterior y contra el promedio

### Extra — Auto-actualización
- `tauri-plugin-updater` contra `josevillah/finanzas` (repo público) vía
  GitHub Releases. Sin backend propio
- Chequeo al arrancar en segundo plano, descarga silenciosa, instalación solo
  con confirmación del usuario
- Instalador NSIS con `installMode: passive`, firmado en CI
- `.github/workflows/publicar.yml` se dispara con un tag `v*`
- `npm run version:sync -- X.Y.Z` mantiene la versión igual en los tres archivos
- Tres capas de protección de datos al migrar (ver decisiones más abajo)

### Extra — Segundo plano y bandeja
- Cerrar con la X pregunta: dejar en bandeja o salir, con "recordar mi elección"
- Ícono de bandeja con menú Abrir / Gasto rápido / Salir
- Inicio automático con Windows, arrancando oculto
- Instancia única
- Pantalla de Preferencias: acción de cierre, autostart y estado del atajo

### Fase 4 — Respaldo y exportación
- Respaldo del `.db` a la ruta elegida
- Restauración con validación previa y copia de seguridad automática
- Exportación a JSON (un archivo) y CSV (uno por tabla)
- Recordatorio si pasaron más de 7 días desde el último respaldo

---

## Decisiones tomadas y por qué

Estas son las que no se deducen del código y conviene no revertir sin pensar.

**Respaldo con la API de SQLite, no copia de archivo.** Con WAL activo, copiar
el `.db` a mano puede dejar fuera transacciones que aún viven en el `-wal` y
producir un respaldo silenciosamente incompleto. Se usa
`Connection::backup` / `Connection::restore`.

**Pagar una cuota genera un movimiento** de tipo gasto con `cuota_id`, imputado
a la categoría con `codigo = 'deudas'`. Un índice único parcial sobre `cuota_id`
garantiza que sea uno solo; repagar con otra fecha lo mueve de período en vez de
duplicarlo. Esos movimientos no se editan ni borran desde la pantalla de gastos.

**Columna `codigo` en `categorias`.** Referencia estable para que el código
ubique la categoría de deudas sin depender del nombre, que el usuario puede
editar. Es la única desviación respecto del esquema del brief original.

**Columna `es_estimado` en `movimientos`.** Los servicios recurrentes generan su
gasto del mes con el monto estimado. Sin esta marca, la comparación estimado vs.
real daría siempre cero. Al cambiar el precio o editar el movimiento pasa a 0.

**Columna `fecha_alta` en `servicios`.** La generación nunca retrocede: solo
crea gastos en meses cuyo último día es posterior o igual al alta. Editar un
servicio no toca su `fecha_alta`, por la misma razón. Esto fue un pedido
explícito del usuario: que los servicios no se cuelen en meses o años previos.

**Sin interés, el residuo va a la última cuota.** Con interés (francesa), la
última cuota absorbe el saldo. En ambos casos `suma(capital) == monto_original`
exacto, con test que lo fija.

**Vencimientos calculados siempre desde la fecha original**, recortando al
último día del mes cuando el día no existe: 31-ene → 28-feb → 31-mar. Si se
acumulara mes a mes se quedaría pegado en el 28.

**"Otras" agrupa las categorías más livianas** en el gráfico de evolución,
pasadas las 6 más pesadas. La agrupación la hace Rust, no el frontend.

**Las variaciones porcentuales devuelven `None` sin base positiva.** Un mes
anterior en $0 daría infinito o un 100% engañoso; la UI muestra "—".

**Categorías y servicios en uso no se borran, se desactivan.** El error dice
cuántos movimientos dependen de ellos. Prioriza no romper el historial.

**Un presupuesto de $0 borra la línea** en vez de guardar un cero.

**CSV con BOM UTF-8.** Sin eso Excel en Windows rompe las tildes.

**`AtomicBool salida_real` en el estado.** El interceptor de `CloseRequested`
la consulta antes que nada. Sin ella, con la preferencia en "bandeja" no
existiría forma de cerrar la app salvo el Administrador de tareas. La levantan
el comando de cierre y la opción Salir del menú de bandeja.

**La ventana nace con `"visible": false`** y se muestra en el setup. Así el
arranque con `--minimizado` (autostart) no produce un parpadeo. La bandeja se
construye antes de decidir si mostrarla: si el tray fallara, la ventana se
muestra igual en vez de dejar la app inalcanzable.

**El orden de `mostrar_y_enfocar` no es intercambiable:** `show()` →
`unminimize()` → `set_focus()`. En Windows `unminimize()` sobre una ventana
oculta no hace nada, y sin `set_focus()` la ventana aparece detrás de la
aplicación activa y el input de captura rápida no recibe el teclado.

**Instalar una actualización levanta `salida_real` antes de reiniciar.** El
instalador reemplaza el ejecutable en marcha, así que la app debe cerrarse; sin
la bandera, el interceptor de cierre de la Feature A bloquearía el reinicio y
la actualización quedaría a medio aplicar. Es el punto donde las features A y B
se tocan.

**El chequeo automático falla en silencio, el manual informa.** Sin internet no
tiene sentido molestar a alguien con un error que no puede resolver; pero si
apretó el botón de "Buscar actualizaciones", espera una respuesta.

**Las tres capas de protección de datos** viven en `db::iniciar`, en este orden:
rechazar una base más nueva que el binario → respaldar antes de migrar → migrar.
Más la copia automática diaria con rotación de 5. La copia automática **no**
actualiza la marca del respaldo manual: vive en el mismo disco y no protege
contra que el disco falle, que es justo lo que el recordatorio busca evitar.

**`atajo_registrado` se expone en Preferencias.** Si otra app tiene tomado
Ctrl+Shift+G, el registro falla y en release nadie ve un `eprintln!`. Con la app
viviendo en bandeja, el usuario solo notaría que el atajo "no hace nada".

---

## Verificación

- `cargo test` → **86 tests, todos verdes**
  (unitarios en `dominio` + 4 archivos de integración contra SQLite en memoria)
- `npm run typecheck` → limpio en `src/` y `vite.config.ts`
- `npm run build` → genera `.msi` y `.exe` sin warnings del código propio

```bash
cd src-tauri && cargo test
```

```bash
npm run typecheck
```

```bash
npm run dev
```

```bash
npm run build
```

**Nada de esto se probó ejecutando la app.** Lo que solo se verifica usándola:
el atajo global con la ventana minimizada, los diálogos de archivo del respaldo,
y cómo se siente la generación automática de gastos al navegar entre meses.

---

## Pendientes conocidos

**Importar desde JSON.** Hoy solo se restaura desde `.db`. La exportación
JSON/CSV es de solo lectura. Implementar la importación exige resolver
colisiones de IDs e integridad referencial.

**Pago parcial de cuota.** Registrar $30.000 en una cuota de $50.000 la marca
como pagada con monto real $30.000, sin arrastrar los $20.000 restantes. Quedó
así porque el brief pedía "permitir monto distinto al programado" sin definir
qué pasa con el resto. Cambiarlo es un cambio de modelo, no de UI.

**El presupuesto incluye las cuotas.** Los pagos de cuota caen en "Deudas y
créditos"; presupuestar esa categoría incluye las cuotas del mes. Está anotado
en la propia pantalla. Si se quiere separarlas, es un cambio chico.

**Bundle JS de ~620 KB** por recharts. Para una app local que carga desde disco
no es problema, pero se puede code-splitear la página de reportes si molesta.

---

## Cómo trabajar en este proyecto

El usuario pidió expresamente:

- **Mostrar el plan de estructura y esperar confirmación antes de escribir
  código** en bloques de trabajo grandes.
- **No avanzar a la fase siguiente sin que él lo pida.**

Ambas cosas se respetaron durante todo el MVP.
