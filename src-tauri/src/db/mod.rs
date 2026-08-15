pub mod conexion;
pub mod migraciones;

use rusqlite::Connection;
use tauri::AppHandle;

use crate::error::{AppError, Resultado};

/// Abre la base y la deja lista para usar, en el orden que protege los datos:
/// primero se comprueba que el binario entienda el esquema, después se guarda
/// una copia si hay migraciones pendientes, y recién entonces se migra.
///
/// Invertir cualquiera de esos pasos deja al usuario sin red: migrar sin
/// respaldo previo no tiene vuelta atrás, y migrar sin verificar puede
/// escribir sobre un esquema que esta versión no conoce.
pub fn iniciar(app: &AppHandle) -> Resultado<Connection> {
    let mut conn = conexion::abrir(app)?;

    migraciones::verificar_compatibilidad(&conn)?;

    let ruta = conexion::ruta_db(app)?;
    let directorio = ruta
        .parent()
        .ok_or_else(|| AppError::validacion("La ruta de datos no es válida."))?;

    if let Some(respaldo) = migraciones::respaldo_pre_migracion(&conn, directorio)? {
        eprintln!(
            "[migraciones] copia previa a migrar en {}",
            respaldo.display()
        );
    }

    migraciones::ejecutar(&mut conn)?;

    Ok(conn)
}
