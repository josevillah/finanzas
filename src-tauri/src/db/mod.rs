pub mod conexion;
pub mod migraciones;

use rusqlite::Connection;
use tauri::AppHandle;

use crate::error::Resultado;

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

    // Junto a las demás copias, no sueltas en el directorio de datos.
    let carpeta = conexion::carpeta_respaldos(app)?;

    if let Some(respaldo) = migraciones::respaldo_pre_migracion(&conn, &carpeta)? {
        eprintln!(
            "[migraciones] copia previa a migrar en {}",
            respaldo.display()
        );
    }

    migraciones::ejecutar(&mut conn)?;

    Ok(conn)
}
