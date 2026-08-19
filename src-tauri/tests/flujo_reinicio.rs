//! Reinicio de datos: dejar la app como recién instalada sin perder las
//! categorías de fábrica ni las preferencias.

use std::path::PathBuf;

use finanzas_lib::comandos::reinicio::vaciar;
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::dominio::{amortizacion, fechas};
use finanzas_lib::modelos::categoria::{NuevaCategoria, TipoCategoria, CODIGO_DEUDAS};
use finanzas_lib::modelos::deuda::{NuevaDeuda, TipoDeuda, DireccionDeuda};
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::modelos::servicio::{NuevoServicio, TipoServicio};
use finanzas_lib::repos;
use rusqlite::{Connection, DatabaseName};

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

fn carpeta(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("finanzas-reinicio-{nombre}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn contar(conn: &Connection, tabla: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |f| f.get(0))
        .unwrap()
}

fn categoria_id(conn: &Connection, nombre: &str) -> i64 {
    repos::categorias::listar(conn, false)
        .unwrap()
        .into_iter()
        .find(|c| c.nombre == nombre)
        .unwrap()
        .id
}

/// Deja datos de las cuatro fases: deuda con cuotas, período con sueldo,
/// movimiento, presupuesto, servicio, una categoría propia y una preferencia.
fn con_datos(conn: &Connection) -> (i64, i64) {
    let super_id = categoria_id(conn, "Supermercado");

    let periodo = repos::periodos::obtener_o_crear(conn, 2026, 8).unwrap();
    repos::periodos::actualizar_ingresos(conn, 2026, 8, 1_450_000, 0).unwrap();
    repos::presupuestos::asignar(conn, periodo.id, super_id, 300_000).unwrap();

    repos::movimientos::insertar(
        conn,
        periodo.id,
        &NuevoMovimiento {
            fecha: "2026-08-10".into(),
            monto: 45_990,
            tipo: TipoMovimiento::Gasto,
            categoria_id: Some(super_id),
            servicio_id: None,
            medio_pago: None,
            descripcion: None,
        },
    )
    .unwrap();

    let deuda = repos::deudas::insertar(
        conn,
        &NuevaDeuda {
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
        },
    )
    .unwrap();

    let cuotas =
        amortizacion::generar(600_000, 0.0, 6, fechas::desde_iso("2026-09-05").unwrap()).unwrap();
    repos::cuotas::insertar_muchas(conn, deuda, &cuotas).unwrap();

    // Categoría creada por el usuario y un servicio que la usa.
    let propia = repos::categorias::insertar(
        conn,
        &NuevaCategoria {
            nombre: "Mascotas".into(),
            tipo: TipoCategoria::Variable,
            color: Some("#22c55e".into()),
            activa: true,
        },
    )
    .unwrap();

    let servicio = repos::servicios::insertar(
        conn,
        &NuevoServicio {
            nombre: "Netflix".into(),
            categoria_id: Some(propia),
            monto_estimado: 9_900,
            dia_vencimiento: Some(8),
            tipo: TipoServicio::Suscripcion,
            activo: true,
            fecha_alta: None,
        },
        "2026-08-01",
    )
    .unwrap();

    repos::configuracion::guardar(conn, "accion_cierre", "bandeja").unwrap();

    (propia, servicio)
}

// ── borrado ──────────────────────────────────────────────────────────────────

#[test]
fn el_reinicio_vacia_las_cinco_tablas_de_datos() {
    let conn = base();
    con_datos(&conn);

    for tabla in ["movimientos", "cuotas", "deudas", "presupuestos", "periodos"] {
        assert!(contar(&conn, tabla) > 0, "{tabla} debe tener datos antes");
    }

    let resultado = vaciar(&conn, false).unwrap();

    for tabla in ["movimientos", "cuotas", "deudas", "presupuestos", "periodos"] {
        assert_eq!(contar(&conn, tabla), 0, "{tabla} debe quedar vacía");
    }
    assert!(resultado.registros_borrados > 0);
}

#[test]
fn las_categorias_de_fabrica_sobreviven_y_quedan_activas() {
    let conn = base();
    con_datos(&conn);

    // Una semilla desactivada por el usuario.
    let delivery = categoria_id(&conn, "Delivery");
    repos::categorias::actualizar(
        &conn,
        delivery,
        &NuevaCategoria {
            nombre: "Delivery".into(),
            tipo: TipoCategoria::Hormiga,
            color: None,
            activa: false,
        },
    )
    .unwrap();

    let resultado = vaciar(&conn, false).unwrap();

    let categorias = repos::categorias::listar(&conn, false).unwrap();
    assert_eq!(categorias.len(), 15, "quedan las 15 de fábrica");
    assert!(
        categorias.iter().all(|c| c.activa),
        "todas deben quedar disponibles otra vez"
    );
    assert_eq!(resultado.categorias_reactivadas, 1);
}

#[test]
fn la_categoria_de_deudas_sobrevive() {
    let conn = base();
    con_datos(&conn);

    vaciar(&conn, false).unwrap();

    let deudas = repos::categorias::por_codigo(&conn, CODIGO_DEUDAS)
        .unwrap()
        .expect("la categoría donde se imputan las cuotas no puede desaparecer");
    assert!(deudas.activa);
    assert!(deudas.es_semilla);
}

#[test]
fn la_categoria_de_deudas_se_recrea_si_falta() {
    let conn = base();

    // Escenario límite: alguien la sacó por fuera de la app.
    conn.execute("DELETE FROM categorias WHERE codigo = ?1", [CODIGO_DEUDAS])
        .unwrap();
    assert!(repos::categorias::por_codigo(&conn, CODIGO_DEUDAS)
        .unwrap()
        .is_none());

    vaciar(&conn, false).unwrap();

    assert!(
        repos::categorias::por_codigo(&conn, CODIGO_DEUDAS)
            .unwrap()
            .is_some(),
        "el reinicio debe dejarla de vuelta"
    );
}

#[test]
fn las_categorias_propias_se_borran() {
    let conn = base();
    con_datos(&conn);

    assert_eq!(contar(&conn, "categorias"), 16, "15 de fábrica + Mascotas");

    let resultado = vaciar(&conn, false).unwrap();

    assert_eq!(resultado.categorias_borradas, 1);
    assert!(!repos::categorias::listar(&conn, false)
        .unwrap()
        .iter()
        .any(|c| c.nombre == "Mascotas"));
}

#[test]
fn renombrar_una_semilla_no_la_convierte_en_propia() {
    let conn = base();

    let delivery = categoria_id(&conn, "Delivery");
    repos::categorias::actualizar(
        &conn,
        delivery,
        &NuevaCategoria {
            nombre: "Comida rápida".into(),
            tipo: TipoCategoria::Hormiga,
            color: None,
            activa: true,
        },
    )
    .unwrap();

    vaciar(&conn, false).unwrap();

    let categorias = repos::categorias::listar(&conn, false).unwrap();
    assert_eq!(categorias.len(), 15);
    assert!(
        categorias.iter().any(|c| c.nombre == "Comida rápida"),
        "la semilla renombrada sobrevive y conserva el nombre que le pusiste"
    );
}

// ── servicios ────────────────────────────────────────────────────────────────

#[test]
fn por_omision_los_servicios_se_conservan() {
    let conn = base();
    let (_, servicio) = con_datos(&conn);

    let resultado = vaciar(&conn, false).unwrap();

    assert_eq!(resultado.servicios_borrados, 0);
    assert_eq!(contar(&conn, "servicios"), 1);

    // Apuntaba a una categoría propia que se fue: queda sin categoría, pero vivo.
    let s = repos::servicios::obtener(&conn, servicio).unwrap();
    assert_eq!(s.nombre, "Netflix");
    assert_eq!(
        s.categoria_id, None,
        "la referencia a la categoría borrada debe quedar nula, no romper el borrado"
    );
}

#[test]
fn se_pueden_borrar_tambien_los_servicios() {
    let conn = base();
    con_datos(&conn);

    let resultado = vaciar(&conn, true).unwrap();

    assert_eq!(resultado.servicios_borrados, 1);
    assert_eq!(contar(&conn, "servicios"), 0);
}

#[test]
fn un_servicio_de_categoria_de_fabrica_conserva_su_categoria() {
    let conn = base();
    let super_id = categoria_id(&conn, "Supermercado");

    let servicio = repos::servicios::insertar(
        &conn,
        &NuevoServicio {
            nombre: "Mercado".into(),
            categoria_id: Some(super_id),
            monto_estimado: 5_000,
            dia_vencimiento: None,
            tipo: TipoServicio::Basico,
            activo: true,
            fecha_alta: None,
        },
        "2026-08-01",
    )
    .unwrap();

    vaciar(&conn, false).unwrap();

    let s = repos::servicios::obtener(&conn, servicio).unwrap();
    assert_eq!(
        s.categoria_id,
        Some(super_id),
        "solo se anulan las referencias a categorías que se borran"
    );
}

// ── preferencias ─────────────────────────────────────────────────────────────

#[test]
fn las_preferencias_no_son_datos_financieros_y_se_conservan() {
    let conn = base();
    con_datos(&conn);

    vaciar(&conn, false).unwrap();

    assert_eq!(
        repos::configuracion::obtener(&conn, "accion_cierre")
            .unwrap()
            .as_deref(),
        Some("bandeja")
    );
}

// ── transacción ──────────────────────────────────────────────────────────────

#[test]
fn deshacer_la_transaccion_deja_la_base_intacta() {
    let mut conn = base();
    con_datos(&conn);

    let antes: Vec<i64> = ["movimientos", "cuotas", "deudas", "presupuestos", "periodos", "categorias", "servicios"]
        .iter()
        .map(|t| contar(&conn, t))
        .collect();

    {
        let tx = conn.transaction().unwrap();
        vaciar(&tx, true).unwrap();
        // Simula que algo falla después del borrado: nada debe quedar aplicado.
        tx.rollback().unwrap();
    }

    let despues: Vec<i64> = ["movimientos", "cuotas", "deudas", "presupuestos", "periodos", "categorias", "servicios"]
        .iter()
        .map(|t| contar(&conn, t))
        .collect();

    assert_eq!(antes, despues, "un rollback debe restituirlo todo");
}

#[test]
fn el_borrado_respeta_las_llaves_foraneas() {
    let conn = base();
    con_datos(&conn);

    // `abrir_en_memoria` deja foreign_keys en ON; si el orden de borrado
    // estuviera invertido, esto fallaría en vez de pasar.
    let activas: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |f| f.get(0))
        .unwrap();
    assert_eq!(activas, 1, "el test no valdría con las FK apagadas");

    assert!(vaciar(&conn, false).is_ok());
}

// ── respaldo previo ──────────────────────────────────────────────────────────

#[test]
fn el_respaldo_previo_es_una_base_valida_y_legible() {
    let dir = carpeta("respaldo-previo");
    let conn = base();
    con_datos(&conn);

    let ruta = dir.join(format!(
        "finanzas-pre-reinicio-{}.db",
        fechas::sello_de_tiempo()
    ));
    conn.backup(DatabaseName::Main, &ruta, None).unwrap();

    // Después de reiniciar, la copia sigue teniendo todo.
    vaciar(&conn, true).unwrap();
    assert_eq!(contar(&conn, "deudas"), 0);

    let copia = Connection::open(&ruta).unwrap();
    assert_eq!(contar(&copia, "deudas"), 1);
    assert_eq!(contar(&copia, "cuotas"), 6);
    assert_eq!(contar(&copia, "servicios"), 1);
    assert_eq!(
        migraciones::version_actual(&copia).unwrap(),
        migraciones::version_objetivo()
    );
}

#[test]
fn el_sello_de_tiempo_evita_que_dos_reinicios_se_pisen() {
    let sello = fechas::sello_de_tiempo();

    assert_eq!(sello.len(), "2026-08-15-143012".len());
    assert!(
        sello.starts_with(&fechas::a_iso(fechas::hoy())),
        "empieza con la fecha del día: {sello}"
    );
}

#[test]
fn el_reinicio_borra_los_ahorros_y_deja_el_saldo_inicial_en_cero() {
    use finanzas_lib::comandos::cuentas::{crear, fijar_inicial, mover, Direccion};
    use finanzas_lib::modelos::cuenta::NuevaCuenta;

    let conn = base();

    fijar_inicial(&conn, 800_000).unwrap();
    let viaje = crear(&conn, &NuevaCuenta { nombre: "Viaje".into() }).unwrap();
    crear(&conn, &NuevaCuenta { nombre: "Emergencias".into() }).unwrap();
    mover(&conn, viaje, 300_000, Direccion::Apartar).unwrap();

    let resultado = vaciar(&conn, false).unwrap();

    assert_eq!(resultado.cuentas_borradas, 2);
    assert_eq!(contar(&conn, "cuentas"), 0);
    assert_eq!(
        repos::configuracion::obtener_monto(&conn, repos::configuracion::SALDO_INICIAL).unwrap(),
        0,
        "el saldo inicial es un dato financiero, no una preferencia: no sobrevive"
    );

    // Sin saldo inicial y sin movimientos, el patrimonio arranca de cero.
    let r = finanzas_lib::comandos::cuentas::armar_resumen(&conn).unwrap();
    assert_eq!(r.disponible, 0);
    assert_eq!(r.patrimonio, 0);
}

#[test]
fn el_reinicio_no_deja_notas_de_ahorro_huerfanas() {
    use finanzas_lib::comandos::cuentas::{crear, fijar_inicial, mover, Direccion};
    use finanzas_lib::modelos::cuenta::NuevaCuenta;
    use finanzas_lib::modelos::nota_ahorro::NuevaNota;

    let conn = base();

    fijar_inicial(&conn, 800_000).unwrap();
    let fan = crear(&conn, &NuevaCuenta { nombre: "Fan".into() }).unwrap();
    mover(&conn, fan, 100_000, Direccion::Apartar).unwrap();

    finanzas_lib::comandos::notas_ahorro::crear(
        &conn,
        &NuevaNota {
            cuenta_id: fan,
            nombre: "Libros".into(),
            monto: 25_000,
        },
    )
    .unwrap();
    assert_eq!(contar(&conn, "notas_ahorro"), 1);

    vaciar(&conn, false).unwrap();

    assert_eq!(
        contar(&conn, "notas_ahorro"),
        0,
        "las cuentas se van y el CASCADE tiene que llevarse sus notas"
    );
}

#[test]
fn el_reinicio_borra_las_metas() {
    use finanzas_lib::comandos::cuentas::{crear as crear_cuenta, mover, Direccion};
    use finanzas_lib::comandos::metas::crear as crear_meta;
    use finanzas_lib::modelos::cuenta::NuevaCuenta;
    use finanzas_lib::modelos::meta::NuevaMeta;

    let conn = base();

    finanzas_lib::comandos::cuentas::fijar_inicial(&conn, 800_000).unwrap();
    let viaje = crear_cuenta(&conn, &NuevaCuenta { nombre: "Viaje".into() }).unwrap();
    mover(&conn, viaje, 300_000, Direccion::Apartar).unwrap();

    for (nombre, objetivo) in [("Japón", 2_500_000), ("Notebook", 900_000)] {
        crear_meta(
            &conn,
            &NuevaMeta {
                nombre: nombre.into(),
                monto_objetivo: objetivo,
                cuenta_id: Some(viaje),
                fecha_objetivo: None,
                notas: None,
            },
        )
        .unwrap();
    }

    // Una meta es un dato financiero: el reinicio se la lleva, y se va antes
    // que la cuenta a la que apunta.
    vaciar(&conn, false).unwrap();

    assert_eq!(contar(&conn, "metas"), 0);
    assert_eq!(contar(&conn, "cuentas"), 0);
}

#[test]
fn el_reinicio_borra_el_historial_de_ahorro() {
    use finanzas_lib::comandos::cuentas::{crear, fijar_inicial, mover, Direccion};
    use finanzas_lib::modelos::cuenta::NuevaCuenta;

    let conn = base();

    fijar_inicial(&conn, 800_000).unwrap();
    let viaje = crear(&conn, &NuevaCuenta { nombre: "Viaje".into() }).unwrap();
    mover(&conn, viaje, 300_000, Direccion::Apartar).unwrap();
    mover(&conn, viaje, 100_000, Direccion::Retirar).unwrap();
    assert_eq!(contar(&conn, "movimientos_ahorro"), 2);

    // Nadie nombra `movimientos_ahorro` en el reinicio: las cuentas se van y el
    // CASCADE se lleva su historial. Esto lo verifica.
    vaciar(&conn, false).unwrap();

    assert_eq!(contar(&conn, "cuentas"), 0);
    assert_eq!(
        contar(&conn, "movimientos_ahorro"),
        0,
        "el CASCADE tiene que llevarse el historial con la cuenta"
    );
}
