use std::path::{Path, PathBuf};

use rusqlite::{Connection, DatabaseName, OpenFlags};
use tauri::{AppHandle, State};

use crate::db::{conexion, migraciones};
use crate::dominio::csv;
use crate::dominio::fechas;
use crate::dominio::respaldos;
use crate::error::{AppError, Resultado};
use crate::modelos::respaldo::{
    ArchivoExportado, EstadoRespaldo, RespaldoJson, ResultadoExportacion, ResultadoRestauracion,
    DIAS_RECORDATORIO,
};
use crate::repos;
use crate::EstadoApp;

/// Las tablas con datos del usuario, en orden de dependencia.
const TABLAS: [&str; 9] = [
    "periodos",
    "categorias",
    "servicios",
    "deudas",
    "cuotas",
    "movimientos",
    "presupuestos",
    "cuentas",
    "notas_ahorro",
];

/// Tablas que identifican un archivo como base de Finanzas.
///
/// Es el esquema inicial a propósito y no [`TABLAS`]: un respaldo hecho con una
/// versión anterior no tiene las tablas que se agregaron después y sigue siendo
/// perfectamente válido —al restaurarlo se le aplican las migraciones que le
/// falten—. Exigir una tabla nueva acá rechazaría respaldos buenos.
const TABLAS_IDENTIDAD: [&str; 7] = [
    "periodos",
    "categorias",
    "servicios",
    "deudas",
    "cuotas",
    "movimientos",
    "presupuestos",
];

/// Cuándo fue el último respaldo y si toca recordar.
#[tauri::command]
pub fn estado_respaldo(app: AppHandle, estado: State<'_, EstadoApp>) -> Resultado<EstadoRespaldo> {
    let ruta = conexion::ruta_db(&app)?;
    let guard = estado.conn();

    let ultimo_respaldo = repos::configuracion::obtener(&guard, repos::configuracion::ULTIMO_RESPALDO)?;

    let dias_desde_ultimo = match ultimo_respaldo.as_deref() {
        Some(iso) => {
            let fecha = fechas::desde_iso(iso)?;
            Some((fechas::hoy() - fecha).num_days())
        }
        None => None,
    };

    let carpeta = carpeta_respaldos(&app)?;
    let automaticas = listar_automaticos(&carpeta);

    Ok(EstadoRespaldo {
        requiere_recordatorio: dias_desde_ultimo.map_or(true, |d| d >= DIAS_RECORDATORIO),
        ultimo_respaldo,
        dias_desde_ultimo,
        tamano_bytes: std::fs::metadata(&ruta).map(|m| m.len()).unwrap_or(0),
        ruta_db: ruta.to_string_lossy().to_string(),
        version_esquema: migraciones::version_actual(&guard)?,
        total_registros: contar_registros(&guard)?,
        respaldo_automatico: repos::configuracion::obtener_bool(
            &guard,
            repos::configuracion::RESPALDO_AUTOMATICO,
            true,
        )?,
        carpeta_respaldos: carpeta.to_string_lossy().to_string(),
        ultimo_automatico: automaticas.last().map(|n| fecha_de_automatico(n)),
        copias_automaticas: automaticas.len() as i32,
    })
}

#[tauri::command]
pub fn fijar_respaldo_automatico(estado: State<'_, EstadoApp>, activo: bool) -> Resultado<()> {
    let guard = estado.conn();
    repos::configuracion::guardar_bool(
        &guard,
        repos::configuracion::RESPALDO_AUTOMATICO,
        activo,
    )
}

// ── respaldo automático local ────────────────────────────────────────────────

/// Carpeta de copias, la misma que usa el respaldo previo a migrar.
pub fn carpeta_respaldos(app: &AppHandle) -> Resultado<PathBuf> {
    conexion::carpeta_respaldos(app)
}

/// Copia local silenciosa, una por día, conservando las últimas
/// [`respaldos::COPIAS_A_CONSERVAR`].
///
/// `forzar` reescribe la copia del día: se usa al salir, para que la última
/// incluya el trabajo de la jornada. Al arrancar va en false, así abrir y
/// cerrar la app diez veces no reescribe el archivo diez veces.
///
/// **No toca la marca de último respaldo manual.** El recordatorio existe para
/// que el usuario se lleve una copia fuera del computador; si estas copias
/// locales lo silenciaran, un disco dañado se llevaría los datos y los
/// respaldos juntos.
pub fn respaldo_automatico(
    app: &AppHandle,
    conn: &Connection,
    forzar: bool,
) -> Resultado<Option<PathBuf>> {
    let activo = repos::configuracion::obtener_bool(
        conn,
        repos::configuracion::RESPALDO_AUTOMATICO,
        true,
    )?;
    if !activo {
        return Ok(None);
    }

    let carpeta = carpeta_respaldos(app)?;
    std::fs::create_dir_all(&carpeta)?;

    let ruta = carpeta.join(respaldos::nombre_para(&fechas::a_iso(fechas::hoy())));
    if ruta.exists() && !forzar {
        return Ok(None);
    }

    conn.backup(DatabaseName::Main, &ruta, None)?;
    rotar(&carpeta);

    Ok(Some(ruta))
}

/// Nombres de respaldo automático presentes en la carpeta, de más antiguo a
/// más reciente.
fn listar_automaticos(carpeta: &Path) -> Vec<String> {
    let Ok(entradas) = std::fs::read_dir(carpeta) else {
        return Vec::new();
    };

    let mut nombres: Vec<String> = entradas
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| respaldos::es_automatico(n))
        .collect();

    nombres.sort();
    nombres
}

/// 'finanzas-auto-2026-08-15.db' -> '2026-08-15'
fn fecha_de_automatico(nombre: &str) -> String {
    nombre
        .trim_start_matches(respaldos::PREFIJO_AUTOMATICO)
        .trim_end_matches(".db")
        .to_string()
}

/// Borra las copias que sobran. Un fallo acá no debe impedir que el respaldo
/// recién hecho quede en pie, así que los errores se ignoran a propósito.
fn rotar(carpeta: &Path) {
    let nombres = listar_automaticos(carpeta);

    for sobrante in respaldos::a_eliminar(&nombres, respaldos::COPIAS_A_CONSERVAR) {
        let _ = std::fs::remove_file(carpeta.join(sobrante));
    }
}

/// Copia la base a la ruta elegida usando la API de respaldo de SQLite.
/// No es una copia de archivo: con WAL activo eso podría dejar fuera las
/// últimas transacciones.
#[tauri::command]
pub fn respaldar_base(estado: State<'_, EstadoApp>, destino: String) -> Resultado<String> {
    let destino = PathBuf::from(destino.trim());
    validar_destino(&destino)?;

    let guard = estado.conn();
    guard.backup(DatabaseName::Main, &destino, None)?;

    repos::configuracion::guardar(
        &guard,
        repos::configuracion::ULTIMO_RESPALDO,
        &fechas::a_iso(fechas::hoy()),
    )?;

    Ok(destino.to_string_lossy().to_string())
}

/// Reemplaza la base actual por la del respaldo.
///
/// Antes de tocar nada deja una copia de seguridad de lo que hay ahora, junto
/// al archivo de la app: si el respaldo resulta ser el equivocado, los datos
/// siguen estando.
#[tauri::command]
pub fn restaurar_base(
    app: AppHandle,
    estado: State<'_, EstadoApp>,
    origen: String,
) -> Resultado<ResultadoRestauracion> {
    let origen = PathBuf::from(origen.trim());
    if !origen.is_file() {
        return Err(AppError::validacion(format!(
            "No se encontró el archivo: {}",
            origen.display()
        )));
    }

    let version_origen = validar_respaldo(&origen)?;

    let mut guard = estado.conn();

    // Copia de seguridad de lo que existe hoy, antes de sobrescribir.
    let respaldo_previo = conexion::ruta_db(&app)?.with_file_name(format!(
        "finanzas-antes-de-restaurar-{}.db",
        fechas::a_iso(fechas::hoy())
    ));
    guard.backup(DatabaseName::Main, &respaldo_previo, None)?;

    guard.restore(
        DatabaseName::Main,
        &origen,
        None::<fn(rusqlite::backup::Progress)>,
    )?;

    // El respaldo puede venir de una versión anterior de la app.
    migraciones::ejecutar(&mut guard)?;

    Ok(ResultadoRestauracion {
        ruta_respaldo_previo: respaldo_previo.to_string_lossy().to_string(),
        version_restaurada: version_origen,
        total_registros: contar_registros(&guard)?,
    })
}

/// Exporta la base completa a un único archivo JSON.
#[tauri::command]
pub fn exportar_json(estado: State<'_, EstadoApp>, destino: String) -> Resultado<ResultadoExportacion> {
    let destino = PathBuf::from(destino.trim());
    validar_destino(&destino)?;

    let guard = estado.conn();

    let respaldo = RespaldoJson {
        app: "finanzas",
        version_esquema: migraciones::version_actual(&guard)?,
        exportado_en: fechas::a_iso(fechas::hoy()),
        periodos: repos::periodos::listar(&guard)?,
        categorias: repos::categorias::listar(&guard, false)?,
        servicios: repos::servicios::listar(&guard, false)?,
        deudas: repos::deudas::listar(&guard, None, None)?,
        cuotas: repos::cuotas::listar_todas(&guard)?,
        movimientos: repos::movimientos::listar_todos(&guard)?,
        presupuestos: repos::presupuestos::listar_todos(&guard)?,
        cuentas: repos::cuentas::listar(&guard, false)?,
        notas_ahorro: repos::notas_ahorro::listar_todas(&guard)?,
        saldo_inicial: repos::configuracion::obtener_monto(
            &guard,
            repos::configuracion::SALDO_INICIAL,
        )?,
    };

    let filas = (respaldo.periodos.len()
        + respaldo.categorias.len()
        + respaldo.servicios.len()
        + respaldo.deudas.len()
        + respaldo.cuotas.len()
        + respaldo.movimientos.len()
        + respaldo.presupuestos.len()
        + respaldo.cuentas.len()
        + respaldo.notas_ahorro.len()) as i64;

    let texto = serde_json::to_string_pretty(&respaldo)
        .map_err(|e| AppError::validacion(format!("No se pudo generar el JSON: {e}")))?;
    std::fs::write(&destino, texto)?;

    Ok(ResultadoExportacion {
        archivos: vec![ArchivoExportado {
            nombre: nombre_de(&destino),
            ruta: destino.to_string_lossy().to_string(),
            filas,
        }],
        total_filas: filas,
    })
}

/// Exporta una tabla por archivo dentro de la carpeta elegida.
#[tauri::command]
pub fn exportar_csv(
    estado: State<'_, EstadoApp>,
    directorio: String,
) -> Resultado<ResultadoExportacion> {
    let directorio = PathBuf::from(directorio.trim());
    if !directorio.is_dir() {
        return Err(AppError::validacion(format!(
            "No existe la carpeta: {}",
            directorio.display()
        )));
    }

    let guard = estado.conn();
    let sello = fechas::a_iso(fechas::hoy());
    let mut archivos = Vec::new();

    for tabla in TABLAS {
        let (contenido, filas) = tabla_a_csv(&guard, tabla)?;
        let ruta = directorio.join(format!("finanzas-{tabla}-{sello}.csv"));

        // El BOM hace que Excel en Windows abra el archivo como UTF-8 y no
        // rompa las tildes.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(contenido.as_bytes());
        std::fs::write(&ruta, bytes)?;

        archivos.push(ArchivoExportado {
            nombre: nombre_de(&ruta),
            ruta: ruta.to_string_lossy().to_string(),
            filas,
        });
    }

    Ok(ResultadoExportacion {
        total_filas: archivos.iter().map(|a| a.filas).sum(),
        archivos,
    })
}

// ── auxiliares ───────────────────────────────────────────────────────────────

/// Vuelca una tabla completa a CSV leyendo sus columnas del propio SQLite,
/// así no hay que mantener una lista de campos por tabla.
///
/// Es pública para poder cubrirla con tests sin levantar Tauri.
pub fn tabla_a_csv(conn: &Connection, tabla: &str) -> Resultado<(String, i64)> {
    // `tabla` viene de la constante TABLAS, nunca de entrada del usuario.
    let mut stmt = conn.prepare(&format!("SELECT * FROM {tabla}"))?;

    let columnas: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let mut salida = csv::linea(&columnas);
    let mut filas = 0;

    let mut rows = stmt.query([])?;
    while let Some(fila) = rows.next()? {
        let campos: Vec<String> = (0..columnas.len())
            .map(|i| valor_a_texto(fila, i))
            .collect::<rusqlite::Result<Vec<_>>>()?;

        salida.push_str(&csv::linea(&campos));
        filas += 1;
    }

    Ok((salida, filas))
}

fn valor_a_texto(fila: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<String> {
    use rusqlite::types::ValueRef;

    Ok(match fila.get_ref(idx)? {
        ValueRef::Null => String::new(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
        ValueRef::Blob(_) => "[binario]".to_string(),
    })
}

/// Total de filas en las tablas con datos del usuario.
pub fn contar_registros(conn: &Connection) -> Resultado<i64> {
    let mut total = 0;
    for tabla in TABLAS {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |f| f.get(0))?;
        total += n;
    }
    Ok(total)
}

/// Abre el archivo en solo lectura y confirma que es una base de esta app.
/// Devuelve la versión de esquema del respaldo.
///
/// Es pública para poder cubrirla con tests sin levantar Tauri.
pub fn validar_respaldo(origen: &Path) -> Resultado<i32> {
    let conn = Connection::open_with_flags(origen, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
        |_| AppError::validacion("El archivo no es una base de datos SQLite válida."),
    )?;

    // La lista se arma desde la constante para que no puedan desincronizarse.
    // Son nombres del binario, nunca entrada del usuario.
    let lista = TABLAS_IDENTIDAD
        .map(|t| format!("'{t}'"))
        .join(",");

    let encontradas: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name IN ({lista})"
            ),
            [],
            |f| f.get(0),
        )
        .map_err(|_| AppError::validacion("No se pudo leer el archivo como respaldo."))?;

    if encontradas < TABLAS_IDENTIDAD.len() as i64 {
        return Err(AppError::validacion(
            "Ese archivo no parece un respaldo de Finanzas: le faltan tablas.",
        ));
    }

    let version: i32 = conn.query_row("PRAGMA user_version", [], |f| f.get(0))?;
    if version > migraciones::version_objetivo() {
        return Err(AppError::conflicto(format!(
            "El respaldo viene de una versión más nueva de la app (esquema {version}, \
             esta versión entiende hasta {}). Actualiza Finanzas antes de restaurarlo.",
            migraciones::version_objetivo()
        )));
    }

    Ok(version)
}

fn validar_destino(destino: &Path) -> Resultado<()> {
    match destino.parent() {
        Some(dir) if dir.as_os_str().is_empty() || dir.is_dir() => Ok(()),
        Some(dir) => Err(AppError::validacion(format!(
            "No existe la carpeta de destino: {}",
            dir.display()
        ))),
        None => Err(AppError::validacion("La ruta de destino no es válida.")),
    }
}

fn nombre_de(ruta: &Path) -> String {
    ruta.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}
