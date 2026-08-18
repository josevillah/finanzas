use rusqlite::{params, Connection, OptionalExtension};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::movimiento::{
    FiltroMovimientos, Movimiento, MovimientoDetalle, NuevoMovimiento, TipoMovimiento,
};
use crate::modelos::periodo::GastoPorCategoria;

/// Columnas de `movimientos` con alias `m`, en el orden de [`Movimiento::desde_fila`].
fn columnas_alias_m() -> String {
    Movimiento::COLUMNAS
        .split(", ")
        .map(|col| format!("m.{col}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn insertar(conn: &Connection, periodo_id: i64, datos: &NuevoMovimiento) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO movimientos
            (periodo_id, fecha, monto, tipo, categoria_id, servicio_id, medio_pago, descripcion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            periodo_id,
            datos.fecha,
            datos.monto,
            datos.tipo.como_texto(),
            datos.categoria_id,
            datos.servicio_id,
            datos.medio_pago.map(|m| m.como_texto()),
            datos.descripcion.as_deref().map(str::trim),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn actualizar(
    conn: &Connection,
    id: i64,
    periodo_id: i64,
    datos: &NuevoMovimiento,
) -> Resultado<()> {
    let filas = conn.execute(
        // Editarlo a mano lo confirma: deja de ser el estimado del sistema.
        "UPDATE movimientos SET
            periodo_id = ?2, fecha = ?3, monto = ?4, tipo = ?5,
            categoria_id = ?6, servicio_id = ?7, medio_pago = ?8, descripcion = ?9,
            es_estimado = 0
         WHERE id = ?1 AND cuota_id IS NULL",
        params![
            id,
            periodo_id,
            datos.fecha,
            datos.monto,
            datos.tipo.como_texto(),
            datos.categoria_id,
            datos.servicio_id,
            datos.medio_pago.map(|m| m.como_texto()),
            datos.descripcion.as_deref().map(str::trim),
        ],
    )?;

    if filas == 0 {
        return Err(AppError::conflicto(
            "No se pudo editar el movimiento: no existe, o es el pago de una cuota \
             (esos se modifican desde la deuda).",
        ));
    }
    Ok(())
}

/// Todos los movimientos de la base. Se usa para exportar.
pub fn listar_todos(conn: &Connection) -> Resultado<Vec<Movimiento>> {
    let sql = format!(
        "SELECT {} FROM movimientos ORDER BY fecha, id",
        Movimiento::COLUMNAS
    );
    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], Movimiento::desde_fila)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Movimiento> {
    let sql = format!(
        "SELECT {} FROM movimientos WHERE id = ?1",
        Movimiento::COLUMNAS
    );
    conn.query_row(&sql, params![id], Movimiento::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("el movimiento #{id}")))
}

/// Cambia solo el monto y da el movimiento por confirmado. Es el camino del
/// botón "Cambiar precio": la boleta llegó con otra cifra.
pub fn cambiar_monto(conn: &Connection, id: i64, monto: Monto) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE movimientos SET monto = ?2, es_estimado = 0
         WHERE id = ?1 AND cuota_id IS NULL",
        params![id, monto],
    )?;

    if filas == 0 {
        return Err(AppError::conflicto(
            "No se pudo cambiar el precio: el movimiento no existe, o es el pago de \
             una cuota (ese monto se ajusta desde la deuda).",
        ));
    }
    Ok(())
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute(
        "DELETE FROM movimientos WHERE id = ?1 AND cuota_id IS NULL",
        params![id],
    )?;
    if filas == 0 {
        return Err(AppError::conflicto(
            "No se pudo eliminar el movimiento: no existe, o es el pago de una cuota \
             (deshaz el pago desde la deuda).",
        ));
    }
    Ok(())
}

// ── enlace con cuotas ────────────────────────────────────────────────────────

/// Registra el movimiento que corresponde al pago o cobro de una cuota.
///
/// El tipo lo decide la dirección de la deuda: pagar una propia es un gasto,
/// cobrar una de un tercero es un ingreso. El índice único sobre `cuota_id`
/// impide duplicarlo en cualquiera de los dos casos.
pub fn insertar_pago_cuota(
    conn: &Connection,
    periodo_id: i64,
    cuota_id: i64,
    categoria_id: Option<i64>,
    fecha: &str,
    monto: Monto,
    descripcion: &str,
    tipo: TipoMovimiento,
) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO movimientos
            (periodo_id, fecha, monto, tipo, categoria_id, cuota_id, descripcion)
         VALUES (?1, ?2, ?3, ?7, ?4, ?5, ?6)",
        params![
            periodo_id,
            fecha,
            monto,
            categoria_id,
            cuota_id,
            descripcion,
            tipo.como_texto()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn eliminar_por_cuota(conn: &Connection, cuota_id: i64) -> Resultado<()> {
    conn.execute(
        "DELETE FROM movimientos WHERE cuota_id = ?1",
        params![cuota_id],
    )?;
    Ok(())
}

// ── lecturas ─────────────────────────────────────────────────────────────────

pub fn listar_detalle(
    conn: &Connection,
    periodo_id: i64,
    filtro: &FiltroMovimientos,
) -> Resultado<Vec<MovimientoDetalle>> {
    // Los filtros opcionales se resuelven con `?N IS NULL OR ...` para no armar
    // SQL dinámico ni concatenar entrada del usuario.
    let sql = format!(
        "SELECT {}, cat.nombre, cat.color, cat.tipo, s.nombre, d.descripcion
         FROM movimientos m
         LEFT JOIN categorias cat ON cat.id = m.categoria_id
         LEFT JOIN servicios s ON s.id = m.servicio_id
         LEFT JOIN cuotas c ON c.id = m.cuota_id
         LEFT JOIN deudas d ON d.id = c.deuda_id
         WHERE m.periodo_id = ?1
           AND (?2 IS NULL OR m.tipo = ?2)
           AND (?3 IS NULL OR m.categoria_id = ?3)
           AND (?4 IS NULL OR m.descripcion LIKE '%' || ?4 || '%')
         ORDER BY m.fecha DESC, m.id DESC",
        columnas_alias_m()
    );

    let busqueda = filtro
        .busqueda
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(
        params![
            periodo_id,
            filtro.tipo.map(|t| t.como_texto()),
            filtro.categoria_id,
            busqueda,
        ],
        |f| {
            Ok(MovimientoDetalle {
                movimiento: Movimiento::desde_fila(f)?,
                categoria_nombre: f.get(11)?,
                categoria_color: f.get(12)?,
                categoria_tipo: f.get(13)?,
                servicio_nombre: f.get(14)?,
                deuda_descripcion: f.get(15)?,
            })
        },
    )?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Totales del período: (gastos, ingresos_extra, cuotas, hormiga, n_movimientos).
pub fn totales(conn: &Connection, periodo_id: i64) -> Resultado<(Monto, Monto, Monto, Monto, i32)> {
    let t = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN m.tipo = 'gasto' THEN m.monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN m.tipo = 'ingreso' THEN m.monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN m.tipo = 'gasto' AND m.cuota_id IS NOT NULL
                              THEN m.monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN m.tipo = 'gasto' AND cat.tipo = 'hormiga'
                              THEN m.monto ELSE 0 END), 0),
            COUNT(*)
         FROM movimientos m
         LEFT JOIN categorias cat ON cat.id = m.categoria_id
         WHERE m.periodo_id = ?1",
        params![periodo_id],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?, f.get(4)?)),
    )?;
    Ok(t)
}

/// Gasto por categoría dentro del período, de mayor a menor.
pub fn por_categoria(conn: &Connection, periodo_id: i64) -> Resultado<Vec<GastoPorCategoria>> {
    let mut stmt = conn.prepare(
        "SELECT m.categoria_id,
                COALESCE(cat.nombre, 'Sin categoría'),
                cat.tipo,
                cat.color,
                COALESCE(SUM(m.monto), 0),
                COUNT(*)
         FROM movimientos m
         LEFT JOIN categorias cat ON cat.id = m.categoria_id
         WHERE m.periodo_id = ?1 AND m.tipo = 'gasto'
         GROUP BY m.categoria_id
         ORDER BY SUM(m.monto) DESC",
    )?;

    let filas = stmt.query_map(params![periodo_id], |f| {
        Ok(GastoPorCategoria {
            categoria_id: f.get(0)?,
            categoria_nombre: f.get(1)?,
            categoria_tipo: f.get(2)?,
            color: f.get(3)?,
            total: f.get(4)?,
            n_movimientos: f.get(5)?,
        })
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Gasto por servicio dentro del período: (servicio_id, total, n, n_estimados).
pub fn real_por_servicio(
    conn: &Connection,
    periodo_id: i64,
) -> Resultado<Vec<(i64, Monto, i32, i32)>> {
    let mut stmt = conn.prepare(
        "SELECT servicio_id,
                COALESCE(SUM(monto), 0),
                COUNT(*),
                COALESCE(SUM(es_estimado), 0)
         FROM movimientos
         WHERE periodo_id = ?1 AND tipo = 'gasto' AND servicio_id IS NOT NULL
         GROUP BY servicio_id",
    )?;

    let filas = stmt.query_map(params![periodo_id], |f| {
        Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?))
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Gasto por mes y categoría dentro de un rango de meses absolutos
/// (`anio * 12 + mes`). Devuelve (anio, mes, categoria_id, nombre, color, total).
pub fn evolucion_por_categoria(
    conn: &Connection,
    desde_abs: i64,
    hasta_abs: i64,
) -> Resultado<Vec<(i32, u32, Option<i64>, String, Option<String>, Monto)>> {
    let mut stmt = conn.prepare(
        "SELECT p.anio,
                p.mes,
                m.categoria_id,
                COALESCE(cat.nombre, 'Sin categoría'),
                cat.color,
                COALESCE(SUM(m.monto), 0)
         FROM movimientos m
         JOIN periodos p ON p.id = m.periodo_id
         LEFT JOIN categorias cat ON cat.id = m.categoria_id
         WHERE m.tipo = 'gasto'
           AND (p.anio * 12 + p.mes) BETWEEN ?1 AND ?2
         GROUP BY p.anio, p.mes, m.categoria_id
         ORDER BY p.anio, p.mes",
    )?;

    let filas = stmt.query_map(params![desde_abs, hasta_abs], |f| {
        Ok((
            f.get(0)?,
            f.get(1)?,
            f.get(2)?,
            f.get(3)?,
            f.get(4)?,
            f.get(5)?,
        ))
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Gasto hormiga mes a mes dentro de un rango de meses absolutos.
/// Devuelve (anio, mes, total_hormiga, n_movimientos_hormiga, total_gastos).
pub fn hormiga_por_periodo(
    conn: &Connection,
    desde_abs: i64,
    hasta_abs: i64,
) -> Resultado<Vec<(i32, u32, Monto, i32, Monto)>> {
    let mut stmt = conn.prepare(
        "SELECT p.anio,
                p.mes,
                COALESCE(SUM(CASE WHEN cat.tipo = 'hormiga' THEN m.monto ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN cat.tipo = 'hormiga' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(m.monto), 0)
         FROM periodos p
         LEFT JOIN movimientos m ON m.periodo_id = p.id AND m.tipo = 'gasto'
         LEFT JOIN categorias cat ON cat.id = m.categoria_id
         WHERE (p.anio * 12 + p.mes) BETWEEN ?1 AND ?2
         GROUP BY p.id
         ORDER BY p.anio, p.mes",
    )?;

    let filas = stmt.query_map(params![desde_abs, hasta_abs], |f| {
        Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?, f.get(4)?))
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Gasto de categorías tipo hormiga en un período, desglosado por categoría.
pub fn hormiga_por_categoria(
    conn: &Connection,
    periodo_id: i64,
) -> Resultado<Vec<GastoPorCategoria>> {
    let mut stmt = conn.prepare(
        "SELECT m.categoria_id,
                cat.nombre,
                cat.tipo,
                cat.color,
                COALESCE(SUM(m.monto), 0),
                COUNT(*)
         FROM movimientos m
         JOIN categorias cat ON cat.id = m.categoria_id
         WHERE m.periodo_id = ?1 AND m.tipo = 'gasto' AND cat.tipo = 'hormiga'
         GROUP BY m.categoria_id
         ORDER BY SUM(m.monto) DESC",
    )?;

    let filas = stmt.query_map(params![periodo_id], |f| {
        Ok(GastoPorCategoria {
            categoria_id: f.get(0)?,
            categoria_nombre: f.get(1)?,
            categoria_tipo: f.get(2)?,
            color: f.get(3)?,
            total: f.get(4)?,
            n_movimientos: f.get(5)?,
        })
    })?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Mueve a otra categoría los gastos que un servicio generó en un período.
/// Devuelve cuántos movió.
pub fn reclasificar_por_servicio(
    conn: &Connection,
    servicio_id: i64,
    periodo_id: i64,
    categoria_id: Option<i64>,
) -> Resultado<usize> {
    Ok(conn.execute(
        "UPDATE movimientos SET categoria_id = ?3
         WHERE servicio_id = ?1 AND periodo_id = ?2",
        params![servicio_id, periodo_id, categoria_id],
    )?)
}

/// Pone al día el gasto estimado de un servicio en un período cuando cambia su
/// monto de referencia.
///
/// Solo toca los que siguen marcados como estimados: si el usuario ya confirmó
/// el monto real con "Cambiar precio", ese dato es suyo y no se pisa.
pub fn actualizar_estimado_de_servicio(
    conn: &Connection,
    servicio_id: i64,
    periodo_id: i64,
    monto: Monto,
) -> Resultado<usize> {
    Ok(conn.execute(
        "UPDATE movimientos SET monto = ?3
         WHERE servicio_id = ?1 AND periodo_id = ?2 AND es_estimado = 1",
        params![servicio_id, periodo_id, monto],
    )?)
}

/// Ids de servicios que ya tienen algún gasto en el período. Se usa para no
/// generar dos veces el gasto estimado.
pub fn servicios_con_gasto(conn: &Connection, periodo_id: i64) -> Resultado<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT servicio_id
         FROM movimientos
         WHERE periodo_id = ?1 AND servicio_id IS NOT NULL",
    )?;

    let filas = stmt.query_map(params![periodo_id], |f| f.get(0))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Crea el gasto del mes de un servicio con su monto estimado, marcado como
/// pendiente de confirmar.
pub fn insertar_estimado_servicio(
    conn: &Connection,
    periodo_id: i64,
    servicio_id: i64,
    categoria_id: Option<i64>,
    fecha: &str,
    monto: Monto,
    descripcion: &str,
) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO movimientos
            (periodo_id, fecha, monto, tipo, categoria_id, servicio_id, descripcion, es_estimado)
         VALUES (?1, ?2, ?3, 'gasto', ?4, ?5, ?6, 1)",
        params![periodo_id, fecha, monto, categoria_id, servicio_id, descripcion],
    )?;
    Ok(conn.last_insert_rowid())
}


/// ¿El servicio ya tiene algún gasto en ese período?
/// Evita que una activación manual repetida duplique el movimiento.
pub fn tiene_movimientos_de_servicio(
    conn: &Connection,
    servicio_id: i64,
    periodo_id: i64,
) -> Resultado<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM movimientos WHERE servicio_id = ?1 AND periodo_id = ?2",
        params![servicio_id, periodo_id],
        |f| f.get(0),
    )?;
    Ok(n > 0)
}

/// Registra a mano el gasto de un servicio en un mes en que su alta no lo
/// cubre. Nace confirmado (`es_estimado = 0`) porque el monto lo escribió el
/// usuario, no lo proyectó el sistema.
pub fn insertar_activacion_manual(
    conn: &Connection,
    periodo_id: i64,
    servicio_id: i64,
    categoria_id: Option<i64>,
    fecha: &str,
    monto: Monto,
    descripcion: &str,
) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO movimientos
            (periodo_id, fecha, monto, tipo, categoria_id, servicio_id, descripcion, es_estimado)
         VALUES (?1, ?2, ?3, 'gasto', ?4, ?5, ?6, 0)",
        params![periodo_id, fecha, monto, categoria_id, servicio_id, descripcion],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Ingresos, gastos y gastos aún estimados de **toda** la tabla.
///
/// Sin filtro de período ni de estado del mes: el patrimonio es acumulativo
/// desde siempre, y un mes cerrado no devuelve la plata que salió.
///
/// Los estimados se suman a los gastos y además se devuelven aparte. Contarlos
/// deja el disponible algo bajo mientras un servicio del mes en curso no
/// vence, pero el error es acotado y se corrige solo al confirmar el monto;
/// excluirlos, en cambio, abriría una brecha creciente con cada mes viejo que
/// quedó sin confirmar. El tercer valor existe para poder explicarlo en
/// pantalla en vez de que el número se vea mal sin razón aparente.
pub fn totales_historicos(conn: &Connection) -> Resultado<(Monto, Monto, Monto)> {
    let fila = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN tipo = 'ingreso' THEN monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN tipo = 'gasto' THEN monto ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN tipo = 'gasto' AND es_estimado = 1
                              THEN monto ELSE 0 END), 0)
         FROM movimientos",
        [],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
    )?;
    Ok(fila)
}
