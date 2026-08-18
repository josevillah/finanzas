use tauri::State;

use crate::error::{AppError, Resultado};
use crate::modelos::categoria::{Categoria, NuevaCategoria, TipoCategoria};
use crate::repos;
use crate::EstadoApp;

#[tauri::command]
pub fn listar_categorias(
    estado: State<'_, EstadoApp>,
    solo_activas: Option<bool>,
) -> Resultado<Vec<Categoria>> {
    let guard = estado.conn();
    repos::categorias::listar(&guard, solo_activas.unwrap_or(false))
}

#[tauri::command]
pub fn crear_categoria(estado: State<'_, EstadoApp>, datos: NuevaCategoria) -> Resultado<i64> {
    validar(&datos)?;
    let guard = estado.conn();
    repos::categorias::insertar(&guard, &datos)
}

#[tauri::command]
pub fn actualizar_categoria(
    estado: State<'_, EstadoApp>,
    id: i64,
    datos: NuevaCategoria,
) -> Resultado<()> {
    validar(&datos)?;
    let guard = estado.conn();

    let actual = repos::categorias::obtener(&guard, id)?;

    // Las categorías que el sistema ubica por código conservan su tipo.
    // Marcar "Deudas y créditos" como hormiga haría que cada pago de cuota
    // contara como gasto hormiga e inflaría el reporte, el total del mes y la
    // comparación contra el promedio.
    if !puede_cambiar_tipo(&actual, datos.tipo) {
        return Err(AppError::conflicto(
            "Esta categoría la usa el sistema y no puede cambiar de tipo. Puedes renombrarla, cambiarle el color o desactivarla.",
        ));
    }

    repos::categorias::actualizar(&guard, id, &datos)
}

/// Solo borra si nadie la usa. Si tiene movimientos o servicios asociados,
/// sugiere desactivarla para no romper el historial.
#[tauri::command]
pub fn eliminar_categoria(estado: State<'_, EstadoApp>, id: i64) -> Resultado<()> {
    let guard = estado.conn();

    let categoria = repos::categorias::obtener(&guard, id)?;
    if categoria.codigo.is_some() {
        return Err(AppError::conflicto(
            "Esta categoría la usa el sistema para imputar los pagos de cuotas. \
             Puedes renombrarla, pero no eliminarla.",
        ));
    }

    let (movimientos, servicios) = repos::categorias::usos(&guard, id)?;
    if movimientos > 0 || servicios > 0 {
        return Err(AppError::conflicto(format!(
            "No se puede eliminar: la usan {movimientos} movimiento(s) y {servicios} servicio(s). \
             Desactívala para que deje de aparecer sin perder el historial."
        )));
    }

    repos::categorias::eliminar(&guard, id)
}

/// Las categorías que el sistema ubica por código conservan su tipo.
///
/// Cambiar "Deudas y créditos" a hormiga haría que cada pago de cuota contara
/// como gasto hormiga: se inflarían el reporte de hormigas, el total del mes y
/// la comparación contra el promedio. El resto de sus campos sí es editable.
pub fn puede_cambiar_tipo(actual: &Categoria, nuevo: TipoCategoria) -> bool {
    actual.codigo.is_none() || actual.tipo == nuevo
}

fn validar(datos: &NuevaCategoria) -> Resultado<()> {
    if datos.nombre.trim().is_empty() {
        return Err(AppError::validacion("El nombre no puede quedar vacío."));
    }
    Ok(())
}
