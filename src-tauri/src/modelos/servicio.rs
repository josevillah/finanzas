use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoServicio {
    /// Luz, agua, gas: el monto cambia todos los meses.
    Basico,
    /// Netflix, Spotify: el monto es estable.
    Suscripcion,
}

impl TipoServicio {
    pub fn como_texto(self) -> &'static str {
        match self {
            TipoServicio::Basico => "basico",
            TipoServicio::Suscripcion => "suscripcion",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "basico" => Ok(TipoServicio::Basico),
            "suscripcion" => Ok(TipoServicio::Suscripcion),
            otro => Err(AppError::validacion(format!(
                "Tipo de servicio desconocido: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Servicio {
    pub id: i64,
    pub nombre: String,
    pub categoria_id: Option<i64>,
    pub monto_estimado: Monto,
    /// Día del mes en que vence, 1-31. Opcional.
    pub dia_vencimiento: Option<i32>,
    pub tipo: TipoServicio,
    pub activo: bool,
    /// ISO 'YYYY-MM-DD'. Desde cuándo existe el servicio: no se generan gastos
    /// en meses anteriores a esta fecha.
    pub fecha_alta: Option<String>,
}

impl Servicio {
    pub const COLUMNAS: &'static str =
        "id, nombre, categoria_id, monto_estimado, dia_vencimiento, tipo, activo, fecha_alta";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let tipo_txt: String = fila.get(5)?;
        let tipo = TipoServicio::desde_texto(&tipo_txt).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Servicio {
            id: fila.get(0)?,
            nombre: fila.get(1)?,
            categoria_id: fila.get(2)?,
            monto_estimado: fila.get(3)?,
            dia_vencimiento: fila.get(4)?,
            tipo,
            activo: fila.get::<_, i64>(6)? != 0,
            fecha_alta: fila.get(7)?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NuevoServicio {
    pub nombre: String,
    pub categoria_id: Option<i64>,
    pub monto_estimado: Monto,
    pub dia_vencimiento: Option<i32>,
    pub tipo: TipoServicio,
    pub activo: bool,
    /// Solo se respeta al crear. Si viene vacía se usa hoy.
    pub fecha_alta: Option<String>,
}

/// Servicio con lo cargado en el período: estimado vs. real.
/// El gasto se materializa con el monto estimado y queda marcado como tal
/// hasta que el usuario le cambie el precio.
#[derive(Debug, Clone, Serialize)]
pub struct ServicioConReal {
    #[serde(flatten)]
    pub servicio: Servicio,
    pub categoria_nombre: Option<String>,
    /// Suma de todos los gastos del servicio en el período.
    pub monto_real: Monto,
    pub n_movimientos: i32,
    /// Cuántos de esos gastos siguen siendo el estimado sin confirmar.
    pub n_estimados: i32,
    /// real - estimado. Positivo = te pasaste.
    pub diferencia: Monto,
    /// ISO del vencimiento de este mes, si el servicio tiene día definido.
    pub fecha_vencimiento: Option<String>,
    /// El servicio ya existía en este mes, así que le toca generar gasto.
    pub corresponde_al_mes: bool,
}

/// Comparación estimado vs. real de todos los servicios activos del período.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenServicios {
    pub anio: i32,
    pub mes: u32,
    pub total_estimado: Monto,
    pub total_real: Monto,
    pub diferencia: Monto,
    /// Servicios activos que todavía no tienen ningún gasto en el período.
    pub sin_registrar: i32,
    /// Servicios cuyo gasto sigue siendo el estimado sin confirmar.
    pub por_confirmar: i32,
    pub servicios: Vec<ServicioConReal>,
}
