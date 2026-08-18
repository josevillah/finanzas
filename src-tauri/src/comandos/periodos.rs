use std::collections::BTreeMap;

use chrono::Datelike;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::dominio::fechas;
use crate::modelos::periodo::{MesConDatos, Periodo, RangoMeses, ResumenPeriodo};
use crate::repos;
use crate::EstadoApp;

/// Devuelve el período del mes, creándolo si hace falta.
#[tauri::command]
pub fn obtener_periodo(estado: State<'_, EstadoApp>, anio: i32, mes: u32) -> Resultado<Periodo> {
    validar_mes(mes)?;
    let guard = estado.conn();
    repos::periodos::obtener_o_crear(&guard, anio, mes)
}

/// Hasta dónde puede navegar el usuario y qué meses tienen contenido.
///
/// El límite superior es el mes actual: no tiene sentido abrir meses futuros.
/// El inferior son 24 meses hacia atrás, o más si hay datos más antiguos, para
/// que nadie quede encerrado sin poder cargar un mes que le faltaba.
#[tauri::command]
pub fn meses_disponibles(estado: State<'_, EstadoApp>) -> Resultado<RangoMeses> {
    let guard = estado.conn();
    let hoy = fechas::hoy();
    armar_rango(
        &repos::periodos::resumen_de_periodos(&guard)?,
        &repos::cuotas::meses_con_vencimientos(&guard)?,
        hoy.year(),
        hoy.month(),
    )
}

/// Núcleo del rango, sin Tauri ni base de datos, para poder cubrirlo con tests
/// sin depender del día en que se ejecutan.
pub fn armar_rango(
    periodos: &[(i32, u32, i32, i32, bool)],
    vencimientos: &[(i32, u32, i32)],
    anio_actual: i32,
    mes_actual: u32,
) -> Resultado<RangoMeses> {
    let hasta_abs = fechas::mes_absoluto(anio_actual, mes_actual);

    // Un mes puede tener cuotas sin período creado, y un período puede existir
    // vacío solo porque alguien lo miró. Se juntan las dos fuentes y después se
    // filtra por contenido real.
    let mut por_mes: BTreeMap<i64, MesConDatos> = BTreeMap::new();

    for (anio, mes, n_movimientos, n_presupuestos, tiene_ingresos) in periodos {
        por_mes.insert(
            fechas::mes_absoluto(*anio, *mes),
            MesConDatos {
                anio: *anio,
                mes: *mes,
                clave: fechas::clave_mes(*anio, *mes),
                n_movimientos: *n_movimientos,
                n_presupuestos: *n_presupuestos,
                n_cuotas: 0,
                tiene_ingresos: *tiene_ingresos,
            },
        );
    }

    for (anio, mes, n_cuotas) in vencimientos {
        por_mes
            .entry(fechas::mes_absoluto(*anio, *mes))
            .or_insert_with(|| MesConDatos {
                anio: *anio,
                mes: *mes,
                clave: fechas::clave_mes(*anio, *mes),
                n_movimientos: 0,
                n_presupuestos: 0,
                n_cuotas: 0,
                tiene_ingresos: false,
            })
            .n_cuotas = *n_cuotas;
    }

    let meses: Vec<MesConDatos> = por_mes
        .into_iter()
        .filter(|(abs, m)| {
            // Los meses futuros quedan fuera aunque tengan cuotas: el selector
            // solo llega hasta hoy.
            *abs <= hasta_abs && tiene_contenido(m)
        })
        .map(|(_, m)| m)
        .collect();

    let mas_antiguo = meses
        .first()
        .map(|m| fechas::mes_absoluto(m.anio, m.mes))
        .unwrap_or(hasta_abs);

    // 24 meses hacia atrás contando el actual, o más si hay datos más viejos.
    let desde_abs = mas_antiguo.min(hasta_abs - 23).min(hasta_abs);
    let (desde_anio, desde_mes) = fechas::desde_mes_absoluto(desde_abs);

    Ok(RangoMeses {
        desde_anio,
        desde_mes,
        hasta_anio: anio_actual,
        hasta_mes: mes_actual,
        meses,
    })
}

/// Un período creado al vuelo por haberlo mirado no cuenta como con datos.
fn tiene_contenido(m: &MesConDatos) -> bool {
    m.n_movimientos > 0 || m.n_presupuestos > 0 || m.n_cuotas > 0 || m.tiene_ingresos
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

#[cfg(test)]
mod tests {
    use super::*;

    /// (anio, mes, n_movimientos, n_presupuestos, tiene_ingresos)
    fn periodo(anio: i32, mes: u32, movs: i32, presup: i32, ingresos: bool) -> (i32, u32, i32, i32, bool) {
        (anio, mes, movs, presup, ingresos)
    }

    fn claves(rango: &RangoMeses) -> Vec<String> {
        rango.meses.iter().map(|m| m.clave.clone()).collect()
    }

    #[test]
    fn un_periodo_vacio_no_cuenta_como_mes_con_datos() {
        // Es el caso real: `obtener_o_crear` corre desde comandos de lectura,
        // así que mirar un mes ya deja su fila.
        let rango = armar_rango(&[periodo(2026, 7, 0, 0, false)], &[], 2026, 8).unwrap();
        assert!(claves(&rango).is_empty());
    }

    #[test]
    fn cuenta_por_movimientos_presupuesto_o_ingresos() {
        let rango = armar_rango(
            &[
                periodo(2026, 5, 3, 0, false),
                periodo(2026, 6, 0, 2, false),
                periodo(2026, 7, 0, 0, true),
                periodo(2026, 8, 0, 0, false),
            ],
            &[],
            2026,
            8,
        )
        .unwrap();

        assert_eq!(claves(&rango), vec!["2026-05", "2026-06", "2026-07"]);
    }

    #[test]
    fn un_mes_con_cuotas_cuenta_aunque_no_tenga_periodo() {
        let rango = armar_rango(&[], &[(2026, 6, 2)], 2026, 8).unwrap();

        assert_eq!(claves(&rango), vec!["2026-06"]);
        assert_eq!(rango.meses[0].n_cuotas, 2);
    }

    #[test]
    fn las_dos_fuentes_se_combinan_en_un_solo_mes() {
        let rango = armar_rango(&[periodo(2026, 6, 4, 0, false)], &[(2026, 6, 2)], 2026, 8).unwrap();

        assert_eq!(rango.meses.len(), 1, "no debe duplicarse el mes");
        assert_eq!(rango.meses[0].n_movimientos, 4);
        assert_eq!(rango.meses[0].n_cuotas, 2);
    }

    #[test]
    fn los_meses_futuros_quedan_fuera_aunque_tengan_cuotas() {
        let rango = armar_rango(&[], &[(2026, 8, 1), (2026, 9, 1), (2027, 3, 1)], 2026, 8).unwrap();

        assert_eq!(claves(&rango), vec!["2026-08"], "el tope es el mes actual");
        assert_eq!((rango.hasta_anio, rango.hasta_mes), (2026, 8));
    }

    #[test]
    fn sin_datos_el_limite_son_24_meses_atras() {
        let rango = armar_rango(&[], &[], 2026, 8).unwrap();

        // 24 meses contando el actual: de septiembre 2024 a agosto 2026.
        assert_eq!((rango.desde_anio, rango.desde_mes), (2024, 9));
        assert_eq!((rango.hasta_anio, rango.hasta_mes), (2026, 8));
    }

    #[test]
    fn con_datos_recientes_igual_se_puede_retroceder_24_meses() {
        // Tres meses de uso no deben encerrar al usuario: puede querer cargar
        // un mes anterior que nunca registró.
        let rango = armar_rango(&[periodo(2026, 6, 5, 0, false)], &[], 2026, 8).unwrap();

        assert_eq!((rango.desde_anio, rango.desde_mes), (2024, 9));
    }

    #[test]
    fn con_datos_mas_viejos_el_limite_se_estira_hasta_ellos() {
        let rango = armar_rango(&[periodo(2023, 2, 7, 0, false)], &[], 2026, 8).unwrap();

        assert_eq!(
            (rango.desde_anio, rango.desde_mes),
            (2023, 2),
            "no se puede dejar fuera de alcance un mes que sí tiene datos"
        );
    }

    #[test]
    fn el_rango_cruza_el_cambio_de_ano() {
        let rango = armar_rango(&[], &[], 2026, 1).unwrap();
        assert_eq!((rango.desde_anio, rango.desde_mes), (2024, 2));
    }
}
