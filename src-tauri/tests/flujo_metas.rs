//! Metas: objetivos de compra o ahorro.
//!
//! Una meta no mueve plata. Lo que estos tests cuidan es que el avance salga
//! del saldo de la cuenta vinculada sin inventar pesos, que varias metas sobre
//! la misma cuenta se repartan ese saldo en orden de prioridad, y —sobre
//! todo— que nada de esto toque el disponible ni el patrimonio.

use finanzas_lib::comandos::cuentas::{self, Direccion};
use finanzas_lib::comandos::metas::{
    actualizar, armar_resumen_al, cambiar_estado, crear, eliminar, reordenar,
};
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::fechas;
use finanzas_lib::modelos::cuenta::NuevaCuenta;
use finanzas_lib::modelos::meta::{EstadoMeta, MetaDetalle, NuevaMeta, ResumenMetas};
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::repos;
use rusqlite::Connection;

/// Día de corte de todos los tests. Fijo, para que el resultado no dependa de
/// cuándo se ejecuten.
const HOY: &str = "2026-08-15";

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn resumen(conn: &Connection) -> ResumenMetas {
    armar_resumen_al(conn, None, fechas::desde_iso(HOY).unwrap()).unwrap()
}

fn resumen_filtrado(conn: &Connection, estado: EstadoMeta) -> ResumenMetas {
    armar_resumen_al(conn, Some(estado), fechas::desde_iso(HOY).unwrap()).unwrap()
}

/// Busca una meta por nombre dentro del resumen. Devuelve una copia para
/// poder escribir `meta(&resumen(&conn), "X")` sin pelear con el préstamo.
fn meta(r: &ResumenMetas, nombre: &str) -> MetaDetalle {
    r.metas
        .iter()
        .find(|d| d.meta.nombre == nombre)
        .unwrap_or_else(|| panic!("no está la meta «{nombre}»"))
        .clone()
}

fn datos(nombre: &str, objetivo: i64, cuenta_id: Option<i64>) -> NuevaMeta {
    NuevaMeta {
        nombre: nombre.into(),
        monto_objetivo: objetivo,
        cuenta_id,
        fecha_objetivo: None,
        notas: None,
    }
}

fn ahorro(conn: &Connection, nombre: &str) -> i64 {
    cuentas::crear(conn, &NuevaCuenta { nombre: nombre.into() }).unwrap()
}

/// Deja plata apartada en una cuenta, con el ingreso que la respalda.
fn apartar(conn: &Connection, cuenta_id: i64, monto: i64) {
    sueldo(conn, 2026, 7, monto);
    cuentas::mover(conn, cuenta_id, monto, Direccion::Apartar).unwrap();
}

fn sueldo(conn: &Connection, anio: i32, mes: u32, monto: i64) {
    let periodo = repos::periodos::obtener_o_crear(conn, anio, mes).unwrap();
    repos::periodos::actualizar_ingresos(
        conn,
        anio,
        mes,
        periodo.sueldo_liquido + monto,
        periodo.otros_ingresos,
    )
    .unwrap();
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

// ── CRUD ─────────────────────────────────────────────────────────────────────

#[test]
fn el_ciclo_de_vida_completo_de_una_meta() {
    let conn = base();

    let id = crear(&conn, &datos("Notebook", 900_000, None)).unwrap();

    let guardada = repos::metas::obtener(&conn, id).unwrap();
    assert_eq!(guardada.nombre, "Notebook");
    assert_eq!(guardada.monto_objetivo, 900_000);
    assert_eq!(guardada.estado, EstadoMeta::Activa);
    assert_eq!(guardada.creada_en, fechas::a_iso(fechas::hoy()));

    actualizar(
        &conn,
        id,
        &NuevaMeta {
            nombre: "Notebook nuevo".into(),
            monto_objetivo: 1_200_000,
            cuenta_id: None,
            fecha_objetivo: Some("2027-01-31".into()),
            notas: Some("  con 32 GB  ".into()),
        },
    )
    .unwrap();

    let editada = repos::metas::obtener(&conn, id).unwrap();
    assert_eq!(editada.nombre, "Notebook nuevo");
    assert_eq!(editada.monto_objetivo, 1_200_000);
    assert_eq!(editada.fecha_objetivo.as_deref(), Some("2027-01-31"));
    assert_eq!(editada.notas.as_deref(), Some("con 32 GB"));

    cambiar_estado(&conn, id, EstadoMeta::Cumplida).unwrap();
    assert_eq!(
        repos::metas::obtener(&conn, id).unwrap().estado,
        EstadoMeta::Cumplida
    );

    eliminar(&conn, id).unwrap();
    assert!(repos::metas::obtener(&conn, id).is_err());
}

#[test]
fn una_meta_necesita_nombre_y_un_objetivo_mayor_a_cero() {
    let conn = base();

    assert!(crear(&conn, &datos("   ", 900_000, None)).is_err());
    assert!(crear(&conn, &datos("Notebook", 0, None)).is_err());
    assert!(crear(&conn, &datos("Notebook", -5_000, None)).is_err());
}

#[test]
fn no_se_puede_vincular_una_meta_a_una_cuenta_que_no_existe() {
    let conn = base();
    assert!(crear(&conn, &datos("Notebook", 900_000, Some(42))).is_err());
}

#[test]
fn una_fecha_objetivo_ilegible_se_rechaza() {
    let conn = base();

    let mut d = datos("Notebook", 900_000, None);
    d.fecha_objetivo = Some("31/12/2026".into());

    assert!(crear(&conn, &d).is_err());
}

#[test]
fn las_metas_nuevas_van_al_final_de_la_lista() {
    let conn = base();

    crear(&conn, &datos("Primera", 100_000, None)).unwrap();
    crear(&conn, &datos("Segunda", 100_000, None)).unwrap();
    crear(&conn, &datos("Tercera", 100_000, None)).unwrap();

    let orden: Vec<String> = repos::metas::listar(&conn, None)
        .unwrap()
        .into_iter()
        .map(|m| m.nombre)
        .collect();

    assert_eq!(orden, vec!["Primera", "Segunda", "Tercera"]);
}

// ── progreso ─────────────────────────────────────────────────────────────────

#[test]
fn el_progreso_sale_del_saldo_de_la_cuenta_vinculada() {
    let conn = base();

    let cuenta = ahorro(&conn, "Viaje");
    apartar(&conn, cuenta, 300_000);
    crear(&conn, &datos("Japón", 1_200_000, Some(cuenta))).unwrap();

    let r = resumen(&conn);
    let d = meta(&r, "Japón");

    assert!(d.tiene_progreso);
    assert_eq!(d.acumulado, 300_000);
    assert_eq!(d.falta, 900_000);
    assert_eq!(d.progreso_pct, 25.0);
    assert_eq!(d.cuenta_nombre.as_deref(), Some("Viaje"));
}

#[test]
fn el_progreso_no_pasa_del_cien_por_ciento_aunque_sobre_saldo() {
    let conn = base();

    let cuenta = ahorro(&conn, "Viaje");
    apartar(&conn, cuenta, 500_000);
    crear(&conn, &datos("Cámara", 300_000, Some(cuenta))).unwrap();

    let d = meta(&resumen(&conn), "Cámara");
    assert_eq!(d.acumulado, 300_000, "nadie acumula más que su objetivo");
    assert_eq!(d.falta, 0);
    assert_eq!(d.progreso_pct, 100.0);
}

#[test]
fn una_meta_sin_cuenta_vinculada_no_muestra_progreso() {
    let conn = base();

    let cuenta = ahorro(&conn, "Viaje");
    apartar(&conn, cuenta, 400_000);
    crear(&conn, &datos("Auto", 5_000_000, None)).unwrap();

    let r = resumen(&conn);
    let d = meta(&r, "Auto");

    assert!(!d.tiene_progreso, "sin cuenta no hay barra que mostrar");
    assert_eq!(d.acumulado, 0);
    assert_eq!(d.falta, 5_000_000, "sigue diciendo cuánto necesita");
    assert_eq!(d.cuenta_nombre, None);
    assert_eq!(d.progreso_pct, 0.0);

    // Y el ahorro que existe no se le adjudica a nadie.
    assert_eq!(r.total_ahorrado, 400_000);
    assert_eq!(r.ahorro_sin_meta, 400_000);
}

// ── reparto por prioridad ────────────────────────────────────────────────────

#[test]
fn dos_metas_sobre_la_misma_cuenta_reparten_segun_prioridad() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 100_000);

    // El ejemplo del brief: A es más prioritaria porque se creó primero.
    crear(&conn, &datos("A", 80_000, Some(cuenta))).unwrap();
    crear(&conn, &datos("B", 50_000, Some(cuenta))).unwrap();

    let r = resumen(&conn);

    assert_eq!(meta(&r, "A").acumulado, 80_000);
    assert_eq!(meta(&r, "A").progreso_pct, 100.0);
    assert_eq!(meta(&r, "B").acumulado, 20_000);
    assert_eq!(meta(&r, "B").falta, 30_000);

    // Ni un peso de más: lo repartido no puede superar el saldo.
    assert_eq!(r.total_acumulado, 100_000);
    assert_eq!(r.ahorro_sin_meta, 0);
}

#[test]
fn cada_cuenta_reparte_su_propio_saldo() {
    let conn = base();

    let viaje = ahorro(&conn, "Viaje");
    let emergencia = ahorro(&conn, "Emergencia");
    apartar(&conn, viaje, 100_000);
    apartar(&conn, emergencia, 60_000);

    crear(&conn, &datos("Japón", 500_000, Some(viaje))).unwrap();
    crear(&conn, &datos("Colchón", 40_000, Some(emergencia))).unwrap();

    let r = resumen(&conn);
    assert_eq!(meta(&r, "Japón").acumulado, 100_000);
    assert_eq!(meta(&r, "Colchón").acumulado, 40_000, "no toma del viaje");
    assert_eq!(r.ahorro_sin_meta, 20_000);
}

#[test]
fn cumplir_la_meta_prioritaria_libera_el_saldo_para_la_siguiente() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 100_000);

    let a = crear(&conn, &datos("A", 80_000, Some(cuenta))).unwrap();
    crear(&conn, &datos("B", 50_000, Some(cuenta))).unwrap();

    cambiar_estado(&conn, a, EstadoMeta::Cumplida).unwrap();

    let r = resumen(&conn);

    // Cumplida: se muestra completa y deja de competir por el saldo.
    assert_eq!(meta(&r, "A").acumulado, 80_000);
    assert_eq!(meta(&r, "A").falta, 0);
    assert_eq!(meta(&r, "B").acumulado, 50_000, "ahora B alcanza el saldo");

    // Los totales son solo de las activas: A ya no suma objetivo.
    assert_eq!(r.total_objetivo, 50_000);
    assert_eq!(r.total_falta, 0);
    assert_eq!(r.n_activas, 1);
    assert_eq!(r.n_cumplidas, 1);
}

#[test]
fn una_meta_archivada_no_reserva_saldo_ni_muestra_avance() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 100_000);

    let a = crear(&conn, &datos("A", 80_000, Some(cuenta))).unwrap();
    crear(&conn, &datos("B", 50_000, Some(cuenta))).unwrap();
    cambiar_estado(&conn, a, EstadoMeta::Archivada).unwrap();

    let r = resumen(&conn);
    assert_eq!(meta(&r, "A").acumulado, 0);
    assert!(!meta(&r, "A").tiene_progreso);
    assert_eq!(meta(&r, "B").acumulado, 50_000);
    assert_eq!(r.n_archivadas, 1);
}

// ── vínculo con la cuenta ────────────────────────────────────────────────────

#[test]
fn borrar_la_cuenta_desvincula_las_metas_sin_borrarlas() {
    let conn = base();

    let cuenta = ahorro(&conn, "Viaje");
    apartar(&conn, cuenta, 200_000);
    let id = crear(&conn, &datos("Japón", 1_000_000, Some(cuenta))).unwrap();

    // Eliminar exige devolver la plata al disponible primero.
    cuentas::mover(&conn, cuenta, 200_000, Direccion::Retirar).unwrap();
    cuentas::eliminar(&conn, cuenta).unwrap();

    let sobreviviente = repos::metas::obtener(&conn, id).unwrap();
    assert_eq!(sobreviviente.nombre, "Japón");
    assert_eq!(sobreviviente.cuenta_id, None, "ON DELETE SET NULL");
    assert_eq!(sobreviviente.monto_objetivo, 1_000_000);

    let d = meta(&resumen(&conn), "Japón");
    assert!(!d.tiene_progreso);
    assert_eq!(d.acumulado, 0);
    assert_eq!(d.falta, 1_000_000);
}

// ── reordenamiento ───────────────────────────────────────────────────────────

#[test]
fn el_reordenamiento_persiste_y_cambia_el_reparto() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 100_000);

    let a = crear(&conn, &datos("A", 80_000, Some(cuenta))).unwrap();
    let b = crear(&conn, &datos("B", 50_000, Some(cuenta))).unwrap();

    reordenar(&conn, &[b, a]).unwrap();

    let orden: Vec<i64> = repos::metas::listar(&conn, None)
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(orden, vec![b, a], "el orden queda guardado");

    let prioridades: Vec<i32> = repos::metas::listar(&conn, None)
        .unwrap()
        .into_iter()
        .map(|m| m.prioridad)
        .collect();
    assert_eq!(prioridades, vec![0, 1], "se renumera desde 0, sin empates");

    let r = resumen(&conn);
    assert_eq!(meta(&r, "B").acumulado, 50_000, "ahora B consume primero");
    assert_eq!(meta(&r, "A").acumulado, 50_000);
}

#[test]
fn reordenar_una_parte_deja_al_resto_detras_y_sin_empates() {
    let conn = base();

    let a = crear(&conn, &datos("A", 10_000, None)).unwrap();
    let b = crear(&conn, &datos("B", 10_000, None)).unwrap();
    let c = crear(&conn, &datos("C", 10_000, None)).unwrap();

    // La pantalla mostraba solo A y C —por el filtro— y el usuario los da
    // vuelta. B no viene en la lista.
    reordenar(&conn, &[c, a]).unwrap();

    let orden: Vec<(i64, i32)> = repos::metas::listar(&conn, None)
        .unwrap()
        .into_iter()
        .map(|m| (m.id, m.prioridad))
        .collect();

    assert_eq!(orden, vec![(c, 0), (a, 1), (b, 2)]);
}

#[test]
fn reordenar_con_una_meta_inexistente_o_repetida_no_cambia_nada() {
    let conn = base();

    let a = crear(&conn, &datos("A", 10_000, None)).unwrap();
    let b = crear(&conn, &datos("B", 10_000, None)).unwrap();

    assert!(reordenar(&conn, &[b, 999]).is_err());
    assert!(reordenar(&conn, &[b, b]).is_err());

    let orden: Vec<i64> = repos::metas::listar(&conn, None)
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(orden, vec![a, b], "el orden original sigue en pie");
}

// ── fecha objetivo y ritmo ───────────────────────────────────────────────────

#[test]
fn el_ritmo_necesario_sale_de_la_fecha_objetivo() {
    let conn = base();

    let mut d = datos("Notebook", 900_000, None);
    // Tres meses después del día de corte.
    d.fecha_objetivo = Some("2026-11-15".into());
    crear(&conn, &d).unwrap();

    let r = resumen(&conn);
    let detalle = meta(&r, "Notebook");

    assert_eq!(detalle.meses_restantes, Some(3));
    assert_eq!(detalle.ritmo_mensual, Some(300_000));
    assert!(!detalle.fecha_pasada);
}

#[test]
fn una_fecha_dentro_del_mismo_mes_deja_un_mes_para_juntar() {
    let conn = base();

    let mut d = datos("Regalo", 90_000, None);
    d.fecha_objetivo = Some("2026-08-28".into());
    crear(&conn, &d).unwrap();

    let detalle = meta(&resumen(&conn), "Regalo");
    assert_eq!(detalle.meses_restantes, Some(1));
    assert_eq!(detalle.ritmo_mensual, Some(90_000));
}

#[test]
fn una_fecha_vencida_se_marca_y_no_inventa_un_ritmo() {
    let conn = base();

    let mut d = datos("Vacaciones", 500_000, None);
    d.fecha_objetivo = Some("2026-06-30".into());
    crear(&conn, &d).unwrap();

    let detalle = meta(&resumen(&conn), "Vacaciones");
    assert!(detalle.fecha_pasada);
    assert_eq!(detalle.ritmo_mensual, None);
    assert_eq!(detalle.meses_restantes, None);
    assert_eq!(detalle.falta, 500_000);
}

#[test]
fn una_meta_cubierta_con_la_fecha_pasada_no_queda_marcada() {
    let conn = base();

    let cuenta = ahorro(&conn, "Viaje");
    apartar(&conn, cuenta, 500_000);

    let mut d = datos("Vacaciones", 500_000, Some(cuenta));
    d.fecha_objetivo = Some("2026-06-30".into());
    crear(&conn, &d).unwrap();

    let detalle = meta(&resumen(&conn), "Vacaciones");
    assert!(
        !detalle.fecha_pasada,
        "la fecha pasó, pero la plata está: no hay nada que avisar"
    );
    assert_eq!(detalle.falta, 0);
}

// ── proyección contra el balance ─────────────────────────────────────────────

#[test]
fn con_balance_promedio_negativo_no_se_proyecta_nada() {
    let conn = base();

    // Tres meses cerrados gastando más de lo que entra.
    for mes in 5..=7 {
        sueldo(&conn, 2026, mes, 500_000);
        gasto(&conn, 2026, mes, 700_000);
    }

    crear(&conn, &datos("Notebook", 900_000, None)).unwrap();

    let r = resumen(&conn);

    assert_eq!(r.balance_promedio, Some(-200_000));
    assert_eq!(r.meses_considerados, 3);
    assert_eq!(r.meses_al_ritmo, None, "al ritmo actual no se alcanza");
    assert_eq!(meta(&r, "Notebook").meses_al_ritmo, None);
}

#[test]
fn con_balance_en_cero_tampoco_hay_proyeccion() {
    let conn = base();

    for mes in 5..=7 {
        sueldo(&conn, 2026, mes, 500_000);
        gasto(&conn, 2026, mes, 500_000);
    }

    crear(&conn, &datos("Notebook", 900_000, None)).unwrap();

    let r = resumen(&conn);
    assert_eq!(r.balance_promedio, Some(0));
    assert_eq!(r.meses_al_ritmo, None);
}

#[test]
fn con_balance_positivo_se_dice_en_cuantos_meses_se_llega() {
    let conn = base();

    for mes in 5..=7 {
        sueldo(&conn, 2026, mes, 900_000);
        gasto(&conn, 2026, mes, 700_000);
    }

    crear(&conn, &datos("Notebook", 900_000, None)).unwrap();

    let r = resumen(&conn);

    assert_eq!(r.balance_promedio, Some(200_000));
    // 900.000 / 200.000 = 4,5 meses -> 5, redondeando hacia arriba.
    assert_eq!(r.meses_al_ritmo, Some(5));
    assert_eq!(meta(&r, "Notebook").meses_al_ritmo, Some(5));
}

#[test]
fn el_mes_en_curso_no_entra_en_el_promedio() {
    let conn = base();

    for mes in 5..=7 {
        sueldo(&conn, 2026, mes, 900_000);
        gasto(&conn, 2026, mes, 700_000);
    }
    // Agosto recién empieza: el sueldo todavía no entra y ya hay gastos.
    gasto(&conn, 2026, 8, 400_000);

    let r = resumen(&conn);
    assert_eq!(r.balance_promedio, Some(200_000));
    assert_eq!(r.meses_considerados, 3);
}

#[test]
fn un_mes_sin_actividad_no_hunde_el_promedio() {
    let conn = base();

    // Julio con datos; junio y mayo existen como fila pero están vacíos, que
    // es lo que pasa con solo navegar a esos meses.
    repos::periodos::obtener_o_crear(&conn, 2026, 5).unwrap();
    repos::periodos::obtener_o_crear(&conn, 2026, 6).unwrap();
    sueldo(&conn, 2026, 7, 900_000);
    gasto(&conn, 2026, 7, 700_000);

    let r = resumen(&conn);
    assert_eq!(r.balance_promedio, Some(200_000));
    assert_eq!(r.meses_considerados, 1);
}

#[test]
fn sin_historial_no_hay_promedio_que_mostrar() {
    let conn = base();
    crear(&conn, &datos("Notebook", 900_000, None)).unwrap();

    let r = resumen(&conn);
    assert_eq!(r.balance_promedio, None);
    assert_eq!(r.meses_considerados, 0);
    assert_eq!(r.meses_al_ritmo, None);
}

// ── totales y filtro ─────────────────────────────────────────────────────────

#[test]
fn los_totales_comparan_los_objetivos_activos_contra_lo_ahorrado() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 300_000);

    crear(&conn, &datos("A", 200_000, Some(cuenta))).unwrap();
    crear(&conn, &datos("B", 500_000, Some(cuenta))).unwrap();
    crear(&conn, &datos("C", 1_000_000, None)).unwrap();

    let r = resumen(&conn);

    assert_eq!(r.total_objetivo, 1_700_000);
    assert_eq!(r.total_acumulado, 300_000);
    assert_eq!(r.total_falta, 1_400_000);
    assert_eq!(r.total_ahorrado, 300_000);
    assert_eq!(r.ahorro_sin_meta, 0);
}

#[test]
fn el_filtro_cambia_la_lista_pero_no_los_totales() {
    let conn = base();

    let a = crear(&conn, &datos("Cumplida", 100_000, None)).unwrap();
    crear(&conn, &datos("Activa", 300_000, None)).unwrap();
    cambiar_estado(&conn, a, EstadoMeta::Cumplida).unwrap();

    let solo_cumplidas = resumen_filtrado(&conn, EstadoMeta::Cumplida);
    assert_eq!(solo_cumplidas.metas.len(), 1);
    assert_eq!(solo_cumplidas.metas[0].meta.nombre, "Cumplida");
    assert_eq!(
        solo_cumplidas.total_objetivo, 300_000,
        "los totales siguen siendo los de las activas"
    );

    let solo_activas = resumen_filtrado(&conn, EstadoMeta::Activa);
    assert_eq!(solo_activas.metas.len(), 1);
    assert_eq!(solo_activas.metas[0].meta.nombre, "Activa");
}

// ── independencia de los saldos ──────────────────────────────────────────────

#[test]
fn las_metas_no_afectan_el_disponible_ni_el_patrimonio() {
    let conn = base();

    cuentas::fijar_inicial(&conn, 200_000).unwrap();
    sueldo(&conn, 2026, 7, 900_000);
    gasto(&conn, 2026, 7, 300_000);

    let cuenta = ahorro(&conn, "Viaje");
    cuentas::mover(&conn, cuenta, 400_000, Direccion::Apartar).unwrap();

    let antes = cuentas::armar_resumen(&conn).unwrap();

    // Crear, cumplir, archivar y borrar metas: nada de esto es plata.
    let a = crear(&conn, &datos("Japón", 3_000_000, Some(cuenta))).unwrap();
    let b = crear(&conn, &datos("Notebook", 900_000, None)).unwrap();
    cambiar_estado(&conn, a, EstadoMeta::Cumplida).unwrap();
    reordenar(&conn, &[b, a]).unwrap();
    eliminar(&conn, b).unwrap();

    let despues = cuentas::armar_resumen(&conn).unwrap();

    assert_eq!(despues.disponible, antes.disponible);
    assert_eq!(despues.patrimonio, antes.patrimonio);
    assert_eq!(despues.total_ahorrado, antes.total_ahorrado);
    assert_eq!(despues.desglose.gastos, antes.desglose.gastos);
    assert_eq!(despues.desglose.apartado, antes.desglose.apartado);
}

#[test]
fn una_meta_mayor_que_todo_lo_ahorrado_no_rompe_ningun_calculo() {
    let conn = base();

    let cuenta = ahorro(&conn, "Ahorro");
    apartar(&conn, cuenta, 50_000);
    crear(&conn, &datos("Casa", 90_000_000, Some(cuenta))).unwrap();

    let r = resumen(&conn);
    let d = meta(&r, "Casa");

    assert_eq!(d.acumulado, 50_000);
    assert_eq!(d.falta, 89_950_000);
    assert!(d.progreso_pct > 0.0 && d.progreso_pct < 1.0);
    assert_eq!(cuentas::armar_resumen(&conn).unwrap().total_ahorrado, 50_000);
}
