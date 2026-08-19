//! Historial de apartados y retiros, y la línea de contexto del Resumen.
//!
//! Lo que estos tests cuidan: que cada cruce de plata deje su fila junto al
//! saldo —o ninguna de las dos cosas—, que el neto del mes sume solo lo del
//! mes que se está mirando, y que nada de esto se filtre al balance, al
//! disponible ni a los reportes. Un apartado no es un gasto.

use finanzas_lib::comandos::cuentas::{self, Direccion};
use finanzas_lib::comandos::periodos::armar_resumen;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::fechas;
use finanzas_lib::modelos::cuenta::NuevaCuenta;
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::modelos::movimiento_ahorro::TipoMovimientoAhorro;
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

/// El mes en curso: es el único en el que `apartar` puede escribir, porque la
/// fecha del registro siempre es hoy.
fn mes_actual() -> (i32, u32) {
    use chrono::Datelike;
    let hoy = fechas::hoy();
    (hoy.year(), hoy.month())
}

fn ahorro(conn: &Connection, nombre: &str) -> i64 {
    cuentas::crear(conn, &NuevaCuenta { nombre: nombre.into() }).unwrap()
}

fn sueldo(conn: &Connection, anio: i32, mes: u32, monto: i64) {
    repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::periodos::actualizar_ingresos(conn, anio, mes, monto, 0).unwrap();
}

fn gasto(conn: &Connection, anio: i32, mes: u32, monto: i64) {
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::movimientos::insertar(
        conn,
        periodo.id,
        &NuevoMovimiento {
            fecha: format!("{anio:04}-{mes:02}-10"),
            monto,
            tipo: TipoMovimiento::Gasto,
            categoria_id: None,
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();
}

fn historial(conn: &Connection) -> Vec<finanzas_lib::modelos::movimiento_ahorro::MovimientoAhorro> {
    repos::movimientos_ahorro::listar_todos(conn).unwrap()
}

fn contar(conn: &Connection, tabla: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |f| f.get(0))
        .unwrap()
}

// ── el registro ──────────────────────────────────────────────────────────────

#[test]
fn apartar_deja_su_fila_en_el_historial() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();

    let filas = historial(&conn);
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].cuenta_id, viaje);
    assert_eq!(filas[0].monto, 90_000, "el monto se guarda positivo");
    assert_eq!(filas[0].tipo, TipoMovimientoAhorro::Apartar);
    assert_eq!(filas[0].fecha, fechas::a_iso(fechas::hoy()));
    assert_eq!(filas[0].nota, None);

    // Y el saldo quedó igual que siempre.
    assert_eq!(repos::cuentas::obtener(&conn, viaje).unwrap().saldo, 90_000);
}

#[test]
fn retirar_tambien_deja_su_fila() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();
    cuentas::mover(&conn, viaje, 30_000, Direccion::Retirar).unwrap();

    let filas = historial(&conn);
    assert_eq!(filas.len(), 2);
    assert_eq!(filas[1].tipo, TipoMovimientoAhorro::Retirar);
    assert_eq!(filas[1].monto, 30_000, "el retiro también va positivo");
    assert_eq!(repos::cuentas::obtener(&conn, viaje).unwrap().saldo, 60_000);
}

#[test]
fn un_movimiento_rechazado_no_deja_rastro() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 50_000);
    let viaje = ahorro(&conn, "Viaje");

    // Más de lo disponible, y más de lo que hay en la cuenta.
    assert!(cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).is_err());
    assert!(cuentas::mover(&conn, viaje, 10_000, Direccion::Retirar).is_err());

    assert!(historial(&conn).is_empty());
    assert_eq!(repos::cuentas::obtener(&conn, viaje).unwrap().saldo, 0);
}

// ── atomicidad ───────────────────────────────────────────────────────────────

#[test]
fn el_registro_y_el_saldo_se_aplican_en_la_misma_transaccion() {
    let mut conn = base();
    let (anio, mes) = mes_actual();
    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");

    // Igual que los comandos `apartar` y `retirar`: una transacción por cruce.
    let tx = conn.transaction().unwrap();
    cuentas::mover(&tx, viaje, 90_000, Direccion::Apartar).unwrap();
    // Dentro de la transacción ya se ven las dos cosas...
    assert_eq!(contar(&tx, "movimientos_ahorro"), 1);
    assert_eq!(repos::cuentas::obtener(&tx, viaje).unwrap().saldo, 90_000);
    // ...y al deshacerla no queda ninguna.
    tx.rollback().unwrap();

    assert_eq!(contar(&conn, "movimientos_ahorro"), 0);
    assert_eq!(repos::cuentas::obtener(&conn, viaje).unwrap().saldo, 0);
}

#[test]
fn si_el_ajuste_del_saldo_falla_el_registro_no_queda() {
    let mut conn = base();
    let viaje = ahorro(&conn, "Viaje");

    let tx = conn.transaction().unwrap();

    // El orden real: primero el registro, después el saldo. Si el
    // `CHECK (saldo >= 0)` aborta el ajuste, la transacción se descarta entera.
    repos::movimientos_ahorro::insertar(
        &tx,
        viaje,
        "2026-08-15",
        50_000,
        TipoMovimientoAhorro::Retirar,
        None,
    )
    .unwrap();
    assert!(
        repos::cuentas::ajustar_saldo(&tx, viaje, -50_000).is_err(),
        "el CHECK del esquema tiene que rechazar un ahorro en rojo"
    );
    drop(tx); // sin commit: rollback implícito

    assert_eq!(contar(&conn, "movimientos_ahorro"), 0);
    assert_eq!(repos::cuentas::obtener(&conn, viaje).unwrap().saldo, 0);
}

// ── el neto del mes ──────────────────────────────────────────────────────────

#[test]
fn el_neto_suma_solo_el_mes_consultado() {
    let conn = base();
    let viaje = ahorro(&conn, "Viaje");

    for (fecha, monto) in [
        ("2026-07-31", 11_000),
        ("2026-08-01", 40_000),
        ("2026-08-31", 50_000),
        ("2026-09-01", 22_000),
    ] {
        repos::movimientos_ahorro::insertar(
            &conn,
            viaje,
            fecha,
            monto,
            TipoMovimientoAhorro::Apartar,
            None,
        )
        .unwrap();
    }

    let agosto = repos::movimientos_ahorro::neto_en_rango(&conn, "2026-08-01", "2026-08-31").unwrap();
    assert_eq!(agosto, 90_000, "los bordes del mes entran, los vecinos no");

    let julio = repos::movimientos_ahorro::neto_en_rango(&conn, "2026-07-01", "2026-07-31").unwrap();
    assert_eq!(julio, 11_000);
}

#[test]
fn apartados_y_retiros_del_mismo_mes_se_netean() {
    let conn = base();
    let viaje = ahorro(&conn, "Viaje");

    for (monto, tipo) in [
        (90_000, TipoMovimientoAhorro::Apartar),
        (30_000, TipoMovimientoAhorro::Retirar),
        (10_000, TipoMovimientoAhorro::Apartar),
    ] {
        repos::movimientos_ahorro::insertar(&conn, viaje, "2026-08-10", monto, tipo, None).unwrap();
    }

    assert_eq!(
        repos::movimientos_ahorro::neto_en_rango(&conn, "2026-08-01", "2026-08-31").unwrap(),
        70_000
    );
}

#[test]
fn retirar_mas_de_lo_apartado_deja_el_neto_negativo() {
    let conn = base();
    let viaje = ahorro(&conn, "Viaje");

    repos::movimientos_ahorro::insertar(&conn, viaje, "2026-08-05", 20_000, TipoMovimientoAhorro::Apartar, None).unwrap();
    repos::movimientos_ahorro::insertar(&conn, viaje, "2026-08-20", 65_000, TipoMovimientoAhorro::Retirar, None).unwrap();

    assert_eq!(
        repos::movimientos_ahorro::neto_en_rango(&conn, "2026-08-01", "2026-08-31").unwrap(),
        -45_000
    );
}

#[test]
fn un_mes_sin_movimientos_de_ahorro_da_cero() {
    let conn = base();
    assert_eq!(
        repos::movimientos_ahorro::neto_en_rango(&conn, "2026-08-01", "2026-08-31").unwrap(),
        0
    );
}

// ── el resumen del mes ───────────────────────────────────────────────────────

#[test]
fn el_resumen_reporta_el_apartado_y_lo_que_queda_libre() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    gasto(&conn, anio, mes, 400_602);
    let viaje = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();

    let r = armar_resumen(&conn, anio, mes).unwrap();

    assert_eq!(r.balance, 99_398, "el balance no cambia: ingresos - gastos");
    assert_eq!(r.apartado_neto, 90_000);
    assert_eq!(r.libre, 9_398, "balance menos lo que ya se fue a un ahorro");
}

#[test]
fn sin_apartados_lo_libre_es_el_balance() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    gasto(&conn, anio, mes, 400_602);

    let r = armar_resumen(&conn, anio, mes).unwrap();
    assert_eq!(r.apartado_neto, 0);
    assert_eq!(r.libre, r.balance);
}

#[test]
fn el_apartado_de_un_mes_no_aparece_en_otro() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();

    // El mes anterior al actual, sin importar cuándo se corran los tests.
    let anterior = fechas::desde_mes_absoluto(fechas::mes_absoluto(anio, mes) - 1);
    let r = armar_resumen(&conn, anterior.0, anterior.1).unwrap();

    assert_eq!(r.apartado_neto, 0);
    assert_eq!(r.libre, r.balance);
}

// ── independencia del resto de la app ────────────────────────────────────────

#[test]
fn apartar_no_mueve_el_balance_ni_los_gastos_del_mes() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    gasto(&conn, anio, mes, 120_000);
    let viaje = ahorro(&conn, "Viaje");

    let antes = armar_resumen(&conn, anio, mes).unwrap();
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();
    let despues = armar_resumen(&conn, anio, mes).unwrap();

    assert_eq!(despues.balance, antes.balance);
    assert_eq!(despues.total_gastos, antes.total_gastos);
    assert_eq!(despues.total_ingresos, antes.total_ingresos);
    assert_eq!(despues.n_movimientos, antes.n_movimientos);
    assert_eq!(
        despues.por_categoria.len(),
        antes.por_categoria.len(),
        "un apartado no es un gasto de ninguna categoría"
    );
}

#[test]
fn el_historial_no_toca_el_disponible_ni_el_patrimonio() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");

    let antes = cuentas::armar_resumen(&conn).unwrap();
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();
    let despues = cuentas::armar_resumen(&conn).unwrap();

    // Apartar mueve plata de bolsillo: el patrimonio no cambia, el disponible
    // baja por el saldo de la cuenta —no por el registro—, y el registro no
    // agrega ni un peso a ningún lado.
    assert_eq!(despues.patrimonio, antes.patrimonio);
    assert_eq!(despues.disponible, antes.disponible - 90_000);
    assert_eq!(despues.total_ahorrado, 90_000);
    assert_eq!(despues.desglose.gastos, antes.desglose.gastos);
    assert_eq!(despues.desglose.ingresos(), antes.desglose.ingresos());
}

#[test]
fn los_movimientos_de_ahorro_no_entran_en_los_reportes() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    gasto(&conn, anio, mes, 120_000);
    let viaje = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();

    let periodo = repos::periodos::obtener_o_crear(&conn, anio, mes).unwrap();
    let (gastos, ingresos, _, _, n) = repos::movimientos::totales(&conn, periodo.id).unwrap();

    assert_eq!(gastos, 120_000, "solo el gasto de verdad");
    assert_eq!(ingresos, 0);
    assert_eq!(n, 1, "la tabla de movimientos no supo nada del apartado");

    let evolucion = repos::movimientos::evolucion_por_categoria(
        &conn,
        fechas::mes_absoluto(anio, mes),
        fechas::mes_absoluto(anio, mes),
    )
    .unwrap();
    // (anio, mes, categoria_id, nombre, color, total)
    let total: i64 = evolucion.iter().map(|f| f.5).sum();
    assert_eq!(total, 120_000, "la evolución de gastos tampoco lo ve");
}

// ── borrado ──────────────────────────────────────────────────────────────────

#[test]
fn borrar_la_cuenta_se_lleva_su_historial() {
    let conn = base();
    let (anio, mes) = mes_actual();

    sueldo(&conn, anio, mes, 500_000);
    let viaje = ahorro(&conn, "Viaje");
    let emergencia = ahorro(&conn, "Emergencia");
    cuentas::mover(&conn, viaje, 90_000, Direccion::Apartar).unwrap();
    cuentas::mover(&conn, emergencia, 40_000, Direccion::Apartar).unwrap();
    assert_eq!(contar(&conn, "movimientos_ahorro"), 2);

    // Eliminar exige devolver la plata primero: eso deja un retiro más.
    cuentas::mover(&conn, viaje, 90_000, Direccion::Retirar).unwrap();
    cuentas::eliminar(&conn, viaje).unwrap();

    let filas = historial(&conn);
    assert_eq!(filas.len(), 1, "el CASCADE se llevó las dos del viaje");
    assert_eq!(filas[0].cuenta_id, emergencia);
}
