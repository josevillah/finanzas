//! Tests de integración contra una base SQLite en memoria: migraciones,
//! materialización de cuotas y ciclo de pago.

use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::amortizacion;
use finanzas_lib::dominio::fechas;
use finanzas_lib::modelos::cuota::EstadoCuota;
use finanzas_lib::modelos::deuda::{EstadoDeuda, NuevaDeuda, TipoDeuda, DireccionDeuda};
use finanzas_lib::repos;
use rusqlite::Connection;

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn deuda_ejemplo() -> NuevaDeuda {
    NuevaDeuda {
        descripcion: "Notebook en 12 cuotas".into(),
        tipo: TipoDeuda::CompraCuotas,
        institucion: Some("Falabella".into()),
        monto_original: 899_990,
        tasa_mensual: 0.0,
        n_cuotas: 12,
        fecha_primera_cuota: "2026-09-05".into(),
        notas: None,
        direccion: DireccionDeuda::Propia,
        deudor: None,
    }
}

/// Replica lo que hace el comando `crear_deuda`.
fn crear(conn: &Connection, datos: &NuevaDeuda) -> i64 {
    let primera = fechas::desde_iso(&datos.fecha_primera_cuota).unwrap();
    let cuotas = amortizacion::generar(
        datos.monto_original,
        datos.tasa_mensual,
        datos.n_cuotas,
        primera,
    )
    .unwrap();

    let id = repos::deudas::insertar(conn, datos).unwrap();
    repos::cuotas::insertar_muchas(conn, id, &cuotas).unwrap();
    id
}

#[test]
fn las_migraciones_dejan_el_esquema_en_la_version_objetivo() {
    let conn = base();

    assert_eq!(
        migraciones::version_actual(&conn).unwrap(),
        migraciones::version_objetivo()
    );

    let tablas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name IN
             ('periodos','categorias','servicios','movimientos','deudas','cuotas','presupuestos')",
            [],
            |f| f.get(0),
        )
        .unwrap();
    assert_eq!(tablas, 7, "deben existir las 7 tablas del modelo");

    let categorias: i64 = conn
        .query_row("SELECT COUNT(*) FROM categorias", [], |f| f.get(0))
        .unwrap();
    assert!(categorias > 0, "las semillas deben cargar categorías base");
}

#[test]
fn ejecutar_migraciones_dos_veces_no_falla() {
    let mut conn = base();
    migraciones::ejecutar(&mut conn).expect("las migraciones deben ser idempotentes");

    let categorias: i64 = conn
        .query_row("SELECT COUNT(*) FROM categorias", [], |f| f.get(0))
        .unwrap();
    // 14 de gasto más "Préstamos cobrados", que llegó con las deudas de terceros.
    let esperado: i64 = 15;
    assert_eq!(categorias, esperado, "las semillas no deben duplicarse");
}

#[test]
fn crear_deuda_materializa_las_cuotas_en_la_base() {
    let conn = base();
    let datos = deuda_ejemplo();
    let id = crear(&conn, &datos);

    let cuotas = repos::cuotas::listar_por_deuda(&conn, id).unwrap();
    assert_eq!(cuotas.len(), 12);
    assert_eq!(
        cuotas.iter().map(|c| c.monto).sum::<i64>(),
        datos.monto_original,
        "las cuotas guardadas deben sumar el monto original"
    );
    assert_eq!(cuotas[0].fecha_vencimiento, "2026-09-05");
    assert_eq!(cuotas[11].fecha_vencimiento, "2027-08-05");
}

#[test]
fn eliminar_la_deuda_arrastra_sus_cuotas() {
    let conn = base();
    let id = crear(&conn, &deuda_ejemplo());

    repos::deudas::eliminar(&conn, id).unwrap();

    let cuotas = repos::cuotas::listar_por_deuda(&conn, id).unwrap();
    assert!(cuotas.is_empty(), "el ON DELETE CASCADE debe borrar las cuotas");
}

#[test]
fn pagar_cuotas_actualiza_avance_y_estado_de_la_deuda() {
    let conn = base();
    let id = crear(&conn, &deuda_ejemplo());
    let cuotas = repos::cuotas::listar_por_deuda(&conn, id).unwrap();

    // Pago con monto distinto al programado: se registra el real.
    repos::cuotas::registrar_pago(&conn, cuotas[0].id, "2026-09-04", 75_000).unwrap();
    repos::deudas::sincronizar_estado(&conn, id).unwrap();

    let r = repos::cuotas::resumen(&conn, id).unwrap();
    assert_eq!(r.cuotas_pagadas, 1);
    assert_eq!(r.monto_pagado, 75_000);
    assert_eq!(r.monto_pendiente, 899_990 - cuotas[0].monto);
    assert_eq!(
        repos::deudas::obtener(&conn, id).unwrap().estado,
        EstadoDeuda::Vigente
    );

    // Al pagar la última cuota pendiente, la deuda queda pagada.
    for c in &cuotas[1..] {
        repos::cuotas::registrar_pago(&conn, c.id, "2026-09-04", c.monto).unwrap();
    }
    repos::deudas::sincronizar_estado(&conn, id).unwrap();
    assert_eq!(
        repos::deudas::obtener(&conn, id).unwrap().estado,
        EstadoDeuda::Pagada
    );

    // Y al deshacer un pago vuelve a vigente.
    repos::cuotas::deshacer_pago(&conn, cuotas[5].id).unwrap();
    repos::deudas::sincronizar_estado(&conn, id).unwrap();
    assert_eq!(
        repos::deudas::obtener(&conn, id).unwrap().estado,
        EstadoDeuda::Vigente
    );
}

#[test]
fn marcar_atrasadas_es_idempotente_y_reversible() {
    let conn = base();
    let id = crear(&conn, &deuda_ejemplo());

    // Con "hoy" en enero 2027, las cuotas de sep-dic 2026 están atrasadas.
    let atrasadas = repos::cuotas::marcar_atrasadas(&conn, "2027-01-15").unwrap();
    assert_eq!(atrasadas, 5, "sep, oct, nov, dic 2026 y ene 2027 (día 5)");

    let repetido = repos::cuotas::marcar_atrasadas(&conn, "2027-01-15").unwrap();
    assert_eq!(repetido, 0, "correr de nuevo no debe marcar nada más");

    let r = repos::cuotas::resumen(&conn, id).unwrap();
    assert_eq!(r.cuotas_atrasadas, 5);

    // Volviendo "hoy" al pasado, ninguna sigue atrasada.
    repos::cuotas::marcar_atrasadas(&conn, "2026-08-01").unwrap();
    let r = repos::cuotas::resumen(&conn, id).unwrap();
    assert_eq!(r.cuotas_atrasadas, 0);
}

#[test]
fn el_calendario_agrupa_por_mes_solo_deudas_vigentes() {
    let conn = base();
    let id_vigente = crear(&conn, &deuda_ejemplo());

    let mut otra = deuda_ejemplo();
    otra.descripcion = "Deuda repactada".into();
    otra.monto_original = 120_000;
    otra.n_cuotas = 12;
    let id_repactada = crear(&conn, &otra);
    repos::deudas::cambiar_estado(&conn, id_repactada, EstadoDeuda::Repactada).unwrap();

    let meses = repos::cuotas::carga_por_mes(&conn, "2026-09-01", "2026-09-30").unwrap();
    assert_eq!(meses.len(), 1);

    let (clave, total, pendiente, n) = &meses[0];
    assert_eq!(clave, "2026-09");
    assert_eq!(*n, 1, "la deuda repactada no debe contarse");

    let cuota_sep = repos::cuotas::listar_por_deuda(&conn, id_vigente).unwrap()[0].monto;
    assert_eq!(*total, cuota_sep);
    assert_eq!(*pendiente, cuota_sep);
}

#[test]
fn las_cuotas_del_mes_cuadran_con_el_total_de_la_carga() {
    let conn = base();
    let id = crear(&conn, &deuda_ejemplo());
    let cuotas = repos::cuotas::listar_por_deuda(&conn, id).unwrap();

    // Se paga la cuota de septiembre: sigue siendo compromiso de ese mes.
    repos::cuotas::registrar_pago(&conn, cuotas[0].id, "2026-09-03", cuotas[0].monto).unwrap();

    let del_mes = repos::cuotas::en_rango_con_deuda(&conn, "2026-09-01", "2026-09-30").unwrap();
    let (total, n) = repos::cuotas::total_en_rango(&conn, "2026-09-01", "2026-09-30").unwrap();

    assert_eq!(del_mes.len() as i32, n, "el listado y el total deben contar lo mismo");
    assert_eq!(del_mes.iter().map(|(c, _)| c.monto).sum::<i64>(), total);
    assert_eq!(del_mes[0].1, "Notebook en 12 cuotas", "debe traer el nombre de la deuda");
    assert_eq!(del_mes[0].0.estado, EstadoCuota::Pagada);
}

#[test]
fn las_pendientes_excluyen_las_pagadas() {
    let conn = base();
    let id = crear(&conn, &deuda_ejemplo());
    let cuotas = repos::cuotas::listar_por_deuda(&conn, id).unwrap();

    repos::cuotas::registrar_pago(&conn, cuotas[0].id, "2026-09-03", cuotas[0].monto).unwrap();

    let pendientes = repos::cuotas::pendientes_con_deuda(&conn).unwrap();
    assert_eq!(pendientes.len(), 11);
    assert!(pendientes.iter().all(|(c, _)| c.estado != EstadoCuota::Pagada));
    // Vienen ordenadas por vencimiento: la primera pendiente es la de octubre.
    assert_eq!(pendientes[0].0.fecha_vencimiento, "2026-10-05");
}

#[test]
fn el_periodo_se_crea_al_vuelo_y_guarda_el_sueldo() {
    let conn = base();

    let p = repos::periodos::obtener_o_crear(&conn, 2026, 8).unwrap();
    assert_eq!(p.sueldo_liquido, 0);

    repos::periodos::actualizar_ingresos(&conn, 2026, 8, 1_450_000, 120_000).unwrap();

    let p = repos::periodos::obtener_o_crear(&conn, 2026, 8).unwrap();
    assert_eq!(p.sueldo_liquido, 1_450_000);
    assert_eq!(p.otros_ingresos, 120_000);
    assert_eq!(p.estado, "abierto");
}
