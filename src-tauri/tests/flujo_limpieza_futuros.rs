//! Migración 0010: la limpieza de los estimados que quedaron en meses futuros.
//!
//! El bug que los generó estuvo en el código que corrieron todos los equipos
//! con la app instalada, así que la limpieza viaja como migración: en las otras
//! máquinas nadie puede correr SQL a mano.
//!
//! Lo que estos tests cuidan es sobre todo lo que la migración **no** debe
//! tocar. El criterio es conservador a propósito: mejor que sobre un fantasma a
//! que se borre algo que alguien registró.

use chrono::Datelike;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::fechas;
use rusqlite::{params, Connection};

/// Base en la versión anterior a la limpieza, para poder poblarla con los datos
/// que tendría en la vida real antes de que la migración corra.
fn base_v9() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar_hasta(&mut conn, 9).expect("migrar hasta la 9");
    conn
}

fn limpiar(conn: &mut Connection) {
    migraciones::ejecutar(conn).expect("aplicar la 0010");
}

// Meses lejanos en los dos sentidos, para que estos tests no dependan del día
// en que se ejecuten.
const FUTURO: (i32, u32) = (2999, 1);
const PASADO: (i32, u32) = (2020, 1);

fn mes_en_curso() -> (i32, u32) {
    let hoy = fechas::hoy();
    (hoy.year(), hoy.month())
}

fn periodo(conn: &Connection, (anio, mes): (i32, u32), estado: &str, sueldo: i64) -> i64 {
    conn.execute(
        "INSERT INTO periodos (anio, mes, sueldo_liquido, otros_ingresos, estado)
         VALUES (?1, ?2, ?3, 0, ?4)",
        params![anio, mes, sueldo, estado],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn servicio(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO servicios (nombre, monto_estimado, tipo, activo, fecha_alta)
         VALUES ('Gastos Comunes', 56816, 'basico', 1, '2026-08-17')",
        [],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// Un movimiento cualquiera. `servicio_id` y `cuota_id` van por parámetro
/// porque son justamente las columnas que deciden si la migración lo toca.
fn movimiento(
    conn: &Connection,
    periodo_id: i64,
    tipo: &str,
    monto: i64,
    es_estimado: bool,
    servicio_id: Option<i64>,
    cuota_id: Option<i64>,
    descripcion: &str,
) -> i64 {
    conn.execute(
        "INSERT INTO movimientos
            (periodo_id, fecha, monto, tipo, servicio_id, cuota_id, descripcion, es_estimado)
         VALUES (?1, '2999-01-01', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            periodo_id,
            monto,
            tipo,
            servicio_id,
            cuota_id,
            descripcion,
            es_estimado as i64
        ],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// Estimado automático: es lo que la migración viene a borrar.
fn estimado(conn: &Connection, periodo_id: i64, servicio_id: i64) -> i64 {
    movimiento(
        conn,
        periodo_id,
        "gasto",
        56_816,
        true,
        Some(servicio_id),
        None,
        "Gastos Comunes",
    )
}

fn cuota_pendiente(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO deudas (descripcion, tipo, monto_original, n_cuotas, fecha_primera_cuota)
         VALUES ('Samsung S26 Ultra', 'compra_cuotas', 1180008, 36, '2026-05-01')",
        [],
    )
    .unwrap();
    let deuda = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO cuotas (deuda_id, numero, fecha_vencimiento, monto)
         VALUES (?1, 1, '2999-01-01', 32778)",
        params![deuda],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn existe_movimiento(conn: &Connection, id: i64) -> bool {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM movimientos WHERE id = ?1",
            params![id],
            |f| f.get(0),
        )
        .unwrap();
    n > 0
}

fn existe_periodo(conn: &Connection, id: i64) -> bool {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM periodos WHERE id = ?1",
            params![id],
            |f| f.get(0),
        )
        .unwrap();
    n > 0
}

// ── lo que sí borra ──────────────────────────────────────────────────────────

#[test]
fn borra_el_estimado_automatico_de_un_mes_futuro() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "abierto", 0);
    let fantasma = estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(!existe_movimiento(&conn, fantasma));
}

#[test]
fn el_periodo_futuro_que_queda_vacio_se_va_con_el() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "abierto", 0);
    estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(
        !existe_periodo(&conn, p),
        "existía solo porque alguien pasó por ese mes; dejarlo lo seguiría \
         mostrando como un mes con datos"
    );
}

// ── lo que no toca ───────────────────────────────────────────────────────────

#[test]
fn no_toca_un_movimiento_ingresado_a_mano() {
    let mut conn = base_v9();
    let p = periodo(&conn, FUTURO, "abierto", 0);
    // Un gasto anotado con fecha adelantada: es una decisión de quien lo cargó.
    let manual = movimiento(&conn, p, "gasto", 40_000, false, None, None, "Regalo");

    limpiar(&mut conn);

    assert!(existe_movimiento(&conn, manual));
    assert!(
        existe_periodo(&conn, p),
        "el período todavía tiene algo adentro"
    );
}

#[test]
fn no_toca_una_activacion_manual_de_servicio() {
    // Mismo servicio y mismo mes futuro, pero el monto lo escribió una persona:
    // por eso nace con es_estimado = 0.
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "abierto", 0);
    let activacion = movimiento(
        &conn,
        p,
        "gasto",
        50_000,
        false,
        Some(s),
        None,
        "Gastos Comunes",
    );

    limpiar(&mut conn);

    assert!(existe_movimiento(&conn, activacion));
}

#[test]
fn no_toca_el_pago_de_una_cuota() {
    let mut conn = base_v9();
    let p = periodo(&conn, FUTURO, "abierto", 0);
    let cuota = cuota_pendiente(&conn);
    let pago = movimiento(
        &conn,
        p,
        "gasto",
        32_778,
        true,
        None,
        Some(cuota),
        "Samsung S26 Ultra · cuota 1/36",
    );

    limpiar(&mut conn);

    assert!(
        existe_movimiento(&conn, pago),
        "borrarlo dejaría una cuota pagada sin su gasto"
    );
}

#[test]
fn no_toca_un_ingreso() {
    let mut conn = base_v9();
    let p = periodo(&conn, FUTURO, "abierto", 0);
    let ingreso = movimiento(&conn, p, "ingreso", 90_000, true, None, None, "Bono");

    limpiar(&mut conn);

    assert!(existe_movimiento(&conn, ingreso));
}

#[test]
fn no_toca_los_estimados_del_mes_en_curso() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, mes_en_curso(), "abierto", 0);
    let del_mes = estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(
        existe_movimiento(&conn, del_mes),
        "el estimado del mes que se está viviendo es legítimo: el servicio \
         todavía no vence, pero va a vencer"
    );
}

#[test]
fn no_toca_los_estimados_de_un_mes_pasado() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, PASADO, "abierto", 0);
    let viejo = estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(
        existe_movimiento(&conn, viejo),
        "un estimado sin confirmar de un mes real no se distingue de un gasto \
         que efectivamente ocurrió"
    );
}

#[test]
fn no_toca_un_mes_futuro_cerrado() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "cerrado", 0);
    let en_cerrado = estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(
        existe_movimiento(&conn, en_cerrado),
        "cerrar ese mes fue una decisión deliberada de alguien"
    );
    assert!(existe_periodo(&conn, p));
}

#[test]
fn conserva_el_periodo_futuro_con_sueldo_declarado() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "abierto", 900_000);
    estimado(&conn, p, s);

    limpiar(&mut conn);

    assert!(
        existe_periodo(&conn, p),
        "el sueldo lo declaró una persona; borrar el período lo perdería"
    );
}

#[test]
fn conserva_el_periodo_futuro_con_presupuesto() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, FUTURO, "abierto", 0);
    estimado(&conn, p, s);

    let categoria: i64 = conn
        .query_row("SELECT id FROM categorias LIMIT 1", [], |f| f.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO presupuestos (periodo_id, categoria_id, monto_asignado)
         VALUES (?1, ?2, 120000)",
        params![p, categoria],
    )
    .unwrap();

    limpiar(&mut conn);

    assert!(
        existe_periodo(&conn, p),
        "planificar un mes que viene es normal, y el presupuesto vive colgado \
         del período"
    );
}

// ── una base sana ────────────────────────────────────────────────────────────

#[test]
fn una_base_sin_datos_futuros_queda_igual() {
    let mut conn = base_v9();
    let s = servicio(&conn);
    let p = periodo(&conn, PASADO, "abierto", 800_000);
    estimado(&conn, p, s);
    movimiento(&conn, p, "gasto", 12_000, false, None, None, "Feria");

    let conteos = |conn: &Connection| -> (i64, i64) {
        conn.query_row(
            "SELECT (SELECT COUNT(*) FROM movimientos), (SELECT COUNT(*) FROM periodos)",
            [],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .unwrap()
    };

    let antes = conteos(&conn);
    limpiar(&mut conn);

    assert_eq!(conteos(&conn), antes, "no había nada que limpiar");
}

#[test]
fn deja_la_base_en_la_version_esperada() {
    let mut conn = base_v9();
    limpiar(&mut conn);

    assert_eq!(
        migraciones::version_actual(&conn).unwrap(),
        migraciones::version_objetivo()
    );
}
