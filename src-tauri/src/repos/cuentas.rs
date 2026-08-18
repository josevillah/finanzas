use rusqlite::{params, Connection, OptionalExtension};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::cuenta::{Cuenta, NuevaCuenta};

/// Cuentas de ahorro, en el orden en que se muestran.
pub fn listar(conn: &Connection, solo_activas: bool) -> Resultado<Vec<Cuenta>> {
    let sql = format!(
        "SELECT {} FROM cuentas
         WHERE ?1 = 0 OR activa = 1
         ORDER BY orden, nombre",
        Cuenta::COLUMNAS
    );

    let mut stmt = conn.prepare(&sql)?;
    let filas = stmt.query_map(params![solo_activas as i64], Cuenta::desde_fila)?;

    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn obtener(conn: &Connection, id: i64) -> Resultado<Cuenta> {
    let sql = format!("SELECT {} FROM cuentas WHERE id = ?1", Cuenta::COLUMNAS);
    conn.query_row(&sql, params![id], Cuenta::desde_fila)
        .optional()?
        .ok_or_else(|| AppError::no_encontrado(format!("la cuenta #{id}")))
}

pub fn insertar(conn: &Connection, datos: &NuevaCuenta) -> Resultado<i64> {
    conn.execute(
        "INSERT INTO cuentas (nombre, saldo, activa, orden, actualizado_en)
         VALUES (?1, 0, 1, COALESCE((SELECT MAX(orden) + 1 FROM cuentas), 0), ?2)",
        params![
            datos.nombre.trim(),
            crate::dominio::fechas::a_iso(crate::dominio::fechas::hoy()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Cambia nombre y estado. El saldo se mueve solo apartando o retirando, para
/// que nunca cambie sin que la plata venga o vaya a algún lado.
pub fn actualizar_datos(conn: &Connection, id: i64, nombre: &str, activa: bool) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE cuentas SET nombre = ?2, activa = ?3 WHERE id = ?1",
        params![id, nombre.trim(), activa as i64],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la cuenta #{id}")));
    }
    Ok(())
}

/// Suma (o resta, con monto negativo) al saldo de una cuenta.
///
/// El `CHECK (saldo >= 0)` del esquema aborta la operación si dejara la cuenta
/// en rojo: es la última línea de defensa por si una validación se saltara.
pub fn ajustar_saldo(conn: &Connection, id: i64, delta: Monto) -> Resultado<()> {
    let filas = conn.execute(
        "UPDATE cuentas SET saldo = saldo + ?2, actualizado_en = ?3 WHERE id = ?1",
        params![
            id,
            delta,
            crate::dominio::fechas::a_iso(crate::dominio::fechas::hoy())
        ],
    )?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la cuenta #{id}")));
    }
    Ok(())
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let filas = conn.execute("DELETE FROM cuentas WHERE id = ?1", params![id])?;

    if filas == 0 {
        return Err(AppError::no_encontrado(format!("la cuenta #{id}")));
    }
    Ok(())
}

/// Suma de lo apartado, sobre todas las filas.
///
/// Archivar exige saldo 0, así que una cuenta archivada no debería aportar
/// nada; sumarlas igual evita que, si alguna quedara con plata, esa plata
/// reapareciera sola como disponible.
pub fn total_ahorrado(conn: &Connection) -> Resultado<Monto> {
    let total: Monto = conn.query_row("SELECT COALESCE(SUM(saldo), 0) FROM cuentas", [], |f| {
        f.get(0)
    })?;
    Ok(total)
}
