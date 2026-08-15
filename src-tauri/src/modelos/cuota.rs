use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoCuota {
    Pendiente,
    Pagada,
    Atrasada,
}

impl EstadoCuota {
    pub fn como_texto(self) -> &'static str {
        match self {
            EstadoCuota::Pendiente => "pendiente",
            EstadoCuota::Pagada => "pagada",
            EstadoCuota::Atrasada => "atrasada",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "pendiente" => Ok(EstadoCuota::Pendiente),
            "pagada" => Ok(EstadoCuota::Pagada),
            "atrasada" => Ok(EstadoCuota::Atrasada),
            otro => Err(AppError::validacion(format!(
                "Estado de cuota desconocido: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cuota {
    pub id: i64,
    pub deuda_id: i64,
    pub numero: i32,
    /// ISO 'YYYY-MM-DD'.
    pub fecha_vencimiento: String,
    pub monto: Monto,
    pub capital: Monto,
    pub interes: Monto,
    pub estado: EstadoCuota,
    pub fecha_pago: Option<String>,
    pub monto_pagado: Option<Monto>,
}

impl Cuota {
    /// Lista de columnas en el orden que espera [`Cuota::desde_fila`].
    pub const COLUMNAS: &'static str = "id, deuda_id, numero, fecha_vencimiento, monto, capital, \
                                        interes, estado, fecha_pago, monto_pagado";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let estado_txt: String = fila.get(7)?;
        let estado = EstadoCuota::desde_texto(&estado_txt).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Cuota {
            id: fila.get(0)?,
            deuda_id: fila.get(1)?,
            numero: fila.get(2)?,
            fecha_vencimiento: fila.get(3)?,
            monto: fila.get(4)?,
            capital: fila.get(5)?,
            interes: fila.get(6)?,
            estado,
            fecha_pago: fila.get(8)?,
            monto_pagado: fila.get(9)?,
        })
    }
}

/// Cuota junto al nombre de su deuda, para listados transversales
/// (por ejemplo "cuotas que vencen este mes").
#[derive(Debug, Clone, Serialize)]
pub struct CuotaConDeuda {
    #[serde(flatten)]
    pub cuota: Cuota,
    pub deuda_descripcion: String,
}

/// Datos que envía el frontend al marcar una cuota como pagada.
/// `monto_pagado` puede diferir del monto programado (pago parcial o con recargo).
#[derive(Debug, Clone, Deserialize)]
pub struct PagoCuota {
    pub cuota_id: i64,
    /// ISO 'YYYY-MM-DD'. Si viene vacío se usa la fecha de hoy.
    pub fecha_pago: Option<String>,
    pub monto_pagado: Monto,
}
