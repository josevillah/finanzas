use rusqlite::Connection;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::nota_ahorro::NuevaNota;
use crate::repos;
use crate::EstadoApp;

#[tauri::command]
pub fn crear_nota(estado: State<'_, EstadoApp>, datos: NuevaNota) -> Resultado<i64> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;
    let id = crear(&tx, &datos)?;
    tx.commit()?;
    Ok(id)
}

#[tauri::command]
pub fn actualizar_nota(
    estado: State<'_, EstadoApp>,
    id: i64,
    nombre: String,
    monto: Monto,
) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;
    actualizar(&tx, id, &nombre, monto)?;
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_nota(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();
    eliminar(&guard, id)
}

// ── núcleos ──────────────────────────────────────────────────────────────────
//
// Reciben `&Connection` —o una transacción, vía Deref— para poder cubrirlos con
// tests sin levantar Tauri. Leen la suma y después escriben, así que los
// comandos los envuelven en una transacción.

pub fn crear(conn: &Connection, datos: &NuevaNota) -> Resultado<i64> {
    validar(&datos.nombre, datos.monto)?;

    let cuenta = repos::cuentas::obtener(conn, datos.cuenta_id)?;
    let suma_actual = repos::notas_ahorro::suma_de_cuenta(conn, datos.cuenta_id)?;

    validar_cuadratura(&cuenta.nombre, cuenta.saldo, suma_actual, suma_actual + datos.monto)?;

    repos::notas_ahorro::insertar(conn, datos)
}

pub fn actualizar(conn: &Connection, id: i64, nombre: &str, monto: Monto) -> Resultado<()> {
    validar(nombre, monto)?;

    let nota = repos::notas_ahorro::obtener(conn, id)?;
    let cuenta = repos::cuentas::obtener(conn, nota.cuenta_id)?;
    let suma_actual = repos::notas_ahorro::suma_de_cuenta(conn, nota.cuenta_id)?;

    // La nota vieja sale de la suma y entra la nueva.
    let suma_nueva = suma_actual - nota.monto + monto;
    validar_cuadratura(&cuenta.nombre, cuenta.saldo, suma_actual, suma_nueva)?;

    repos::notas_ahorro::actualizar(conn, id, nombre, monto)
}

/// Borrar no se valida: sacar una nota siempre baja la suma.
pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    repos::notas_ahorro::eliminar(conn, id)
}

fn validar(nombre: &str, monto: Monto) -> Resultado<()> {
    if nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }
    if monto < 0 {
        return Err(AppError::validacion("El monto no puede ser negativo."));
    }
    Ok(())
}

/// La suma de las notas no debería pasarse del saldo de la cuenta, pero la
/// regla es **asimétrica**: se mira contra la suma que ya había, no solo contra
/// el saldo.
///
/// Sin eso el usuario queda encerrado. Basta que retire plata de la cuenta para
/// que sus notas queden por encima del saldo, y con una regla simétrica no
/// podría corregir ninguna: cada intento de bajar una nota se rechazaría por
/// estar ya excedido.
///
/// El caso "queda igual" se permite a propósito (`<=` y no `<`): estando
/// excedido, **renombrar** una nota no mueve la suma, y bloquear eso sería
/// exactamente la trampa que esta asimetría viene a evitar. La regla es
/// rechazar solo cuando el exceso *aumenta*.
pub fn validar_cuadratura(
    cuenta: &str,
    saldo: Monto,
    suma_actual: Monto,
    suma_nueva: Monto,
) -> Resultado<()> {
    if suma_nueva <= saldo || suma_nueva <= suma_actual {
        return Ok(());
    }

    // Con los tres números el usuario sabe cuánto tiene que bajar; un rechazo
    // sin cifras no se puede corregir.
    Err(AppError::conflicto(format!(
        "Tus notas sumarían {suma_nueva} y «{cuenta}» tiene {saldo}. \
         Baja el monto o retira menos de la cuenta."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permite(saldo: Monto, actual: Monto, nueva: Monto) -> bool {
        validar_cuadratura("Fan", saldo, actual, nueva).is_ok()
    }

    #[test]
    fn dentro_del_saldo_se_permite() {
        assert!(permite(100_000, 0, 25_000));
        assert!(permite(100_000, 25_000, 90_000));
    }

    #[test]
    fn cuadrar_exacto_se_permite() {
        assert!(permite(100_000, 90_000, 100_000), "el borde entra");
    }

    #[test]
    fn pasarse_del_saldo_se_rechaza() {
        assert!(!permite(100_000, 90_000, 100_001));
    }

    #[test]
    fn ya_excedido_bajar_se_permite() {
        // El caso real: retiró plata y las notas quedaron por encima.
        assert!(permite(50_000, 100_000, 80_000));
        assert!(permite(50_000, 100_000, 50_000), "hasta cuadrar de nuevo");
    }

    #[test]
    fn ya_excedido_dejar_igual_se_permite() {
        assert!(
            permite(50_000, 100_000, 100_000),
            "renombrar no mueve la suma y no puede quedar bloqueado"
        );
    }

    #[test]
    fn ya_excedido_subir_se_rechaza() {
        assert!(!permite(50_000, 100_000, 100_001));
    }

    #[test]
    fn con_saldo_cero_solo_se_puede_bajar() {
        assert!(permite(0, 0, 0));
        assert!(!permite(0, 0, 1));
        assert!(permite(0, 30_000, 10_000));
    }
}
