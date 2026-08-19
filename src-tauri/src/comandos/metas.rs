use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate};
use rusqlite::Connection;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::dominio::{fechas, metas as calculo};
use crate::error::{AppError, Resultado};
use crate::modelos::meta::{EstadoMeta, Meta, MetaDetalle, NuevaMeta, ResumenMetas};
use crate::repos;
use crate::EstadoApp;

/// Cuántos meses cerrados entran en el balance promedio.
pub const MESES_DE_BALANCE: i64 = 3;

/// Metas, sus cálculos y los totales del conjunto.
///
/// `filtro` es solo de presentación: los totales se calculan siempre sobre las
/// metas activas, mire el usuario la lista que mire.
#[tauri::command]
pub fn resumen_metas(estado: State<'_, EstadoApp>, filtro: Option<String>) -> Resultado<ResumenMetas> {
    let filtro = match filtro.as_deref() {
        None | Some("todas") | Some("") => None,
        Some(texto) => Some(EstadoMeta::desde_texto(texto)?),
    };

    let guard = estado.conn();
    armar_resumen(&guard, filtro)
}

#[tauri::command]
pub fn crear_meta(estado: State<'_, EstadoApp>, datos: NuevaMeta) -> Resultado<i64> {
    let guard = estado.conn();
    crear(&guard, &datos)
}

#[tauri::command]
pub fn actualizar_meta(estado: State<'_, EstadoApp>, id: i64, datos: NuevaMeta) -> Resultado<()> {
    let guard = estado.conn();
    actualizar(&guard, id, &datos)
}

/// Marcar cumplida, archivar o volver a activar. Nunca borra: el historial de
/// lo que uno se propuso es parte de lo que la pantalla tiene para contar.
#[tauri::command]
pub fn cambiar_estado_meta(
    estado: State<'_, EstadoApp>,
    id: i64,
    nuevo_estado: String,
) -> Resultado<()> {
    let guard = estado.conn();
    cambiar_estado(&guard, id, EstadoMeta::desde_texto(&nuevo_estado)?)
}

#[tauri::command]
pub fn eliminar_meta(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();
    eliminar(&guard, id)
}

/// Reordena la lista completa. Recibe los ids en el orden deseado.
#[tauri::command]
pub fn reordenar_metas(estado: State<'_, EstadoApp>, ids: Vec<i64>) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;
    reordenar(&tx, &ids)?;
    tx.commit()?;
    Ok(())
}

// ── núcleos ──────────────────────────────────────────────────────────────────
//
// Reciben `&Connection` —o una transacción, vía Deref— para poder cubrirlos con
// tests sin levantar Tauri.

pub fn armar_resumen(conn: &Connection, filtro: Option<EstadoMeta>) -> Resultado<ResumenMetas> {
    armar_resumen_al(conn, filtro, fechas::hoy())
}

/// Núcleo del resumen, con el día de corte explícito.
///
/// El día llega por parámetro y no se lee adentro para que los tests no
/// dependan de la fecha en que se ejecutan. Es el mismo arreglo que usan
/// `desglose_hasta` y `armar_rango`.
pub fn armar_resumen_al(
    conn: &Connection,
    filtro: Option<EstadoMeta>,
    hoy: NaiveDate,
) -> Resultado<ResumenMetas> {
    let todas = repos::metas::listar(conn, None)?;
    let cuentas = repos::cuentas::listar(conn, false)?;

    let saldos: HashMap<i64, (String, Monto)> = cuentas
        .into_iter()
        .map(|c| (c.id, (c.nombre, c.saldo)))
        .collect();

    let acumulados = repartir_entre_metas(&todas, &saldos);
    let (balance_promedio, meses_considerados) = balance_promedio(conn, hoy)?;

    let mut metas = Vec::with_capacity(todas.len());
    for meta in todas {
        metas.push(detalle(meta, &acumulados, &saldos, balance_promedio, hoy)?);
    }

    let activas = || metas.iter().filter(|d| d.meta.estado == EstadoMeta::Activa);

    let total_objetivo: Monto = activas().map(|d| d.meta.monto_objetivo).sum();
    let total_acumulado: Monto = activas().map(|d| d.acumulado).sum();
    let total_falta: Monto = activas().map(|d| d.falta).sum();
    let total_ahorrado = repos::cuentas::total_ahorrado(conn)?;
    let (n_activas, n_cumplidas, n_archivadas) = repos::metas::contar_por_estado(conn)?;

    // Lo apartado que ninguna meta activa reclama. Nunca negativo: el reparto
    // no puede entregar más saldo del que hay.
    let ahorro_sin_meta = (total_ahorrado - total_acumulado).max(0);

    if let Some(estado) = filtro {
        metas.retain(|d| d.meta.estado == estado);
    }

    Ok(ResumenMetas {
        metas,
        total_objetivo,
        total_acumulado,
        total_falta,
        total_ahorrado,
        ahorro_sin_meta,
        balance_promedio,
        meses_al_ritmo: balance_promedio.and_then(|b| calculo::meses_al_ritmo(total_falta, b)),
        meses_considerados,
        n_activas,
        n_cumplidas,
        n_archivadas,
    })
}

/// Cuánto le toca a cada meta del saldo de su cuenta.
///
/// Solo participan las activas: una meta cumplida es historial, y dejarla
/// reservando saldo mostraría comprometida una plata que ya se usó. Las
/// archivadas tampoco compiten, por lo mismo.
fn repartir_entre_metas(
    metas: &[Meta],
    saldos: &HashMap<i64, (String, Monto)>,
) -> HashMap<i64, Monto> {
    let mut acumulados = HashMap::new();

    for (cuenta_id, (_, saldo)) in saldos {
        // `metas` ya viene ordenado por prioridad: el filtro conserva el orden.
        let de_esta_cuenta: Vec<&Meta> = metas
            .iter()
            .filter(|m| m.estado == EstadoMeta::Activa && m.cuenta_id == Some(*cuenta_id))
            .collect();

        let objetivos: Vec<Monto> = de_esta_cuenta.iter().map(|m| m.monto_objetivo).collect();

        for (meta, acumulado) in de_esta_cuenta
            .iter()
            .zip(calculo::repartir_por_prioridad(*saldo, &objetivos))
        {
            acumulados.insert(meta.id, acumulado);
        }
    }

    acumulados
}

/// Arma la fila que ve la pantalla: avance, cuánto falta, ritmo y proyección.
fn detalle(
    meta: Meta,
    acumulados: &HashMap<i64, Monto>,
    saldos: &HashMap<i64, (String, Monto)>,
    balance_promedio: Option<Monto>,
    hoy: NaiveDate,
) -> Resultado<MetaDetalle> {
    let cuenta_nombre = meta
        .cuenta_id
        .and_then(|id| saldos.get(&id))
        .map(|(nombre, _)| nombre.clone());

    // Una meta cumplida se muestra completa aunque su cuenta ya no tenga la
    // plata: se cumplió, y lo más probable es que se haya gastado en eso.
    let (acumulado, tiene_progreso) = match meta.estado {
        EstadoMeta::Activa => (
            acumulados.get(&meta.id).copied().unwrap_or(0),
            meta.cuenta_id.is_some() && cuenta_nombre.is_some(),
        ),
        EstadoMeta::Cumplida => (meta.monto_objetivo, meta.cuenta_id.is_some()),
        EstadoMeta::Archivada => (0, false),
    };

    let falta = (meta.monto_objetivo - acumulado).max(0);

    let (meses_restantes, fecha_pasada) = match meta.fecha_objetivo.as_deref() {
        Some(iso) => {
            let objetivo = fechas::desde_iso(iso)?;
            if objetivo < hoy {
                (None, falta > 0)
            } else {
                // Una fecha dentro de este mismo mes deja un mes para juntar,
                // no cero: con cero no habría ritmo que mostrar.
                (Some(fechas::meses_entre(hoy, objetivo).max(1)), false)
            }
        }
        None => (None, false),
    };

    Ok(MetaDetalle {
        cuenta_nombre,
        acumulado,
        falta,
        progreso_pct: calculo::progreso_pct(acumulado, meta.monto_objetivo),
        tiene_progreso,
        ritmo_mensual: meses_restantes.and_then(|meses| calculo::ritmo_necesario(falta, meses)),
        meses_restantes,
        fecha_pasada,
        meses_al_ritmo: balance_promedio.and_then(|b| calculo::meses_al_ritmo(falta, b)),
        meta,
    })
}

/// Promedio del balance de los [`MESES_DE_BALANCE`] meses cerrados anteriores
/// al actual, y cuántos entraron en la cuenta.
///
/// El mes en curso queda fuera a propósito: está a medias —el sueldo puede no
/// haber entrado todavía, o los gastos del mes recién empiezan— y promediarlo
/// tiraría la proyección para cualquier lado según el día en que se mire.
///
/// Los meses sin actividad tampoco entran: la fila del período existe apenas
/// uno navega a ese mes, y contarla como balance $0 castigaría a quien lleva
/// poco tiempo con la app.
pub fn balance_promedio(conn: &Connection, hoy: NaiveDate) -> Resultado<(Option<Monto>, i32)> {
    let actual = fechas::mes_absoluto(hoy.year(), hoy.month());
    let filas = repos::periodos::balances_por_mes(conn, actual - MESES_DE_BALANCE, actual - 1)?;

    let balances: Vec<Monto> = filas
        .into_iter()
        .filter(|(_, ingresos, _, n_movimientos)| *n_movimientos > 0 || *ingresos > 0)
        .map(|(_, ingresos, gastos, _)| ingresos - gastos)
        .collect();

    Ok((calculo::promedio(&balances), balances.len() as i32))
}

pub fn crear(conn: &Connection, datos: &NuevaMeta) -> Resultado<i64> {
    validar(conn, datos)?;
    repos::metas::insertar(conn, datos)
}

pub fn actualizar(conn: &Connection, id: i64, datos: &NuevaMeta) -> Resultado<()> {
    repos::metas::obtener(conn, id)?;
    validar(conn, datos)?;
    repos::metas::actualizar(conn, id, datos)
}

pub fn cambiar_estado(conn: &Connection, id: i64, nuevo: EstadoMeta) -> Resultado<()> {
    repos::metas::obtener(conn, id)?;
    repos::metas::cambiar_estado(conn, id, nuevo)
}

pub fn eliminar(conn: &Connection, id: i64) -> Resultado<()> {
    repos::metas::obtener(conn, id)?;
    repos::metas::eliminar(conn, id)
}

/// Aplica el orden pedido y renumera la lista entera de 0 en adelante.
///
/// Las metas que no vengan en `ids` —las que el filtro de la pantalla tenía
/// ocultas— conservan su orden relativo y quedan detrás. Renumerar todo evita
/// que dos metas terminen con la misma prioridad y que el reparto de saldo
/// dependa del desempate.
pub fn reordenar(conn: &Connection, ids: &[i64]) -> Resultado<()> {
    let mut vistos = HashSet::new();
    for id in ids {
        repos::metas::obtener(conn, *id)?;
        if !vistos.insert(*id) {
            return Err(AppError::validacion(
                "La lista de orden trae una meta repetida.",
            ));
        }
    }

    let resto: Vec<i64> = repos::metas::listar(conn, None)?
        .into_iter()
        .map(|m| m.id)
        .filter(|id| !vistos.contains(id))
        .collect();

    for (posicion, id) in ids.iter().chain(resto.iter()).enumerate() {
        repos::metas::fijar_prioridad(conn, *id, posicion as i32)?;
    }

    Ok(())
}

fn validar(conn: &Connection, datos: &NuevaMeta) -> Resultado<()> {
    if datos.nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }

    if datos.monto_objetivo <= 0 {
        return Err(AppError::validacion(
            "El monto objetivo tiene que ser mayor a 0.",
        ));
    }

    if let Some(fecha) = datos.fecha_objetivo.as_deref().filter(|f| !f.trim().is_empty()) {
        fechas::desde_iso(fecha)?;
    }

    // Vincularla a una cuenta que no existe dejaría una meta sin progreso
    // posible y sin explicación visible.
    if let Some(cuenta_id) = datos.cuenta_id {
        repos::cuentas::obtener(conn, cuenta_id)?;
    }

    Ok(())
}
