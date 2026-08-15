// En release no se abre la consola de Windows detrás de la ventana.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    finanzas_lib::run()
}
