use rusqlite::{params, Connection};

use crate::dominio::dinero::Monto;
use crate::error::Resultado;
use crate::modelos::movimiento_ahorro::{MovimientoAhorro, TipoMovimientoAhorro};

/// Registra que la plata cruzó. Quien llama ya está dentro de la transacción
/// que ajusta el saldo: el registro y el saldo se aplican juntos o no se
/// aplica ninguno.
pub fn insertar(
    conn: &Connection,
    cuenta_id: i64,
    fecha: &str,
    monto: Monto,
    tipo: TipoMovimientoAhorro,
    nota: Option<&str>,
) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO movimientos_ahorro (cuenta_id, fecha, monto, tipo, nota)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            cuenta_id,
            fecha,
            monto,
            tipo.como_texto(),
            nota.map(str::trim).filter(|n| !n.is_empty()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Apartado menos retirado entre dos fechas ISO, ambas inclusive.
///
/// Negativo cuando en el rango se sacó más de lo que se guardó, que es un
/// resultado legítimo y la pantalla lo redacta distinto.
pub fn neto_en_rango(conn: &Connection, desde: &str, hasta: &str) -> Resultado<Monto> {
    let neto: Monto = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN tipo = 'apartar' THEN monto ELSE -monto END), 0)
           FROM movimientos_ahorro
          WHERE fecha BETWEEN ?1 AND ?2",
        params![desde, hasta],
        |f| f.get(0),
    )?;
    Ok(neto)
}

/// Historial completo, para la exportación.
pub fn listar_todos(conn: &Connection) -> Resultado<Vec<MovimientoAhorro>> {
    let sql = format!(
        "SELECT {} FROM movimientos_ahorro ORDER BY fecha, id",
        MovimientoAhorro::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map([], MovimientoAhorro::desde_fila)?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}
