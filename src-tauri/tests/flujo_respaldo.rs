//! Fase 4: respaldo, restauración y exportación.

use std::path::PathBuf;

use finanzas_lib::comandos::respaldo::{contar_registros, tabla_a_csv, validar_respaldo};
use finanzas_lib::db::{conexion, migraciones};
use finanzas_lib::modelos::deuda::{NuevaDeuda, TipoDeuda, DireccionDeuda};
use finanzas_lib::modelos::movimiento::{NuevoMovimiento, TipoMovimiento};
use finanzas_lib::repos;
use rusqlite::{Connection, DatabaseName};

fn base() -> Connection {
    let mut conn = conexion::abrir_en_memoria().expect("abrir base en memoria");
    migraciones::ejecutar(&mut conn).expect("ejecutar migraciones");
    conn
}

/// Carpeta propia por test, para que no se pisen al correr en paralelo.
fn carpeta(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("finanzas-test-{nombre}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn categoria_id(conn: &Connection, nombre: &str) -> i64 {
    repos::categorias::listar(conn, false)
        .unwrap()
        .into_iter()
        .find(|c| c.nombre == nombre)
        .unwrap()
        .id
}

/// Deja datos de las tres fases anteriores para que el respaldo tenga sustancia.
fn con_datos(conn: &Connection) {
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
            // Coma y comillas: justo lo que tiene que escapar el CSV.
            descripcion: Some("Pan, queso y el \"super\"".into()),
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

    let cuotas = finanzas_lib::dominio::amortizacion::generar(
        600_000,
        0.0,
        6,
        finanzas_lib::dominio::fechas::desde_iso("2026-09-05").unwrap(),
    )
    .unwrap();
    repos::cuotas::insertar_muchas(conn, deuda, &cuotas).unwrap();
}

// ── configuración ────────────────────────────────────────────────────────────

#[test]
fn la_marca_de_ultimo_respaldo_se_guarda_y_se_pisa() {
    let conn = base();
    let clave = repos::configuracion::ULTIMO_RESPALDO;

    assert_eq!(repos::configuracion::obtener(&conn, clave).unwrap(), None);

    repos::configuracion::guardar(&conn, clave, "2026-08-01").unwrap();
    assert_eq!(
        repos::configuracion::obtener(&conn, clave).unwrap().as_deref(),
        Some("2026-08-01")
    );

    repos::configuracion::guardar(&conn, clave, "2026-08-15").unwrap();
    assert_eq!(
        repos::configuracion::obtener(&conn, clave).unwrap().as_deref(),
        Some("2026-08-15"),
        "la clave es única: se actualiza en vez de duplicarse"
    );
}

// ── respaldo y restauración ──────────────────────────────────────────────────

#[test]
fn respaldar_y_restaurar_devuelve_los_mismos_datos() {
    let dir = carpeta("ida-y-vuelta");
    let archivo = dir.join("respaldo.db");

    let origen = base();
    con_datos(&origen);
    let registros_originales = contar_registros(&origen).unwrap();
    assert!(registros_originales > 0);

    origen.backup(DatabaseName::Main, &archivo, None).unwrap();
    assert!(archivo.is_file(), "el respaldo debe existir en disco");

    // Una base nueva y vacía recibe el respaldo.
    let mut destino = base();
    assert_ne!(contar_registros(&destino).unwrap(), registros_originales);

    destino
        .restore(
            DatabaseName::Main,
            &archivo,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .unwrap();

    assert_eq!(contar_registros(&destino).unwrap(), registros_originales);

    let periodo = repos::periodos::obtener(&destino, 2026, 8).unwrap().unwrap();
    assert_eq!(periodo.sueldo_liquido, 1_450_000);

    let deudas = repos::deudas::listar(&destino, None, None).unwrap();
    assert_eq!(deudas.len(), 1);
    assert_eq!(repos::cuotas::listar_por_deuda(&destino, deudas[0].id).unwrap().len(), 6);
}

#[test]
fn restaurar_pisa_los_datos_que_habia() {
    let dir = carpeta("pisa-datos");
    let archivo = dir.join("respaldo.db");

    let vacia = base();
    vacia.backup(DatabaseName::Main, &archivo, None).unwrap();

    let mut destino = base();
    con_datos(&destino);
    assert!(contar_registros(&destino).unwrap() > 14);

    destino
        .restore(
            DatabaseName::Main,
            &archivo,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .unwrap();

    // Quedan solo las categorías semilla del respaldo vacío.
    assert!(repos::deudas::listar(&destino, None, None).unwrap().is_empty());
    assert!(repos::periodos::listar(&destino).unwrap().is_empty());
}

#[test]
fn un_respaldo_de_la_app_pasa_la_validacion() {
    let dir = carpeta("valida-ok");
    let archivo = dir.join("respaldo.db");

    let conn = base();
    conn.backup(DatabaseName::Main, &archivo, None).unwrap();

    assert_eq!(
        validar_respaldo(&archivo).unwrap(),
        migraciones::version_objetivo()
    );
}

#[test]
fn se_rechaza_un_archivo_que_no_es_base_de_datos() {
    let dir = carpeta("valida-basura");
    let archivo = dir.join("cualquier-cosa.db");
    std::fs::write(&archivo, "esto no es sqlite").unwrap();

    assert!(validar_respaldo(&archivo).is_err());
}

#[test]
fn se_rechaza_una_base_sqlite_ajena() {
    let dir = carpeta("valida-ajena");
    let archivo = dir.join("otra-app.db");

    let ajena = Connection::open(&archivo).unwrap();
    ajena
        .execute_batch("CREATE TABLE cosas (id INTEGER PRIMARY KEY);")
        .unwrap();
    drop(ajena);

    let error = validar_respaldo(&archivo).unwrap_err().to_string();
    assert!(
        error.contains("no parece un respaldo"),
        "el mensaje debe explicar qué pasó: {error}"
    );
}

#[test]
fn se_rechaza_un_respaldo_de_una_version_mas_nueva() {
    let dir = carpeta("valida-futura");
    let archivo = dir.join("del-futuro.db");

    let conn = base();
    conn.backup(DatabaseName::Main, &archivo, None).unwrap();
    drop(conn);

    // Simula un respaldo hecho por una versión posterior de la app.
    let futura = Connection::open(&archivo).unwrap();
    futura
        .execute_batch(&format!(
            "PRAGMA user_version = {};",
            migraciones::version_objetivo() + 5
        ))
        .unwrap();
    drop(futura);

    let error = validar_respaldo(&archivo).unwrap_err().to_string();
    assert!(
        error.contains("versión más nueva"),
        "el mensaje debe pedir actualizar la app: {error}"
    );
}

#[test]
fn un_respaldo_viejo_se_migra_al_restaurar() {
    let dir = carpeta("respaldo-viejo");
    let archivo = dir.join("v2.db");

    // Base congelada en la versión 2, como la que dejaría una app anterior.
    let vieja = Connection::open(&archivo).unwrap();
    vieja
        .execute_batch(include_str!("../migrations/0001_esquema_inicial.sql"))
        .unwrap();
    vieja
        .execute_batch(include_str!("../migrations/0002_semillas.sql"))
        .unwrap();
    vieja.execute_batch("PRAGMA user_version = 2;").unwrap();
    drop(vieja);

    assert_eq!(validar_respaldo(&archivo).unwrap(), 2);

    let mut destino = base();
    destino
        .restore(
            DatabaseName::Main,
            &archivo,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .unwrap();
    assert_eq!(migraciones::version_actual(&destino).unwrap(), 2);

    // Restaurar deja la base al día con esta versión de la app.
    migraciones::ejecutar(&mut destino).unwrap();
    assert_eq!(
        migraciones::version_actual(&destino).unwrap(),
        migraciones::version_objetivo()
    );

    // Y las columnas nuevas quedan utilizables.
    let periodo = repos::periodos::obtener_o_crear(&destino, 2026, 8).unwrap();
    assert!(repos::movimientos::listar_todos(&destino).unwrap().is_empty());
    assert_eq!(
        repos::presupuestos::por_categoria(&destino, periodo.id)
            .unwrap()
            .len(),
        0
    );
}

// ── protecciones al migrar ───────────────────────────────────────────────────

/// Base en disco congelada en la versión 2, como la dejaría una app anterior.
fn base_en_v2(archivo: &PathBuf) -> Connection {
    let conn = Connection::open(archivo).unwrap();
    conn.execute_batch(include_str!("../migrations/0001_esquema_inicial.sql"))
        .unwrap();
    conn.execute_batch(include_str!("../migrations/0002_semillas.sql"))
        .unwrap();
    conn.execute_batch("PRAGMA user_version = 2;").unwrap();
    conn
}

#[test]
fn una_base_al_dia_o_anterior_es_compatible() {
    let conn = base();
    assert!(migraciones::verificar_compatibilidad(&conn).is_ok());

    let dir = carpeta("compat-vieja");
    let vieja = base_en_v2(&dir.join("v2.db"));
    assert!(
        migraciones::verificar_compatibilidad(&vieja).is_ok(),
        "una base anterior se migra, no se rechaza"
    );
}

#[test]
fn se_rechaza_una_base_mas_nueva_que_el_binario() {
    let conn = base();
    conn.execute_batch(&format!(
        "PRAGMA user_version = {};",
        migraciones::version_objetivo() + 1
    ))
    .unwrap();

    let error = migraciones::verificar_compatibilidad(&conn)
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("más nueva"),
        "el mensaje debe explicar que hay que actualizar la app: {error}"
    );
    assert!(
        error.contains("dañarlos"),
        "y advertir del riesgo de continuar: {error}"
    );
}

#[test]
fn una_base_nueva_no_genera_respaldo_previo() {
    let dir = carpeta("pre-vacia");

    // Recién creada: user_version 0 y sin datos que perder.
    let conn = Connection::open(dir.join("nueva.db")).unwrap();
    assert_eq!(
        migraciones::respaldo_pre_migracion(&conn, &dir).unwrap(),
        None
    );
}

#[test]
fn una_base_al_dia_no_genera_respaldo_previo() {
    let dir = carpeta("pre-al-dia");
    let mut conn = Connection::open(dir.join("aldia.db")).unwrap();
    migraciones::ejecutar(&mut conn).unwrap();

    assert_eq!(
        migraciones::respaldo_pre_migracion(&conn, &dir).unwrap(),
        None,
        "sin migraciones pendientes no hay nada que resguardar"
    );
}

#[test]
fn se_respalda_antes_de_migrar_y_la_copia_conserva_el_esquema_viejo() {
    let dir = carpeta("pre-migracion");
    let archivo = dir.join("datos.db");
    let mut conn = base_en_v2(&archivo);

    let copia = migraciones::respaldo_pre_migracion(&conn, &dir)
        .unwrap()
        .expect("con migraciones pendientes debe respaldar");

    let nombre = copia.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        nombre.starts_with(&format!("finanzas-pre-v{}-", migraciones::version_objetivo())),
        "el nombre lleva la versión destino, que es como uno la busca: {nombre}"
    );
    assert!(nombre.ends_with(".db"));
    assert!(copia.is_file());

    // Migrar la base viva no debe tocar la copia.
    migraciones::ejecutar(&mut conn).unwrap();
    assert_eq!(
        migraciones::version_actual(&conn).unwrap(),
        migraciones::version_objetivo()
    );

    let respaldada = Connection::open(&copia).unwrap();
    assert_eq!(
        migraciones::version_actual(&respaldada).unwrap(),
        2,
        "la copia queda como estaba antes de migrar"
    );
}

#[test]
fn el_respaldo_previo_conserva_los_datos() {
    let dir = carpeta("pre-con-datos");
    let conn = base_en_v2(&dir.join("datos.db"));

    conn.execute(
        "INSERT INTO periodos (anio, mes, sueldo_liquido, otros_ingresos, estado)
         VALUES (2026, 8, 1450000, 0, 'abierto')",
        [],
    )
    .unwrap();

    let copia = migraciones::respaldo_pre_migracion(&conn, &dir)
        .unwrap()
        .unwrap();

    let respaldada = Connection::open(&copia).unwrap();
    let sueldo: i64 = respaldada
        .query_row("SELECT sueldo_liquido FROM periodos", [], |f| f.get(0))
        .unwrap();
    assert_eq!(sueldo, 1_450_000);
}

// ── exportación ──────────────────────────────────────────────────────────────

#[test]
fn el_csv_lleva_encabezado_y_una_fila_por_registro() {
    let conn = base();
    con_datos(&conn);

    let (contenido, filas) = tabla_a_csv(&conn, "movimientos").unwrap();

    assert_eq!(filas, 1);
    let lineas: Vec<&str> = contenido.trim_end().split("\r\n").collect();
    assert_eq!(lineas.len(), 2, "encabezado + una fila");
    assert!(lineas[0].starts_with("id,periodo_id,fecha,monto,tipo"));
    assert!(lineas[1].contains("45990"));
}

#[test]
fn el_csv_escapa_comas_y_comillas_de_las_descripciones() {
    let conn = base();
    con_datos(&conn);

    let (contenido, _) = tabla_a_csv(&conn, "movimientos").unwrap();

    assert!(
        contenido.contains("\"Pan, queso y el \"\"super\"\"\""),
        "la descripción debe quedar encomillada y con comillas duplicadas: {contenido}"
    );
}

#[test]
fn los_nulos_quedan_como_celda_vacia() {
    let conn = base();
    con_datos(&conn);

    let (contenido, _) = tabla_a_csv(&conn, "deudas").unwrap();
    // `institucion` y `notas` van nulas en el dato de prueba.
    assert!(contenido.contains(",,"), "un NULL no debe imprimir 'None'");
    assert!(!contenido.contains("None"));
    assert!(!contenido.contains("NULL"));
}

#[test]
fn se_exportan_las_siete_tablas_del_modelo() {
    let conn = base();
    con_datos(&conn);

    for tabla in [
        "periodos",
        "categorias",
        "servicios",
        "deudas",
        "cuotas",
        "movimientos",
        "presupuestos",
    ] {
        let (contenido, _) = tabla_a_csv(&conn, tabla).unwrap();
        assert!(
            contenido.starts_with("id,"),
            "la tabla {tabla} debe exportar su encabezado"
        );
    }
}

#[test]
fn una_tabla_vacia_exporta_solo_el_encabezado() {
    let conn = base();

    let (contenido, filas) = tabla_a_csv(&conn, "movimientos").unwrap();
    assert_eq!(filas, 0);
    assert_eq!(contenido.matches("\r\n").count(), 1);
}

// ── dónde y con qué nombre queda la copia previa a migrar ────────────────────

#[test]
fn la_copia_previa_a_migrar_va_junto_a_las_demas() {
    use finanzas_lib::db::conexion::carpeta_respaldos_de;
    use std::path::Path;

    let datos = Path::new("/datos/cl.local.finanzas");

    assert_eq!(
        carpeta_respaldos_de(datos),
        datos.join("respaldos"),
        "dejarla en el directorio de datos hizo creer que el respaldo no corría"
    );
}

#[test]
fn la_copia_previa_a_migrar_no_la_barre_la_rotacion() {
    use finanzas_lib::db::migraciones::PREFIJO_PRE_MIGRACION;
    use finanzas_lib::dominio::respaldos;

    let nombre = format!("{PREFIJO_PRE_MIGRACION}v7-2026-08-17-224312.db");

    assert!(
        !respaldos::es_automatico(&nombre),
        "es la red de una actualización, no una copia rutinaria: {nombre}"
    );

    // Y si conviviera con cinco copias automáticas, sigue sin ser candidata.
    let mut carpeta: Vec<String> = (11..=15)
        .map(|d| respaldos::nombre_para(&format!("2026-08-{d}")))
        .collect();
    carpeta.push(nombre.clone());

    let automaticos: Vec<String> = carpeta
        .iter()
        .filter(|n| respaldos::es_automatico(n))
        .cloned()
        .collect();

    assert_eq!(automaticos.len(), 5, "la copia previa no entra al conteo");
    assert!(!respaldos::a_eliminar(&automaticos, respaldos::COPIAS_A_CONSERVAR).contains(&nombre));
}

#[test]
fn dos_migraciones_el_mismo_dia_no_se_pisan() {
    let dir = carpeta("pre-migracion-dos-veces");

    let primera = base_en_v2(&dir.join("una.db"));
    let copia_a = migraciones::respaldo_pre_migracion(&primera, &dir)
        .unwrap()
        .unwrap();

    // Restaurar una base vieja y volver a migrar el mismo día es raro pero
    // posible; el sello de tiempo evita perder la primera copia.
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let segunda = base_en_v2(&dir.join("otra.db"));
    let copia_b = migraciones::respaldo_pre_migracion(&segunda, &dir)
        .unwrap()
        .unwrap();

    assert_ne!(copia_a, copia_b);
    assert!(copia_a.is_file() && copia_b.is_file());
}
