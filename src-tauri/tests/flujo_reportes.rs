//! Reportes: evolución del gasto por categoría y gastos hormiga.
//!
//! El módulo no tenía tests, y por eso sobrevivió un promedio que dividía por
//! el largo de la ventana en vez de por los meses con gasto: una categoría con
//! $82.696 en un solo mes se mostraba como "$13.782 al mes", que se lee como
//! un gasto recurrente que no existe.

use finanzas_lib::comandos::reportes::{armar_evolucion, armar_hormiga};
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::modelos::categoria::{NuevaCategoria, TipoCategoria};
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::modelos::reporte::{EvolucionGastos, SerieCategoria};
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
        .unwrap()
        .id
}

/// Registra un gasto en el mes indicado.
fn gasto(conn: &Connection, anio: i32, mes: u32, categoria: i64, monto: i64) {
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::movimientos::insertar(
        conn,
        periodo.id,
        &NuevoMovimiento {
            fecha: format!("{anio:04}-{mes:02}-10"),
            monto,
            tipo: TipoMovimiento::Gasto,
            categoria_id: Some(categoria),
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();
}

fn serie<'a>(r: &'a EvolucionGastos, nombre: &str) -> &'a SerieCategoria {
    r.series
        .iter()
        .find(|s| s.categoria_nombre == nombre)
        .unwrap_or_else(|| panic!("no está la serie «{nombre}»"))
}

/// Ventana de 6 meses terminando en agosto de 2026: marzo a agosto.
fn evolucion(conn: &Connection) -> EvolucionGastos {
    armar_evolucion(conn, 2026, 8, Some(6)).unwrap()
}

// ── promedio por categoría ───────────────────────────────────────────────────

#[test]
fn una_categoria_con_gasto_en_un_solo_mes_no_se_promedia_por_la_ventana() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    // El caso exacto del reporte: todo en agosto.
    gasto(&conn, 2026, 8, super_id, 82_696);

    let s = serie(&evolucion(&conn), "Supermercado").clone();

    assert_eq!(s.total, 82_696);
    assert_eq!(s.meses_con_gasto, 1);
    assert_eq!(
        s.promedio, 82_696,
        "gastó una vez: el promedio es ese gasto, no la sexta parte"
    );
}

#[test]
fn una_categoria_recurrente_promedia_igual_que_antes() {
    let conn = base();
    let arriendo = categoria_id(&conn, "Arriendo / Dividendo");

    for mes in 3..=8 {
        gasto(&conn, 2026, mes, arriendo, 143_335);
    }

    let s = serie(&evolucion(&conn), "Arriendo / Dividendo").clone();

    assert_eq!(s.meses_con_gasto, 6);
    assert_eq!(
        s.promedio, 143_335,
        "con gasto en los 6 meses el denominador es el mismo de siempre"
    );
}

#[test]
fn una_categoria_intermitente_divide_por_los_meses_en_que_aparecio() {
    let conn = base();
    let salud = categoria_id(&conn, "Salud");

    gasto(&conn, 2026, 4, salud, 30_000);
    gasto(&conn, 2026, 7, salud, 20_000);

    let s = serie(&evolucion(&conn), "Salud").clone();

    assert_eq!(s.total, 50_000);
    assert_eq!(s.meses_con_gasto, 2);
    assert_eq!(s.promedio, 25_000);
}

#[test]
fn el_promedio_trunca_como_el_resto_de_la_aplicacion() {
    let conn = base();
    let hogar = categoria_id(&conn, "Hogar");

    gasto(&conn, 2026, 7, hogar, 10_000);
    gasto(&conn, 2026, 8, hogar, 10_001);

    let s = serie(&evolucion(&conn), "Hogar").clone();
    assert_eq!(s.promedio, 10_000, "20.001 / 2 = 10.000,5 -> 10.000");
}

// ── promedio general ─────────────────────────────────────────────────────────

#[test]
fn el_promedio_general_ignora_los_meses_sin_gasto() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    // Cinco meses con gasto dentro de una ventana de seis: marzo queda vacío,
    // que es lo que pasa cuando la ventana llega más atrás que el historial.
    for mes in 4..=8 {
        gasto(&conn, 2026, mes, super_id, 100_000);
    }

    let r = evolucion(&conn);

    assert_eq!(r.total_ventana, 500_000);
    assert_eq!(r.meses.len(), 6, "la ventana sigue siendo de 6 meses");
    assert_eq!(r.meses_con_gasto, 5);
    assert_eq!(r.promedio_mensual, 100_000, "500.000 / 5, no / 6");
}

#[test]
fn con_gasto_en_toda_la_ventana_el_promedio_general_no_cambia() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    for mes in 3..=8 {
        gasto(&conn, 2026, mes, super_id, 60_000);
    }

    let r = evolucion(&conn);
    assert_eq!(r.meses_con_gasto, 6);
    assert_eq!(r.promedio_mensual, 60_000);
}

#[test]
fn una_ventana_sin_ningun_gasto_no_inventa_un_promedio() {
    let conn = base();
    let r = evolucion(&conn);

    assert_eq!(r.total_ventana, 0);
    assert_eq!(r.meses_con_gasto, 0);
    assert_eq!(r.promedio_mensual, 0, "sin meses que promediar, cero");
    assert!(r.series.is_empty());
}

#[test]
fn el_periodo_que_cubre_el_reporte_viaja_en_las_claves() {
    let conn = base();
    let r = evolucion(&conn);

    // La pantalla arma "marzo a agosto de 2026" desde estas dos claves.
    assert_eq!(r.meses.first().map(String::as_str), Some("2026-03"));
    assert_eq!(r.meses.last().map(String::as_str), Some("2026-08"));
}

// ── la serie "Otras" ─────────────────────────────────────────────────────────

#[test]
fn la_serie_otras_tambien_promedia_por_sus_meses_con_gasto() {
    let conn = base();

    // Ocho categorías para forzar el agrupado (se dibujan 6 y el resto va a
    // "Otras"). Las dos más chicas caen ahí, cada una en un mes distinto.
    let pesadas = [
        "Arriendo / Dividendo",
        "Supermercado",
        "Café y snacks",
        "Servicios básicos",
        "Salidas y carrete",
        "Transporte",
    ];
    for (i, nombre) in pesadas.iter().enumerate() {
        let id = categoria_id(&conn, nombre);
        gasto(&conn, 2026, 8, id, 500_000 - (i as i64 * 10_000));
    }

    let salud = categoria_id(&conn, "Salud");
    let hogar = categoria_id(&conn, "Hogar");
    gasto(&conn, 2026, 7, salud, 5_000);
    gasto(&conn, 2026, 8, hogar, 4_000);

    let r = evolucion(&conn);
    let otras = r
        .series
        .iter()
        .find(|s| s.categoria_nombre.starts_with("Otras"))
        .expect("con 8 categorías tiene que aparecer la serie agrupada");

    assert_eq!(otras.total, 9_000);
    assert_eq!(otras.meses_con_gasto, 2, "julio y agosto");
    assert_eq!(otras.promedio, 4_500);
}

// ── hormigas ─────────────────────────────────────────────────────────────────

/// Una categoría hormiga propia, para no depender de las semillas.
fn categoria_hormiga(conn: &Connection) -> i64 {
    repos::categorias::insertar(
        conn,
        &NuevaCategoria {
            nombre: "Cafecitos".into(),
            tipo: TipoCategoria::Hormiga,
            color: None,
            activa: true,
        },
    )
    .unwrap()
}

#[test]
fn el_promedio_de_hormigas_ignora_los_meses_sin_hormigas() {
    let conn = base();
    let cafe = categoria_hormiga(&conn);

    // Solo dos meses previos con hormigas, dentro de una ventana de seis.
    gasto(&conn, 2026, 6, cafe, 40_000);
    gasto(&conn, 2026, 7, cafe, 60_000);
    gasto(&conn, 2026, 8, cafe, 50_000);

    let r = armar_hormiga(&conn, 2026, 8, Some(6)).unwrap();

    assert_eq!(r.meses_previos_con_gasto, 2);
    assert_eq!(
        r.promedio_previos, 50_000,
        "(40.000 + 60.000) / 2, no / 5 meses previos"
    );

    // Y la comparación deja de ser alarmista: el mes actual está en el
    // promedio, no un 200% por encima de una referencia hundida por ceros.
    assert_eq!(r.variacion_promedio, Some(0.0));
}

#[test]
fn sin_meses_previos_con_hormigas_no_hay_promedio_ni_variacion() {
    let conn = base();
    let cafe = categoria_hormiga(&conn);

    gasto(&conn, 2026, 8, cafe, 50_000);

    let r = armar_hormiga(&conn, 2026, 8, Some(6)).unwrap();

    assert_eq!(r.meses_previos_con_gasto, 0);
    assert_eq!(r.promedio_previos, 0);
    assert_eq!(
        r.variacion_promedio, None,
        "sin base contra la cual comparar no se reporta un porcentaje"
    );
}

#[test]
fn la_variacion_contra_el_mes_anterior_no_cambio_de_criterio() {
    let conn = base();
    let cafe = categoria_hormiga(&conn);

    gasto(&conn, 2026, 7, cafe, 40_000);
    gasto(&conn, 2026, 8, cafe, 50_000);

    let r = armar_hormiga(&conn, 2026, 8, Some(6)).unwrap();

    // Sigue siendo contra el mes inmediatamente anterior, tenga o no gasto.
    assert_eq!(r.variacion_mes_anterior, Some(25.0));
}
