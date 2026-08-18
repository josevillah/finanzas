//! Activar a mano un servicio en un mes que su alta no cubre.
//!
//! La generación automática nunca retrocede, y eso es deliberado. Esta es la
//! salida manual para "ya lo pagaba, pero lo di de alta recién ahora".

use finanzas_lib::comandos::servicios::activar_en_mes;
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
        .unwrap()
        .id
}

/// Servicio dado de alta en agosto: junio y julio quedan fuera de su alcance.
fn servicio_de_agosto(conn: &Connection, dia: Option<i32>) -> i64 {
    let cat = categoria_id(conn, "Suscripciones");
    repos::servicios::insertar(
        conn,
        &NuevoServicio {
            nombre: "Netflix".into(),
            categoria_id: Some(cat),
            monto_estimado: 9_900,
            dia_vencimiento: dia,
            tipo: TipoServicio::Suscripcion,
            activo: true,
            fecha_alta: None,
        },
        "2026-08-01",
    )
    .unwrap()
}

fn movimientos_del_mes(conn: &Connection, anio: i32, mes: u32) -> Vec<(i64, String, i64, bool)> {
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::movimientos::listar_detalle(conn, periodo.id, &FiltroMovimientos::default())
        .unwrap()
        .into_iter()
        .map(|m| {
            (
                m.movimiento.monto,
                m.movimiento.fecha.clone(),
                m.movimiento.servicio_id.unwrap_or(0),
                m.movimiento.es_estimado,
            )
        })
        .collect()
}

#[test]
fn activar_genera_el_gasto_del_mes_pedido() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    let id = activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();
    assert!(id > 0);

    let junio = movimientos_del_mes(&conn, 2026, 6);
    assert_eq!(junio.len(), 1);
    assert_eq!(junio[0].0, 8_500, "usa el monto que escribió el usuario");
    assert_eq!(junio[0].2, servicio);
}

#[test]
fn el_gasto_nace_confirmado_no_estimado() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();

    let junio = movimientos_del_mes(&conn, 2026, 6);
    assert!(
        !junio[0].3,
        "el monto lo escribió el usuario, no lo proyectó el sistema"
    );
}

#[test]
fn no_modifica_la_fecha_de_alta_del_servicio() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();

    assert_eq!(
        repos::servicios::obtener(&conn, servicio)
            .unwrap()
            .fecha_alta
            .as_deref(),
        Some("2026-08-01"),
        "es una activación puntual, no un cambio retroactivo del servicio"
    );
}

#[test]
fn activar_un_mes_no_arrastra_los_otros() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();

    assert_eq!(movimientos_del_mes(&conn, 2026, 6).len(), 1);
    assert!(
        movimientos_del_mes(&conn, 2026, 7).is_empty(),
        "julio sigue sin el servicio: cada mes se activa por separado"
    );
}

#[test]
fn la_fecha_usa_el_dia_de_vencimiento_recortado_al_mes() {
    let conn = base();

    let mensual = servicio_de_agosto(&conn, Some(31));
    activar_en_mes(&conn, mensual, 2026, 2, 10_000).unwrap();
    assert_eq!(
        movimientos_del_mes(&conn, 2026, 2)[0].1,
        "2026-02-28",
        "el 31 no existe en febrero"
    );

    let cat = categoria_id(&conn, "Servicios básicos");
    let sin_dia = repos::servicios::insertar(
        &conn,
        &NuevoServicio {
            nombre: "Agua".into(),
            categoria_id: Some(cat),
            monto_estimado: 20_000,
            dia_vencimiento: None,
            tipo: TipoServicio::Basico,
            activo: true,
            fecha_alta: None,
        },
        "2026-08-01",
    )
    .unwrap();

    activar_en_mes(&conn, sin_dia, 2026, 3, 21_000).unwrap();
    let marzo = movimientos_del_mes(&conn, 2026, 3);
    assert_eq!(marzo[0].1, "2026-03-01", "sin día definido, el 1");
}

#[test]
fn no_se_puede_activar_dos_veces_el_mismo_mes() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();

    let error = activar_en_mes(&conn, servicio, 2026, 6, 9_000)
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("ya tiene un gasto"),
        "el mensaje debe explicar por qué: {error}"
    );
    assert_eq!(movimientos_del_mes(&conn, 2026, 6).len(), 1);
}

#[test]
fn un_mes_cerrado_no_acepta_activaciones() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    repos::periodos::obtener_o_crear(&conn, 2026, 6).unwrap();
    repos::periodos::cambiar_estado(&conn, 2026, 6, "cerrado").unwrap();

    assert!(activar_en_mes(&conn, servicio, 2026, 6, 8_500).is_err());
    assert!(movimientos_del_mes(&conn, 2026, 6).is_empty());
}

#[test]
fn el_monto_tiene_que_ser_positivo() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));

    assert!(activar_en_mes(&conn, servicio, 2026, 6, 0).is_err());
    assert!(activar_en_mes(&conn, servicio, 2026, 6, -100).is_err());
}

#[test]
fn tras_activarlo_el_servicio_cuenta_en_los_totales_del_mes() {
    let conn = base();
    let servicio = servicio_de_agosto(&conn, Some(8));
    let periodo = repos::periodos::obtener_o_crear(&conn, 2026, 6).unwrap();

    // Antes: junio no tiene nada de este servicio.
    assert!(repos::movimientos::real_por_servicio(&conn, periodo.id)
        .unwrap()
        .is_empty());

    activar_en_mes(&conn, servicio, 2026, 6, 8_500).unwrap();

    let reales = repos::movimientos::real_por_servicio(&conn, periodo.id).unwrap();
    assert_eq!(reales.len(), 1);
    assert_eq!(reales[0].0, servicio);
    assert_eq!(reales[0].1, 8_500);
    assert_eq!(reales[0].3, 0, "ninguno queda pendiente de confirmar");
}
