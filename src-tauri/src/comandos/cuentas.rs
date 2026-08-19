use std::collections::HashMap;

use chrono::Datelike;
use rusqlite::Connection;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::cuenta::{CuentaConNotas, DesgloseSaldo, NuevaCuenta, ResumenCuentas};
use crate::modelos::movimiento_ahorro::TipoMovimientoAhorro;
use crate::modelos::nota_ahorro::NotaAhorro;
use crate::repos;
use crate::EstadoApp;

/// Disponible, patrimonio, ahorros y el desglose que explica el número.
#[tauri::command]
pub fn resumen_cuentas(estado: State<'_, EstadoApp>) -> Resultado<ResumenCuentas> {
    let guard = estado.conn();
    armar_resumen(&guard)
}

/// Ajusta lo que había antes de empezar a usar la app.
///
/// Es la única perilla para cuadrar con la realidad: se sube o se baja hasta
/// que el disponible calce con el banco, y después no se toca más.
#[tauri::command]
pub fn fijar_saldo_inicial(estado: State<'_, EstadoApp>, saldo: Monto) -> Resultado<()> {
    let guard = estado.conn();
    fijar_inicial(&guard, saldo)
}

/// Mueve plata del disponible a una cuenta de ahorro.
#[tauri::command]
pub fn apartar(estado: State<'_, EstadoApp>, ahorro_id: i64, monto: Monto) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;
    mover(&tx, ahorro_id, monto, Direccion::Apartar)?;
    tx.commit()?;
    Ok(())
}

/// Devuelve plata de un ahorro al disponible.
#[tauri::command]
pub fn retirar(estado: State<'_, EstadoApp>, ahorro_id: i64, monto: Monto) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;
    mover(&tx, ahorro_id, monto, Direccion::Retirar)?;
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn crear_cuenta(estado: State<'_, EstadoApp>, datos: NuevaCuenta) -> Resultado<i64> {
    let guard = estado.conn();
    crear(&guard, &datos)
}

#[tauri::command]
pub fn actualizar_cuenta(
    estado: State<'_, EstadoApp>,
    id: i64,
    nombre: String,
    activa: bool,
) -> Resultado<()> {
    let guard = estado.conn();
    actualizar(&guard, id, &nombre, activa)
}

#[tauri::command]
pub fn eliminar_cuenta(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();
    eliminar(&guard, id)
}

// ── núcleos ──────────────────────────────────────────────────────────────────
//
// Reciben `&Connection` —o una transacción, vía Deref— para poder cubrirlos con
// tests sin levantar Tauri.

/// Arma el desglose leyendo las tres fuentes de plata, hasta el mes en curso.
///
/// El sueldo vive en `periodos` y no en `movimientos`: sin sumarlo, el
/// patrimonio restaría todos los gastos sin casi ningún ingreso. Es la misma
/// definición que usa el resumen del mes.
pub fn desglose(conn: &Connection) -> Resultado<DesgloseSaldo> {
    let hoy = fechas::hoy();
    desglose_hasta(conn, hoy.year(), hoy.month())
}

/// Núcleo del desglose, con el mes de corte explícito.
///
/// Todo lo que caiga en un mes **posterior** al indicado queda fuera: un
/// estimado de un servicio que todavía no venció, o un gasto anotado con fecha
/// adelantada, son una proyección y no plata que ya salió. Contarlos hacía que
/// el disponible se viera más bajo que el banco por algo que no pasó.
///
/// El mes llega por parámetro en vez de leerse adentro para que los tests
/// puedan fijarlo; si no, dependerían del día en que se ejecutan. Es el mismo
/// arreglo que usan `armar_rango` y `aplicar_actualizacion`.
pub fn desglose_hasta(conn: &Connection, anio: i32, mes: u32) -> Resultado<DesgloseSaldo> {
    let hasta_abs = fechas::mes_absoluto(anio, mes);

    let (ingresos_registrados, gastos, gastos_estimados) =
        repos::movimientos::totales_historicos(conn, hasta_abs)?;

    Ok(DesgloseSaldo {
        saldo_inicial: repos::configuracion::obtener_monto(
            conn,
            repos::configuracion::SALDO_INICIAL,
        )?,
        ingresos_declarados: repos::periodos::total_ingresos_declarados(conn, hasta_abs)?,
        ingresos_registrados,
        gastos,
        gastos_estimados,
        apartado: repos::cuentas::total_ahorrado(conn)?,
    })
}

pub fn armar_resumen(conn: &Connection) -> Resultado<ResumenCuentas> {
    let desglose = desglose(conn)?;

    Ok(ResumenCuentas {
        disponible: desglose.disponible(),
        patrimonio: desglose.patrimonio(),
        total_ahorrado: desglose.apartado,
        ahorros: ahorros_con_notas(conn)?,
        desglose,
    })
}

/// Las cuentas con sus notas de propósito colgadas.
///
/// Una sola consulta de notas para todas las cuentas, agrupadas acá: es el
/// mismo arreglo que usa `resumen_servicios` con los gastos por servicio.
/// Las notas no tocan ningún total del resumen; van como anotación.
fn ahorros_con_notas(conn: &Connection) -> Resultado<Vec<CuentaConNotas>> {
    let mut por_cuenta: HashMap<i64, Vec<NotaAhorro>> = HashMap::new();
    for nota in repos::notas_ahorro::listar_todas(conn)? {
        por_cuenta.entry(nota.cuenta_id).or_default().push(nota);
    }

    Ok(repos::cuentas::listar(conn, false)?
        .into_iter()
        .map(|c| {
            let notas = por_cuenta.remove(&c.id).unwrap_or_default();
            CuentaConNotas::nueva(c, notas)
        })
        .collect())
}

/// El saldo inicial admite negativos a propósito: quien empezó a usar la app
/// con la cuenta en rojo o con una línea de crédito usada no podría cuadrar
/// nunca si se le exigiera un número positivo.
pub fn fijar_inicial(conn: &Connection, saldo: Monto) -> Resultado<()> {
    repos::configuracion::guardar_monto(conn, repos::configuracion::SALDO_INICIAL, saldo)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direccion {
    /// Del disponible al ahorro.
    Apartar,
    /// Del ahorro al disponible.
    Retirar,
}

impl From<Direccion> for TipoMovimientoAhorro {
    fn from(direccion: Direccion) -> Self {
        match direccion {
            Direccion::Apartar => TipoMovimientoAhorro::Apartar,
            Direccion::Retirar => TipoMovimientoAhorro::Retirar,
        }
    }
}

/// Apartar y retirar no cambian el patrimonio: mueven plata entre "puedo
/// gastarla" y "no la quiero gastar".
///
/// Apartar se valida contra el disponible **calculado**, que es el único que
/// existe; retirar, contra el saldo de la propia cuenta.
///
/// Cada cruce deja su fila en `movimientos_ahorro`. El registro y el ajuste
/// del saldo van sobre la misma conexión —que desde los comandos es una
/// transacción—, así que o quedan los dos o no queda ninguno: un historial que
/// no cuadre con el saldo sería peor que no tener historial.
pub fn mover(
    conn: &Connection,
    ahorro_id: i64,
    monto: Monto,
    direccion: Direccion,
) -> Resultado<()> {
    if monto <= 0 {
        return Err(AppError::validacion("El monto debe ser mayor a 0."));
    }

    let ahorro = repos::cuentas::obtener(conn, ahorro_id)?;

    match direccion {
        Direccion::Apartar => {
            let disponible = desglose(conn)?.disponible();
            if monto > disponible {
                return Err(AppError::conflicto(format!(
                    "No puedes apartar más de lo disponible ({disponible})."
                )));
            }
            repos::cuentas::ajustar_saldo(conn, ahorro_id, monto)?;
        }
        Direccion::Retirar => {
            if monto > ahorro.saldo {
                return Err(AppError::conflicto(format!(
                    "«{}» solo tiene {}.",
                    ahorro.nombre, ahorro.saldo
                )));
            }
            repos::cuentas::ajustar_saldo(conn, ahorro_id, -monto)?;
        }
    }

    // La fecha es la del día en que se mueve la plata, igual que el
    // `actualizado_en` de la cuenta. No hay forma de apartar con fecha
    // anterior, y por eso los meses previos a esta tabla quedan en cero.
    repos::movimientos_ahorro::insertar(
        conn,
        ahorro_id,
        &fechas::a_iso(fechas::hoy()),
        monto,
        direccion.into(),
        None,
    )?;

    Ok(())
}

pub fn crear(conn: &Connection, datos: &NuevaCuenta) -> Resultado<i64> {
    if datos.nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }

    repos::cuentas::insertar(conn, datos)
}

pub fn actualizar(conn: &Connection, id: i64, nombre: &str, activa: bool) -> Resultado<()> {
    if nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }

    let cuenta = repos::cuentas::obtener(conn, id)?;

    // Archivar una cuenta con plata la escondería sin devolverla al
    // disponible: quedaría apartada sin que se vea dónde.
    if !activa && cuenta.activa && cuenta.saldo > 0 {
        return Err(AppError::conflicto(format!(
            "«{}» todavía tiene plata. Retírala al disponible antes de archivarla.",
            cuenta.nombre
        )));
    }

    repos::cuentas::actualizar_datos(conn, id, nombre, activa)
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    let cuenta = repos::cuentas::obtener(conn, id)?;

    // Borrarla con plata adentro la sumaría de golpe al disponible sin que el
    // usuario lo haya pedido. Retirarla primero deja el paso explícito.
    if cuenta.saldo > 0 {
        return Err(AppError::conflicto(format!(
            "«{}» todavía tiene plata. Retírala al disponible antes de eliminarla.",
            cuenta.nombre
        )));
    }

    repos::cuentas::eliminar(conn, id)
}
