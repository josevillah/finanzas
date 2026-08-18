use rusqlite::{params, Connection, OptionalExtension};

use crate::dominio::amortizacion::CuotaCalculada;
use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::cuota::Cuota;

/// Agregados de una deuda, calculados en SQL.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResumenCuotas {
    pub total_programado: Monto,
    pub monto_pagado: Monto,
    pub monto_pendiente: Monto,
    pub cuotas_pagadas: i32,
    pub cuotas_totales: i32,
    pub cuotas_atrasadas: i32,
}

/// Materializa las cuotas de una deuda. Se llama dentro de la transacción que
/// crea la deuda: las cuotas son filas reales, nunca cálculo al vuelo.
pub fn insertar_muchas(
    conn: &Connection,
    deuda_id: i64,
    cuotas: &[CuotaCalculada],
) -> Resultado<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO cuotas
            (deuda_id, numero, fecha_vencimiento, monto, capital, interes, estado)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pendiente')",
    )?;

    for c in cuotas {
        stmt.execute(params![
            deuda_id,
            c.numero,
            c.fecha_vencimiento,
            c.monto,
            c.capital,
            c.interes,
        ])?;
    }

    Ok(())
}

pub fn eliminar_por_deuda(conn: &Connection, deuda_id: i64) -> Resultado<()> {
    conn.execute("DELETE FROM cuotas WHERE deuda_id = ?1", params![deuda_id])?;
    Ok(())
}

/// Todas las cuotas de la base. Se usa para exportar.
pub fn listar_todas(conn: &Connection) -> Resultado<Vec<Cuota>> {
    let sql = format!(
        "SELECT {} FROM cuotas ORDER BY deuda_id, numero",
        Cuota::COLUMNAS
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Cuota::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn listar_por_deuda(conn: &Connection, deuda_id: i64) -> Resultado<Vec<Cuota>> {
    let sql = format!(
        "SELECT {} FROM cuotas WHERE deuda_id = ?1 ORDER BY numero",
        Cuota::COLUMNAS
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(params![deuda_id], Cuota::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Cuota> {
    let sql = format!("SELECT {} FROM cuotas WHERE id = ?1", Cuota::COLUMNAS);
    conn.query_row(&sql, params![id], Cuota::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la cuota #{id}")))
}

/// Primera cuota no pagada de la deuda.
pub fn proxima_pendiente(conn: &Connection, deuda_id: i64) -> Resultado<Option<Cuota>> {
    let sql = format!(
        "SELECT {} FROM cuotas
         WHERE deuda_id = ?1 AND estado <> 'pagada'
         ORDER BY fecha_vencimiento, numero
         LIMIT 1",
        Cuota::COLUMNAS
    );
    Ok(conn
        .query_row(&sql, params![deuda_id], Cuota::desde_fila)
        .optional()?)
}

pub fn resumen(conn: &Connection, deuda_id: i64) -> Resultado<ResumenCuotas> {
    let resumen = conn.query_row(
        "SELECT
            COALESCE(SUM(monto), 0),
            COALESCE(SUM(CASE WHEN estado = 'pagada'
                              THEN COALESCE(monto_pagado, monto) ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN estado <> 'pagada' THEN monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN estado = 'pagada' THEN 1 ELSE 0 END), 0),
            COUNT(*),
            COALESCE(SUM(CASE WHEN estado = 'atrasada' THEN 1 ELSE 0 END), 0)
         FROM cuotas WHERE deuda_id = ?1",
        params![deuda_id],
        |f| {
            Ok(ResumenCuotas {
                total_programado: f.get(0)?,
                monto_pagado: f.get(1)?,
                monto_pendiente: f.get(2)?,
                cuotas_pagadas: f.get(3)?,
                cuotas_totales: f.get(4)?,
                cuotas_atrasadas: f.get(5)?,
            })
        },
    )?;
    Ok(resumen)
}

/// ¿Ya se pagó alguna cuota de esta deuda? Bloquea la edición que obligaría a
/// regenerar la tabla de amortización.
pub fn tiene_pagos(conn: &Connection, deuda_id: i64) -> Resultado<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cuotas WHERE deuda_id = ?1 AND estado = 'pagada'",
        params![deuda_id],
        |f| f.get(0),
    )?;
    Ok(n > 0)
}

pub fn registrar_pago(
    conn: &Connection,
    id: i64,
    fecha_pago: &str,
    monto_pagado: Monto,
) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE cuotas
         SET estado = 'pagada', fecha_pago = ?2, monto_pagado = ?3
         WHERE id = ?1",
        params![id, fecha_pago, monto_pagado],
    )?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la cuota #{id}")));
    }
    Ok(())
}

pub fn deshacer_pago(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE cuotas
         SET estado = 'pendiente', fecha_pago = NULL, monto_pagado = NULL
         WHERE id = ?1",
        params![id],
    )?;
    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la cuota #{id}")));
    }
    Ok(())
}

/// Sincroniza el estado 'atrasada' contra la fecha de hoy. Se corre al iniciar
/// la app y después de cada pago. Es idempotente.
pub fn marcar_atrasadas(conn: &Connection, hoy_iso: &str) -> Resultado<usize> {
    let atrasadas = conn.execute(
        "UPDATE cuotas SET estado = 'atrasada'
         WHERE estado = 'pendiente' AND fecha_vencimiento < ?1",
        params![hoy_iso],
    )?;

    // Vuelta atrás por si se editó la fecha de vencimiento hacia el futuro.
    conn.execute(
        "UPDATE cuotas SET estado = 'pendiente'
         WHERE estado = 'atrasada' AND fecha_vencimiento >= ?1",
        params![hoy_iso],
    )?;

    Ok(atrasadas)
}

/// Total comprometido por mes en un rango de fechas ISO, solo de deudas
/// vigentes. Devuelve (clave 'YYYY-MM', total, pendiente, n_cuotas).
pub fn carga_por_mes(
    conn: &Connection,
    desde_iso: &str,
    hasta_iso: &str,
) -> Resultado<Vec<(String, Monto, Monto, i32)>> {
    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', c.fecha_vencimiento) AS clave,
                COALESCE(SUM(c.monto), 0),
                COALESCE(SUM(CASE WHEN c.estado <> 'pagada' THEN c.monto ELSE 0 END), 0),
                COUNT(*)
         FROM cuotas c
         JOIN deudas d ON d.id = c.deuda_id
         WHERE d.estado = 'vigente'
           AND d.direccion = 'propia'
           AND c.fecha_vencimiento >= ?1
           AND c.fecha_vencimiento <= ?2
         GROUP BY clave
         ORDER BY clave",
    )?;

    let filas = stmt.query_map(params![desde_iso, hasta_iso], |f| {
        Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?))
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Total de cuotas comprometidas dentro de un rango de fechas.
pub fn total_en_rango(conn: &Connection, desde_iso: &str, hasta_iso: &str) -> Resultado<(Monto, i32)> {
    let par = conn.query_row(
        "SELECT COALESCE(SUM(c.monto), 0), COUNT(*)
         FROM cuotas c
         JOIN deudas d ON d.id = c.deuda_id
         WHERE d.estado = 'vigente'
           AND d.direccion = 'propia'
           AND c.fecha_vencimiento >= ?1
           AND c.fecha_vencimiento <= ?2",
        params![desde_iso, hasta_iso],
        |f| Ok((f.get(0)?, f.get(1)?)),
    )?;
    Ok(par)
}

/// Columnas de `cuotas` calificadas con el alias `c`, para los JOIN.
/// Mantiene el mismo orden que espera [`Cuota::desde_fila`].
fn columnas_alias_c() -> String {
    Cuota::COLUMNAS
        .split(", ")
        .map(|col| format!("c.{col}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Todas las cuotas no pagadas de deudas vigentes, con la descripción de su
/// deuda. Se usa para armar la fecha de libertad financiera.
pub fn pendientes_con_deuda(conn: &Connection) -> Resultado<Vec<(Cuota, String)>> {
    let sql = format!(
        "SELECT {}, d.descripcion
         FROM cuotas c
         JOIN deudas d ON d.id = c.deuda_id
         WHERE d.estado = 'vigente'
           AND d.direccion = 'propia' AND c.estado <> 'pagada'
         ORDER BY c.fecha_vencimiento, c.numero",
        columnas_alias_c()
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], |f| Ok((Cuota::desde_fila(f)?, f.get::<_, String>(10)?)))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Cuotas de deudas vigentes que vencen dentro del rango, pagadas incluidas.
/// El listado del mes debe cuadrar con el total de [`total_en_rango`], que
/// también cuenta lo ya pagado: ese total es el compromiso del mes, no el saldo.
pub fn en_rango_con_deuda(
    conn: &Connection,
    desde_iso: &str,
    hasta_iso: &str,
) -> Resultado<Vec<(Cuota, String)>> {
    let sql = format!(
        "SELECT {}, d.descripcion
         FROM cuotas c
         JOIN deudas d ON d.id = c.deuda_id
         WHERE d.estado = 'vigente'
           AND d.direccion = 'propia'
           AND c.fecha_vencimiento >= ?1
           AND c.fecha_vencimiento <= ?2
         ORDER BY c.fecha_vencimiento, c.numero",
        columnas_alias_c()
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(params![desde_iso, hasta_iso], |f| {
        Ok((Cuota::desde_fila(f)?, f.get::<_, String>(10)?))
    })?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Meses que tienen cuotas venciendo, de deudas vigentes.
/// Devuelve (anio, mes, n_cuotas).
///
/// Un mes puede tener cuotas sin que exista su período: las cuotas cuelgan de
/// la deuda, no del período.
pub fn meses_con_vencimientos(conn: &Connection) -> Resultado<Vec<(i32, u32, i32)>> {
    let mut stmt = conn.prepare(
        "SELECT CAST(strftime('%Y', c.fecha_vencimiento) AS INTEGER),
                CAST(strftime('%m', c.fecha_vencimiento) AS INTEGER),
                COUNT(*)
         FROM cuotas c
         JOIN deudas d ON d.id = c.deuda_id
         WHERE d.estado = 'vigente'
         GROUP BY 1, 2
         ORDER BY 1, 2",
    )?;

    let filas = stmt.query_map([], |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}
