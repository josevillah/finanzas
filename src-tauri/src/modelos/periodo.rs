use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;

/// Un mes calendario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Periodo {
    pub id: i64,
    pub anio: i32,
    pub mes: u32,
    pub sueldo_liquido: Monto,
    pub otros_ingresos: Monto,
    /// abierto | cerrado
    pub estado: String,
}

impl Periodo {
    pub const COLUMNAS: &'static str = "id, anio, mes, sueldo_liquido, otros_ingresos, estado";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Periodo {
            id: fila.get(0)?,
            anio: fila.get(1)?,
            mes: fila.get(2)?,
            sueldo_liquido: fila.get(3)?,
            otros_ingresos: fila.get(4)?,
            estado: fila.get(5)?,
        })
    }
}

/// Cuánto se gastó en una categoría dentro del período.
#[derive(Debug, Clone, Serialize)]
pub struct GastoPorCategoria {
    pub categoria_id: Option<i64>,
    pub categoria_nombre: String,
    pub categoria_tipo: Option<String>,
    pub color: Option<String>,
    pub total: Monto,
    pub n_movimientos: i32,
}

/// Foto completa del mes: ingresos, gastos y desglose.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenPeriodo {
    #[serde(flatten)]
    pub periodo: Periodo,
    /// sueldo_liquido + otros_ingresos + movimientos de tipo ingreso.
    pub total_ingresos: Monto,
    /// Ingresos registrados como movimiento, aparte de los del período.
    pub ingresos_extra: Monto,
    pub total_gastos: Monto,
    /// total_ingresos - total_gastos.
    ///
    /// Apartar plata **no** entra acá: no sale del patrimonio, solo cambia de
    /// bolsillo. Para eso están los dos campos de abajo.
    pub balance: Monto,
    /// Apartado menos retirado en las cuentas de ahorro durante el mes.
    /// Negativo si se sacó más de lo que se guardó; 0 si no hubo movimiento.
    pub apartado_neto: Monto,
    /// `balance - apartado_neto`: lo que quedó del mes sin contar lo que ya se
    /// mandó a un ahorro. Es contexto para leer el balance, no un balance
    /// distinto.
    pub libre: Monto,
    /// Gastos que son pago de cuotas.
    pub total_cuotas: Monto,
    /// Gastos de categorías de tipo hormiga.
    pub total_hormiga: Monto,
    pub n_movimientos: i32,
    pub por_categoria: Vec<GastoPorCategoria>,
}

/// Un mes que tiene algo que mostrar. Alimenta el selector de mes y año.
#[derive(Debug, Clone, Serialize)]
pub struct MesConDatos {
    pub anio: i32,
    pub mes: u32,
    /// 'YYYY-MM'.
    pub clave: String,
    pub n_movimientos: i32,
    pub n_presupuestos: i32,
    /// Cuotas que vencen ese mes, de deudas vigentes.
    pub n_cuotas: i32,
    pub tiene_ingresos: bool,
}

/// Hasta dónde puede navegar el usuario y qué meses tienen contenido.
#[derive(Debug, Clone, Serialize)]
pub struct RangoMeses {
    pub desde_anio: i32,
    pub desde_mes: u32,
    /// Siempre el mes calendario actual: no tiene sentido navegar al futuro.
    pub hasta_anio: i32,
    pub hasta_mes: u32,
    /// Solo los meses con datos, en orden cronológico.
    pub meses: Vec<MesConDatos>,
}
