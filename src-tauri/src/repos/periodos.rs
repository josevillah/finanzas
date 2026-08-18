use rusqlite::{params, Connection, OptionalExtension};

use crate::dominio::dinero::Monto;
use crate::error::Resultado;
use crate::modelos::periodo::Periodo;

/// Devuelve el período del mes indicado, creándolo vacío si no existe.
/// En Fase 1 esto permite tener dónde guardar el sueldo líquido sin construir
/// todavía el módulo completo de períodos.
pub fn obtener_o_crear(conn: &Connection, anio: i32, mes: u32) -> Resultado<Periodo> {
    if let Some(p) = obtener(conn, anio, mes)? {
        return Ok(p);
    }

    conn.execute(
        "INSERT INTO periodos (anio, mes, sueldo_liquido, otros_ingresos, estado)
         VALUES (?1, ?2, 0, 0, 'abierto')
         ON CONFLICT(anio, mes) DO NOTHING",
        params![anio, mes],
    )?;

    obtener(conn, anio, mes)?.ok_or_else(|| {
        crate::error::AppError::no_encontrado(format!("el período {mes:02}/{anio}"))
    })
}

pub fn obtener(conn: &Connection, anio: i32, mes: u32) -> Resultado<Option<Periodo>> {
    let sql = format!(
        "SELECT {} FROM periodos WHERE anio = ?1 AND mes = ?2",
        Periodo::COLUMNAS
    );
    Ok(conn
        .query_row(&sql, params![anio, mes], Periodo::desde_fila)
        .optional()?)
}

/// Períodos existentes, del más reciente al más antiguo.
pub fn listar(conn: &Connection) -> Resultado<Vec<Periodo>> {
    let sql = format!(
        "SELECT {} FROM periodos ORDER BY anio DESC, mes DESC",
        Periodo::COLUMNAS
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Periodo::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Cambia entre 'abierto' y 'cerrado'. Un período cerrado no acepta cambios
/// en sus movimientos.
pub fn cambiar_estado(conn: &Connection, anio: i32, mes: u32, estado: &str) -> Resultado<()> {
    if estado != "abierto" && estado != "cerrado" {
        return Err(crate::error::AppError::validacion(format!(
            "Estado de período desconocido: '{estado}'"
        )));
    }

    let filas = conn.execute(
        "UPDATE periodos SET estado = ?3 WHERE anio = ?1 AND mes = ?2",
        params![anio, mes, estado],
    )?;
    if filas == 0 {
        return Err(crate::error::AppError::no_encontrado(format!(
            "el período {mes:02}/{anio}"
        )));
    }
    Ok(())
}

/// Falla si el período está cerrado. Se llama antes de tocar sus movimientos.
pub fn exigir_abierto(conn: &Connection, periodo_id: i64) -> Resultado<()> {
    let estado: String = conn.query_row(
        "SELECT estado FROM periodos WHERE id = ?1",
        params![periodo_id],
        |f| f.get(0),
    )?;

    if estado == "cerrado" {
        return Err(crate::error::AppError::conflicto(
            "Este mes está cerrado. Ábrelo de nuevo si necesitas modificarlo.",
        ));
    }
    Ok(())
}

pub fn actualizar_ingresos(
    conn: &Connection,
    anio: i32,
    mes: u32,
    sueldo_liquido: Monto,
    otros_ingresos: Monto,
) -> Resultado<()> {
    obtener_o_crear(conn, anio, mes)?;
    conn.execute(
        "UPDATE periodos SET sueldo_liquido = ?3, otros_ingresos = ?4
         WHERE anio = ?1 AND mes = ?2",
        params![anio, mes, sueldo_liquido, otros_ingresos],
    )?;
    Ok(())
}

/// Por cada período existente, cuánto contenido real tiene.
/// Devuelve (anio, mes, n_movimientos, n_presupuestos, tiene_ingresos).
///
/// Hace falta contar de verdad: `obtener_o_crear` corre desde comandos de solo
/// lectura, así que con navegar a un mes ya queda su fila creada. La existencia
/// de la fila no dice nada.
pub fn resumen_de_periodos(conn: &Connection) -> Resultado<Vec<(i32, u32, i32, i32, bool)>> {
    let mut stmt = conn.prepare(
        "SELECT p.anio,
                p.mes,
                (SELECT COUNT(*) FROM movimientos m WHERE m.periodo_id = p.id),
                (SELECT COUNT(*) FROM presupuestos b WHERE b.periodo_id = p.id),
                (p.sueldo_liquido > 0 OR p.otros_ingresos > 0)
         FROM periodos p
         ORDER BY p.anio, p.mes",
    )?;

    let filas = stmt.query_map([], |f| {
        Ok((
            f.get(0)?,
            f.get(1)?,
            f.get(2)?,
            f.get(3)?,
            f.get::<_, i64>(4)? != 0,
        ))
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Suma de sueldos y otros ingresos declarados en todos los períodos.
///
/// El sueldo vive en `periodos`, no en `movimientos`: sin esto el patrimonio
/// restaría todos los gastos sumando casi ningún ingreso. Es la misma
/// definición de ingreso que usa el resumen del mes.
pub fn total_ingresos_declarados(conn: &Connection) -> Resultado<Monto> {
    let total: Monto = conn.query_row(
        "SELECT COALESCE(SUM(sueldo_liquido + otros_ingresos), 0) FROM periodos",
        [],
        |f| f.get(0),
    )?;
    Ok(total)
}
