use std::collections::BTreeMap;

use chrono::Datelike;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::deuda::{CargaFinanciera, FechaLibertad, Liberacion, MesCarga, Semaforo};
use crate::repos;
use crate::EstadoApp;

/// Umbrales del semáforo de carga financiera, en % del sueldo líquido.
const UMBRAL_VERDE: f64 = 15.0;
const UMBRAL_AMARILLO: f64 = 25.0;

/// Total de cuotas comprometidas mes a mes, partiendo del mes en curso.
/// Devuelve siempre `meses` elementos: los meses sin cuotas van en 0 para que
/// el gráfico de barras no tenga huecos.
#[tauri::command]
pub fn calendario_carga(estado: State<'_, EstadoApp>, meses: Option<u32>) -> Resultado<Vec<MesCarga>> {
    let meses = meses.unwrap_or(24).clamp(1, 120);

    let hoy = fechas::hoy();
    let inicio = fechas::primer_dia(hoy.year(), hoy.month())?;
    let fin_mes = fechas::avanzar_meses(inicio, meses - 1);
    let fin = fechas::ultimo_dia(fin_mes.year(), fin_mes.month())?;

    let guard = estado.conn();
    let filas = repos::cuotas::carga_por_mes(&guard, &fechas::a_iso(inicio), &fechas::a_iso(fin))?;

    let por_clave: BTreeMap<String, (Monto, Monto, i32)> = filas
        .into_iter()
        .map(|(clave, total, pendiente, n)| (clave, (total, pendiente, n)))
        .collect();

    let calendario = (0..meses)
        .map(|offset| {
            let m = fechas::avanzar_meses(inicio, offset);
            let clave = format!("{:04}-{:02}", m.year(), m.month());
            let (total, total_pendiente, n_cuotas) =
                por_clave.get(&clave).copied().unwrap_or((0, 0, 0));

            MesCarga {
                anio: m.year(),
                mes: m.month(),
                clave,
                total,
                total_pendiente,
                n_cuotas,
            }
        })
        .collect();

    Ok(calendario)
}

/// Cuotas del mes divididas por el sueldo líquido del período, con semáforo.
/// Sin argumentos usa el mes en curso.
#[tauri::command]
pub fn carga_financiera(
    estado: State<'_, EstadoApp>,
    anio: Option<i32>,
    mes: Option<u32>,
) -> Resultado<CargaFinanciera> {
    let hoy = fechas::hoy();
    let anio = anio.unwrap_or_else(|| hoy.year());
    let mes = mes.unwrap_or_else(|| hoy.month());

    if !(1..=12).contains(&mes) {
        return Err(AppError::validacion(format!("Mes inválido: {mes}")));
    }

    let desde = fechas::a_iso(fechas::primer_dia(anio, mes)?);
    let hasta = fechas::a_iso(fechas::ultimo_dia(anio, mes)?);

    let guard = estado.conn();
    let (total_cuotas, n_cuotas) = repos::cuotas::total_en_rango(&guard, &desde, &hasta)?;
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;

    let porcentaje = if periodo.sueldo_liquido > 0 {
        Some((total_cuotas as f64 / periodo.sueldo_liquido as f64) * 100.0)
    } else {
        None
    };

    let semaforo = match porcentaje {
        None => Semaforo::SinDatos,
        Some(p) if p < UMBRAL_VERDE => Semaforo::Verde,
        Some(p) if p <= UMBRAL_AMARILLO => Semaforo::Amarillo,
        Some(_) => Semaforo::Rojo,
    };

    Ok(CargaFinanciera {
        anio,
        mes,
        total_cuotas,
        sueldo_liquido: periodo.sueldo_liquido,
        otros_ingresos: periodo.otros_ingresos,
        porcentaje,
        semaforo,
        n_cuotas,
    })
}

/// Mes en que vence la última cuota vigente y cuánto se libera al terminar
/// cada deuda.
#[tauri::command]
pub fn fecha_libertad(estado: State<'_, EstadoApp>) -> Resultado<FechaLibertad> {
    let guard = estado.conn();
    let pendientes = repos::cuotas::pendientes_con_deuda(&guard)?;

    // BTreeMap para que el agrupado sea determinista entre corridas.
    let mut por_deuda: BTreeMap<i64, (String, Vec<Monto>, String)> = BTreeMap::new();
    let mut total_pendiente: Monto = 0;

    for (cuota, descripcion) in pendientes {
        total_pendiente += cuota.monto;

        let entrada = por_deuda
            .entry(cuota.deuda_id)
            .or_insert_with(|| (descripcion, Vec::new(), String::new()));

        entrada.1.push(cuota.monto);
        if cuota.fecha_vencimiento > entrada.2 {
            // Las fechas ISO ordenan lexicográficamente igual que cronológicamente.
            entrada.2 = cuota.fecha_vencimiento;
        }
    }

    let mut liberaciones: Vec<Liberacion> = por_deuda
        .into_iter()
        .map(|(deuda_id, (descripcion, montos, fecha_ultima))| Liberacion {
            deuda_id,
            descripcion,
            fecha_ultima_cuota: fecha_ultima,
            monto_mensual_liberado: mediana(montos.clone()),
            cuotas_restantes: montos.len() as i32,
        })
        .collect();

    liberaciones.sort_by(|a, b| {
        a.fecha_ultima_cuota
            .cmp(&b.fecha_ultima_cuota)
            .then_with(|| a.descripcion.cmp(&b.descripcion))
    });

    let fecha_ultima_cuota = liberaciones
        .iter()
        .map(|l| l.fecha_ultima_cuota.clone())
        .max();

    let meses_restantes = match &fecha_ultima_cuota {
        Some(f) => Some(fechas::meses_entre(fechas::hoy(), fechas::desde_iso(f)?)),
        None => None,
    };

    Ok(FechaLibertad {
        fecha_ultima_cuota,
        meses_restantes,
        total_pendiente,
        liberaciones,
    })
}

/// Cuota mensual representativa de una deuda. Se usa la mediana en vez del
/// promedio porque la última cuota suele traer el residuo del reparto.
fn mediana(mut montos: Vec<Monto>) -> Monto {
    if montos.is_empty() {
        return 0;
    }
    montos.sort_unstable();
    montos[montos.len() / 2]
}
