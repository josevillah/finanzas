use std::collections::BTreeMap;

use rusqlite::Connection;
use tauri::State;

use crate::dominio::dinero::{self, Monto};
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::reporte::{
    EvolucionGastos, MesHormiga, PuntoMes, ReporteHormiga, SerieCategoria,
};
use crate::repos;
use crate::EstadoApp;

/// Ventana por omisión de los reportes, en meses.
const MESES_POR_DEFECTO: u32 = 12;
/// Categorías que se dibujan por separado antes de agrupar el resto en "Otras".
const SERIES_VISIBLES: usize = 6;

/// Evolución del gasto por categoría en los últimos `meses`, terminando en el
/// mes indicado.
#[tauri::command]
pub fn evolucion_gastos(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    meses: Option<u32>,
) -> Resultado<EvolucionGastos> {
    let guard = estado.conn();
    armar_evolucion(&guard, anio, mes, meses)
}

/// Núcleo de la evolución. Recibe `&Connection` para poder cubrirlo con tests
/// sin levantar Tauri, como el resto de los comandos.
pub fn armar_evolucion(
    conn: &Connection,
    anio: i32,
    mes: u32,
    meses: Option<u32>,
) -> Resultado<EvolucionGastos> {
    let ventana = ventana_meses(anio, mes, meses)?;

    let filas =
        repos::movimientos::evolucion_por_categoria(conn, ventana.desde_abs, ventana.hasta_abs)?;

    // (categoria_id, nombre, color) -> mes absoluto -> total
    let mut por_categoria: BTreeMap<(Option<i64>, String, Option<String>), BTreeMap<i64, Monto>> =
        BTreeMap::new();

    for (a, m, categoria_id, nombre, color, total) in filas {
        por_categoria
            .entry((categoria_id, nombre, color))
            .or_default()
            .insert(fechas::mes_absoluto(a, m), total);
    }

    let mut series: Vec<SerieCategoria> = por_categoria
        .into_iter()
        .map(|((categoria_id, categoria_nombre, color), totales)| {
            let puntos = ventana.puntos(&totales);
            let total: Monto = puntos.iter().map(|p| p.total).sum();
            let meses_con_gasto = contar_meses_con_gasto(&puntos);

            SerieCategoria {
                categoria_id,
                categoria_nombre,
                color,
                total,
                promedio: dinero::promedio_mensual(total, meses_con_gasto as i64),
                meses_con_gasto,
                puntos,
            }
        })
        .collect();

    series.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then_with(|| a.categoria_nombre.cmp(&b.categoria_nombre))
    });

    // Con 14 categorías el gráfico se vuelve ilegible: se dejan las más
    // pesadas y el resto se suma en una sola serie.
    if series.len() > SERIES_VISIBLES + 1 {
        let resto: Vec<SerieCategoria> = series.split_off(SERIES_VISIBLES);
        let puntos: Vec<PuntoMes> = ventana
            .claves
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                let total = resto.iter().map(|s| s.puntos[idx].total).sum();
                ventana.punto(idx, total)
            })
            .collect();

        let total: Monto = puntos.iter().map(|p| p.total).sum();
        let meses_con_gasto = contar_meses_con_gasto(&puntos);
        series.push(SerieCategoria {
            categoria_id: None,
            categoria_nombre: format!("Otras ({})", resto.len()),
            color: Some("#94a3b8".into()),
            total,
            promedio: dinero::promedio_mensual(total, meses_con_gasto as i64),
            meses_con_gasto,
            puntos,
        });
    }

    let total_por_mes: Vec<PuntoMes> = ventana
        .claves
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            let total = series.iter().map(|s| s.puntos[idx].total).sum();
            ventana.punto(idx, total)
        })
        .collect();

    let total_ventana: Monto = total_por_mes.iter().map(|p| p.total).sum();
    let meses_con_gasto = contar_meses_con_gasto(&total_por_mes);

    Ok(EvolucionGastos {
        total_ventana,
        meses_con_gasto,
        promedio_mensual: dinero::promedio_mensual(total_ventana, meses_con_gasto as i64),
        meses: ventana.claves.clone(),
        series,
        total_por_mes,
    })
}

/// Cuántos meses de la serie tuvieron gasto.
///
/// Es el denominador de todos los promedios del reporte. Un mes en cero puede
/// serlo porque no se gastó en esa categoría o porque la ventana llega más
/// atrás que el primer registro de la aplicación; en los dos casos meterlo en
/// el divisor inventa una regularidad que no existe.
fn contar_meses_con_gasto(puntos: &[PuntoMes]) -> i32 {
    puntos.iter().filter(|p| p.total > 0).count() as i32
}

/// Gasto hormiga mes a mes, con la comparación contra los meses previos.
#[tauri::command]
pub fn reporte_hormiga(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    meses: Option<u32>,
) -> Resultado<ReporteHormiga> {
    let guard = estado.conn();
    armar_hormiga(&guard, anio, mes, meses)
}

/// Núcleo del reporte de hormigas, sobre `&Connection`.
pub fn armar_hormiga(
    conn: &Connection,
    anio: i32,
    mes: u32,
    meses: Option<u32>,
) -> Resultado<ReporteHormiga> {
    let ventana = ventana_meses(anio, mes, meses)?;

    let filas =
        repos::movimientos::hormiga_por_periodo(conn, ventana.desde_abs, ventana.hasta_abs)?;

    let por_mes: BTreeMap<i64, (Monto, i32, Monto)> = filas
        .into_iter()
        .map(|(a, m, total, n, total_gastos)| {
            (fechas::mes_absoluto(a, m), (total, n, total_gastos))
        })
        .collect();

    // Los meses sin registros van en cero para que el gráfico no tenga huecos.
    let meses_reporte: Vec<MesHormiga> = ventana
        .claves
        .iter()
        .enumerate()
        .map(|(idx, clave)| {
            let abs = ventana.desde_abs + idx as i64;
            let (anio, mes) = fechas::desde_mes_absoluto(abs);
            let (total, n_movimientos, total_gastos) = por_mes.get(&abs).copied().unwrap_or((0, 0, 0));

            MesHormiga {
                anio,
                mes,
                clave: clave.clone(),
                total,
                n_movimientos,
                total_gastos,
                porcentaje: if total_gastos > 0 {
                    Some((total as f64 / total_gastos as f64) * 100.0)
                } else {
                    None
                },
            }
        })
        .collect();

    let mes_actual = meses_reporte.last().cloned();
    let previos = &meses_reporte[..meses_reporte.len().saturating_sub(1)];

    // Mismo criterio que la evolución: promedian solo los meses en que hubo
    // hormigas. Con los vacíos adentro, el promedio bajaba y cualquier mes
    // normal aparecía como un desvío enorme contra una referencia inventada.
    let meses_previos_con_gasto = previos.iter().filter(|m| m.total > 0).count() as i32;
    let promedio_previos = dinero::promedio_mensual(
        previos.iter().map(|m| m.total).sum(),
        meses_previos_con_gasto as i64,
    );

    let variacion_mes_anterior = match (mes_actual.as_ref(), previos.last()) {
        (Some(actual), Some(anterior)) => dinero::variacion_porcentual(anterior.total, actual.total),
        _ => None,
    };

    let variacion_promedio = mes_actual
        .as_ref()
        .and_then(|actual| dinero::variacion_porcentual(promedio_previos, actual.total));

    let periodo = repos::periodos::obtener(conn, anio, mes)?;
    let por_categoria = match periodo {
        Some(p) => repos::movimientos::hormiga_por_categoria(conn, p.id)?,
        None => Vec::new(),
    };

    Ok(ReporteHormiga {
        total_ventana: meses_reporte.iter().map(|m| m.total).sum(),
        mes_actual,
        promedio_previos,
        meses_previos_con_gasto,
        variacion_mes_anterior,
        variacion_promedio,
        por_categoria,
        meses: meses_reporte,
    })
}

// ── auxiliares ───────────────────────────────────────────────────────────────

/// Ventana de meses consecutivos que termina en el mes pedido.
struct Ventana {
    desde_abs: i64,
    hasta_abs: i64,
    claves: Vec<String>,
}

impl Ventana {
    fn punto(&self, idx: usize, total: Monto) -> PuntoMes {
        let (anio, mes) = fechas::desde_mes_absoluto(self.desde_abs + idx as i64);
        PuntoMes {
            anio,
            mes,
            clave: self.claves[idx].clone(),
            total,
        }
    }

    /// Serie completa de la ventana, con ceros donde el mapa no tiene datos.
    fn puntos(&self, totales: &BTreeMap<i64, Monto>) -> Vec<PuntoMes> {
        (0..self.claves.len())
            .map(|idx| {
                let abs = self.desde_abs + idx as i64;
                self.punto(idx, totales.get(&abs).copied().unwrap_or(0))
            })
            .collect()
    }
}

fn ventana_meses(anio: i32, mes: u32, meses: Option<u32>) -> Resultado<Ventana> {
    if !(1..=12).contains(&mes) {
        return Err(AppError::validacion(format!("Mes inválido: {mes}")));
    }

    let cantidad = meses.unwrap_or(MESES_POR_DEFECTO).clamp(2, 60);
    let meses_ventana = fechas::ventana_de_meses(anio, mes, cantidad);

    Ok(Ventana {
        desde_abs: fechas::mes_absoluto(meses_ventana[0].0, meses_ventana[0].1),
        hasta_abs: fechas::mes_absoluto(anio, mes),
        claves: meses_ventana
            .into_iter()
            .map(|(a, m)| fechas::clave_mes(a, m))
            .collect(),
    })
}
