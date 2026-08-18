use std::collections::BTreeMap;

use rusqlite::Connection;
use tauri::State;

use crate::dominio::amortizacion::{self, CuotaCalculada};
use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::deuda::{
    Deuda, DeudaDetalle, DeudaResumen, DeudorResumen, DireccionDeuda, EstadoDeuda, NuevaDeuda,
    ResumenTerceros,
};
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
    direccion: Option<DireccionDeuda>,
) -> Resultado<Vec<DeudaResumen>> {
    let guard = estado.conn();
    let deudas = repos::deudas::listar(&guard, filtro_estado, direccion)?;

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

    // Sin nombre, la vista "Me deben" no puede agrupar por persona ni el cobro
    // deja rastro de quién pagó.
    if datos.direccion == DireccionDeuda::Tercero
        && datos.deudor.as_deref().map(str::trim).unwrap_or("").is_empty()
    {
        return Err(AppError::validacion(
            "Indica quién te debe esta plata.",
        ));
    }
    // El resto de las reglas (monto, tasa, número de cuotas) las valida
    // `amortizacion::generar`, que es la única fuente de verdad del cálculo.
    Ok(())
}

/// Cuánto me debe cada persona. Alimenta la vista "Me deben".
#[tauri::command]
pub fn resumen_terceros(estado: State<'_, EstadoApp>) -> Resultado<ResumenTerceros> {
    let guard = estado.conn();
    let deudas = repos::deudas::listar(&guard, None, Some(DireccionDeuda::Tercero))?;

    // BTreeMap para que el orden sea estable entre corridas antes de ordenar
    // por monto.
    let mut por_persona: BTreeMap<String, DeudorResumen> = BTreeMap::new();

    for deuda in deudas {
        let r = repos::cuotas::resumen(&guard, deuda.id)?;
        let proxima = repos::cuotas::proxima_pendiente(&guard, deuda.id)?;

        // El deudor es obligatorio al crear, pero una base editada a mano
        // podría traerlo vacío.
        let nombre = deuda.deudor.clone().unwrap_or_else(|| "Sin nombre".into());

        let entrada = por_persona.entry(nombre.clone()).or_insert(DeudorResumen {
            deudor: nombre,
            n_deudas: 0,
            total_pendiente: 0,
            total_cobrado: 0,
            cuotas_pendientes: 0,
            cuotas_atrasadas: 0,
            proxima_fecha: None,
        });

        entrada.n_deudas += 1;
        entrada.total_pendiente += r.monto_pendiente;
        entrada.total_cobrado += r.monto_pagado;
        entrada.cuotas_pendientes += r.cuotas_totales - r.cuotas_pagadas;
        entrada.cuotas_atrasadas += r.cuotas_atrasadas;

        if let Some(cuota) = proxima {
            // Las fechas ISO ordenan lexicográficamente igual que cronológicamente.
            let mas_temprana = entrada
                .proxima_fecha
                .as_deref()
                .map_or(true, |actual| cuota.fecha_vencimiento.as_str() < actual);
            if mas_temprana {
                entrada.proxima_fecha = Some(cuota.fecha_vencimiento);
            }
        }
    }

    let mut deudores: Vec<DeudorResumen> = por_persona.into_values().collect();
    deudores.sort_by(|a, b| {
        b.total_pendiente
            .cmp(&a.total_pendiente)
            .then_with(|| a.deudor.cmp(&b.deudor))
    });

    Ok(ResumenTerceros {
        total_pendiente: deudores.iter().map(|d| d.total_pendiente).sum(),
        total_cobrado: deudores.iter().map(|d| d.total_cobrado).sum(),
        cuotas_atrasadas: deudores.iter().map(|d| d.cuotas_atrasadas).sum(),
        deudores,
    })
}
