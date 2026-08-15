use std::path::PathBuf;

use rusqlite::{Connection, DatabaseName};
use tauri::{AppHandle, State};

use crate::dominio::fechas;
use crate::error::{AppError, Resultado};
use crate::modelos::categoria::CODIGO_DEUDAS;
use crate::modelos::reinicio::{ResultadoReinicio, ResumenReinicio, CONFIRMACION};
use crate::repos;
use crate::EstadoApp;

/// Tablas que se vacían, en orden de dependencia: los hijos antes que los
/// padres. Con `foreign_keys = ON` el orden inverso aborta el borrado.
const TABLAS_A_VACIAR: [&str; 5] = [
    "movimientos",
    "cuotas",
    "deudas",
    "presupuestos",
    "periodos",
];

/// Cuántos registros se perderían. Se consulta antes de mostrar el diálogo.
#[tauri::command]
pub fn resumen_reinicio(estado: State<'_, EstadoApp>) -> Resultado<ResumenReinicio> {
    let guard = estado.conn();
    let contar = |tabla: &str| -> Resultado<i64> {
        Ok(guard.query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |f| f.get(0))?)
    };

    let deudas = contar("deudas")?;
    let cuotas = contar("cuotas")?;
    let movimientos = contar("movimientos")?;
    let presupuestos = contar("presupuestos")?;
    let periodos = contar("periodos")?;

    let categorias_propias: i64 = guard.query_row(
        "SELECT COUNT(*) FROM categorias WHERE es_semilla = 0",
        [],
        |f| f.get(0),
    )?;

    Ok(ResumenReinicio {
        total: deudas + cuotas + movimientos + presupuestos + periodos,
        deudas,
        cuotas,
        movimientos,
        presupuestos,
        periodos,
        servicios: contar("servicios")?,
        categorias_propias,
    })
}

/// Deja la app como recién instalada: sin deudas, movimientos, presupuestos ni
/// períodos. Conserva las categorías de fábrica y las preferencias.
///
/// Antes de tocar nada guarda un respaldo aparte, fuera de la rotación de
/// copias automáticas: es la única forma de volver atrás.
#[tauri::command]
pub fn reiniciar_datos(
    app: AppHandle,
    estado: State<'_, EstadoApp>,
    confirmacion: String,
    borrar_servicios: bool,
) -> Resultado<ResultadoReinicio> {
    // La interfaz ya lo exige, pero un comando expuesto tiene que defenderse
    // solo: es una operación sin vuelta atrás.
    if confirmacion.trim() != CONFIRMACION {
        return Err(AppError::validacion(format!(
            "Para confirmar hay que escribir exactamente «{CONFIRMACION}»."
        )));
    }

    let mut guard = estado.conn();

    // Sin respaldo no se borra. Si esto falla, la base queda intacta.
    let ruta_respaldo = respaldar_antes(&app, &guard)?;

    let resultado = {
        let tx = guard.transaction()?;
        let borrado = vaciar(&tx, borrar_servicios)?;
        tx.commit()?;
        borrado
    };

    // VACUUM no corre dentro de una transacción, y sin él el archivo conserva
    // el tamaño que tenía aunque esté vacío.
    if let Err(e) = guard.execute_batch("VACUUM;") {
        eprintln!("[reinicio] no se pudo compactar la base: {e}");
    }

    Ok(ResultadoReinicio {
        ruta_respaldo: ruta_respaldo.to_string_lossy().to_string(),
        ..resultado
    })
}

/// Copia de seguridad previa al reinicio.
///
/// Va con un prefijo propio para quedar **fuera** de la rotación de las copias
/// automáticas: si entrara, cinco días después la barrería una copia nueva y
/// se perdería la única vuelta atrás.
fn respaldar_antes(app: &AppHandle, conn: &Connection) -> Resultado<PathBuf> {
    let carpeta = crate::comandos::respaldo::carpeta_respaldos(app)?;
    std::fs::create_dir_all(&carpeta)?;

    let ruta = carpeta.join(format!(
        "finanzas-pre-reinicio-{}.db",
        fechas::sello_de_tiempo()
    ));

    conn.backup(DatabaseName::Main, &ruta, None).map_err(|e| {
        AppError::conflicto(format!(
            "No se pudo guardar el respaldo previo, así que no se borró nada: {e}"
        ))
    })?;

    Ok(ruta)
}

/// El borrado en sí. Recibe la conexión —o la transacción— para poder cubrirlo
/// con tests sin levantar Tauri.
pub fn vaciar(conn: &Connection, borrar_servicios: bool) -> Resultado<ResultadoReinicio> {
    let mut registros_borrados = 0i64;

    for tabla in TABLAS_A_VACIAR {
        registros_borrados += conn.execute(&format!("DELETE FROM {tabla}"), [])? as i64;
    }

    let servicios_borrados = if borrar_servicios {
        conn.execute("DELETE FROM servicios", [])? as i64
    } else {
        // Los servicios que sobreviven no pueden quedar apuntando a una
        // categoría que se va: la FK abortaría el borrado.
        conn.execute(
            "UPDATE servicios SET categoria_id = NULL
             WHERE categoria_id IN (SELECT id FROM categorias WHERE es_semilla = 0)",
            [],
        )?;
        0
    };

    // Las de fábrica vuelven a estar disponibles; no se les toca el nombre ni
    // el color, que son personalizaciones del usuario y no datos financieros.
    let categorias_reactivadas =
        conn.execute("UPDATE categorias SET activa = 1 WHERE es_semilla = 1 AND activa = 0", [])?
            as i64;

    let categorias_borradas =
        conn.execute("DELETE FROM categorias WHERE es_semilla = 0", [])? as i64;

    asegurar_categoria_de_deudas(conn)?;

    Ok(ResultadoReinicio {
        ruta_respaldo: String::new(),
        registros_borrados,
        servicios_borrados,
        categorias_borradas,
        categorias_reactivadas,
    })
}

/// La categoría donde se imputan los pagos de cuotas tiene que existir sí o sí.
/// Si alguien logró dejarla fuera, se recrea.
fn asegurar_categoria_de_deudas(conn: &Connection) -> Resultado<()> {
    if repos::categorias::por_codigo(conn, CODIGO_DEUDAS)?.is_some() {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO categorias (nombre, tipo, color, activa, codigo, es_semilla)
         VALUES ('Deudas y créditos', 'fijo', '#ef4444', 1, ?1, 1)",
        [CODIGO_DEUDAS],
    )?;

    Ok(())
}
