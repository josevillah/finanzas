use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Resultado};
use crate::modelos::servicio::{NuevoServicio, Servicio};

pub fn listar(conn: &Connection, solo_activos: bool) -> Resultado<Vec<Servicio>> {
    let filtro = if solo_activos { "WHERE activo = 1" } else { "" };
    let sql = format!(
        "SELECT {} FROM servicios {filtro} ORDER BY activo DESC, dia_vencimiento, nombre",
        Servicio::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Servicio::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Servicio> {
    let sql = format!("SELECT {} FROM servicios WHERE id = ?1", Servicio::COLUMNAS);
    conn.query_row(&sql, params![id], Servicio::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("el servicio #{id}")))
}

pub fn insertar(conn: &Connection, datos: &NuevoServicio, fecha_alta: &str) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO servicios
            (nombre, categoria_id, monto_estimado, dia_vencimiento, tipo, activo, fecha_alta)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            datos.nombre.trim(),
            datos.categoria_id,
            datos.monto_estimado,
            datos.dia_vencimiento,
            datos.tipo.como_texto(),
            datos.activo as i64,
            fecha_alta,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// No toca `fecha_alta`: mover el alta hacia atrás dejaría al servicio
/// generando gastos en meses en que todavía no existía.
pub fn actualizar(conn: &Connection, id: i64, datos: &NuevoServicio) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE servicios SET
            nombre = ?2, categoria_id = ?3, monto_estimado = ?4,
            dia_vencimiento = ?5, tipo = ?6, activo = ?7
         WHERE id = ?1",
        params![
            id,
            datos.nombre.trim(),
            datos.categoria_id,
            datos.monto_estimado,
            datos.dia_vencimiento,
            datos.tipo.como_texto(),
            datos.activo as i64,
        ],
    )?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("el servicio #{id}")));
    }
    Ok(())
}

/// Cuántos movimientos apuntan al servicio.
pub fn usos(conn: &Connection, id: i64) -> Resultado<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM movimientos WHERE servicio_id = ?1",
        params![id],
        |f| f.get(0),
    )?;
    Ok(n)
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute("DELETE FROM servicios WHERE id = ?1", params![id])?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("el servicio #{id}")));
    }
    Ok(())
}
