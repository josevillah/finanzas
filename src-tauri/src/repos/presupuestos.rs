use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::dominio::dinero::Monto;
use crate::error::Resultado;
use crate::modelos::presupuesto::Presupuesto;

/// Todas las asignaciones de la base. Se usa para exportar.
pub fn listar_todos(conn: &Connection) -> Resultado<Vec<Presupuesto>> {
    let sql = format!(
        "SELECT {} FROM presupuestos ORDER BY periodo_id, categoria_id",
        Presupuesto::COLUMNAS
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Presupuesto::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Montos asignados del período, indexados por categoría.
pub fn por_categoria(conn: &Connection, periodo_id: i64) -> Resultado<HashMap<i64, Monto>> {
    let mut stmt = conn.prepare(
        "SELECT categoria_id, monto_asignado FROM presupuestos WHERE periodo_id = ?1",
    )?;

    let filas = stmt.query_map(params![periodo_id], |f| Ok((f.get(0)?, f.get(1)?)))?;
    Ok(filas.collect::<rusqlite::Result<HashMap<_, _>>>()?)
}

pub fn total_asignado(conn: &Connection, periodo_id: i64) -> Resultado<Monto> {
    let total: Monto = conn.query_row(
        "SELECT COALESCE(SUM(monto_asignado), 0) FROM presupuestos WHERE periodo_id = ?1",
        params![periodo_id],
        |f| f.get(0),
    )?;
    Ok(total)
}

/// Asigna (o reasigna) el monto de una categoría. Un monto de 0 borra la
/// línea: no tiene sentido guardar un presupuesto vacío.
pub fn asignar(
    conn: &Connection,
    periodo_id: i64,
    categoria_id: i64,
    monto: Monto,
) -> Resultado<()> {
    if monto <= 0 {
        return eliminar(conn, periodo_id, categoria_id);
    }

    conn.execute(
        "INSERT INTO presupuestos (periodo_id, categoria_id, monto_asignado)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(periodo_id, categoria_id)
         DO UPDATE SET monto_asignado = excluded.monto_asignado",
        params![periodo_id, categoria_id, monto],
    )?;
    Ok(())
}

pub fn eliminar(conn: &Connection, periodo_id: i64, categoria_id: i64) -> Resultado<()> {
    conn.execute(
        "DELETE FROM presupuestos WHERE periodo_id = ?1 AND categoria_id = ?2",
        params![periodo_id, categoria_id],
    )?;
    Ok(())
}

/// Copia las asignaciones de un período a otro. Las categorías que ya tenían
/// monto en el destino se sobrescriben. Devuelve cuántas líneas copió.
pub fn copiar(conn: &Connection, desde_periodo: i64, hacia_periodo: i64) -> Resultado<i32> {
    let copiadas = conn.execute(
        "INSERT INTO presupuestos (periodo_id, categoria_id, monto_asignado)
         SELECT ?2, categoria_id, monto_asignado
         FROM presupuestos
         WHERE periodo_id = ?1
         ON CONFLICT(periodo_id, categoria_id)
         DO UPDATE SET monto_asignado = excluded.monto_asignado",
        params![desde_periodo, hacia_periodo],
    )?;
    Ok(copiadas as i32)
}
