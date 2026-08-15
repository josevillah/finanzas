use serde::Serialize;

use crate::dominio::dinero::Monto;
use crate::modelos::periodo::GastoPorCategoria;

/// Un mes dentro de una serie de tiempo.
#[derive(Debug, Clone, Serialize)]
pub struct PuntoMes {
    pub anio: i32,
    pub mes: u32,
    /// 'YYYY-MM', clave estable para el frontend.
    pub clave: String,
    pub total: Monto,
}

/// Cómo evolucionó el gasto de una categoría a lo largo de los meses pedidos.
#[derive(Debug, Clone, Serialize)]
pub struct SerieCategoria {
    pub categoria_id: Option<i64>,
    pub categoria_nombre: String,
    pub color: Option<String>,
    /// Suma de todos los meses de la ventana.
    pub total: Monto,
    pub promedio: Monto,
    /// Un punto por mes de la ventana, con ceros donde no hubo gasto.
    pub puntos: Vec<PuntoMes>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvolucionGastos {
    /// Claves 'YYYY-MM' en orden cronológico.
    pub meses: Vec<String>,
    /// Ordenadas por total descendente.
    pub series: Vec<SerieCategoria>,
    pub total_por_mes: Vec<PuntoMes>,
    pub total_ventana: Monto,
}

/// Un mes en el reporte de gastos hormiga.
#[derive(Debug, Clone, Serialize)]
pub struct MesHormiga {
    pub anio: i32,
    pub mes: u32,
    pub clave: String,
    pub total: Monto,
    pub n_movimientos: i32,
    /// Todo el gasto del mes, para poder sacar el peso relativo.
    pub total_gastos: Monto,
    /// Qué parte del gasto del mes se fue en hormigas. `None` si no hubo gastos.
    pub porcentaje: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReporteHormiga {
    /// En orden cronológico, con ceros en los meses sin registros.
    pub meses: Vec<MesHormiga>,
    /// Mes seleccionado, el último de la ventana.
    pub mes_actual: Option<MesHormiga>,
    /// Promedio de la ventana sin contar el mes actual.
    pub promedio_previos: Monto,
    /// Variación porcentual del mes actual contra el mes anterior.
    pub variacion_mes_anterior: Option<f64>,
    /// Variación porcentual del mes actual contra el promedio de los previos.
    pub variacion_promedio: Option<f64>,
    /// Desglose hormiga del mes actual.
    pub por_categoria: Vec<GastoPorCategoria>,
    /// Cuánto llevas gastado en hormigas en toda la ventana.
    pub total_ventana: Monto,
}
