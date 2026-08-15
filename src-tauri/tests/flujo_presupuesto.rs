//! Fase 3: asignación de presupuesto y consultas de los reportes.

use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::fechas;
use finanzas_lib::modelos::movimiento::{MedioPago, NuevoMovimiento, TipoMovimiento};
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

fn registrar_gasto(conn: &Connection, fecha: &str, monto: i64, categoria_id: i64) {
    let f = fechas::desde_iso(fecha).unwrap();
    let periodo = repos::periodos::obtener_o_crear(
        conn,
        chrono::Datelike::year(&f),
        chrono::Datelike::month(&f),
    )
    .unwrap();

    repos::movimientos::insertar(
        conn,
        periodo.id,
        &NuevoMovimiento {
            fecha: fecha.into(),
            monto,
            tipo: TipoMovimiento::Gasto,
            categoria_id: Some(categoria_id),
            servicio_id: None,
            medio_pago: Some(MedioPago::Debito),
            descripcion: None,
        },
    )
    .unwrap();
}

fn periodo_id(conn: &Connection, anio: i32, mes: u32) -> i64 {
    repos::periodos::obtener_o_crear(conn, anio, mes).unwrap().id
}

// ── asignación ───────────────────────────────────────────────────────────────

#[test]
fn asignar_crea_actualiza_y_un_monto_cero_borra() {
    let conn = base();
    let periodo = periodo_id(&conn, 2026, 8);
    let super_id = categoria_id(&conn, "Supermercado");

    repos::presupuestos::asignar(&conn, periodo, super_id, 300_000).unwrap();
    assert_eq!(
        repos::presupuestos::por_categoria(&conn, periodo).unwrap()[&super_id],
        300_000
    );

    // Reasignar sobrescribe en vez de duplicar.
    repos::presupuestos::asignar(&conn, periodo, super_id, 350_000).unwrap();
    let mapa = repos::presupuestos::por_categoria(&conn, periodo).unwrap();
    assert_eq!(mapa.len(), 1);
    assert_eq!(mapa[&super_id], 350_000);

    // Un monto de 0 borra la línea: no se guardan presupuestos vacíos.
    repos::presupuestos::asignar(&conn, periodo, super_id, 0).unwrap();
    assert!(repos::presupuestos::por_categoria(&conn, periodo)
        .unwrap()
        .is_empty());
}

#[test]
fn el_total_asignado_suma_solo_su_periodo() {
    let conn = base();
    let agosto = periodo_id(&conn, 2026, 8);
    let septiembre = periodo_id(&conn, 2026, 9);

    let super_id = categoria_id(&conn, "Supermercado");
    let transporte = categoria_id(&conn, "Transporte");

    repos::presupuestos::asignar(&conn, agosto, super_id, 300_000).unwrap();
    repos::presupuestos::asignar(&conn, agosto, transporte, 80_000).unwrap();
    repos::presupuestos::asignar(&conn, septiembre, super_id, 400_000).unwrap();

    assert_eq!(repos::presupuestos::total_asignado(&conn, agosto).unwrap(), 380_000);
    assert_eq!(
        repos::presupuestos::total_asignado(&conn, septiembre).unwrap(),
        400_000
    );
}

#[test]
fn copiar_el_presupuesto_traslada_las_lineas_y_pisa_las_existentes() {
    let conn = base();
    let agosto = periodo_id(&conn, 2026, 8);
    let septiembre = periodo_id(&conn, 2026, 9);

    let super_id = categoria_id(&conn, "Supermercado");
    let transporte = categoria_id(&conn, "Transporte");

    repos::presupuestos::asignar(&conn, agosto, super_id, 300_000).unwrap();
    repos::presupuestos::asignar(&conn, agosto, transporte, 80_000).unwrap();
    // Septiembre ya traía otro monto para supermercado.
    repos::presupuestos::asignar(&conn, septiembre, super_id, 999_999).unwrap();

    let copiadas = repos::presupuestos::copiar(&conn, agosto, septiembre).unwrap();
    assert_eq!(copiadas, 2);

    let mapa = repos::presupuestos::por_categoria(&conn, septiembre).unwrap();
    assert_eq!(mapa.len(), 2);
    assert_eq!(mapa[&super_id], 300_000, "el monto del origen manda");
    assert_eq!(mapa[&transporte], 80_000);

    // El origen queda intacto.
    assert_eq!(repos::presupuestos::total_asignado(&conn, agosto).unwrap(), 380_000);
}

#[test]
fn borrar_una_categoria_del_presupuesto_no_toca_las_otras() {
    let conn = base();
    let periodo = periodo_id(&conn, 2026, 8);
    let super_id = categoria_id(&conn, "Supermercado");
    let transporte = categoria_id(&conn, "Transporte");

    repos::presupuestos::asignar(&conn, periodo, super_id, 300_000).unwrap();
    repos::presupuestos::asignar(&conn, periodo, transporte, 80_000).unwrap();

    repos::presupuestos::eliminar(&conn, periodo, super_id).unwrap();

    let mapa = repos::presupuestos::por_categoria(&conn, periodo).unwrap();
    assert_eq!(mapa.len(), 1);
    assert_eq!(mapa[&transporte], 80_000);
}

// ── reportes ─────────────────────────────────────────────────────────────────

#[test]
fn la_evolucion_no_toma_meses_fuera_de_la_ventana() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    registrar_gasto(&conn, "2026-05-10", 50_000, super_id); // fuera
    registrar_gasto(&conn, "2026-06-10", 60_000, super_id); // primer mes
    registrar_gasto(&conn, "2026-08-10", 80_000, super_id); // último mes
    registrar_gasto(&conn, "2026-09-10", 90_000, super_id); // fuera

    // Ventana de junio a agosto.
    let desde = fechas::mes_absoluto(2026, 6);
    let hasta = fechas::mes_absoluto(2026, 8);
    let filas = repos::movimientos::evolucion_por_categoria(&conn, desde, hasta).unwrap();

    let total: i64 = filas.iter().map(|f| f.5).sum();
    assert_eq!(total, 140_000, "solo junio y agosto entran");
    assert!(filas.iter().all(|f| f.0 == 2026 && (6..=8).contains(&f.1)));
}

#[test]
fn la_ventana_cruza_el_cambio_de_ano() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    registrar_gasto(&conn, "2025-12-10", 40_000, super_id);
    registrar_gasto(&conn, "2026-01-10", 30_000, super_id);

    let ventana = fechas::ventana_de_meses(2026, 1, 3);
    assert_eq!(ventana, vec![(2025, 11), (2025, 12), (2026, 1)]);

    let filas = repos::movimientos::evolucion_por_categoria(
        &conn,
        fechas::mes_absoluto(2025, 11),
        fechas::mes_absoluto(2026, 1),
    )
    .unwrap();

    assert_eq!(filas.iter().map(|f| f.5).sum::<i64>(), 70_000);
}

#[test]
fn la_evolucion_separa_los_gastos_sin_categoria() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    registrar_gasto(&conn, "2026-08-10", 80_000, super_id);

    let periodo = periodo_id(&conn, 2026, 8);
    repos::movimientos::insertar(
        &conn,
        periodo,
        &NuevoMovimiento {
            fecha: "2026-08-11".into(),
            monto: 12_000,
            tipo: TipoMovimiento::Gasto,
            categoria_id: None,
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();

    let abs = fechas::mes_absoluto(2026, 8);
    let filas = repos::movimientos::evolucion_por_categoria(&conn, abs, abs).unwrap();

    let sin_categoria = filas.iter().find(|f| f.2.is_none()).expect("debe existir");
    assert_eq!(sin_categoria.3, "Sin categoría");
    assert_eq!(sin_categoria.5, 12_000);
}

#[test]
fn los_ingresos_no_entran_en_los_reportes_de_gasto() {
    let conn = base();
    let periodo = periodo_id(&conn, 2026, 8);
    let super_id = categoria_id(&conn, "Supermercado");

    registrar_gasto(&conn, "2026-08-10", 80_000, super_id);
    repos::movimientos::insertar(
        &conn,
        periodo,
        &NuevoMovimiento {
            fecha: "2026-08-05".into(),
            monto: 1_500_000,
            tipo: TipoMovimiento::Ingreso,
            categoria_id: Some(super_id),
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();

    let abs = fechas::mes_absoluto(2026, 8);
    let filas = repos::movimientos::evolucion_por_categoria(&conn, abs, abs).unwrap();

    assert_eq!(filas.iter().map(|f| f.5).sum::<i64>(), 80_000);
}

#[test]
fn el_reporte_hormiga_separa_hormiga_del_gasto_total() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");
    let cafe = categoria_id(&conn, "Café y snacks");
    let delivery = categoria_id(&conn, "Delivery");

    registrar_gasto(&conn, "2026-07-10", 100_000, super_id);
    registrar_gasto(&conn, "2026-07-11", 10_000, cafe);

    registrar_gasto(&conn, "2026-08-10", 200_000, super_id);
    registrar_gasto(&conn, "2026-08-11", 4_000, cafe);
    registrar_gasto(&conn, "2026-08-12", 16_000, delivery);

    let filas = repos::movimientos::hormiga_por_periodo(
        &conn,
        fechas::mes_absoluto(2026, 7),
        fechas::mes_absoluto(2026, 8),
    )
    .unwrap();

    assert_eq!(filas.len(), 2);

    let julio = &filas[0];
    assert_eq!((julio.0, julio.1), (2026, 7));
    assert_eq!(julio.2, 10_000, "hormiga de julio");
    assert_eq!(julio.3, 1, "un movimiento hormiga");
    assert_eq!(julio.4, 110_000, "gasto total de julio");

    let agosto = &filas[1];
    assert_eq!(agosto.2, 20_000);
    assert_eq!(agosto.3, 2);
    assert_eq!(agosto.4, 220_000);
}

#[test]
fn el_desglose_hormiga_del_mes_ordena_de_mayor_a_menor() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");
    let cafe = categoria_id(&conn, "Café y snacks");
    let delivery = categoria_id(&conn, "Delivery");

    registrar_gasto(&conn, "2026-08-10", 200_000, super_id);
    registrar_gasto(&conn, "2026-08-11", 4_000, cafe);
    registrar_gasto(&conn, "2026-08-12", 16_000, delivery);

    let periodo = periodo_id(&conn, 2026, 8);
    let desglose = repos::movimientos::hormiga_por_categoria(&conn, periodo).unwrap();

    assert_eq!(desglose.len(), 2, "el supermercado no es hormiga");
    assert_eq!(desglose[0].categoria_nombre, "Delivery");
    assert_eq!(desglose[0].total, 16_000);
    assert_eq!(desglose[1].categoria_nombre, "Café y snacks");
}

#[test]
fn un_mes_sin_movimientos_no_aparece_en_la_consulta() {
    let conn = base();
    let cafe = categoria_id(&conn, "Café y snacks");

    registrar_gasto(&conn, "2026-08-11", 4_000, cafe);
    // Julio existe como período pero sin movimientos.
    periodo_id(&conn, 2026, 7);

    let filas = repos::movimientos::hormiga_por_periodo(
        &conn,
        fechas::mes_absoluto(2026, 6),
        fechas::mes_absoluto(2026, 8),
    )
    .unwrap();

    // Junio nunca se creó; julio sí, en cero. El relleno de la ventana
    // completa lo hace la capa de comandos.
    assert_eq!(filas.len(), 2);
    assert_eq!((filas[0].1, filas[0].2), (7, 0));
    assert_eq!((filas[1].1, filas[1].2), (8, 4_000));
}
