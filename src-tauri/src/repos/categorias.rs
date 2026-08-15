use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Resultado};
use crate::modelos::categoria::{Categoria, NuevaCategoria};

pub fn listar(conn: &Connection, solo_activas: bool) -> Resultado<Vec<Categoria>> {
    let filtro = if solo_activas { "WHERE activa = 1" } else { "" };
    let sql = format!(
        "SELECT {} FROM categorias {filtro} ORDER BY tipo, nombre",
        Categoria::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Categoria::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Categoria> {
    let sql = format!("SELECT {} FROM categorias WHERE id = ?1", Categoria::COLUMNAS);
    conn.query_row(&sql, params![id], Categoria::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la categoría #{id}")))
}

/// Busca por código estable (ver [`crate::modelos::categoria::CODIGO_DEUDAS`]).
pub fn por_codigo(conn: &Connection, codigo: &str) -> Resultado<Option<Categoria>> {
    let sql = format!(
        "SELECT {} FROM categorias WHERE codigo = ?1",
        Categoria::COLUMNAS
    );
    Ok(conn
        .query_row(&sql, params![codigo], Categoria::desde_fila)
        .optional()?)
}

pub fn insertar(conn: &Connection, datos: &NuevaCategoria) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO categorias (nombre, tipo, color, activa) VALUES (?1, ?2, ?3, ?4)",
        params![
            datos.nombre.trim(),
            datos.tipo.como_texto(),
            datos.color.as_deref(),
            datos.activa as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn actualizar(conn: &Connection, id: i64, datos: &NuevaCategoria) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE categorias SET nombre = ?2, tipo = ?3, color = ?4, activa = ?5 WHERE id = ?1",
        params![
            id,
            datos.nombre.trim(),
            datos.tipo.como_texto(),
            datos.color.as_deref(),
            datos.activa as i64,
        ],
    )?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la categoría #{id}")));
    }
    Ok(())
}

/// Cuántos movimientos y servicios dependen de la categoría. Se usa para
/// decidir entre borrar y desactivar.
pub fn usos(conn: &Connection, id: i64) -> Resultado<(i64, i64)> {
    let movimientos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM movimientos WHERE categoria_id = ?1",
        params![id],
        |f| f.get(0),
    )?;
    let servicios: i64 = conn.query_row(
        "SELECT COUNT(*) FROM servicios WHERE categoria_id = ?1",
        params![id],
        |f| f.get(0),
    )?;
    Ok((movimientos, servicios))
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute("DELETE FROM categorias WHERE id = ?1", params![id])?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la categoría #{id}")));
    }
    Ok(())
}
