use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager, State};

use tauri_plugin_autostart::ManagerExt;

use crate::error::{AppError, Resultado};
use crate::modelos::configuracion::{AccionCierre, AjustesApp};
use crate::repos;
use crate::ventana;
use crate::{EstadoApp, ATAJO_CAPTURA};

#[tauri::command]
pub fn obtener_ajustes(app: AppHandle, estado: State<'_, EstadoApp>) -> Resultado<AjustesApp> {
    let guard = estado.conn();
    let guardada = repos::configuracion::obtener(&guard, repos::configuracion::ACCION_CIERRE)?;

    Ok(AjustesApp {
        accion_cierre: AccionCierre::desde_texto(guardada.as_deref()),
        // Se consulta al sistema, no a una preferencia nuestra: si el usuario
        // lo desactivó desde el Administrador de tareas, hay que reflejarlo.
        autostart_activo: app.autolaunch().is_enabled().unwrap_or(false),
        atajo_registrado: estado.atajo_registrado.load(Ordering::SeqCst),
        atajo: ATAJO_CAPTURA,
    })
}

#[tauri::command]
pub fn fijar_accion_cierre(estado: State<'_, EstadoApp>, accion: AccionCierre) -> Resultado<()> {
    let guard = estado.conn();
    repos::configuracion::guardar(
        &guard,
        repos::configuracion::ACCION_CIERRE,
        accion.como_texto(),
    )
}

#[tauri::command]
pub fn fijar_autostart(app: AppHandle, activo: bool) -> Resultado<bool> {
    let gestor = app.autolaunch();

    let resultado = if activo {
        gestor.enable()
    } else {
        gestor.disable()
    };

    resultado.map_err(|e| {
        AppError::conflicto(format!(
            "No se pudo cambiar el inicio automático: {e}. \
             Puede que Windows lo esté bloqueando por permisos."
        ))
    })?;

    Ok(gestor.is_enabled().unwrap_or(false))
}

/// Recibe la decisión del diálogo de cierre y la ejecuta.
/// Si `recordar` viene en true, la preferencia queda guardada y el diálogo no
/// vuelve a aparecer hasta que se cambie desde configuración.
#[tauri::command]
pub fn resolver_cierre(
    app: AppHandle,
    estado: State<'_, EstadoApp>,
    accion: AccionCierre,
    recordar: bool,
) -> Resultado<()> {
    // `Preguntar` no es una decisión: significaría volver a preguntar en el
    // acto. El diálogo solo envía Bandeja o Salir.
    if accion == AccionCierre::Preguntar {
        return Err(AppError::validacion(
            "El diálogo de cierre debe resolverse con 'bandeja' o 'salir'.",
        ));
    }

    if recordar {
        let guard = estado.conn();
        repos::configuracion::guardar(
            &guard,
            repos::configuracion::ACCION_CIERRE,
            accion.como_texto(),
        )?;
    }

    ejecutar_cierre(&app, accion);
    Ok(())
}

/// Aplica la acción de cierre. Levanta la bandera de salida real antes de
/// salir para que el interceptor de `CloseRequested` deje pasar el cierre.
pub fn ejecutar_cierre(app: &AppHandle, accion: AccionCierre) {
    match accion {
        AccionCierre::Bandeja | AccionCierre::Preguntar => ventana::ocultar(app),
        AccionCierre::Salir => salir(app),
    }
}

/// Salida real de la aplicación. Es la única vía: sin levantar la bandera, el
/// interceptor volvería a bloquear el cierre y la app quedaría inmatable salvo
/// desde el Administrador de tareas.
pub fn salir(app: &AppHandle) {
    // Última copia local antes de irse, con el trabajo del día incluido. Si
    // fallara, no es motivo para dejar al usuario sin poder cerrar.
    {
        let estado = app.state::<EstadoApp>();
        let conn = estado.conn();
        if let Err(e) = crate::comandos::respaldo::respaldo_automatico(app, &conn, true) {
            eprintln!("[respaldo] no se pudo guardar la copia automática al salir: {e}");
        }
    }

    crate::marcar_salida_real(app);
    app.exit(0);
}
