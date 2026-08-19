use rusqlite::{params, Connection, OptionalExtension};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::nota_ahorro::{NotaAhorro, NuevaNota};

/// Todas las notas de todas las cuentas, agrupables por `cuenta_id`.
///
/// Una sola consulta en vez de una por cuenta: la usan tanto el resumen de
/// cuentas como la exportación.
pub fn listar_todas(conn: &Connection) -> Resultado<Vec<NotaAhorro>> {
    let sql = format!(
        "SELECT {} FROM notas_ahorro ORDER BY cuenta_id, orden, id",
        NotaAhorro::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], NotaAhorro::desde_fila)?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<NotaAhorro> {
    let sql = format!(
        "SELECT {} FROM notas_ahorro WHERE id = ?1",
        NotaAhorro::COLUMNAS
    );

    conn.query_row(&sql, params![id], NotaAhorro::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la nota #{id}")))
}

/// Cuánto suman las notas de una cuenta. Es el número contra el que se valida
/// cualquier cambio.
pub fn suma_de_cuenta(conn: &Connection, cuenta_id: i64) -> Resultado<Monto> {
    let total: Monto = conn.query_row(
        "SELECT COALESCE(SUM(monto), 0) FROM notas_ahorro WHERE cuenta_id = ?1",
        params![cuenta_id],
        |f| f.get(0),
    )?;
    Ok(total)
}

/// Una nota nueva va al final de su cuenta.
pub fn insertar(conn: &Connection, datos: &NuevaNota) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO notas_ahorro (cuenta_id, nombre, monto, orden)
         VALUES (?1, ?2, ?3,
                 COALESCE((SELECT MAX(orden) + 1 FROM notas_ahorro WHERE cuenta_id = ?1), 0))",
        params![datos.cuenta_id, datos.nombre.trim(), datos.monto],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn actualizar(conn: &Connection, id: i64, nombre: &str, monto: Monto) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE notas_ahorro SET nombre = ?2, monto = ?3 WHERE id = ?1",
        params![id, nombre.trim(), monto],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la nota #{id}")));
    }
    Ok(())
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute("DELETE FROM notas_ahorro WHERE id = ?1", params![id])?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la nota #{id}")));
    }
    Ok(())
}
