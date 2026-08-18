pub mod comandos;
pub mod db;
pub mod dominio;
pub mod error;
pub mod modelos;
pub mod repos;
pub mod ventana;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use tauri::{Emitter, Manager};

use modelos::configuracion::AccionCierre;

/// Evento que recibe el frontend cuando se presiona el atajo global.
pub const EVENTO_CAPTURA_RAPIDA: &str = "abrir-captura-rapida";
/// Evento que pide al frontend mostrar el diálogo de cierre.
pub const EVENTO_SOLICITAR_CIERRE: &str = "solicitar-cierre";

/// Combinación del atajo de captura rápida, para mostrarla en la interfaz.
pub const ATAJO_CAPTURA: &str = "Ctrl+Shift+G";

/// Argumento con que el autostart lanza la app: arranca directo a la bandeja.
pub const ARG_MINIMIZADO: &str = "--minimizado";

/// Estado compartido de la app. Un solo usuario, un solo escritor: basta un
/// Mutex sobre la conexión, no hace falta un pool.
pub struct EstadoApp {
    pub db: Mutex<Connection>,
    /// El usuario pidió salir de verdad. El interceptor de cierre la consulta
    /// antes que nada: sin esto no habría forma de cerrar la aplicación.
    pub salida_real: AtomicBool,
    /// El atajo global quedó registrado. Si otra aplicación ya tenía la
    /// combinación, esto queda en false y la configuración lo avisa.
    pub atajo_registrado: AtomicBool,
}

/// Marca que la aplicación va a cerrarse de verdad, para que el interceptor de
/// `CloseRequested` deje pasar el cierre. La usan la salida explícita y la
/// instalación de una actualización, que reemplaza el ejecutable en marcha.
pub fn marcar_salida_real(app: &tauri::AppHandle) {
    app.state::<EstadoApp>()
        .salida_real
        .store(true, Ordering::SeqCst);
}

impl EstadoApp {
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // Si un comando entró en pánico con el lock tomado, preferimos
        // recuperar la conexión antes que dejar la app inservible.
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance debe ir primero: si ya hay una copia corriendo, esta
        // le pasa el foco y se cierra sin llegar a tocar la base.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            ventana::mostrar_y_enfocar(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![ARG_MINIMIZADO]),
        ))
        .setup(|app| {
            // `iniciar` verifica el esquema y respalda antes de migrar. Si algo
            // impide abrir la base hay que decírselo al usuario con una ventana:
            // en release no existe consola donde leer un error.
            let conn = match db::iniciar(app.handle()) {
                Ok(conn) => conn,
                Err(e) => {
                    avisar_y_salir(app.handle(), &e.to_string());
                }
            };

            // Al abrir la app ponemos al día el estado 'atrasada' de las cuotas.
            let hoy = dominio::fechas::a_iso(dominio::fechas::hoy());
            repos::cuotas::marcar_atrasadas(&conn, &hoy)?;

            app.manage(EstadoApp {
                db: Mutex::new(conn),
                salida_real: AtomicBool::new(false),
                atajo_registrado: AtomicBool::new(false),
            });
            app.manage(comandos::actualizacion::EstadoActualizador::nuevo());

            // Copia local del día si todavía no existe. Cubre a quien deja la
            // app en la bandeja durante semanas y nunca dispara el cierre.
            {
                let estado = app.state::<EstadoApp>();
                let conn = estado.conn();
                if let Err(e) = comandos::respaldo::respaldo_automatico(app.handle(), &conn, false)
                {
                    eprintln!("[respaldo] no se pudo guardar la copia automática: {e}");
                }
            }

            #[cfg(desktop)]
            {
                // La bandeja se construye antes de decidir si mostrar la
                // ventana: si fallara, ocultarla dejaría la app inalcanzable.
                let hay_bandeja = construir_bandeja(app.handle()).is_ok();

                let arranque_silencioso =
                    hay_bandeja && std::env::args().any(|a| a == ARG_MINIMIZADO);

                if !arranque_silencioso {
                    ventana::mostrar_y_enfocar(app.handle());
                }

                registrar_atajo_global(app.handle())?;

                // Chequeo de actualizaciones en segundo plano. Sin internet o
                // con GitHub caído no pasa nada: la app funciona igual y no se
                // molesta al usuario con un error que no puede resolver.
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    match comandos::actualizacion::buscar_y_descargar(&handle).await {
                        Ok(true) => eprintln!("[updater] actualización descargada"),
                        Ok(false) => {}
                        Err(e) => eprintln!("[updater] sin novedades: {e}"),
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|ventana_evento, evento| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = evento {
                manejar_cierre(ventana_evento.app_handle(), api);
            }
        })
        .invoke_handler(tauri::generate_handler![
            // deudas
            comandos::deudas::simular_cuotas,
            comandos::deudas::crear_deuda,
            comandos::deudas::actualizar_deuda,
            comandos::deudas::eliminar_deuda,
            comandos::deudas::cambiar_estado_deuda,
            comandos::deudas::listar_deudas,
            comandos::deudas::obtener_deuda,
            comandos::deudas::resumen_terceros,
            // cuotas
            comandos::cuotas::pagar_cuota,
            comandos::cuotas::deshacer_pago_cuota,
            comandos::cuotas::listar_cuotas_deuda,
            comandos::cuotas::listar_cuotas_mes,
            // análisis
            comandos::analisis::calendario_carga,
            comandos::analisis::carga_financiera,
            comandos::analisis::fecha_libertad,
            // períodos
            comandos::periodos::obtener_periodo,
            comandos::periodos::meses_disponibles,
            comandos::periodos::guardar_ingresos_periodo,
            comandos::periodos::cambiar_estado_periodo,
            comandos::periodos::resumen_periodo,
            // movimientos
            comandos::movimientos::registrar_movimiento,
            comandos::movimientos::actualizar_movimiento,
            comandos::movimientos::cambiar_monto_movimiento,
            comandos::movimientos::eliminar_movimiento,
            comandos::movimientos::listar_movimientos,
            comandos::movimientos::captura_rapida,
            // categorías
            comandos::categorias::listar_categorias,
            comandos::categorias::crear_categoria,
            comandos::categorias::actualizar_categoria,
            comandos::categorias::eliminar_categoria,
            // servicios
            comandos::servicios::listar_servicios,
            comandos::servicios::crear_servicio,
            comandos::servicios::actualizar_servicio,
            comandos::servicios::eliminar_servicio,
            comandos::servicios::generar_gastos_servicios,
            comandos::servicios::activar_servicio_en_mes,
            comandos::servicios::resumen_servicios,
            // cuentas
            comandos::cuentas::resumen_cuentas,
            comandos::cuentas::fijar_saldo_inicial,
            comandos::cuentas::apartar,
            comandos::cuentas::retirar,
            comandos::cuentas::crear_cuenta,
            comandos::cuentas::actualizar_cuenta,
            comandos::cuentas::eliminar_cuenta,
            // presupuesto
            comandos::presupuestos::resumen_presupuesto,
            comandos::presupuestos::asignar_presupuesto,
            comandos::presupuestos::copiar_presupuesto,
            // reportes
            comandos::reportes::evolucion_gastos,
            comandos::reportes::reporte_hormiga,
            // respaldo y exportación
            comandos::respaldo::estado_respaldo,
            comandos::respaldo::respaldar_base,
            comandos::respaldo::restaurar_base,
            comandos::respaldo::exportar_json,
            comandos::respaldo::exportar_csv,
            comandos::respaldo::fijar_respaldo_automatico,
            // configuración
            comandos::configuracion::obtener_ajustes,
            comandos::configuracion::fijar_accion_cierre,
            comandos::configuracion::fijar_autostart,
            comandos::configuracion::resolver_cierre,
            // actualización
            comandos::actualizacion::estado_actualizacion,
            comandos::actualizacion::buscar_actualizacion,
            comandos::actualizacion::instalar_actualizacion,
            // reinicio de datos
            comandos::reinicio::resumen_reinicio,
            comandos::reinicio::reiniciar_datos,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}

/// Muestra un error bloqueante y termina el proceso. Se usa cuando la base no
/// se puede abrir: seguir adelante a medias es peor que no arrancar.
fn avisar_y_salir(app: &tauri::AppHandle, mensaje: &str) -> ! {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

    app.dialog()
        .message(mensaje)
        .title("No se puede abrir Finanzas")
        .kind(MessageDialogKind::Error)
        .blocking_show();

    // Salida inmediata: en este punto no hay estado que valga la pena conservar
    // y `app.exit` dejaría continuar el setup con la base a medio abrir.
    std::process::exit(1);
}

/// Intercepta el cierre de la ventana según la preferencia guardada.
fn manejar_cierre(app: &tauri::AppHandle, api: &tauri::CloseRequestApi) {
    let estado = app.state::<EstadoApp>();

    // Salida pedida explícitamente: se deja cerrar sin más trámite.
    if estado.salida_real.load(Ordering::SeqCst) {
        return;
    }

    api.prevent_close();

    let guardada = {
        let conn = estado.conn();
        repos::configuracion::obtener(&conn, repos::configuracion::ACCION_CIERRE)
            .ok()
            .flatten()
    };

    match AccionCierre::desde_texto(guardada.as_deref()) {
        AccionCierre::Salir => comandos::configuracion::salir(app),
        AccionCierre::Bandeja => ventana::ocultar(app),
        AccionCierre::Preguntar => {
            // Si el frontend no respondiera —por ejemplo si aún no terminó de
            // cargar—, la ventana simplemente sigue abierta.
            let _ = app.emit(EVENTO_SOLICITAR_CIERRE, ());
        }
    }
}

/// Ícono en la bandeja del sistema. Sin esto, ocultar la ventana dejaría la
/// aplicación sin ninguna forma de volver a mostrarse.
#[cfg(desktop)]
fn construir_bandeja(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let abrir = MenuItem::with_id(app, "abrir", "Abrir Finanzas", true, None::<&str>)?;
    let gasto_rapido = MenuItem::with_id(
        app,
        "gasto_rapido",
        &format!("Gasto rápido  ({ATAJO_CAPTURA})"),
        true,
        None::<&str>,
    )?;
    let separador = PredefinedMenuItem::separator(app)?;
    let salir = MenuItem::with_id(app, "salir", "Salir", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&abrir, &gasto_rapido, &separador, &salir])?;

    let mut constructor = TrayIconBuilder::with_id("principal")
        .tooltip("Finanzas")
        .menu(&menu)
        // En Windows el click izquierdo debe abrir la ventana, no el menú:
        // el menú queda en el click derecho, que es lo que la gente espera.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id.as_ref() {
            "abrir" => ventana::mostrar_y_enfocar(app),
            "gasto_rapido" => {
                ventana::mostrar_y_enfocar(app);
                let _ = app.emit(EVENTO_CAPTURA_RAPIDA, ());
            }
            "salir" => comandos::configuracion::salir(app),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = evento
            {
                ventana::mostrar_y_enfocar(bandeja.app_handle());
            }
        });

    if let Some(icono) = app.default_window_icon() {
        constructor = constructor.icon(icono.clone());
    }

    constructor.build(app)?;
    Ok(())
}

/// Ctrl+Shift+G abre la captura rápida de gastos hormiga desde cualquier parte
/// del sistema, incluso con la app oculta en la bandeja.
#[cfg(desktop)]
fn registrar_atajo_global(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    let atajo = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyG);

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, presionado, evento| {
                // El handler se dispara al presionar y al soltar; solo nos
                // interesa lo primero.
                if presionado != &atajo || evento.state() != ShortcutState::Pressed {
                    return;
                }

                ventana::mostrar_y_enfocar(app);
                let _ = app.emit(EVENTO_CAPTURA_RAPIDA, ());
            })
            .build(),
    )?;

    // Si otra aplicación ya tomó la combinación, la app igual debe abrir. El
    // resultado queda en el estado para que la configuración pueda avisarlo:
    // en release nadie ve un eprintln.
    let registrado = app.global_shortcut().register(atajo).is_ok();
    app.state::<EstadoApp>()
        .atajo_registrado
        .store(registrado, Ordering::SeqCst);

    if !registrado {
        eprintln!("[atajo] no se pudo registrar {ATAJO_CAPTURA}: ya está en uso");
    }

    Ok(())
}
