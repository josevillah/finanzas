use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Resultado;

/// Fecha ISO del último respaldo exitoso.
pub const ULTIMO_RESPALDO: &str = "ultimo_respaldo";

/// Respaldo automático local activado. "1" o "0"; ausente equivale a activo.
pub const RESPALDO_AUTOMATICO: &str = "respaldo_automatico";

/// Qué hacer cuando el usuario cierra la ventana con la X.
/// Valores: preguntar | bandeja | salir. Ver [`crate::modelos::configuracion::AccionCierre`].
pub const ACCION_CIERRE: &str = "accion_cierre";

pub fn obtener(conn: &Connection, clave: &str) -> Resultado<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT valor FROM configuracion WHERE clave = ?1",
            params![clave],
            |f| f.get(0),
        )
        .optional()?)
}

/// Lee una preferencia booleana. Una clave ausente toma `por_defecto`, para
/// que agregar un ajuste nuevo no exija migrar ni sembrar filas.
pub fn obtener_bool(conn: &Connection, clave: &str, por_defecto: bool) -> Resultado<bool> {
    Ok(match obtener(conn, clave)?.as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => por_defecto,
    })
}

pub fn guardar_bool(conn: &Connection, clave: &str, valor: bool) -> Resultado<()> {
    guardar(conn, clave, if valor { "1" } else { "0" })
}

pub fn guardar(conn: &Connection, clave: &str, valor: &str) -> Resultado<()> {
    conn.execute(
        "INSERT INTO configuracion (clave, valor) VALUES (?1, ?2)
         ON CONFLICT(clave) DO UPDATE SET valor = excluded.valor",
        params![clave, valor],
    )?;
    Ok(())
}
