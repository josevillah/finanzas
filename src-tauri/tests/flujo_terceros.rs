//! Deudas de terceros: plata que me deben.
//!
//! Lo crítico es que no se cuelen en las vistas de carga, que son sobre lo que
//! yo debo. Un préstamo a un amigo no puede subir el semáforo a rojo.

use finanzas_lib::comandos::cuotas::sincronizar_movimiento_de_cuota;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::{amortizacion, fechas};
use finanzas_lib::modelos::categoria::{TipoCategoria, CODIGO_COBROS};
use finanzas_lib::modelos::deuda::{DireccionDeuda, NuevaDeuda, TipoDeuda};
use finanzas_lib::modelos::movimiento::{FiltroMovimientos, TipoMovimiento};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn datos(descripcion: &str, direccion: DireccionDeuda, deudor: Option<&str>) -> NuevaDeuda {
    NuevaDeuda {
        descripcion: descripcion.into(),
        tipo: TipoDeuda::CompraCuotas,
        institucion: None,
        monto_original: 600_000,
        tasa_mensual: 0.0,
        n_cuotas: 6,
        fecha_primera_cuota: "2026-09-05".into(),
        notas: None,
        direccion,
        deudor: deudor.map(String::from),
    }
}

fn crear(conn: &Connection, datos: &NuevaDeuda) -> i64 {
    let cuotas = amortizacion::generar(
        datos.monto_original,
        datos.tasa_mensual,
        datos.n_cuotas,
        fechas::desde_iso(&datos.fecha_primera_cuota).unwrap(),
    )
    .unwrap();

    let id = repos::deudas::insertar(conn, datos).unwrap();
    repos::cuotas::insertar_muchas(conn, id, &cuotas).unwrap();
    id
}

// ── migración ────────────────────────────────────────────────────────────────

#[test]
fn las_deudas_existentes_quedan_como_propias() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    // Base congelada antes de la migración de terceros.
    conn.execute_batch(include_str!("../migrations/0001_esquema_inicial.sql"))
        .unwrap();
    conn.execute(
        "INSERT INTO deudas
            (descripcion, tipo, monto_original, tasa_mensual, n_cuotas,
             fecha_primera_cuota, estado)
         VALUES ('Notebook', 'compra_cuotas', 600000, 0, 6, '2026-09-05', 'vigente')",
        [],
    )
    .unwrap();
    conn.execute_batch("PRAGMA user_version = 1;").unwrap();

    migraciones::ejecutar(&mut conn).unwrap();

    let deuda = &repos::deudas::listar(&conn, None, None).unwrap()[0];
    assert_eq!(
        deuda.direccion,
        DireccionDeuda::Propia,
        "el DEFAULT hace el backfill: lo que ya existía es mío"
    );
    assert_eq!(deuda.deudor, None);
}

#[test]
fn la_categoria_de_cobros_llega_como_semilla_de_ingreso() {
    let conn = base();

    let cat = repos::categorias::por_codigo(&conn, CODIGO_COBROS)
        .unwrap()
        .expect("debe existir la categoría con código 'cobros'");

    assert_eq!(cat.nombre, "Préstamos cobrados");
    assert_eq!(cat.tipo, TipoCategoria::Ingreso);
    assert!(cat.es_semilla, "el reinicio de datos no debe borrarla");
    assert!(!cat.tipo.es_de_gasto(), "el presupuesto la ignora");
}

// ── las vistas de carga solo miran deudas propias ────────────────────────────

/// Una deuda propia y una de tercero, ambas con cuotas el mismo mes.
fn con_ambas(conn: &Connection) -> (i64, i64) {
    let propia = crear(conn, &datos("Notebook", DireccionDeuda::Propia, None));
    let tercero = crear(
        conn,
        &datos("Préstamo", DireccionDeuda::Tercero, Some("Ignacia")),
    );
    (propia, tercero)
}

#[test]
fn la_carga_financiera_ignora_lo_que_me_deben() {
    let conn = base();
    let (propia, _) = con_ambas(&conn);

    let (total, n) = repos::cuotas::total_en_rango(&conn, "2026-09-01", "2026-09-30").unwrap();
    let cuota_propia = repos::cuotas::listar_por_deuda(&conn, propia).unwrap()[0].monto;

    assert_eq!(n, 1, "solo la cuota de la deuda propia");
    assert_eq!(
        total, cuota_propia,
        "un préstamo a un tercero no es plata que yo deba"
    );
}

#[test]
fn el_calendario_de_carga_ignora_lo_que_me_deben() {
    let conn = base();
    let (propia, _) = con_ambas(&conn);

    let meses = repos::cuotas::carga_por_mes(&conn, "2026-09-01", "2026-09-30").unwrap();
    let cuota_propia = repos::cuotas::listar_por_deuda(&conn, propia).unwrap()[0].monto;

    assert_eq!(meses.len(), 1);
    assert_eq!(meses[0].3, 1, "una sola cuota comprometida");
    assert_eq!(meses[0].1, cuota_propia);
}

#[test]
fn la_fecha_de_libertad_ignora_lo_que_me_deben() {
    let conn = base();
    let (propia, _) = con_ambas(&conn);

    let pendientes = repos::cuotas::pendientes_con_deuda(&conn).unwrap();

    assert_eq!(pendientes.len(), 6, "solo las 6 cuotas de la deuda propia");
    assert!(
        pendientes.iter().all(|(c, _)| c.deuda_id == propia),
        "estar libre de deudas no depende de que me paguen a mí"
    );
}

#[test]
fn el_listado_de_cuotas_del_mes_ignora_lo_que_me_deben() {
    let conn = base();
    let (propia, _) = con_ambas(&conn);

    // Es el listado que se muestra bajo el semáforo de carga financiera.
    let del_mes = repos::cuotas::en_rango_con_deuda(&conn, "2026-09-01", "2026-09-30").unwrap();

    assert_eq!(del_mes.len(), 1);
    assert_eq!(del_mes[0].0.deuda_id, propia);
}

#[test]
fn el_selector_de_mes_si_marca_los_meses_con_cobros() {
    let conn = base();
    crear(
        &conn,
        &datos("Préstamo", DireccionDeuda::Tercero, Some("Ignacia")),
    );

    let meses = repos::cuotas::meses_con_vencimientos(&conn).unwrap();

    assert!(
        !meses.is_empty(),
        "el selector sirve para navegar, no es una vista de carga: un mes con \
         un cobro esperando tiene algo que mirar"
    );
}

// ── cobros ───────────────────────────────────────────────────────────────────

#[test]
fn cobrar_una_cuota_de_tercero_genera_un_ingreso() {
    let conn = base();
    let tercero = crear(
        &conn,
        &datos("Préstamo", DireccionDeuda::Tercero, Some("Ignacia")),
    );
    let cuota = repos::cuotas::listar_por_deuda(&conn, tercero).unwrap()[0].clone();

    sincronizar_movimiento_de_cuota(&conn, &cuota, "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();

    assert_eq!(detalle.len(), 1);
    assert_eq!(
        detalle[0].movimiento.tipo,
        TipoMovimiento::Ingreso,
        "cobrar es plata que entra, no un gasto"
    );
    assert_eq!(
        detalle[0].categoria_nombre.as_deref(),
        Some("Préstamos cobrados")
    );
    assert!(
        detalle[0]
            .movimiento
            .descripcion
            .as_deref()
            .unwrap()
            .contains("Ignacia"),
        "la descripción debe decir quién pagó"
    );
}

#[test]
fn pagar_una_cuota_propia_sigue_siendo_un_gasto() {
    let conn = base();
    let propia = crear(&conn, &datos("Notebook", DireccionDeuda::Propia, None));
    let cuota = repos::cuotas::listar_por_deuda(&conn, propia).unwrap()[0].clone();

    sincronizar_movimiento_de_cuota(&conn, &cuota, "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();

    assert_eq!(detalle[0].movimiento.tipo, TipoMovimiento::Gasto);
    assert_eq!(
        detalle[0].categoria_nombre.as_deref(),
        Some("Deudas y créditos")
    );
}

#[test]
fn el_cobro_entra_como_ingreso_del_mes_sin_tocar_el_sueldo() {
    let conn = base();
    let tercero = crear(
        &conn,
        &datos("Préstamo", DireccionDeuda::Tercero, Some("Ignacia")),
    );
    repos::periodos::actualizar_ingresos(&conn, 2026, 9, 1_450_000, 0).unwrap();

    let cuota = repos::cuotas::listar_por_deuda(&conn, tercero).unwrap()[0].clone();
    sincronizar_movimiento_de_cuota(&conn, &cuota, "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let (gastos, ingresos_extra, _, _, _) =
        repos::movimientos::totales(&conn, periodo.id).unwrap();

    assert_eq!(gastos, 0, "un cobro no es gasto");
    assert_eq!(ingresos_extra, 100_000);
    assert_eq!(
        periodo.sueldo_liquido, 1_450_000,
        "el sueldo es el que declaró el usuario y no lo mueve un cobro"
    );
}

#[test]
fn el_cobro_no_altera_el_porcentaje_de_carga_financiera() {
    let conn = base();
    let (propia, tercero) = con_ambas(&conn);
    repos::periodos::actualizar_ingresos(&conn, 2026, 9, 1_000_000, 0).unwrap();

    let antes = repos::cuotas::total_en_rango(&conn, "2026-09-01", "2026-09-30").unwrap();

    // Se cobra la cuota del tercero: entra plata al mes.
    let cuota = repos::cuotas::listar_por_deuda(&conn, tercero).unwrap()[0].clone();
    sincronizar_movimiento_de_cuota(&conn, &cuota, "2026-09-05", 100_000).unwrap();

    let despues = repos::cuotas::total_en_rango(&conn, "2026-09-01", "2026-09-30").unwrap();
    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();

    assert_eq!(antes, despues, "el numerador no se mueve");
    assert_eq!(
        periodo.sueldo_liquido, 1_000_000,
        "y el denominador tampoco: la carga se calcula solo sobre el sueldo"
    );

    let cuota_propia = repos::cuotas::listar_por_deuda(&conn, propia).unwrap()[0].monto;
    assert_eq!(despues.0, cuota_propia);
}

#[test]
fn deshacer_un_cobro_borra_el_ingreso() {
    let conn = base();
    let tercero = crear(
        &conn,
        &datos("Préstamo", DireccionDeuda::Tercero, Some("Ignacia")),
    );
    let cuota = repos::cuotas::listar_por_deuda(&conn, tercero).unwrap()[0].clone();

    sincronizar_movimiento_de_cuota(&conn, &cuota, "2026-09-05", 100_000).unwrap();
    repos::movimientos::eliminar_por_cuota(&conn, cuota.id).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let (_, ingresos_extra, _, _, n) = repos::movimientos::totales(&conn, periodo.id).unwrap();

    assert_eq!(n, 0);
    assert_eq!(ingresos_extra, 0);
}

// ── listado y deudor ─────────────────────────────────────────────────────────

#[test]
fn el_listado_separa_las_dos_direcciones() {
    let conn = base();
    con_ambas(&conn);

    let todas = repos::deudas::listar(&conn, None, None).unwrap();
    let mias = repos::deudas::listar(&conn, None, Some(DireccionDeuda::Propia)).unwrap();
    let me_deben = repos::deudas::listar(&conn, None, Some(DireccionDeuda::Tercero)).unwrap();

    assert_eq!(todas.len(), 2);
    assert_eq!(mias.len(), 1);
    assert_eq!(me_deben.len(), 1);
    assert_eq!(me_deben[0].deudor.as_deref(), Some("Ignacia"));
}

#[test]
fn una_deuda_propia_no_guarda_deudor() {
    let conn = base();

    // Aunque venga un nombre por error, en una deuda propia no tiene sentido.
    let id = crear(
        &conn,
        &datos("Notebook", DireccionDeuda::Propia, Some("Ignacia")),
    );

    assert_eq!(repos::deudas::obtener(&conn, id).unwrap().deudor, None);
}

#[test]
fn el_deudor_en_blanco_no_se_guarda_como_texto_vacio() {
    let conn = base();
    let id = crear(&conn, &datos("Préstamo", DireccionDeuda::Tercero, Some("   ")));

    assert_eq!(
        repos::deudas::obtener(&conn, id).unwrap().deudor,
        None,
        "un nombre en blanco es lo mismo que no tenerlo"
    );
}
