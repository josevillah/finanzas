use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Resultado};
use crate::modelos::deuda::{Deuda, EstadoDeuda, NuevaDeuda};

pub fn insertar(conn: &Connection, nueva: &NuevaDeuda) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO deudas
            (descripcion, tipo, institucion, monto_original, tasa_mensual,
             n_cuotas, fecha_primera_cuota, estado, notas)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'vigente', ?8)",
        params![
            nueva.descripcion.trim(),
            nueva.tipo.como_texto(),
            nueva.institucion.as_deref().map(str::trim),
            nueva.monto_original,
            nueva.tasa_mensual,
            nueva.n_cuotas,
            nueva.fecha_primera_cuota,
            nueva.notas.as_deref().map(str::trim),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn actualizar(conn: &Connection, id: i64, datos: &NuevaDeuda) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE deudas SET
            descripcion = ?2, tipo = ?3, institucion = ?4, monto_original = ?5,
            tasa_mensual = ?6, n_cuotas = ?7, fecha_primera_cuota = ?8, notas = ?9
         WHERE id = ?1",
        params![
            id,
            datos.descripcion.trim(),
            datos.tipo.como_texto(),
            datos.institucion.as_deref().map(str::trim),
            datos.monto_original,
            datos.tasa_mensual,
            datos.n_cuotas,
            datos.fecha_primera_cuota,
            datos.notas.as_deref().map(str::trim),
        ],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la deuda #{id}")));
    }
    Ok(())
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Deuda> {
    let sql = format!("SELECT {} FROM deudas WHERE id = ?1", Deuda::COLUMNAS);
    conn.query_row(&sql, params![id], Deuda::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la deuda #{id}")))
}

/// Lista deudas. Si `estado` es `None` devuelve todas.
pub fn listar(conn: &Connection, estado: Option<EstadoDeuda>) -> Resultado<Vec<Deuda>> {
    let orden = "ORDER BY estado = 'pagada', fecha_primera_cuota DESC, id DESC";

    let deudas = match estado {
        Some(e) => {
            let sql = format!(
                "SELECT {} FROM deudas WHERE estado = ?1 {orden}",
                Deuda::COLUMNAS
            );
            let mut stmt = conn.prepare(&sql)?;
            let filas = stmt.query_map(params![e.como_texto()], Deuda::desde_fila)?;
            filas.collect::<rusqlite::Result<Vec<_>>>()?
        }
        None => {
            let sql = format!("SELECT {} FROM deudas {orden}", Deuda::COLUMNAS);
            let mut stmt = conn.prepare(&sql)?;
            let filas = stmt.query_map([], Deuda::desde_fila)?;
            filas.collect::<rusqlite::Result<Vec<_>>>()?
        }
    };

    Ok(deudas)
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    // Las cuotas se van solas por el ON DELETE CASCADE.
    let filas = conn.execute("DELETE FROM deudas WHERE id = ?1", params![id])?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la deuda #{id}")));
    }
    Ok(())
}

pub fn cambiar_estado(conn: &Connection, id: i64, estado: EstadoDeuda) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE deudas SET estado = ?2 WHERE id = ?1",
        params![id, estado.como_texto()],
    )?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la deuda #{id}")));
    }
    Ok(())
}

/// Marca la deuda como pagada si no le queda ninguna cuota pendiente, y la
/// devuelve a vigente si vuelve a quedar alguna (por ejemplo al deshacer un
/// pago). No toca las deudas repactadas.
pub fn sincronizar_estado(conn: &Connection, id: i64) -> Resultado<()> {
    let pendientes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cuotas WHERE deuda_id = ?1 AND estado <> 'pagada'",
        params![id],
        |f| f.get(0),
    )?;

    conn.execute(
        "UPDATE deudas SET estado = ?2
         WHERE id = ?1 AND estado <> 'repactada'",
        params![
            id,
            if pendientes == 0 {
                EstadoDeuda::Pagada.como_texto()
            } else {
                EstadoDeuda::Vigente.como_texto()
            }
        ],
    )?;

    Ok(())
}
