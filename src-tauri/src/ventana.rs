//! Manejo de la ventana principal. El orden de las llamadas importa y se
//! repite en cuatro lugares (atajo global, click en la bandeja, menú Abrir y
//! menú Gasto rápido), así que vive en un solo sitio.

use tauri::{AppHandle, Manager};

pub const ETIQUETA_VENTANA: &str = "main";

/// Trae la ventana al frente desde cualquier estado: oculta en la bandeja,
/// minimizada o detrás de otra aplicación.
///
/// El orden no es intercambiable. `show()` va primero porque en Windows
/// `unminimize()` sobre una ventana oculta no hace nada, y `set_focus()` va al
/// final porque sin él la ventana aparece detrás de la aplicación activa y el
/// teclado sigue yendo a la otra.
pub fn mostrar_y_enfocar(app: &AppHandle) {
    let Some(ventana) = app.get_webview_window(ETIQUETA_VENTANA) else {
        return;
    };

    let _ = ventana.show();
    let _ = ventana.unminimize();
    let _ = ventana.set_focus();
}

/// Oculta la ventana. La app sigue viva en la bandeja.
pub fn ocultar(app: &AppHandle) {
    if let Some(ventana) = app.get_webview_window(ETIQUETA_VENTANA) {
        let _ = ventana.hide();
    }
}
