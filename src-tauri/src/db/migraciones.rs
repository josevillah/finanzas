use std::path::{Path, PathBuf};

use rusqlite::{Connection, DatabaseName};

use crate::error::{AppError, Resultado};

/// Prefijo de las copias previas a migrar. No entran en la rotación de las
/// copias automáticas: son la red de una actualización, no una copia rutinaria.
pub const PREFIJO_PRE_MIGRACION: &str = "finanzas-pre-";

/// Migraciones versionadas. Para agregar una: crear el .sql en
/// `src-tauri/migrations/` y sumar una entrada aquí con la versión siguiente.
/// El orden importa y las versiones deben ser consecutivas desde 1.
const MIGRACIONES: &[(i32, &str, &str)] = &[
    (
        1,
        "0001_esquema_inicial",
        include_str!("../../migrations/0001_esquema_inicial.sql"),
    ),
    (
        2,
        "0002_semillas",
        include_str!("../../migrations/0002_semillas.sql"),
    ),
    (
        3,
        "0003_fase2",
        include_str!("../../migrations/0003_fase2.sql"),
    ),
    (
        4,
        "0004_gastos_estimados",
        include_str!("../../migrations/0004_gastos_estimados.sql"),
    ),
    (
        5,
        "0005_configuracion",
        include_str!("../../migrations/0005_configuracion.sql"),
    ),
    (
        6,
        "0006_categorias_semilla",
        include_str!("../../migrations/0006_categorias_semilla.sql"),
    ),
    (
        7,
        "0007_deudas_terceros",
        include_str!("../../migrations/0007_deudas_terceros.sql"),
    ),
    (
        8,
        "0008_cuentas",
        include_str!("../../migrations/0008_cuentas.sql"),
    ),
    (
        9,
        "0009_saldo_calculado",
        include_str!("../../migrations/0009_saldo_calculado.sql"),
    ),
    (
        10,
        "0010_limpieza_estimados_futuros",
        include_str!("../../migrations/0010_limpieza_estimados_futuros.sql"),
    ),
    (
        11,
        "0011_notas_ahorro",
        include_str!("../../migrations/0011_notas_ahorro.sql"),
    ),
    (
        12,
        "0012_metas",
        include_str!("../../migrations/0012_metas.sql"),
    ),
];

/// Versión de esquema esperada por este binario.
pub fn version_objetivo() -> i32 {
    MIGRACIONES.last().map(|(v, _, _)| *v).unwrap_or(0)
}

/// Versión actual del archivo de base de datos.
pub fn version_actual(conn: &Connection) -> Resultado<i32> {
    let v: i32 = conn.query_row("PRAGMA user_version", [], |f| f.get(0))?;
    Ok(v)
}

/// Rechaza una base creada por una versión más nueva de la aplicación.
///
/// El escenario es real con auto-actualización: el usuario instala una versión
/// que migra el esquema, algo falla, vuelve a la anterior, y esa versión no
/// entiende las tablas nuevas. Sin este chequeo abriría igual y escribiría
/// datos inconsistentes sin avisar.
pub fn verificar_compatibilidad(conn: &Connection) -> Resultado<()> {
    let actual = version_actual(conn)?;
    let objetivo = version_objetivo();

    if actual > objetivo {
        return Err(AppError::conflicto(format!(
            "Tus datos fueron guardados por una versión más nueva de Finanzas \
             (esquema {actual}; esta versión entiende hasta el {objetivo}).\n\n\
             Instala la última versión de la aplicación para poder abrirlos. \
             Continuar con esta versión podría dañarlos.",
        )));
    }

    Ok(())
}

/// Copia la base antes de aplicar migraciones pendientes.
///
/// El nombre lleva la versión **destino**, que es como uno la busca: "la copia
/// previa a tener el esquema 7". Y un sello de tiempo, para que restaurar una
/// base vieja y volver a migrar el mismo día no pise la copia anterior.
///
/// Devuelve la ruta del respaldo, o `None` si no había nada que resguardar:
/// una base recién creada (versión 0) no tiene datos, y una ya al día no se
/// va a tocar.
pub fn respaldo_pre_migracion(conn: &Connection, directorio: &Path) -> Resultado<Option<PathBuf>> {
    let actual = version_actual(conn)?;

    if actual == 0 || actual >= version_objetivo() {
        return Ok(None);
    }

    std::fs::create_dir_all(directorio)?;
    let ruta = directorio.join(format!(
        "{PREFIJO_PRE_MIGRACION}v{}-{}.db",
        version_objetivo(),
        crate::dominio::fechas::sello_de_tiempo()
    ));

    // La API de respaldo de SQLite y no una copia de archivo: con WAL activo
    // una copia plana puede dejar fuera las últimas transacciones.
    conn.backup(DatabaseName::Main, &ruta, None)?;

    Ok(Some(ruta))
}

/// Aplica las migraciones pendientes. Cada una corre dentro de su propia
/// transacción: si falla, el archivo queda en la versión anterior intacto.
/// Es idempotente, así que se llama en cada arranque de la app.
pub fn ejecutar(conn: &mut Connection) -> Resultado<()> {
    ejecutar_hasta(conn, version_objetivo())
}

/// Igual que [`ejecutar`], pero frenando en la versión indicada.
///
/// Existe para los tests: poder dejar una base en la versión anterior, poblarla
/// con los datos que tendría en la vida real y recién entonces aplicar la
/// migración que se quiere observar. En producción siempre se llama a
/// [`ejecutar`], que va hasta el final.
pub fn ejecutar_hasta(conn: &mut Connection, objetivo: i32) -> Resultado<()> {
    let actual = version_actual(conn)?;

    for (version, nombre, sql) in MIGRACIONES {
        if *version <= actual || *version > objetivo {
            continue;
        }

        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        // PRAGMA no admite parámetros ligados, por eso el format!.
        // `version` viene de una constante del binario, no de entrada externa.
        tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        tx.commit()?;

        eprintln!("[migraciones] aplicada {nombre} (v{version})");
    }

    Ok(())
}
