use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Resultado};
use crate::modelos::deuda::{Deuda, DireccionDeuda, EstadoDeuda, NuevaDeuda};

pub fn insertar(conn: &Connection, nueva: &NuevaDeuda) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO deudas
            (descripcion, tipo, institucion, monto_original, tasa_mensual,
             n_cuotas, fecha_primera_cuota, estado, notas, direccion, deudor)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'vigente', ?8, ?9, ?10)",
        params![
            nueva.descripcion.trim(),
            nueva.tipo.como_texto(),
            nueva.institucion.as_deref().map(str::trim),
            nueva.monto_original,
            nueva.tasa_mensual,
            nueva.n_cuotas,
            nueva.fecha_primera_cuota,
            nueva.notas.as_deref().map(str::trim),
            nueva.direccion.como_texto(),
            deudor_normalizado(nueva),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn actualizar(conn: &Connection, id: i64, datos: &NuevaDeuda) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE deudas SET
            descripcion = ?2, tipo = ?3, institucion = ?4, monto_original = ?5,
            tasa_mensual = ?6, n_cuotas = ?7, fecha_primera_cuota = ?8, notas = ?9,
            direccion = ?10, deudor = ?11
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
            datos.direccion.como_texto(),
            deudor_normalizado(datos),
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

/// El deudor solo tiene sentido si me deben a mí. En una deuda propia se
/// guarda como NULL para que no quede un nombre colgando si el usuario cambia
/// la dirección después de haberlo escrito.
fn deudor_normalizado(datos: &NuevaDeuda) -> Option<String> {
    match datos.direccion {
        DireccionDeuda::Tercero => datos
            .deudor
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(String::from),
        DireccionDeuda::Propia => None,
    }
}

/// Lista deudas. Cada filtro en `None` significa "todas".
///
/// El filtro de dirección es lo que separa "Mis deudas" de "Me deben": sin él
/// las dos listas mostrarían lo mismo.
pub fn listar(
    conn: &Connection,
    estado: Option<EstadoDeuda>,
    direccion: Option<DireccionDeuda>,
) -> Resultado<Vec<Deuda>> {
    // Los filtros opcionales se resuelven con `?N IS NULL OR ...` para no armar
    // SQL dinámico.
    let sql = format!(
        "SELECT {} FROM deudas
         WHERE (?1 IS NULL OR estado = ?1)
           AND (?2 IS NULL OR direccion = ?2)
         ORDER BY estado = 'pagada', fecha_primera_cuota DESC, id DESC",
        Deuda::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(
        params![
            estado.map(|e| e.como_texto()),
            direccion.map(|d| d.como_texto()),
        ],
        Deuda::desde_fila,
    )?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
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
