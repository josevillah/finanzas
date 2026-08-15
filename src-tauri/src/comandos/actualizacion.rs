use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::{AppError, Resultado};

/// Evento que avisa al frontend que ya hay una actualización descargada.
pub const EVENTO_ACTUALIZACION_LISTA: &str = "actualizacion-lista";

/// Actualización ya bajada, esperando que el usuario confirme.
pub struct ActualizacionPendiente {
    pub version: String,
    pub notas: Option<String>,
    /// El instalador completo en memoria. Se descarga en silencio y se aplica
    /// recién cuando el usuario acepta.
    pub bytes: Vec<u8>,
    pub update: Update,
}

pub struct EstadoActualizador {
    pub pendiente: Mutex<Option<ActualizacionPendiente>>,
}

impl EstadoActualizador {
    pub fn nuevo() -> Self {
        EstadoActualizador {
            pendiente: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EstadoActualizacion {
    pub version_actual: String,
    pub version_disponible: Option<String>,
    pub notas: Option<String>,
    pub lista_para_instalar: bool,
}

/// Qué versión corre y si hay uná bajada esperando.
#[tauri::command]
pub fn estado_actualizacion(
    app: AppHandle,
    estado: State<'_, EstadoActualizador>,
) -> EstadoActualizacion {
    let pendiente = estado.pendiente.lock().ok();
    let datos = pendiente
        .as_ref()
        .and_then(|p| p.as_ref())
        .map(|p| (p.version.clone(), p.notas.clone()));

    EstadoActualizacion {
        version_actual: app.package_info().version.to_string(),
        version_disponible: datos.as_ref().map(|(v, _)| v.clone()),
        notas: datos.and_then(|(_, n)| n),
        lista_para_instalar: pendiente.map_or(false, |p| p.is_some()),
    }
}

/// Búsqueda manual desde configuración. A diferencia del chequeo automático,
/// acá los errores sí se informan: el usuario apretó un botón y espera una
/// respuesta.
#[tauri::command]
pub async fn buscar_actualizacion(app: AppHandle) -> Resultado<bool> {
    buscar_y_descargar(&app).await.map_err(|e| {
        AppError::conflicto(format!(
            "No se pudo comprobar si hay actualizaciones: {e}. \
             Revisa tu conexión a internet."
        ))
    })
}

/// Aplica la actualización descargada y reinicia.
#[tauri::command]
pub fn instalar_actualizacion(
    app: AppHandle,
    estado: State<'_, EstadoActualizador>,
) -> Resultado<()> {
    let pendiente = estado
        .pendiente
        .lock()
        .map_err(|_| AppError::conflicto("No se pudo leer el estado del actualizador."))?
        .take()
        .ok_or_else(|| AppError::conflicto("No hay ninguna actualización descargada."))?;

    pendiente
        .update
        .install(pendiente.bytes)
        .map_err(|e| AppError::conflicto(format!("No se pudo instalar la actualización: {e}")))?;

    // El instalador reemplaza el .exe que está corriendo, así que la app tiene
    // que morir sí o sí. Hay que levantar la bandera de salida real o el
    // interceptor de cierre bloquearía el reinicio y quedaría a medio instalar.
    crate::marcar_salida_real(&app);
    app.restart();
}

/// Busca, y si hay novedad la descarga en memoria. Devuelve si encontró algo.
///
/// Lo usan tanto el chequeo silencioso del arranque como el botón manual; la
/// diferencia está en quién decide mostrar el error.
pub async fn buscar_y_descargar(app: &AppHandle) -> Result<bool, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;

    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(false);
    };

    let version = update.version.clone();
    let notas = update.body.clone();

    // Descarga silenciosa: no se instala nada todavía.
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    {
        let estado = app.state::<EstadoActualizador>();
        let mut guard = estado
            .pendiente
            .lock()
            .map_err(|_| "estado del actualizador corrupto".to_string())?;

        *guard = Some(ActualizacionPendiente {
            version,
            notas,
            bytes,
            update,
        });
    }

    let _ = app.emit(EVENTO_ACTUALIZACION_LISTA, ());
    Ok(true)
}
