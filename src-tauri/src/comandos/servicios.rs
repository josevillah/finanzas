use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate};
use rusqlite::Connection;
use tauri::State;

use crate::dominio::dinero::Monto;
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::servicio::{NuevoServicio, ResumenServicios, Servicio, ServicioConReal};
use crate::repos;
use crate::EstadoApp;

#[tauri::command]
pub fn listar_servicios(
    estado: State<'_, EstadoApp>,
    solo_activos: Option<bool>,
) -> Resultado<Vec<Servicio>> {
    let guard = estado.conn();
    repos::servicios::listar(&guard, solo_activos.unwrap_or(false))
}

#[tauri::command]
pub fn crear_servicio(estado: State<'_, EstadoApp>, datos: NuevoServicio) -> Resultado<i64> {
    validar(&datos)?;

    // El alta define desde qué mes el servicio empieza a generar gastos.
    let alta = match datos.fecha_alta.as_deref().map(str::trim) {
        Some(f) if !f.is_empty() => fechas::a_iso(fechas::desde_iso(f)?),
        _ => fechas::a_iso(fechas::hoy()),
    };

    let guard = estado.conn();
    repos::servicios::insertar(&guard, &datos, &alta)
}

/// Ajustes que arrastró editar un servicio sobre los gastos que ya había
/// generado en el mes en curso.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CambiosServicio {
    pub reclasificados: usize,
    pub estimados_actualizados: usize,
}

#[tauri::command]
pub fn actualizar_servicio(
    estado: State<'_, EstadoApp>,
    id: i64,
    datos: NuevoServicio,
) -> Resultado<()> {
    validar(&datos)?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    // El servicio y sus gastos se ajustan juntos: si algo falla, no queda el
    // servicio en una categoría y sus movimientos en otra.
    aplicar_actualizacion(&tx, id, &datos, fechas::hoy())?;

    tx.commit()?;
    Ok(())
}

/// Actualiza el servicio y pone al día los gastos que ya generó **en el mes
/// calendario en curso**.
///
/// `hoy` llega por parámetro en vez de leerse adentro para que los tests
/// puedan fijar el mes; si no, dependerían del día en que se ejecutan.
///
/// Los meses anteriores no se tocan a propósito: una clasificación pasada ya
/// quedó registrada así, y reescribirla cambiaría en silencio reportes que el
/// usuario ya dio por buenos. El mes que se esté mirando en pantalla tampoco
/// influye: lo que manda es el mes real.
pub fn aplicar_actualizacion(
    conn: &Connection,
    id: i64,
    datos: &NuevoServicio,
    hoy: NaiveDate,
) -> Resultado<CambiosServicio> {
    let antes = repos::servicios::obtener(conn, id)?;
    repos::servicios::actualizar(conn, id, datos)?;

    let cambio_categoria = antes.categoria_id != datos.categoria_id;
    let cambio_monto = antes.monto_estimado != datos.monto_estimado;

    // Editar solo el nombre o el día de vencimiento no mueve ningún gasto.
    if !cambio_categoria && !cambio_monto {
        return Ok(CambiosServicio::default());
    }

    // Si el mes en curso todavía no tiene período, no hay gastos que ajustar.
    let Some(periodo) = repos::periodos::obtener(conn, hoy.year(), hoy.month())? else {
        return Ok(CambiosServicio::default());
    };

    // Un mes cerrado está congelado, también para los ajustes automáticos.
    if periodo.estado == "cerrado" {
        return Ok(CambiosServicio::default());
    }

    let mut cambios = CambiosServicio::default();

    if cambio_categoria {
        cambios.reclasificados = repos::movimientos::reclasificar_por_servicio(
            conn,
            id,
            periodo.id,
            datos.categoria_id,
        )?;
    }

    if cambio_monto {
        cambios.estimados_actualizados = repos::movimientos::actualizar_estimado_de_servicio(
            conn,
            id,
            periodo.id,
            datos.monto_estimado,
        )?;
    }

    Ok(cambios)
}

#[tauri::command]
pub fn eliminar_servicio(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();

    let usos = repos::servicios::usos(&guard, id)?;
    if usos > 0 {
        return Err(AppError::conflicto(format!(
            "No se puede eliminar: {usos} gasto(s) apuntan a este servicio. \
             Desactívalo para dejar de generarlo sin perder el historial."
        )));
    }

    repos::servicios::eliminar(&guard, id)
}

/// Materializa el gasto del mes de cada servicio activo que todavía no lo
/// tenga, usando su monto estimado. Es idempotente y no retrocede: un servicio
/// nunca genera gastos en meses anteriores a su fecha de alta.
/// Devuelve cuántos gastos creó.
#[tauri::command]
pub fn generar_gastos_servicios(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
) -> Resultado<i32> {
    validar_mes(mes)?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let periodo = repos::periodos::obtener_o_crear(&tx, anio, mes)?;
    // Un mes cerrado está congelado; se sale sin ruido para que abrirlo a
    // mirar no reviente.
    if periodo.estado == "cerrado" {
        return Ok(0);
    }

    let ultimo_dia = fechas::ultimo_dia(anio, mes)?;
    let dias_mes = fechas::dias_del_mes(anio, mes);

    let ya_tienen: HashSet<i64> = repos::movimientos::servicios_con_gasto(&tx, periodo.id)?
        .into_iter()
        .collect();

    let mut creados = 0;

    for servicio in repos::servicios::listar(&tx, true)? {
        if ya_tienen.contains(&servicio.id) {
            continue;
        }
        if !corresponde_al_mes(&servicio, &fechas::a_iso(ultimo_dia)) {
            continue;
        }
        if servicio.monto_estimado <= 0 {
            continue;
        }

        // El día de vencimiento se recorta al mes: un servicio que vence el 31
        // vence el 28 en febrero.
        let dia = servicio
            .dia_vencimiento
            .map(|d| (d.clamp(1, 31) as u32).min(dias_mes))
            .unwrap_or(1);
        let fecha = format!("{anio:04}-{mes:02}-{dia:02}");

        repos::movimientos::insertar_estimado_servicio(
            &tx,
            periodo.id,
            servicio.id,
            servicio.categoria_id,
            &fecha,
            servicio.monto_estimado,
            &servicio.nombre,
        )?;
        creados += 1;
    }

    tx.commit()?;
    Ok(creados)
}

/// Registra a mano el gasto de un servicio en un mes que su alta no cubre.
///
/// Es la salida para el caso "este servicio ya lo pagaba, pero lo di de alta
/// recién ahora". **No mueve `fecha_alta`**: es una activación puntual para
/// ese mes, no un cambio retroactivo del servicio, así que la generación
/// automática sigue sin retroceder por su cuenta.
#[tauri::command]
pub fn activar_servicio_en_mes(
    estado: State<'_, EstadoApp>,
    servicio_id: i64,
    anio: i32,
    mes: u32,
    monto: Monto,
) -> Resultado<i64> {
    validar_mes(mes)?;

    let mut guard = estado.conn();
    let tx = guard.transaction()?;

    let id = activar_en_mes(&tx, servicio_id, anio, mes, monto)?;

    tx.commit()?;
    Ok(id)
}

/// Núcleo de la activación manual, sin la capa de Tauri para poder cubrirlo
/// con tests.
pub fn activar_en_mes(
    conn: &Connection,
    servicio_id: i64,
    anio: i32,
    mes: u32,
    monto: Monto,
) -> Resultado<i64> {
    if monto <= 0 {
        return Err(AppError::validacion("El monto debe ser mayor a 0."));
    }

    let servicio = repos::servicios::obtener(conn, servicio_id)?;
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes)?;
    repos::periodos::exigir_abierto(conn, periodo.id)?;

    if repos::movimientos::tiene_movimientos_de_servicio(conn, servicio_id, periodo.id)? {
        return Err(AppError::conflicto(format!(
            "{} ya tiene un gasto registrado en ese mes.",
            servicio.nombre
        )));
    }

    // Misma regla de fecha que la generación automática: el día de vencimiento
    // recortado al mes, o el 1 si el servicio no tiene día definido.
    let dia = servicio
        .dia_vencimiento
        .map(|d| (d.clamp(1, 31) as u32).min(fechas::dias_del_mes(anio, mes)))
        .unwrap_or(1);

    repos::movimientos::insertar_activacion_manual(
        conn,
        periodo.id,
        servicio_id,
        servicio.categoria_id,
        &format!("{anio:04}-{mes:02}-{dia:02}"),
        monto,
        &servicio.nombre,
    )
}

/// Estimado vs. real del mes para cada servicio activo.
#[tauri::command]
pub fn resumen_servicios(
    estado: State<'_, EstadoApp>,
    anio: i32,
    mes: u32,
) -> Resultado<ResumenServicios> {
    validar_mes(mes)?;

    let guard = estado.conn();
    let periodo = repos::periodos::obtener_o_crear(&guard, anio, mes)?;
    let servicios = repos::servicios::listar(&guard, true)?;
    let categorias = repos::categorias::listar(&guard, false)?;

    let nombres: HashMap<i64, String> =
        categorias.into_iter().map(|c| (c.id, c.nombre)).collect();

    let reales: HashMap<i64, (i64, i32, i32)> =
        repos::movimientos::real_por_servicio(&guard, periodo.id)?
            .into_iter()
            .map(|(id, total, n, estimados)| (id, (total, n, estimados)))
            .collect();

    let ultimo_dia_iso = fechas::a_iso(fechas::ultimo_dia(anio, mes)?);
    let dias_mes = fechas::dias_del_mes(anio, mes);

    let filas: Vec<ServicioConReal> = servicios
        .into_iter()
        .map(|s| {
            let (monto_real, n_movimientos, n_estimados) =
                reales.get(&s.id).copied().unwrap_or((0, 0, 0));
            let categoria_nombre = s.categoria_id.and_then(|id| nombres.get(&id).cloned());

            let fecha_vencimiento = s.dia_vencimiento.map(|dia| {
                let dia = (dia.clamp(1, 31) as u32).min(dias_mes);
                format!("{anio:04}-{mes:02}-{dia:02}")
            });

            let cubierto = corresponde_al_mes(&s, &ultimo_dia_iso);

            ServicioConReal {
                diferencia: monto_real - s.monto_estimado,
                corresponde_al_mes: cubierto,
                // Activarlo a mano no mueve su fecha de alta: lo que lo hace
                // contar para el mes es tener gasto registrado.
                incluido_en_el_mes: cubierto || n_movimientos > 0,
                categoria_nombre,
                monto_real,
                n_movimientos,
                n_estimados,
                fecha_vencimiento,
                servicio: s,
            }
        })
        .collect();

    // Cuentan los que el alta cubre, más los que se activaron a mano en este
    // mes. Los dados de alta después y sin activar no deben inflar el estimado
    // de meses previos.
    let vigentes: Vec<&ServicioConReal> =
        filas.iter().filter(|f| f.incluido_en_el_mes).collect();

    let total_estimado = vigentes.iter().map(|f| f.servicio.monto_estimado).sum();
    let total_real = vigentes.iter().map(|f| f.monto_real).sum::<i64>();
    let sin_registrar = vigentes.iter().filter(|f| f.n_movimientos == 0).count() as i32;
    let por_confirmar = vigentes.iter().filter(|f| f.n_estimados > 0).count() as i32;

    Ok(ResumenServicios {
        anio,
        mes,
        total_estimado,
        total_real,
        diferencia: total_real - total_estimado,
        sin_registrar,
        por_confirmar,
        periodo_cerrado: periodo.estado == "cerrado",
        servicios: filas,
    })
}

// ── auxiliares ───────────────────────────────────────────────────────────────

/// ¿El servicio ya existía dentro del mes? Las fechas ISO se comparan como
/// texto sin problema. Sin fecha de alta se asume que siempre existió.
fn corresponde_al_mes(servicio: &Servicio, ultimo_dia_del_mes_iso: &str) -> bool {
    match servicio.fecha_alta.as_deref() {
        Some(alta) => alta <= ultimo_dia_del_mes_iso,
        None => true,
    }
}

fn validar_mes(mes: u32) -> Resultado<()> {
    if !(1..=12).contains(&mes) {
        return Err(AppError::validacion(format!("Mes inválido: {mes}")));
    }
    Ok(())
}

fn validar(datos: &NuevoServicio) -> Resultado<()> {
    if datos.nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }
    if datos.monto_estimado < 0 {
        return Err(AppError::validacion(
            "El monto estimado no puede ser negativo.",
        ));
    }
    if let Some(dia) = datos.dia_vencimiento {
        if !(1..=31).contains(&dia) {
            return Err(AppError::validacion(
                "El día de vencimiento debe estar entre 1 y 31.",
            ));
        }
    }
    Ok(())
}
