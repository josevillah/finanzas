use chrono::Datelike;
use rusqlite::Connection;
use tauri::State;

use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::categoria::{CODIGO_COBROS, CODIGO_DEUDAS};
use crate::modelos::deuda::DireccionDeuda;
use crate::modelos::movimiento::TipoMovimiento;
use crate::modelos::cuota::{Cuota, CuotaConDeuda, PagoCuota};
use crate::repos;
use crate::EstadoApp;

/// Marca una cuota como pagada. El monto real puede diferir del programado.
/// El pago también queda registrado como gasto del mes en `movimientos`,
/// enlazado por `cuota_id`.
#[tauri::command]
pub fn pagar_cuota(estado: State<'_, EstadoApp>, pago: PagoCuota) -> Resultado<()> {
    if pago.monto_pagado < 0 {
        return Err(AppError::validacion(
            "El monto pagado no puede ser negativo.",
        ));
    }

    // Si no viene fecha, se asume hoy.
    let fecha = match pago.fecha_pago.as_deref().map(str::trim) {
        Some(f) if !f.is_empty() => fechas::a_iso(fechas::desde_iso(f)?),
        _ => fechas::a_iso(fechas::hoy()),
    };

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let cuota = repos::cuotas::obtener(&tx, pago.cuota_id)?;
    repos::cuotas::registrar_pago(&tx, pago.cuota_id, &fecha, pago.monto_pagado)?;
    repos::deudas::sincronizar_estado(&tx, cuota.deuda_id)?;
    repos::cuotas::marcar_atrasadas(&tx, &fechas::a_iso(fechas::hoy()))?;

    sincronizar_movimiento_de_cuota(&tx, &cuota, &fecha, pago.monto_pagado)?;

    tx.commit()?;
    Ok(())
}

/// Revierte el pago de una cuota y borra el gasto que había generado.
#[tauri::command]
pub fn deshacer_pago_cuota(estado: State<'_, EstadoApp>, cuota_id: i64) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let cuota = repos::cuotas::obtener(&tx, cuota_id)?;
    repos::cuotas::deshacer_pago(&tx, cuota_id)?;
    repos::deudas::sincronizar_estado(&tx, cuota.deuda_id)?;
    repos::cuotas::marcar_atrasadas(&tx, &fechas::a_iso(fechas::hoy()))?;
    repos::movimientos::eliminar_por_cuota(&tx, cuota_id)?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn listar_cuotas_deuda(estado: State<'_, EstadoApp>, deuda_id: i64) -> Resultado<Vec<Cuota>> {
    let guard = estado.conn();
    repos::cuotas::listar_por_deuda(&guard, deuda_id)
}

/// Cuotas que vencen en el mes indicado, de deudas vigentes.
/// Incluye las ya pagadas para que el listado cuadre con el total que muestra
/// la carga financiera del mes.
#[tauri::command]
pub fn listar_cuotas_mes(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
) -> Resultado<Vec<CuotaConDeuda>> {
    let desde = fechas::a_iso(fechas::primer_dia(anio, mes)?);
    let hasta = fechas::a_iso(fechas::ultimo_dia(anio, mes)?);

    let guard = estado.conn();
    let cuotas = repos::cuotas::en_rango_con_deuda(&guard, &desde, &hasta)?;

    Ok(cuotas
        .into_iter()
        .map(|(cuota, deuda_descripcion)| CuotaConDeuda {
            cuota,
            deuda_descripcion,
        })
        .collect())
}

// ── auxiliares ───────────────────────────────────────────────────────────────

/// Deja exactamente un movimiento asociado a la cuota, en el período que
/// corresponde a la fecha de pago. Se borra primero por si se está repitiendo
/// el pago con otra fecha o monto.
///
/// Es pública para poder cubrirla con tests de integración sin levantar Tauri.
pub fn sincronizar_movimiento_de_cuota(
    conn: &Connection,
    cuota: &Cuota,
    fecha_pago: &str,
    monto: i64,
) -> Resultado<()> {
    repos::movimientos::eliminar_por_cuota(conn, cuota.id)?;

    // Un pago de $0 (cuota condonada, por ejemplo) no genera gasto.
    if monto == 0 {
        return Ok(());
    }

    let fecha = fechas::desde_iso(fecha_pago)?;
    let periodo = repos::periodos::obtener_o_crear(conn, fecha.year(), fecha.month())?;
    // Un mes cerrado está congelado: tampoco puede recibir el gasto de una cuota.
    repos::periodos::exigir_abierto(conn, periodo.id)?;

    let deuda = repos::deudas::obtener(conn, cuota.deuda_id)?;

    // Pagar una deuda propia es un gasto; cobrar una de un tercero es plata
    // que entra. Cada uno cae en su categoría del sistema, ubicada por código
    // para no depender del nombre. Si el usuario la eliminó, el movimiento
    // queda sin categoría antes que perderse.
    let (codigo, tipo) = match deuda.direccion {
        DireccionDeuda::Propia => (CODIGO_DEUDAS, TipoMovimiento::Gasto),
        DireccionDeuda::Tercero => (CODIGO_COBROS, TipoMovimiento::Ingreso),
    };
    let categoria_id = repos::categorias::por_codigo(conn, codigo)?.map(|c| c.id);

    let descripcion = match (deuda.direccion, deuda.deudor.as_deref()) {
        (DireccionDeuda::Tercero, Some(deudor)) => format!(
            "{deudor} · {} · cuota {}/{}",
            deuda.descripcion, cuota.numero, deuda.n_cuotas
        ),
        _ => format!("{} · cuota {}/{}", deuda.descripcion, cuota.numero, deuda.n_cuotas),
    };

    repos::movimientos::insertar_pago_cuota(
        conn,
        periodo.id,
        cuota.id,
        categoria_id,
        fecha_pago,
        monto,
        &descripcion,
        tipo,
    )?;

    Ok(())
}
