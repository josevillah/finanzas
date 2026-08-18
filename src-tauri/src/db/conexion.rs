use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, Resultado};

/// Nombre del archivo de base de datos dentro del app data dir del sistema.
/// En Windows queda en `%APPDATA%\cl.local.finanzas\finanzas.db`.
pub const NOMBRE_ARCHIVO: &str = "finanzas.db";

/// Ruta absoluta del archivo .db. También sirve para el respaldo (Fase 4).
pub fn ruta_db(app: &AppHandle) -> Resultado<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(NOMBRE_ARCHIVO))
}

/// Carpeta donde viven todas las copias: automáticas diarias, previas a un
/// reinicio y previas a una migración.
///
/// Es una función aparte y pura para poder verificar en un test que el destino
/// es la subcarpeta y no el directorio de datos. Tener dos destinos distintos
/// para lo mismo fue justo el bug que hizo parecer que el respaldo previo a
/// migrar no se ejecutaba.
pub fn carpeta_respaldos_de(directorio_datos: &Path) -> PathBuf {
    directorio_datos.join("respaldos")
}

/// Carpeta de respaldos de esta instalación.
pub fn carpeta_respaldos(app: &AppHandle) -> Resultado<PathBuf> {
    let ruta = ruta_db(app)?;
    let datos = ruta
        .parent()
        .ok_or_else(|| AppError::validacion("La ruta de datos no es válida."))?;

    Ok(carpeta_respaldos_de(datos))
}

/// Abre (creando si no existe) la base de datos local y aplica los PRAGMA.
pub fn abrir(app: &AppHandle) -> Resultado<Connection> {
    let conn = Connection::open(ruta_db(app)?)?;
    configurar(&conn)?;
    Ok(conn)
}

/// Base en memoria, para los tests.
pub fn abrir_en_memoria() -> Resultado<Connection> {
    let conn = Connection::open_in_memory()?;
    configurar(&conn)?;
    Ok(conn)
}

fn configurar(conn: &Connection) -> Resultado<()> {
    // WAL mejora la concurrencia lectura/escritura; foreign_keys no viene
    // activo por defecto en SQLite y lo necesitamos para el ON DELETE CASCADE
    // de cuotas.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}
