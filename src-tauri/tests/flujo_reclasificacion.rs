//! Editar un servicio tiene que arrastrar los gastos que ya generó en el mes
//! en curso, y solo esos.

use chrono::NaiveDate;
use finanzas_lib::comandos::servicios::aplicar_actualizacion;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::modelos::movimiento::FiltroMovimientos;
use finanzas_lib::modelos::servicio::{NuevoServicio, TipoServicio};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn categoria_id(conn: &Connection, nombre: &str) -> i64 {
    repos::categorias::listar(conn, false)
        .unwrap()
        .into_iter()
        .find(|c| c.nombre == nombre)
        .unwrap_or_else(|| panic!("falta la categoría semilla '{nombre}'"))
        .id
}

fn dia(anio: i32, mes: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(anio, mes, d).unwrap()
}

fn datos(nombre: &str, categoria_id: Option<i64>, monto: i64) -> NuevoServicio {
    NuevoServicio {
        nombre: nombre.into(),
        categoria_id,
        monto_estimado: monto,
        dia_vencimiento: Some(8),
        tipo: TipoServicio::Suscripcion,
        activo: true,
        fecha_alta: None,
    }
}

fn crear(conn: &Connection, nombre: &str, categoria: i64, monto: i64, alta: &str) -> i64 {
    repos::servicios::insertar(conn, &datos(nombre, Some(categoria), monto), alta).unwrap()
}

/// Réplica del comando `generar_gastos_servicios` sin la capa de Tauri.
fn generar(conn: &Connection, anio: i32, mes: u32) {
    use finanzas_lib::dominio::fechas;
    use std::collections::HashSet;

    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    if periodo.estado == "cerrado" {
        return;
    }

    let ultimo = fechas::a_iso(fechas::ultimo_dia(anio, mes).unwrap());
    let dias_mes = fechas::dias_del_mes(anio, mes);

    let ya: HashSet<i64> = repos::movimientos::servicios_con_gasto(conn, periodo.id)
        .unwrap()
        .into_iter()
        .collect();

    for s in repos::servicios::listar(conn, true).unwrap() {
        if ya.contains(&s.id) || s.monto_estimado <= 0 {
            continue;
        }
        if s.fecha_alta.as_deref().map(|a| a > ultimo.as_str()) == Some(true) {
            continue;
        }

        let d = s
            .dia_vencimiento
            .map(|v| (v.clamp(1, 31) as u32).min(dias_mes))
            .unwrap_or(1);

        repos::movimientos::insertar_estimado_servicio(
            conn,
            periodo.id,
            s.id,
            s.categoria_id,
            &format!("{anio:04}-{mes:02}-{d:02}"),
            s.monto_estimado,
            &s.nombre,
        )
        .unwrap();
    }
}

/// Categoría con la que quedó el gasto de un servicio en un mes.
fn categoria_del_gasto(conn: &Connection, servicio: i64, anio: i32, mes: u32) -> Option<i64> {
    let periodo = repos::periodos::obtener(conn, anio, mes).unwrap().unwrap();
    repos::movimientos::listar_detalle(conn, periodo.id, &FiltroMovimientos::default())
        .unwrap()
        .into_iter()
        .find(|m| m.movimiento.servicio_id == Some(servicio))
        .and_then(|m| m.movimiento.categoria_id)
}

// ── categoría ────────────────────────────────────────────────────────────────

#[test]
fn cambiar_la_categoria_reclasifica_los_gastos_del_mes_actual() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-08-01");
    generar(&conn, 2026, 8);
    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 8), Some(cafe));

    let cambios = aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(basicos), 35_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(cambios.reclasificados, 1);
    assert_eq!(
        categoria_del_gasto(&conn, servicio, 2026, 8),
        Some(basicos),
        "el gasto del mes debe seguir al servicio"
    );
}

#[test]
fn los_meses_anteriores_conservan_su_categoria() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-06-01");
    generar(&conn, 2026, 6);
    generar(&conn, 2026, 7);
    generar(&conn, 2026, 8);

    aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(basicos), 35_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 8), Some(basicos));
    assert_eq!(
        categoria_del_gasto(&conn, servicio, 2026, 7),
        Some(cafe),
        "reescribir el pasado cambiaría reportes que el usuario ya dio por buenos"
    );
    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 6), Some(cafe));
}

#[test]
fn el_mes_que_manda_es_el_real_no_el_que_se_este_mirando() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-07-01");
    generar(&conn, 2026, 7);
    generar(&conn, 2026, 8);

    // El usuario está parado en julio en la pantalla, pero hoy es agosto.
    aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(basicos), 35_000),
        dia(2026, 8, 3),
    )
    .unwrap();

    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 8), Some(basicos));
    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 7), Some(cafe));
}

#[test]
fn un_mes_cerrado_no_se_reclasifica() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-08-01");
    generar(&conn, 2026, 8);
    repos::periodos::cambiar_estado(&conn, 2026, 8, "cerrado").unwrap();

    let cambios = aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(basicos), 35_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(cambios.reclasificados, 0);
    assert_eq!(
        categoria_del_gasto(&conn, servicio, 2026, 8),
        Some(cafe),
        "un mes cerrado está congelado también para los ajustes automáticos"
    );
    assert_eq!(
        repos::servicios::obtener(&conn, servicio).unwrap().categoria_id,
        Some(basicos),
        "el servicio sí queda actualizado"
    );
}

#[test]
fn editar_solo_el_nombre_no_mueve_nada() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-08-01");
    generar(&conn, 2026, 8);

    let cambios = aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix Premium", Some(cafe), 35_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(cambios, Default::default(), "no había nada que reclasificar");
    assert_eq!(
        repos::servicios::obtener(&conn, servicio).unwrap().nombre,
        "Netflix Premium"
    );
}

// ── monto estimado ───────────────────────────────────────────────────────────

#[test]
fn cambiar_el_monto_estimado_actualiza_el_gasto_sin_confirmar() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");

    let servicio = crear(&conn, "Netflix", cafe, 9_900, "2026-08-01");
    generar(&conn, 2026, 8);

    let cambios = aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(cafe), 12_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(cambios.estimados_actualizados, 1);

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let reales = repos::movimientos::real_por_servicio(&conn, periodo.id).unwrap();
    assert_eq!(reales[0].1, 12_000);
}

#[test]
fn un_monto_ya_confirmado_a_mano_no_se_pisa() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");

    let servicio = crear(&conn, "Enel", cafe, 45_000, "2026-08-01");
    generar(&conn, 2026, 8);

    // Llegó la boleta y el usuario fijó el monto real.
    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let id = repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
        .unwrap()[0]
        .movimiento
        .id;
    repos::movimientos::cambiar_monto(&conn, id, 51_300).unwrap();

    aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Enel", Some(cafe), 60_000),
        dia(2026, 8, 20),
    )
    .unwrap();

    assert_eq!(
        repos::movimientos::obtener(&conn, id).unwrap().monto,
        51_300,
        "el monto que el usuario confirmó es suyo y no se sobreescribe"
    );
}

// ── bordes ───────────────────────────────────────────────────────────────────

#[test]
fn sin_periodo_del_mes_actual_no_revienta() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-08-01");

    // Diciembre nunca se abrió: no hay gastos que ajustar.
    let cambios = aplicar_actualizacion(
        &conn,
        servicio,
        &datos("Netflix", Some(basicos), 35_000),
        dia(2026, 12, 5),
    )
    .unwrap();

    assert_eq!(cambios, Default::default());
    assert_eq!(
        repos::servicios::obtener(&conn, servicio).unwrap().categoria_id,
        Some(basicos)
    );
}

#[test]
fn si_falla_algo_no_queda_el_servicio_cambiado_y_los_gastos_atras() {
    let mut conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");
    let basicos = categoria_id(&conn, "Servicios básicos");

    let servicio = crear(&conn, "Netflix", cafe, 35_000, "2026-08-01");
    generar(&conn, 2026, 8);

    {
        let tx = conn.transaction().unwrap();
        aplicar_actualizacion(
            &tx,
            servicio,
            &datos("Netflix", Some(basicos), 35_000),
            dia(2026, 8, 20),
        )
        .unwrap();
        // Simula un fallo posterior dentro de la misma transacción.
        tx.rollback().unwrap();
    }

    assert_eq!(
        repos::servicios::obtener(&conn, servicio).unwrap().categoria_id,
        Some(cafe),
        "el servicio y sus gastos se aplican juntos o no se aplican"
    );
    assert_eq!(categoria_del_gasto(&conn, servicio, 2026, 8), Some(cafe));
}

// ── categoría del sistema ────────────────────────────────────────────────────

#[test]
fn la_categoria_de_deudas_no_puede_cambiar_de_tipo() {
    use finanzas_lib::comandos::categorias::puede_cambiar_tipo;
    use finanzas_lib::modelos::categoria::{TipoCategoria, CODIGO_DEUDAS};

    let conn = base();
    let deudas = repos::categorias::por_codigo(&conn, CODIGO_DEUDAS)
        .unwrap()
        .unwrap();

    assert_eq!(deudas.tipo, TipoCategoria::Fijo);
    assert!(
        !puede_cambiar_tipo(&deudas, TipoCategoria::Hormiga),
        "marcarla hormiga haría que las cuotas contaran como gasto hormiga"
    );
    assert!(
        puede_cambiar_tipo(&deudas, TipoCategoria::Fijo),
        "guardarla sin tocar el tipo tiene que seguir funcionando"
    );

    let propia = repos::categorias::listar(&conn, false)
        .unwrap()
        .into_iter()
        .find(|c| c.nombre == "Supermercado")
        .unwrap();
    assert!(
        puede_cambiar_tipo(&propia, TipoCategoria::Hormiga),
        "las categorías sin código del sistema se editan libremente"
    );
}
