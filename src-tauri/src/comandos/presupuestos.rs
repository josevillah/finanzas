use std::collections::{HashMap, HashSet};

use tauri::State;

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::presupuesto::{
    AsignacionPresupuesto, EstadoPresupuesto, LineaPresupuesto, ResumenPresupuesto,
};
use crate::repos;
use crate::EstadoApp;

/// Sobre este porcentaje del monto asignado la categoría entra en alerta.
const UMBRAL_ALERTA: f64 = 80.0;

/// Presupuesto del mes cruzado con lo realmente gastado.
#[tauri::command]
pub fn resumen_presupuesto(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
) -> Resultado<ResumenPresupuesto> {
    validar_mes(mes)?;

    let guard = estado.conn();
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;

    let asignados = repos::presupuestos::por_categoria(&guard, periodo.id)?;
    let gastos = repos::movimientos::por_categoria(&guard, periodo.id)?;
    let categorias = repos::categorias::listar(&guard, false)?;

    // Lo gastado por categoría, y aparte lo que no tiene categoría asignada.
    let mut gasto_por_cat: HashMap<i64, (Monto, i32)> = HashMap::new();
    let mut gasto_sin_categoria: Monto = 0;
    let mut total_gastos_mes: Monto = 0;

    for g in &gastos {
        total_gastos_mes += g.total;
        match g.categoria_id {
            Some(id) => {
                gasto_por_cat.insert(id, (g.total, g.n_movimientos));
            }
            None => gasto_sin_categoria += g.total,
        }
    }

    // Entran al listado todas las categorías activas, más las inactivas que
    // igual tengan plata puesta o gastada este mes.
    // Las categorías de ingreso quedan fuera: el presupuesto es sobre lo que
    // sale, y una línea de "Préstamos cobrados" con $0 gastado solo confunde.
    let relevantes: HashSet<i64> = categorias
        .iter()
        .filter(|c| c.tipo.es_de_gasto())
        .filter(|c| c.activa || asignados.contains_key(&c.id) || gasto_por_cat.contains_key(&c.id))
        .map(|c| c.id)
        .collect();

    let mut lineas: Vec<LineaPresupuesto> = categorias
        .into_iter()
        .filter(|c| relevantes.contains(&c.id))
        .map(|c| {
            let monto_asignado = asignados.get(&c.id).copied().unwrap_or(0);
            let (monto_gastado, n_movimientos) =
                gasto_por_cat.get(&c.id).copied().unwrap_or((0, 0));

            let porcentaje_usado = if monto_asignado > 0 {
                Some((monto_gastado as f64 / monto_asignado as f64) * 100.0)
            } else {
                None
            };

            let estado = match porcentaje_usado {
                None => EstadoPresupuesto::SinAsignar,
                Some(p) if p > 100.0 => EstadoPresupuesto::Excedido,
                Some(p) if p >= UMBRAL_ALERTA => EstadoPresupuesto::Alerta,
                Some(_) => EstadoPresupuesto::Ok,
            };

            LineaPresupuesto {
                categoria_id: c.id,
                categoria_nombre: c.nombre,
                categoria_tipo: c.tipo.como_texto().to_string(),
                color: c.color,
                monto_asignado,
                monto_gastado,
                disponible: monto_asignado - monto_gastado,
                porcentaje_usado,
                estado,
                n_movimientos,
            }
        })
        .collect();

    // Primero lo que tiene presupuesto, y dentro de eso lo más consumido.
    lineas.sort_by(|a, b| {
        (b.monto_asignado > 0)
            .cmp(&(a.monto_asignado > 0))
            .then_with(|| {
                b.porcentaje_usado
                    .unwrap_or(-1.0)
                    .total_cmp(&a.porcentaje_usado.unwrap_or(-1.0))
            })
            .then_with(|| b.monto_gastado.cmp(&a.monto_gastado))
            .then_with(|| a.categoria_nombre.cmp(&b.categoria_nombre))
    });

    let total_asignado: Monto = lineas.iter().map(|l| l.monto_asignado).sum();
    let total_gastado: Monto = lineas
        .iter()
        .filter(|l| l.monto_asignado > 0)
        .map(|l| l.monto_gastado)
        .sum();

    let gasto_sin_presupuestar = lineas
        .iter()
        .filter(|l| l.monto_asignado == 0)
        .map(|l| l.monto_gastado)
        .sum::<Monto>()
        + gasto_sin_categoria;

    let porcentaje_usado = if total_asignado > 0 {
        Some((total_gastado as f64 / total_asignado as f64) * 100.0)
    } else {
        None
    };

    let (_, ingresos_extra, _, _, _) = repos::movimientos::totales(&guard, periodo.id)?;
    let total_ingresos = periodo.sueldo_liquido + periodo.otros_ingresos + ingresos_extra;

    Ok(ResumenPresupuesto {
        anio,
        mes,
        total_asignado,
        total_gastado,
        disponible: total_asignado - total_gastado,
        porcentaje_usado,
        gasto_sin_presupuestar,
        total_gastos_mes,
        total_ingresos,
        sin_asignar_del_ingreso: total_ingresos - total_asignado,
        categorias_excedidas: lineas
            .iter()
            .filter(|l| l.estado == EstadoPresupuesto::Excedido)
            .count() as i32,
        periodo_cerrado: periodo.estado == "cerrado",
        lineas,
    })
}

/// Guarda varias asignaciones de una vez. Un monto de 0 borra la línea.
#[tauri::command]
pub fn asignar_presupuesto(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
    asignaciones: Vec<AsignacionPresupuesto>,
) -> Resultado<()> {
    validar_mes(mes)?;

    if asignaciones.iter().any(|a| a.monto_asignado < 0) {
        return Err(AppError::validacion(
            "Un presupuesto no puede ser negativo.",
        ));
    }

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let periodo = repos::periodos::obtener_o_crear(&tx, anio, mes)?;
    repos::periodos::exigir_abierto(&tx, periodo.id)?;

    for a in &asignaciones {
        repos::presupuestos::asignar(&tx, periodo.id, a.categoria_id, a.monto_asignado)?;
    }

    tx.commit()?;
    Ok(())
}

/// Copia el presupuesto de un mes a otro. Sirve para arrancar el mes nuevo sin
/// tener que escribir todo de nuevo.
#[tauri::command]
pub fn copiar_presupuesto(
    estado: State<'_, EstadoApp>,
    desde_anio: i32,
    desde_mes: u32,
    hacia_anio: i32,
    hacia_mes: u32,
) -> Resultado<i32> {
    validar_mes(desde_mes)?;
    validar_mes(hacia_mes)?;

    if (desde_anio, desde_mes) == (hacia_anio, hacia_mes) {
        return Err(AppError::validacion(
            "El mes de origen y el de destino son el mismo.",
        ));
    }

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let origen = repos::periodos::obtener(&tx, desde_anio, desde_mes)?.ok_or_else(|| {
        AppError::no_encontrado(format!("el período {desde_mes:02}/{desde_anio}"))
    })?;
    let destino = repos::periodos::obtener_o_crear(&tx, hacia_anio, hacia_mes)?;
    repos::periodos::exigir_abierto(&tx, destino.id)?;

    let copiadas = repos::presupuestos::copiar(&tx, origen.id, destino.id)?;
    if copiadas == 0 {
        return Err(AppError::conflicto(format!(
            "El período {desde_mes:02}/{desde_anio} no tiene presupuesto que copiar."
        )));
    }

    tx.commit()?;
    Ok(copiadas)
}

fn validar_mes(mes: u32) -> Resultado<()> {
    if !(1..=12).contains(&mes) {
        return Err(AppError::validacion(format!("Mes inválido: {mes}")));
    }
    Ok(())
}
