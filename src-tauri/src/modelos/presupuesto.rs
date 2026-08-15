use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;

/// Monto asignado a una categoría en un período.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presupuesto {
    pub id: i64,
    pub periodo_id: i64,
    pub categoria_id: i64,
    pub monto_asignado: Monto,
}

impl Presupuesto {
    pub const COLUMNAS: &'static str = "id, periodo_id, categoria_id, monto_asignado";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Presupuesto {
            id: fila.get(0)?,
            periodo_id: fila.get(1)?,
            categoria_id: fila.get(2)?,
            monto_asignado: fila.get(3)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoPresupuesto {
    /// La categoría tiene gasto pero nadie le asignó presupuesto.
    SinAsignar,
    /// Va bien: menos del 80% consumido.
    Ok,
    /// Entre 80% y 100%.
    Alerta,
    /// Se pasó del monto asignado.
    Excedido,
}

/// Una categoría dentro del presupuesto del mes: cuánto se le asignó y cuánto
/// se lleva gastado.
#[derive(Debug, Clone, Serialize)]
pub struct LineaPresupuesto {
    pub categoria_id: i64,
    pub categoria_nombre: String,
    pub categoria_tipo: String,
    pub color: Option<String>,
    pub monto_asignado: Monto,
    pub monto_gastado: Monto,
    /// asignado - gastado. Negativo = te pasaste.
    pub disponible: Monto,
    /// Porcentaje consumido. `None` si no hay monto asignado.
    pub porcentaje_usado: Option<f64>,
    pub estado: EstadoPresupuesto,
    pub n_movimientos: i32,
}

/// Presupuesto del mes completo, con lo real ya cruzado.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenPresupuesto {
    pub anio: i32,
    pub mes: u32,
    pub total_asignado: Monto,
    /// Gasto de las categorías que sí tienen presupuesto.
    pub total_gastado: Monto,
    pub disponible: Monto,
    pub porcentaje_usado: Option<f64>,
    /// Gasto en categorías sin presupuesto asignado.
    pub gasto_sin_presupuestar: Monto,
    /// Todo el gasto del mes, con y sin presupuesto.
    pub total_gastos_mes: Monto,
    pub total_ingresos: Monto,
    /// Ingresos menos lo asignado. Negativo = presupuestaste más de lo que entra.
    pub sin_asignar_del_ingreso: Monto,
    pub categorias_excedidas: i32,
    /// El mes está cerrado: no acepta cambios de asignación.
    pub periodo_cerrado: bool,
    pub lineas: Vec<LineaPresupuesto>,
}

/// Payload de asignación. Un monto de 0 borra la línea.
#[derive(Debug, Clone, Deserialize)]
pub struct AsignacionPresupuesto {
    pub categoria_id: i64,
    pub monto_asignado: Monto,
}
