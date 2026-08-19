//! Cuentas: disponible calculado y ahorros apartados.
//!
//! El disponible no se declara: sale de
//!
//!     saldo_inicial + ingresos - gastos - apartado
//!
//! Lo que estos tests cuidan es que ese número no invente ni pierda pesos, y
//! que apartar y retirar muevan plata entre "puedo gastarla" y "no la quiero
//! gastar" sin cambiar el patrimonio.

use finanzas_lib::comandos::cuentas::{
    actualizar, armar_resumen, crear, desglose, desglose_hasta, eliminar, fijar_inicial, mover,
    Direccion,
};
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::modelos::cuenta::NuevaCuenta;
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::modelos::servicio::{NuevoServicio, TipoServicio};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn ahorro(conn: &Connection, nombre: &str) -> i64 {
    crear(conn, &NuevaCuenta { nombre: nombre.into() }).unwrap()
}

fn disponible(conn: &Connection) -> i64 {
    armar_resumen(conn).unwrap().disponible
}

fn saldo(conn: &Connection, id: i64) -> i64 {
    repos::cuentas::obtener(conn, id).unwrap().saldo
}

/// Registra un movimiento en el mes indicado.
fn movimiento(conn: &Connection, anio: i32, mes: u32, tipo: TipoMovimiento, monto: i64) {
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::movimientos::insertar(
        conn,
        periodo.id,
        &NuevoMovimiento {
            fecha: format!("{anio:04}-{mes:02}-15"),
            monto,
            tipo,
            categoria_id: None,
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();
}

fn gasto(conn: &Connection, monto: i64) {
    movimiento(conn, 2026, 3, TipoMovimiento::Gasto, monto);
}

fn ingreso(conn: &Connection, monto: i64) {
    movimiento(conn, 2026, 3, TipoMovimiento::Ingreso, monto);
}

/// Sueldo del mes, que vive en `periodos` y no en `movimientos`.
fn sueldo(conn: &Connection, anio: i32, mes: u32, monto: i64) {
    repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::periodos::actualizar_ingresos(conn, anio, mes, monto, 0).unwrap();
}

// ── el cálculo ───────────────────────────────────────────────────────────────

#[test]
fn una_base_nueva_arranca_en_cero() {
    let conn = base();
    let r = armar_resumen(&conn).unwrap();

    assert_eq!(r.disponible, 0);
    assert_eq!(r.patrimonio, 0);
    assert_eq!(r.desglose.saldo_inicial, 0);
    assert!(r.ahorros.is_empty());
}

#[test]
fn el_disponible_es_inicial_mas_ingresos_menos_gastos_menos_apartado() {
    let conn = base();

    fijar_inicial(&conn, 200_000).unwrap();
    sueldo(&conn, 2026, 3, 900_000);
    ingreso(&conn, 50_000);
    gasto(&conn, 340_000);

    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 150_000, Direccion::Apartar).unwrap();

    let r = armar_resumen(&conn).unwrap();

    // 200.000 + 900.000 + 50.000 - 340.000 = 810.000 de patrimonio.
    assert_eq!(r.patrimonio, 810_000);
    assert_eq!(r.disponible, 660_000, "810.000 menos los 150.000 apartados");
    assert_eq!(r.total_ahorrado, 150_000);

    let d = r.desglose;
    assert_eq!(d.saldo_inicial, 200_000);
    assert_eq!(d.ingresos_declarados, 900_000, "el sueldo vive en periodos");
    assert_eq!(d.ingresos_registrados, 50_000);
    assert_eq!(d.gastos, 340_000);
    assert_eq!(d.apartado, 150_000);
}

#[test]
fn el_sueldo_del_periodo_entra_al_calculo() {
    let conn = base();
    sueldo(&conn, 2026, 3, 900_000);

    assert_eq!(
        disponible(&conn),
        900_000,
        "sin esto el disponible restaría todos los gastos sin casi ningún ingreso"
    );
}

#[test]
fn registrar_un_gasto_baja_el_disponible() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    assert_eq!(disponible(&conn), 500_000);

    gasto(&conn, 80_000);
    assert_eq!(disponible(&conn), 420_000);

    gasto(&conn, 20_000);
    assert_eq!(disponible(&conn), 400_000);
}

#[test]
fn los_movimientos_de_meses_cerrados_cuentan_igual() {
    let conn = base();
    fijar_inicial(&conn, 1_000_000).unwrap();

    sueldo(&conn, 2026, 1, 700_000);
    movimiento(&conn, 2026, 1, TipoMovimiento::Gasto, 250_000);
    repos::periodos::cambiar_estado(&conn, 2026, 1, "cerrado").unwrap();

    gasto(&conn, 100_000);

    assert_eq!(
        disponible(&conn),
        1_350_000,
        "cerrar un mes no devuelve la plata que salió"
    );
}

#[test]
fn el_saldo_inicial_admite_negativos() {
    let conn = base();

    // Empezó a usar la app con la cuenta en rojo.
    fijar_inicial(&conn, -120_000).unwrap();
    sueldo(&conn, 2026, 3, 900_000);

    assert_eq!(disponible(&conn), 780_000);
}

#[test]
fn el_disponible_puede_quedar_negativo() {
    let conn = base();
    fijar_inicial(&conn, 50_000).unwrap();
    gasto(&conn, 200_000);

    assert_eq!(
        disponible(&conn),
        -150_000,
        "gastaste más de lo que entró, o el saldo inicial no está ajustado"
    );
}

#[test]
fn ajustar_el_saldo_inicial_desplaza_el_disponible_uno_a_uno() {
    let conn = base();
    sueldo(&conn, 2026, 3, 900_000);
    gasto(&conn, 400_000);
    assert_eq!(disponible(&conn), 500_000);

    // Así se cuadra con el banco: se mueve un solo número.
    fijar_inicial(&conn, 75_000).unwrap();
    assert_eq!(disponible(&conn), 575_000);
}

// ── apartar y retirar ────────────────────────────────────────────────────────

#[test]
fn apartar_mueve_el_monto_sin_cambiar_el_patrimonio() {
    let conn = base();
    fijar_inicial(&conn, 1_000_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");

    mover(&conn, cuenta, 300_000, Direccion::Apartar).unwrap();

    let r = armar_resumen(&conn).unwrap();
    assert_eq!(r.disponible, 700_000);
    assert_eq!(saldo(&conn, cuenta), 300_000);
    assert_eq!(r.patrimonio, 1_000_000, "la plata no se creó ni se destruyó");
}

#[test]
fn retirar_devuelve_la_plata_al_disponible() {
    let conn = base();
    fijar_inicial(&conn, 1_000_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 300_000, Direccion::Apartar).unwrap();

    mover(&conn, cuenta, 120_000, Direccion::Retirar).unwrap();

    assert_eq!(saldo(&conn, cuenta), 180_000);
    assert_eq!(disponible(&conn), 820_000);
    assert_eq!(armar_resumen(&conn).unwrap().patrimonio, 1_000_000);
}

#[test]
fn apartar_mas_que_el_disponible_falla_sin_modificar_nada() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    gasto(&conn, 200_000);
    let cuenta = ahorro(&conn, "Viaje");

    assert!(
        mover(&conn, cuenta, 300_001, Direccion::Apartar).is_err(),
        "el disponible es 300.000"
    );
    assert_eq!(saldo(&conn, cuenta), 0);
    assert_eq!(disponible(&conn), 300_000);

    // Justo en el límite sí entra.
    mover(&conn, cuenta, 300_000, Direccion::Apartar).unwrap();
    assert_eq!(disponible(&conn), 0);
}

#[test]
fn apartar_valida_contra_el_disponible_no_contra_el_patrimonio() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();

    let uno = ahorro(&conn, "Viaje");
    let otro = ahorro(&conn, "Emergencias");
    mover(&conn, uno, 400_000, Direccion::Apartar).unwrap();

    assert!(
        mover(&conn, otro, 200_000, Direccion::Apartar).is_err(),
        "quedan 100.000 disponibles aunque el patrimonio siga siendo 500.000"
    );
    assert_eq!(saldo(&conn, otro), 0);
}

#[test]
fn retirar_mas_de_lo_que_hay_en_el_ahorro_falla() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 100_000, Direccion::Apartar).unwrap();

    assert!(mover(&conn, cuenta, 100_001, Direccion::Retirar).is_err());
    assert_eq!(saldo(&conn, cuenta), 100_000);
    assert_eq!(disponible(&conn), 400_000);
}

#[test]
fn mover_cero_o_negativo_se_rechaza() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 100_000, Direccion::Apartar).unwrap();

    for monto in [0, -50_000] {
        assert!(mover(&conn, cuenta, monto, Direccion::Apartar).is_err());
        assert!(
            mover(&conn, cuenta, monto, Direccion::Retirar).is_err(),
            "retirar un negativo sería apartar por la puerta de atrás"
        );
    }
    assert_eq!(saldo(&conn, cuenta), 100_000);
}

#[test]
fn el_esquema_rechaza_un_ahorro_negativo_aunque_se_salte_la_validacion() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 100_000, Direccion::Apartar).unwrap();

    assert!(
        repos::cuentas::ajustar_saldo(&conn, cuenta, -200_000).is_err(),
        "el CHECK del esquema es la última línea de defensa"
    );
    assert_eq!(saldo(&conn, cuenta), 100_000);
}

// ── CRUD ─────────────────────────────────────────────────────────────────────

#[test]
fn una_cuenta_nace_vacia_y_no_altera_el_disponible() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();

    let cuenta = ahorro(&conn, "Viaje");

    assert_eq!(saldo(&conn, cuenta), 0);
    assert_eq!(disponible(&conn), 500_000);
}

#[test]
fn el_nombre_no_puede_quedar_vacio() {
    let conn = base();

    assert!(crear(&conn, &NuevaCuenta { nombre: "   ".into() }).is_err());

    let cuenta = ahorro(&conn, "Viaje");
    assert!(actualizar(&conn, cuenta, "", true).is_err());
}

#[test]
fn una_cuenta_con_plata_no_se_borra_ni_se_archiva() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 200_000, Direccion::Apartar).unwrap();

    assert!(
        eliminar(&conn, cuenta).is_err(),
        "borrarla sumaría 200.000 al disponible sin que nadie lo pidiera"
    );
    assert!(
        actualizar(&conn, cuenta, "Viaje", false).is_err(),
        "archivarla la escondería con la plata dentro"
    );

    // Vaciada, las dos cosas se pueden.
    mover(&conn, cuenta, 200_000, Direccion::Retirar).unwrap();
    actualizar(&conn, cuenta, "Viaje", false).unwrap();
    eliminar(&conn, cuenta).unwrap();
    assert!(repos::cuentas::obtener(&conn, cuenta).is_err());
}

#[test]
fn una_cuenta_archivada_sigue_contando_en_el_apartado() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    let cuenta = ahorro(&conn, "Viaje");
    mover(&conn, cuenta, 200_000, Direccion::Apartar).unwrap();

    // Se fuerza el archivado saltando la validación, que es el escenario que
    // este total tiene que aguantar.
    repos::cuentas::actualizar_datos(&conn, cuenta, "Viaje", false).unwrap();

    assert_eq!(
        disponible(&conn),
        300_000,
        "la plata archivada no puede reaparecer sola como disponible"
    );
}

// ── estimados ────────────────────────────────────────────────────────────────

#[test]
fn los_gastos_estimados_cuentan_y_se_informan_aparte() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();

    let servicio = repos::servicios::insertar(
        &conn,
        &NuevoServicio {
            nombre: "Luz".into(),
            categoria_id: None,
            monto_estimado: 45_000,
            dia_vencimiento: Some(15),
            tipo: TipoServicio::Basico,
            activo: true,
            fecha_alta: None,
        },
        "2026-03-01",
    )
    .unwrap();

    let periodo = repos::periodos::obtener_o_crear(&conn, 2026, 3).unwrap();
    repos::movimientos::insertar_estimado_servicio(
        &conn,
        periodo.id,
        servicio,
        None,
        "2026-03-15",
        45_000,
        "Luz",
    )
    .unwrap();
    gasto(&conn, 20_000);

    let d = desglose(&conn).unwrap();
    assert_eq!(d.gastos, 65_000, "el estimado se cuenta como gasto");
    assert_eq!(
        d.gastos_estimados, 45_000,
        "y además se informa aparte, para poder explicar el número"
    );
    assert_eq!(disponible(&conn), 435_000);
}

// ── el corte en el mes en curso ──────────────────────────────────────────────
//
// Lo que cae en un mes que todavía no llegó es una proyección, no plata que
// salió. `desglose_hasta` fija el mes de corte para no depender del día en que
// corran estos tests.

#[test]
fn un_gasto_de_un_mes_futuro_no_baja_el_disponible() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    movimiento(&conn, 2026, 3, TipoMovimiento::Gasto, 100_000);
    movimiento(&conn, 2026, 4, TipoMovimiento::Gasto, 80_000);

    let d = desglose_hasta(&conn, 2026, 3).unwrap();

    assert_eq!(d.gastos, 100_000, "el de abril todavía no pasó");
    assert_eq!(d.disponible(), 400_000);
}

#[test]
fn un_ingreso_de_un_mes_futuro_no_sube_el_disponible() {
    let conn = base();
    movimiento(&conn, 2026, 4, TipoMovimiento::Ingreso, 250_000);

    let d = desglose_hasta(&conn, 2026, 3).unwrap();

    assert_eq!(d.ingresos_registrados, 0);
    assert_eq!(
        d.disponible(),
        0,
        "el corte tiene que ser simétrico: si el gasto futuro no resta, el ingreso futuro no suma"
    );
}

#[test]
fn el_sueldo_declarado_en_un_mes_futuro_no_cuenta() {
    let conn = base();
    sueldo(&conn, 2026, 3, 900_000);
    sueldo(&conn, 2026, 4, 900_000);

    let d = desglose_hasta(&conn, 2026, 3).unwrap();

    assert_eq!(d.ingresos_declarados, 900_000, "el de abril no entró todavía");
}

#[test]
fn el_mes_en_curso_cuenta_completo() {
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();
    sueldo(&conn, 2026, 3, 900_000);
    gasto(&conn, 200_000);

    // El corte es hasta el mes en curso inclusive: marzo entero cuenta, aunque
    // el mes no haya terminado.
    let d = desglose_hasta(&conn, 2026, 3).unwrap();

    assert_eq!(d.ingresos_declarados, 900_000);
    assert_eq!(d.gastos, 200_000);
    assert_eq!(d.disponible(), 1_200_000);
}

#[test]
fn el_estimado_de_un_mes_futuro_queda_fuera_del_disponible() {
    // El caso que originó todo esto: el estimado de un servicio materializado
    // en un mes que no llegó descontaba plata que nunca salió.
    let conn = base();
    fijar_inicial(&conn, 500_000).unwrap();

    let servicio = repos::servicios::insertar(
        &conn,
        &NuevoServicio {
            nombre: "Gastos Comunes".into(),
            categoria_id: None,
            monto_estimado: 56_816,
            dia_vencimiento: None,
            tipo: TipoServicio::Basico,
            activo: true,
            fecha_alta: None,
        },
        "2026-03-01",
    )
    .unwrap();

    let futuro = repos::periodos::obtener_o_crear(&conn, 2026, 4).unwrap();
    repos::movimientos::insertar_estimado_servicio(
        &conn,
        futuro.id,
        servicio,
        None,
        "2026-04-01",
        56_816,
        "Gastos Comunes",
    )
    .unwrap();

    let d = desglose_hasta(&conn, 2026, 3).unwrap();

    assert_eq!(d.gastos, 0);
    assert_eq!(d.gastos_estimados, 0);
    assert_eq!(d.disponible(), 500_000);
}

// ── esquema ──────────────────────────────────────────────────────────────────

#[test]
fn la_migracion_deja_el_esquema_sin_tipo_de_cuenta() {
    let conn = base();

    let columnas: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('cuentas')")
        .unwrap()
        .query_map([], |f| f.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert!(
        !columnas.iter().any(|c| c == "tipo"),
        "no quedan ni 'corriente' ni 'informativa' que distinguir: {columnas:?}"
    );

    let indices: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'cuentas'")
        .unwrap()
        .query_map([], |f| f.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert!(
        !indices.iter().any(|i| i == "idx_cuentas_corriente"),
        "el índice único parcial de la corriente se fue con la tabla: {indices:?}"
    );

    // Y no quedó ninguna fila sembrada: el disponible ya no es una fila.
    assert_eq!(repos::cuentas::listar(&conn, false).unwrap().len(), 0);
}

#[test]
fn el_saldo_inicial_queda_sembrado_en_configuracion() {
    let conn = base();

    assert_eq!(
        repos::configuracion::obtener(&conn, repos::configuracion::SALDO_INICIAL).unwrap(),
        Some("0".to_string())
    );
}
