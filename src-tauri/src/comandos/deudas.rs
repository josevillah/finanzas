use rusqlite::Connection;
use tauri::State;

use crate::dominio::amortizacion::{self, CuotaCalculada};
use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::deuda::{Deuda, DeudaDetalle, DeudaResumen, EstadoDeuda, NuevaDeuda};
use crate::repos;
use crate::EstadoApp;

/// Calcula la tabla de cuotas sin guardar nada. Alimenta la vista previa del
/// formulario de creación.
#[tauri::command]
pub fn simular_cuotas(
    monto_original: Monto,
    tasa_mensual: f64,
    n_cuotas: i32,
    fecha_primera_cuota: String,
) -> Resultado<Vec<CuotaCalculada>> {
    let primera = fechas::desde_iso(&fecha_primera_cuota)?;
    amortizacion::generar(monto_original, tasa_mensual, n_cuotas, primera)
}

/// Crea la deuda y materializa sus N cuotas en la misma transacción.
#[tauri::command]
pub fn crear_deuda(estado: State<'_, EstadoApp>, datos: NuevaDeuda) -> Resultado<i64> {
    validar_datos(&datos)?;

    let primera = fechas::desde_iso(&datos.fecha_primera_cuota)?;
    let cuotas = amortizacion::generar(
        datos.monto_original,
        datos.tasa_mensual,
        datos.n_cuotas,
        primera,
    )?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let id = repos::deudas::insertar(&tx, &datos)?;
    repos::cuotas::insertar_muchas(&tx, id, &cuotas)?;
    repos::cuotas::marcar_atrasadas(&tx, &fechas::a_iso(fechas::hoy()))?;

    tx.commit()?;
    Ok(id)
}

/// Reemplaza los datos de la deuda y regenera su tabla de cuotas.
/// Se bloquea si ya hay cuotas pagadas: en ese caso corresponde repactar,
/// no reescribir el historial.
#[tauri::command]
pub fn actualizar_deuda(
    estado: State<'_, EstadoApp>,
    id: i64,
    datos: NuevaDeuda,
) -> Resultado<()> {
    validar_datos(&datos)?;

    let primera = fechas::desde_iso(&datos.fecha_primera_cuota)?;
    let cuotas = amortizacion::generar(
        datos.monto_original,
        datos.tasa_mensual,
        datos.n_cuotas,
        primera,
    )?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    if repos::cuotas::tiene_pagos(&tx, id)? {
        return Err(AppError::conflicto(
            "Esta deuda ya tiene cuotas pagadas, así que no se puede recalcular. \
             Deshaz los pagos o crea una deuda nueva marcando la actual como repactada.",
        ));
    }

    repos::deudas::actualizar(&tx, id, &datos)?;
    repos::cuotas::eliminar_por_deuda(&tx, id)?;
    repos::cuotas::insertar_muchas(&tx, id, &cuotas)?;
    repos::deudas::sincronizar_estado(&tx, id)?;
    repos::cuotas::marcar_atrasadas(&tx, &fechas::a_iso(fechas::hoy()))?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_deuda(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();
    repos::deudas::eliminar(&guard, id)
}

#[tauri::command]
pub fn cambiar_estado_deuda(
    estado: State<'_, EstadoApp>,
    id: i64,
    nuevo_estado: EstadoDeuda,
) -> Resultado<()> {
    let guard = estado.conn();
    repos::deudas::cambiar_estado(&guard, id, nuevo_estado)
}

/// Listado con avance calculado. `filtro_estado` en `None` trae todas.
#[tauri::command]
pub fn listar_deudas(
    estado: State<'_, EstadoApp>,
    filtro_estado: Option<EstadoDeuda>,
) -> Resultado<Vec<DeudaResumen>> {
    let guard = estado.conn();
    let deudas = repos::deudas::listar(&guard, filtro_estado)?;

    deudas
        .into_iter()
        .map(|d| armar_resumen(&guard, d))
        .collect()
}

/// Deuda + tabla de amortización completa.
#[tauri::command]
pub fn obtener_deuda(estado: State<'_, EstadoApp>, id: i64) -> Resultado<DeudaDetalle> {
    let guard = estado.conn();
    let deuda = repos::deudas::obtener(&guard, id)?;
    let cuotas = repos::cuotas::listar_por_deuda(&guard, id)?;
    let resumen = armar_resumen(&guard, deuda)?;

    Ok(DeudaDetalle { resumen, cuotas })
}

// ── auxiliares ───────────────────────────────────────────────────────────────

pub(crate) fn armar_resumen(conn: &Connection, deuda: Deuda) -> Resultado<DeudaResumen> {
    let r = repos::cuotas::resumen(conn, deuda.id)?;
    let proxima = repos::cuotas::proxima_pendiente(conn, deuda.id)?;

    let avance_pct = if r.total_programado > 0 {
        (r.monto_pagado as f64 / r.total_programado as f64) * 100.0
    } else {
        0.0
    };

    Ok(DeudaResumen {
        deuda,
        total_programado: r.total_programado,
        monto_pagado: r.monto_pagado,
        monto_pendiente: r.monto_pendiente,
        cuotas_pagadas: r.cuotas_pagadas,
        cuotas_totales: r.cuotas_totales,
        avance_pct,
        cuotas_atrasadas: r.cuotas_atrasadas,
        proxima_cuota: proxima,
    })
}

fn validar_datos(datos: &NuevaDeuda) -> Resultado<()> {
    if datos.descripcion.trim().is_empty() {
        return Err(AppError::validacion("La descripción no puede quedar vacía."));
    }
    // El resto de las reglas (monto, tasa, número de cuotas) las valida
    // `amortizacion::generar`, que es la única fuente de verdad del cálculo.
    Ok(())
}
