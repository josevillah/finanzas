use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoMovimientoAhorro {
    /// Del disponible a la cuenta de ahorro.
    Apartar,
    /// De la cuenta de ahorro al disponible.
    Retirar,
}

impl TipoMovimientoAhorro {
    pub fn como_texto(self) -> &'static str {
        match self {
            TipoMovimientoAhorro::Apartar => "apartar",
            TipoMovimientoAhorro::Retirar => "retirar",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "apartar" => Ok(TipoMovimientoAhorro::Apartar),
            "retirar" => Ok(TipoMovimientoAhorro::Retirar),
            otro => Err(AppError::validacion(format!(
                "Tipo de movimiento de ahorro desconocido: '{otro}'"
            ))),
        }
    }
}

/// Una vez que la plata cruzó entre el disponible y un ahorro.
///
/// Es un registro histórico: el saldo de la cuenta no se calcula desde acá.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovimientoAhorro {
    pub id: i64,
    pub cuenta_id: i64,
    /// ISO 'YYYY-MM-DD'.
    pub fecha: String,
    /// Siempre positivo; la dirección la dice `tipo`.
    pub monto: Monto,
    pub tipo: TipoMovimientoAhorro,
    pub nota: Option<String>,
}

impl MovimientoAhorro {
    pub const COLUMNAS: &'static str = "id, cuenta_id, fecha, monto, tipo, nota";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let tipo_txt: String = fila.get(4)?;
        let tipo = TipoMovimientoAhorro::desde_texto(&tipo_txt).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(MovimientoAhorro {
            id: fila.get(0)?,
            cuenta_id: fila.get(1)?,
            fecha: fila.get(2)?,
            monto: fila.get(3)?,
            tipo,
            nota: fila.get(5)?,
        })
    }
}
