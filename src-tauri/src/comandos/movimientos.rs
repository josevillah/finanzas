use chrono::Datelike;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::movimiento::{
    FiltroMovimientos, MedioPago, MovimientoDetalle, NuevoMovimiento, TipoMovimiento,
};
use crate::repos;
use crate::EstadoApp;

/// Registra un ingreso o gasto. El período se deduce de la fecha y se crea
/// solo si no existe.
#[tauri::command]
pub fn registrar_movimiento(
    estado: State<'_, EstadoApp>,
    datos: NuevoMovimiento,
) -> Resultado<i64> {
    validar(&datos)?;
    let fecha = fechas::desde_iso(&datos.fecha)?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let periodo = repos::periodos::obtener_o_crear(&tx, fecha.year(), fecha.month())?;
    repos::periodos::exigir_abierto(&tx, periodo.id)?;
    let id = repos::movimientos::insertar(&tx, periodo.id, &datos)?;

    tx.commit()?;
    Ok(id)
}

#[tauri::command]
pub fn actualizar_movimiento(
    estado: State<'_, EstadoApp>,
    id: i64,
    datos: NuevoMovimiento,
) -> Resultado<()> {
    validar(&datos)?;
    let fecha = fechas::desde_iso(&datos.fecha)?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    // Si cambió la fecha de mes, el movimiento se muda de período.
    let anterior = repos::movimientos::obtener(&tx, id)?;
    repos::periodos::exigir_abierto(&tx, anterior.periodo_id)?;

    let periodo = repos::periodos::obtener_o_crear(&tx, fecha.year(), fecha.month())?;
    repos::periodos::exigir_abierto(&tx, periodo.id)?;
    repos::movimientos::actualizar(&tx, id, periodo.id, &datos)?;

    tx.commit()?;
    Ok(())
}

/// Cambia solo el monto: el caso de "llegó la boleta con otra cifra".
/// El movimiento queda confirmado, así deja de contar como estimado.
#[tauri::command]
pub fn cambiar_monto_movimiento(
    estado: State<'_, EstadoApp>,
    id: i64,
    monto: Monto,
) -> Resultado<()> {
    if monto <= 0 {
        return Err(AppError::validacion("El monto debe ser mayor a 0."));
    }

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let movimiento = repos::movimientos::obtener(&tx, id)?;
    repos::periodos::exigir_abierto(&tx, movimiento.periodo_id)?;
    repos::movimientos::cambiar_monto(&tx, id, monto)?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_movimiento(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let movimiento = repos::movimientos::obtener(&tx, id)?;
    repos::periodos::exigir_abierto(&tx, movimiento.periodo_id)?;
    repos::movimientos::eliminar(&tx, id)?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn listar_movimientos(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    filtro: Option<FiltroMovimientos>,
) -> Resultado<Vec<MovimientoDetalle>> {
    let guard = estado.conn();
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;
    repos::movimientos::listar_detalle(&guard, periodo.id, &filtro.unwrap_or_default())
}

/// Captura rápida de gasto hormiga: monto + categoría, con fecha de hoy.
/// Es el camino de menos clics; todo lo demás queda por omisión.
#[tauri::command]
pub fn captura_rapida(
    estado: State<'_, EstadoApp>,
    monto: Monto,
    categoria_id: i64,
    medio_pago: Option<MedioPago>,
    descripcion: Option<String>,
) -> Resultado<i64> {
    let datos = NuevoMovimiento {
        fecha: fechas::a_iso(fechas::hoy()),
        monto,
        tipo: TipoMovimiento::Gasto,
        categoria_id: Some(categoria_id),
        servicio_id: None,
        medio_pago,
        descripcion,
    };

    registrar_movimiento(estado, datos)
}

fn validar(datos: &NuevoMovimiento) -> Resultado<()> {
    if datos.monto <= 0 {
        return Err(AppError::validacion("El monto debe ser mayor a 0."));
    }
    if datos.servicio_id.is_some() && datos.tipo == TipoMovimiento::Ingreso {
        return Err(AppError::validacion(
            "Un ingreso no puede estar asociado a un servicio.",
        ));
    }
    Ok(())
}
