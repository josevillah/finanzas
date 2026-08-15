use tauri::State;

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::periodo::{Periodo, ResumenPeriodo};
use crate::repos;
use crate::EstadoApp;

/// Devuelve el período del mes, creándolo si hace falta.
#[tauri::command]
pub fn obtener_periodo(estado: State<'_, EstadoApp>, anio: i32, mes: u32) -> Resultado<Periodo> {
    validar_mes(mes)?;
    let guard = estado.conn();
    repos::periodos::obtener_o_crear(&guard, anio, mes)
}

#[tauri::command]
pub fn listar_periodos(estado: State<'_, EstadoApp>) -> Resultado<Vec<Periodo>> {
    let guard = estado.conn();
    repos::periodos::listar(&guard)
}

#[tauri::command]
pub fn guardar_ingresos_periodo(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    sueldo_liquido: Monto,
    otros_ingresos: Monto,
) -> Resultado<Periodo> {
    validar_mes(mes)?;
    if sueldo_liquido < 0 || otros_ingresos < 0 {
        return Err(AppError::validacion("Los ingresos no pueden ser negativos."));
    }

    let guard = estado.conn();
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;
    repos::periodos::exigir_abierto(&guard, periodo.id)?;
    repos::periodos::actualizar_ingresos(&guard, anio, mes, sueldo_liquido, otros_ingresos)?;
    repos::periodos::obtener_o_crear(&guard, anio, mes)
}

/// Cierra o reabre el mes. Cerrado = no se aceptan cambios en sus movimientos.
#[tauri::command]
pub fn cambiar_estado_periodo(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    nuevo_estado: String,
) -> Resultado<Periodo> {
    validar_mes(mes)?;
    let guard = estado.conn();
    repos::periodos::obtener_o_crear(&guard, anio, mes)?;
    repos::periodos::cambiar_estado(&guard, anio, mes, &nuevo_estado)?;
    repos::periodos::obtener_o_crear(&guard, anio, mes)
}

/// Foto del mes: ingresos, gastos, balance y desglose por categoría.
#[tauri::command]
pub fn resumen_periodo(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
) -> Resultado<ResumenPeriodo> {
    validar_mes(mes)?;

    let guard = estado.conn();
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;

    let (total_gastos, ingresos_extra, total_cuotas, total_hormiga, n_movimientos) =
        repos::movimientos::totales(&guard, periodo.id)?;
    let por_categoria = repos::movimientos::por_categoria(&guard, periodo.id)?;

    // Los ingresos del período (sueldo + otros) se suman a los ingresos que
    // hayas registrado como movimiento suelto.
    let total_ingresos = periodo.sueldo_liquido + periodo.otros_ingresos + ingresos_extra;

    Ok(ResumenPeriodo {
        total_ingresos,
        ingresos_extra,
        total_gastos,
        balance: total_ingresos - total_gastos,
        total_cuotas,
        total_hormiga,
        n_movimientos,
        por_categoria,
        periodo,
    })
}

fn validar_mes(mes: u32) -> Resultado<()> {
    if !(1..=12).contains(&mes) {
        return Err(AppError::validacion(format!("Mes inválido: {mes}")));
    }
    Ok(())
}
