//! Fase 2: gastos, servicios recurrentes y el enlace cuota -> movimiento.

use finanzas_lib::comandos::cuotas::sincronizar_movimiento_de_cuota;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::amortizacion;
use finanzas_lib::dominio::fechas;
use finanzas_lib::modelos::categoria::{NuevaCategoria, TipoCategoria, CODIGO_DEUDAS};
use finanzas_lib::modelos::deuda::{NuevaDeuda, TipoDeuda, DireccionDeuda};
use finanzas_lib::modelos::movimiento::{
    FiltroMovimientos, MedioPago, NuevoMovimiento, TipoMovimiento,
};
use finanzas_lib::modelos::servicio::{NuevoServicio, TipoServicio};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn gasto(fecha: &str, monto: i64, categoria_id: Option<i64>) -> NuevoMovimiento {
    NuevoMovimiento {
        fecha: fecha.into(),
        monto,
        tipo: TipoMovimiento::Gasto,
        categoria_id,
        servicio_id: None,
        medio_pago: Some(MedioPago::Debito),
        descripcion: Some("Compra de prueba".into()),
    }
}

/// Id de una categoría semilla por nombre.
fn categoria_id(conn: &Connection, nombre: &str) -> i64 {
    repos::categorias::listar(conn, false)
        .unwrap()
        .into_iter()
        .find(|c| c.nombre == nombre)
        .unwrap_or_else(|| panic!("falta la categoría semilla '{nombre}'"))
        .id
}

fn registrar(conn: &Connection, datos: &NuevoMovimiento) -> i64 {
    let fecha = fechas::desde_iso(&datos.fecha).unwrap();
    let periodo = repos::periodos::obtener_o_crear(
        conn,
        chrono::Datelike::year(&fecha),
        chrono::Datelike::month(&fecha),
    )
    .unwrap();
    repos::movimientos::insertar(conn, periodo.id, datos).unwrap()
}

// ── migración ────────────────────────────────────────────────────────────────

#[test]
fn la_migracion_deja_la_categoria_de_deudas_con_codigo() {
    let conn = base();

    assert_eq!(
        migraciones::version_actual(&conn).unwrap(),
        migraciones::version_objetivo()
    );

    let cat = repos::categorias::por_codigo(&conn, CODIGO_DEUDAS)
        .unwrap()
        .expect("debe existir la categoría con código 'deudas'");
    assert_eq!(cat.nombre, "Deudas y créditos");
    assert_eq!(cat.tipo, TipoCategoria::Fijo);
}

// ── gastos ───────────────────────────────────────────────────────────────────

#[test]
fn el_movimiento_se_asigna_al_periodo_de_su_fecha() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    registrar(&conn, &gasto("2026-08-15", 45_000, Some(super_id)));
    registrar(&conn, &gasto("2026-09-02", 12_000, Some(super_id)));

    let agosto = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let septiembre = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();

    let (gastos_ago, _, _, _, n_ago) = repos::movimientos::totales(&conn, agosto.id).unwrap();
    let (gastos_sep, _, _, _, n_sep) = repos::movimientos::totales(&conn, septiembre.id).unwrap();

    assert_eq!((gastos_ago, n_ago), (45_000, 1));
    assert_eq!((gastos_sep, n_sep), (12_000, 1));
}

#[test]
fn los_totales_separan_hormiga_de_lo_demas() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");
    let cafe_id = categoria_id(&conn, "Café y snacks");
    let delivery_id = categoria_id(&conn, "Delivery");

    registrar(&conn, &gasto("2026-08-05", 120_000, Some(super_id)));
    registrar(&conn, &gasto("2026-08-06", 3_500, Some(cafe_id)));
    registrar(&conn, &gasto("2026-08-07", 2_800, Some(cafe_id)));
    registrar(&conn, &gasto("2026-08-08", 14_900, Some(delivery_id)));

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let (gastos, ingresos, cuotas, hormiga, n) =
        repos::movimientos::totales(&conn, periodo.id).unwrap();

    assert_eq!(gastos, 141_200);
    assert_eq!(ingresos, 0);
    assert_eq!(cuotas, 0, "ninguno viene de una cuota");
    assert_eq!(hormiga, 3_500 + 2_800 + 14_900);
    assert_eq!(n, 4);
}

#[test]
fn el_desglose_por_categoria_ordena_de_mayor_a_menor() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");
    let cafe_id = categoria_id(&conn, "Café y snacks");

    registrar(&conn, &gasto("2026-08-05", 10_000, Some(cafe_id)));
    registrar(&conn, &gasto("2026-08-06", 90_000, Some(super_id)));
    registrar(&conn, &gasto("2026-08-07", 5_000, None));

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let desglose = repos::movimientos::por_categoria(&conn, periodo.id).unwrap();

    assert_eq!(desglose.len(), 3);
    assert_eq!(desglose[0].categoria_nombre, "Supermercado");
    assert_eq!(desglose[0].total, 90_000);
    assert_eq!(desglose[2].categoria_nombre, "Sin categoría");
    assert_eq!(desglose[2].categoria_id, None);
}

#[test]
fn el_filtro_de_listado_acota_por_tipo_categoria_y_texto() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");
    let cafe_id = categoria_id(&conn, "Café y snacks");

    registrar(&conn, &gasto("2026-08-05", 90_000, Some(super_id)));

    let mut con_texto = gasto("2026-08-06", 4_000, Some(cafe_id));
    con_texto.descripcion = Some("Café en la esquina".into());
    registrar(&conn, &con_texto);

    let mut ingreso = gasto("2026-08-07", 50_000, None);
    ingreso.tipo = TipoMovimiento::Ingreso;
    registrar(&conn, &ingreso);

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();

    let todos = repos::movimientos::listar_detalle(
        &conn,
        periodo.id,
        &FiltroMovimientos::default(),
    )
    .unwrap();
    assert_eq!(todos.len(), 3);

    let solo_gastos = repos::movimientos::listar_detalle(
        &conn,
        periodo.id,
        &FiltroMovimientos {
            tipo: Some(TipoMovimiento::Gasto),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(solo_gastos.len(), 2);

    let por_categoria = repos::movimientos::listar_detalle(
        &conn,
        periodo.id,
        &FiltroMovimientos {
            categoria_id: Some(cafe_id),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(por_categoria.len(), 1);
    assert_eq!(por_categoria[0].categoria_nombre.as_deref(), Some("Café y snacks"));

    let por_texto = repos::movimientos::listar_detalle(
        &conn,
        periodo.id,
        &FiltroMovimientos {
            busqueda: Some("esquina".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(por_texto.len(), 1);
}

// ── enlace cuota -> movimiento ───────────────────────────────────────────────

fn deuda_con_cuotas(conn: &Connection) -> i64 {
    let datos = NuevaDeuda {
        descripcion: "Notebook".into(),
        tipo: TipoDeuda::CompraCuotas,
        institucion: None,
        monto_original: 600_000,
        tasa_mensual: 0.0,
        n_cuotas: 6,
        fecha_primera_cuota: "2026-09-05".into(),
        notas: None,
        direccion: DireccionDeuda::Propia,
        deudor: None,
    };

    let cuotas = amortizacion::generar(600_000, 0.0, 6, fechas::desde_iso("2026-09-05").unwrap())
        .unwrap();
    let id = repos::deudas::insertar(conn, &datos).unwrap();
    repos::cuotas::insertar_muchas(conn, id, &cuotas).unwrap();
    id
}

#[test]
fn pagar_una_cuota_genera_el_gasto_del_mes() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();

    repos::cuotas::registrar_pago(&conn, cuotas[0].id, "2026-09-05", 100_000).unwrap();
    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();

    assert_eq!(detalle.len(), 1);
    let m = &detalle[0];
    assert_eq!(m.movimiento.cuota_id, Some(cuotas[0].id));
    assert_eq!(m.movimiento.monto, 100_000);
    assert_eq!(m.movimiento.tipo, TipoMovimiento::Gasto);
    assert_eq!(m.categoria_nombre.as_deref(), Some("Deudas y créditos"));
    assert_eq!(m.deuda_descripcion.as_deref(), Some("Notebook"));
    assert!(m.es_pago_de_cuota());
    assert_eq!(
        m.movimiento.descripcion.as_deref(),
        Some("Notebook · cuota 1/6")
    );

    let (_, _, total_cuotas, _, _) = repos::movimientos::totales(&conn, periodo.id).unwrap();
    assert_eq!(total_cuotas, 100_000);
}

#[test]
fn repagar_con_otra_fecha_mueve_el_gasto_sin_duplicarlo() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();

    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 100_000).unwrap();
    // Corrección: en realidad se pagó en octubre y con otro monto.
    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-10-02", 95_000).unwrap();

    let septiembre = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let octubre = repos::periodos::obtener(&conn, 2026, 10).unwrap().unwrap();

    let (_, _, _, _, n_sep) = repos::movimientos::totales(&conn, septiembre.id).unwrap();
    let (gastos_oct, _, cuotas_oct, _, n_oct) =
        repos::movimientos::totales(&conn, octubre.id).unwrap();

    assert_eq!(n_sep, 0, "el gasto ya no debe estar en septiembre");
    assert_eq!(n_oct, 1);
    assert_eq!(gastos_oct, 95_000);
    assert_eq!(cuotas_oct, 95_000);
}

#[test]
fn deshacer_el_pago_borra_el_gasto() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();

    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 100_000).unwrap();
    repos::movimientos::eliminar_por_cuota(&conn, cuotas[0].id).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let (_, _, _, _, n) = repos::movimientos::totales(&conn, periodo.id).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn una_cuota_condonada_no_genera_gasto() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();

    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 0).unwrap();

    // No se crea movimiento, y tampoco se crea el período de la nada.
    let periodo = repos::periodos::obtener_o_crear(&conn, 2026, 9).unwrap();
    let (_, _, _, _, n) = repos::movimientos::totales(&conn, periodo.id).unwrap();
    assert_eq!(n, 0, "un pago de $0 no debe ensuciar los gastos del mes");
}

#[test]
fn el_gasto_de_una_cuota_no_se_edita_ni_borra_desde_movimientos() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();

    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();
    let id = detalle[0].movimiento.id;

    assert!(repos::movimientos::eliminar(&conn, id).is_err());
    assert!(
        repos::movimientos::actualizar(&conn, id, periodo.id, &gasto("2026-09-05", 1, None))
            .is_err()
    );
}

// ── períodos cerrados ────────────────────────────────────────────────────────

#[test]
fn un_periodo_cerrado_rechaza_cambios() {
    let conn = base();
    let periodo = repos::periodos::obtener_o_crear(&conn, 2026, 8).unwrap();

    assert!(repos::periodos::exigir_abierto(&conn, periodo.id).is_ok());

    repos::periodos::cambiar_estado(&conn, 2026, 8, "cerrado").unwrap();
    assert!(repos::periodos::exigir_abierto(&conn, periodo.id).is_err());

    repos::periodos::cambiar_estado(&conn, 2026, 8, "abierto").unwrap();
    assert!(repos::periodos::exigir_abierto(&conn, periodo.id).is_ok());

    assert!(
        repos::periodos::cambiar_estado(&conn, 2026, 8, "archivado").is_err(),
        "solo se aceptan 'abierto' y 'cerrado'"
    );
}

// ── servicios recurrentes ────────────────────────────────────────────────────

/// Alta de un servicio replicando lo que hace el comando `crear_servicio`.
fn crear_servicio(
    conn: &Connection,
    nombre: &str,
    categoria_id: i64,
    monto: i64,
    dia: Option<i32>,
    fecha_alta: &str,
) -> i64 {
    repos::servicios::insertar(
        conn,
        &NuevoServicio {
            nombre: nombre.into(),
            categoria_id: Some(categoria_id),
            monto_estimado: monto,
            dia_vencimiento: dia,
            tipo: TipoServicio::Basico,
            activo: true,
            fecha_alta: None,
        },
        fecha_alta,
    )
    .unwrap()
}

#[test]
fn el_gasto_real_por_servicio_se_agrupa_correctamente() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");

    let luz = crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");
    let agua = crear_servicio(&conn, "Aguas Andinas", cat_id, 22_000, None, "2026-08-01");

    let mut boleta = gasto("2026-08-20", 51_300, Some(cat_id));
    boleta.servicio_id = Some(luz);
    registrar(&conn, &boleta);

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let reales = repos::movimientos::real_por_servicio(&conn, periodo.id).unwrap();

    assert_eq!(reales.len(), 1, "el agua todavía no tiene gasto registrado");
    assert_eq!(reales[0], (luz, 51_300, 1, 0), "el registrado a mano no es estimado");
    assert!(!reales.iter().any(|(id, _, _, _)| *id == agua));
}

#[test]
fn un_servicio_con_gastos_no_se_puede_eliminar() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Suscripciones");
    let netflix = crear_servicio(&conn, "Netflix", cat_id, 9_900, Some(8), "2026-08-01");

    assert_eq!(repos::servicios::usos(&conn, netflix).unwrap(), 0);

    let mut cobro = gasto("2026-08-08", 9_900, Some(cat_id));
    cobro.servicio_id = Some(netflix);
    registrar(&conn, &cobro);

    assert_eq!(repos::servicios::usos(&conn, netflix).unwrap(), 1);
}

// ── generación automática del gasto del mes ──────────────────────────────────

/// Réplica de `generar_gastos_servicios` sin la capa de Tauri.
fn generar(conn: &Connection, anio: i32, mes: u32) -> i32 {
    use std::collections::HashSet;

    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    if periodo.estado == "cerrado" {
        return 0;
    }

    let ultimo_dia = fechas::a_iso(fechas::ultimo_dia(anio, mes).unwrap());
    let dias_mes = fechas::dias_del_mes(anio, mes);

    let ya_tienen: HashSet<i64> = repos::movimientos::servicios_con_gasto(conn, periodo.id)
        .unwrap()
        .into_iter()
        .collect();

    let mut creados = 0;
    for s in repos::servicios::listar(conn, true).unwrap() {
        if ya_tienen.contains(&s.id) || s.monto_estimado <= 0 {
            continue;
        }
        if s.fecha_alta.as_deref().map(|a| a > ultimo_dia.as_str()) == Some(true) {
            continue;
        }

        let dia = s
            .dia_vencimiento
            .map(|d| (d.clamp(1, 31) as u32).min(dias_mes))
            .unwrap_or(1);

        repos::movimientos::insertar_estimado_servicio(
            conn,
            periodo.id,
            s.id,
            s.categoria_id,
            &format!("{anio:04}-{mes:02}-{dia:02}"),
            s.monto_estimado,
            &s.nombre,
        )
        .unwrap();
        creados += 1;
    }
    creados
}

#[test]
fn generar_crea_el_gasto_estimado_una_sola_vez() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");

    assert_eq!(generar(&conn, 2026, 8), 1);
    assert_eq!(generar(&conn, 2026, 8), 0, "correr de nuevo no duplica");

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();

    assert_eq!(detalle.len(), 1);
    let m = &detalle[0].movimiento;
    assert!(m.es_estimado, "nace marcado como estimado");
    assert_eq!(m.monto, 45_000);
    assert_eq!(m.fecha, "2026-08-20", "usa el día de vencimiento");
    assert_eq!(m.tipo, TipoMovimiento::Gasto);
    assert_eq!(detalle[0].servicio_nombre.as_deref(), Some("Enel"));
}

#[test]
fn nunca_genera_gastos_antes_del_alta_del_servicio() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Suscripciones");
    crear_servicio(&conn, "Netflix", cat_id, 9_900, Some(8), "2026-08-01");

    // Meses y años anteriores al alta quedan intactos.
    assert_eq!(generar(&conn, 2025, 12), 0);
    assert_eq!(generar(&conn, 2026, 7), 0);
    // El mes del alta y los siguientes sí generan.
    assert_eq!(generar(&conn, 2026, 8), 1);
    assert_eq!(generar(&conn, 2026, 9), 1);

    for (anio, mes) in [(2025, 12), (2026, 7)] {
        let p = repos::periodos::obtener(&conn, anio, mes).unwrap().unwrap();
        let (_, _, _, _, n) = repos::movimientos::totales(&conn, p.id).unwrap();
        assert_eq!(n, 0, "{mes:02}/{anio} no debe tener gastos");
    }
}

#[test]
fn el_vencimiento_se_recorta_en_meses_cortos() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    crear_servicio(&conn, "Gas", cat_id, 30_000, Some(31), "2026-01-01");

    generar(&conn, 2026, 2);
    let periodo = repos::periodos::obtener(&conn, 2026, 2).unwrap().unwrap();
    let detalle =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();

    assert_eq!(detalle[0].movimiento.fecha, "2026-02-28");
}

#[test]
fn un_gasto_registrado_a_mano_evita_que_se_genere_el_estimado() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    let luz = crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");

    let mut boleta = gasto("2026-08-19", 51_300, Some(cat_id));
    boleta.servicio_id = Some(luz);
    registrar(&conn, &boleta);

    assert_eq!(generar(&conn, 2026, 8), 0, "ya hay un gasto para ese servicio");
}

#[test]
fn cambiar_el_precio_confirma_el_gasto_estimado() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");
    generar(&conn, 2026, 8);

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let antes =
        repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
            .unwrap();
    let id = antes[0].movimiento.id;
    assert!(antes[0].movimiento.es_estimado);

    repos::movimientos::cambiar_monto(&conn, id, 51_300).unwrap();

    let despues = repos::movimientos::obtener(&conn, id).unwrap();
    assert_eq!(despues.monto, 51_300);
    assert!(!despues.es_estimado, "cambiar el precio lo da por confirmado");

    let reales = repos::movimientos::real_por_servicio(&conn, periodo.id).unwrap();
    assert_eq!(reales[0].1, 51_300);
    assert_eq!(reales[0].3, 0, "ya no queda ninguno por confirmar");
}

#[test]
fn editar_un_gasto_estimado_tambien_lo_confirma() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");
    generar(&conn, 2026, 8);

    let periodo = repos::periodos::obtener(&conn, 2026, 8).unwrap().unwrap();
    let id = repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
        .unwrap()[0]
        .movimiento
        .id;

    repos::movimientos::actualizar(&conn, id, periodo.id, &gasto("2026-08-21", 47_000, Some(cat_id)))
        .unwrap();

    let despues = repos::movimientos::obtener(&conn, id).unwrap();
    assert!(!despues.es_estimado);
    assert_eq!(despues.monto, 47_000);
}

#[test]
fn el_precio_de_un_pago_de_cuota_no_se_cambia_desde_gastos() {
    let conn = base();
    let deuda_id = deuda_con_cuotas(&conn);
    let cuotas = repos::cuotas::listar_por_deuda(&conn, deuda_id).unwrap();
    sincronizar_movimiento_de_cuota(&conn, &cuotas[0], "2026-09-05", 100_000).unwrap();

    let periodo = repos::periodos::obtener(&conn, 2026, 9).unwrap().unwrap();
    let id = repos::movimientos::listar_detalle(&conn, periodo.id, &FiltroMovimientos::default())
        .unwrap()[0]
        .movimiento
        .id;

    assert!(repos::movimientos::cambiar_monto(&conn, id, 1).is_err());
}

#[test]
fn un_servicio_inactivo_deja_de_generar() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Suscripciones");
    let netflix = crear_servicio(&conn, "Netflix", cat_id, 9_900, Some(8), "2026-08-01");

    assert_eq!(generar(&conn, 2026, 8), 1);

    repos::servicios::actualizar(
        &conn,
        netflix,
        &NuevoServicio {
            nombre: "Netflix".into(),
            categoria_id: Some(cat_id),
            monto_estimado: 9_900,
            dia_vencimiento: Some(8),
            tipo: TipoServicio::Suscripcion,
            activo: false,
            fecha_alta: None,
        },
    )
    .unwrap();

    assert_eq!(generar(&conn, 2026, 9), 0, "inactivo no genera");

    // Y la edición no debe haber movido el alta hacia atrás.
    let s = repos::servicios::obtener(&conn, netflix).unwrap();
    assert_eq!(s.fecha_alta.as_deref(), Some("2026-08-01"));
}

#[test]
fn un_mes_cerrado_no_recibe_gastos_generados() {
    let conn = base();
    let cat_id = categoria_id(&conn, "Servicios básicos");
    crear_servicio(&conn, "Enel", cat_id, 45_000, Some(20), "2026-08-01");

    repos::periodos::obtener_o_crear(&conn, 2026, 8).unwrap();
    repos::periodos::cambiar_estado(&conn, 2026, 8, "cerrado").unwrap();

    assert_eq!(generar(&conn, 2026, 8), 0);
}

// ── categorías ───────────────────────────────────────────────────────────────

#[test]
fn una_categoria_en_uso_reporta_sus_dependencias() {
    let conn = base();

    let nueva = repos::categorias::insertar(
        &conn,
        &NuevaCategoria {
            nombre: "Mascotas".into(),
            tipo: TipoCategoria::Variable,
            color: Some("#22c55e".into()),
            activa: true,
        },
    )
    .unwrap();

    assert_eq!(repos::categorias::usos(&conn, nueva).unwrap(), (0, 0));
    registrar(&conn, &gasto("2026-08-10", 18_000, Some(nueva)));
    assert_eq!(repos::categorias::usos(&conn, nueva).unwrap(), (1, 0));

    // Sin usos sí se puede borrar.
    let libre = repos::categorias::insertar(
        &conn,
        &NuevaCategoria {
            nombre: "Temporal".into(),
            tipo: TipoCategoria::Variable,
            color: None,
            activa: true,
        },
    )
    .unwrap();
    assert!(repos::categorias::eliminar(&conn, libre).is_ok());
}

#[test]
fn las_categorias_inactivas_se_filtran() {
    let conn = base();
    let todas = repos::categorias::listar(&conn, false).unwrap().len();

    let id = categoria_id(&conn, "Delivery");
    repos::categorias::actualizar(
        &conn,
        id,
        &NuevaCategoria {
            nombre: "Delivery".into(),
            tipo: TipoCategoria::Hormiga,
            color: None,
            activa: false,
        },
    )
    .unwrap();

    let activas = repos::categorias::listar(&conn, true).unwrap().len();
    assert_eq!(activas, todas - 1);
}
