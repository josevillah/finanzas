use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, Resultado};
use crate::modelos::meta::{EstadoMeta, Meta, NuevaMeta};

/// Metas en orden de prioridad. `estado` en `None` trae todas.
///
/// El desempate por `id` importa: dos metas con la misma prioridad tienen que
/// repartirse el saldo siempre igual, o el avance bailaría entre consultas.
pub fn listar(conn: &Connection, estado: Option<EstadoMeta>) -> Resultado<Vec<Meta>> {
    let sql = format!(
        "SELECT {} FROM metas
         WHERE ?1 IS NULL OR estado = ?1
         ORDER BY prioridad, id",
        Meta::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(params![estado.map(|e| e.como_texto())], Meta::desde_fila)?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Meta> {
    let sql = format!("SELECT {} FROM metas WHERE id = ?1", Meta::COLUMNAS);
    conn.query_row(&sql, params![id], Meta::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la meta #{id}")))
}

/// Inserta al final de la lista: una meta nueva no se cuela delante de las que
/// ya estaban sin que el usuario lo pida.
pub fn insertar(conn: &Connection, datos: &NuevaMeta) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO metas
           (nombre, monto_objetivo, cuenta_id, prioridad, fecha_objetivo, estado, notas, creada_en)
         VALUES (?1, ?2, ?3, COALESCE((SELECT MAX(prioridad) + 1 FROM metas), 0), ?4, 'activa', ?5, ?6)",
        params![
            datos.nombre.trim(),
            datos.monto_objetivo,
            datos.cuenta_id,
            texto_opcional(datos.fecha_objetivo.as_deref()),
            texto_opcional(datos.notas.as_deref()),
            crate::dominio::fechas::a_iso(crate::dominio::fechas::hoy()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Edita los datos declarados. No toca prioridad ni estado: cada uno tiene su
/// propia operación, así una edición de nombre no reordena la lista sin querer.
pub fn actualizar(conn: &Connection, id: i64, datos: &NuevaMeta) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE metas
            SET nombre = ?2, monto_objetivo = ?3, cuenta_id = ?4,
                fecha_objetivo = ?5, notas = ?6
          WHERE id = ?1",
        params![
            id,
            datos.nombre.trim(),
            datos.monto_objetivo,
            datos.cuenta_id,
            texto_opcional(datos.fecha_objetivo.as_deref()),
            texto_opcional(datos.notas.as_deref()),
        ],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la meta #{id}")));
    }
    Ok(())
}

pub fn cambiar_estado(conn: &Connection, id: i64, estado: EstadoMeta) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE metas SET estado = ?2 WHERE id = ?1",
        params![id, estado.como_texto()],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la meta #{id}")));
    }
    Ok(())
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute("DELETE FROM metas WHERE id = ?1", params![id])?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la meta #{id}")));
    }
    Ok(())
}

/// Escribe la prioridad de una meta. Quien llama va en orden y dentro de una
/// transacción: la lista queda con 0..n o no cambia nada.
pub fn fijar_prioridad(conn: &Connection, id: i64, prioridad: i32) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE metas SET prioridad = ?2 WHERE id = ?1",
        params![id, prioridad],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la meta #{id}")));
    }
    Ok(())
}

/// Cuántas metas hay en cada estado: (activas, cumplidas, archivadas).
pub fn contar_por_estado(conn: &Connection) -> Resultado<(i32, i32, i32)> {
    let t = conn.query_row(
        "SELECT
            COALESCE(SUM(estado = 'activa'), 0),
            COALESCE(SUM(estado = 'cumplida'), 0),
            COALESCE(SUM(estado = 'archivada'), 0)
         FROM metas",
        [],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
    )?;
    Ok(t)
}

/// Un texto vacío es lo mismo que no haber escrito nada: se guarda NULL para
/// que la columna tenga un solo valor que signifique "sin dato".
fn texto_opcional(texto: Option<&str>) -> Option<String> {
    texto
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}
