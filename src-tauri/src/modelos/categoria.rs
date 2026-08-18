use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoCategoria {
    /// Sale todos los meses sí o sí: arriendo, luz, suscripciones.
    Fijo,
    /// Varía mes a mes pero es necesario: supermercado, transporte.
    Variable,
    /// Gasto chico y frecuente que se acumula sin que te des cuenta.
    Hormiga,
    /// Clasifica plata que entra, no que sale. Las vistas de gasto y el
    /// presupuesto la ignoran.
    Ingreso,
}

impl TipoCategoria {
    /// ¿Clasifica plata que sale? Lo usan el presupuesto y los reportes, que
    /// solo miran gastos.
    pub fn es_de_gasto(self) -> bool {
        !matches!(self, TipoCategoria::Ingreso)
    }

    pub fn como_texto(self) -> &'static str {
        match self {
            TipoCategoria::Fijo => "fijo",
            TipoCategoria::Variable => "variable",
            TipoCategoria::Hormiga => "hormiga",
            TipoCategoria::Ingreso => "ingreso",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "fijo" => Ok(TipoCategoria::Fijo),
            "variable" => Ok(TipoCategoria::Variable),
            "hormiga" => Ok(TipoCategoria::Hormiga),
            "ingreso" => Ok(TipoCategoria::Ingreso),
            otro => Err(AppError::validacion(format!(
                "Tipo de categoría desconocido: '{otro}'"
            ))),
        }
    }
}

/// Código estable de la categoría a la que se imputan los pagos de cuotas.
/// El usuario puede renombrarla; el código no cambia.
pub const CODIGO_DEUDAS: &str = "deudas";

/// Categoría donde entran los cobros de deudas de terceros.
pub const CODIGO_COBROS: &str = "cobros";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Categoria {
    pub id: i64,
    pub nombre: String,
    pub tipo: TipoCategoria,
    pub color: Option<String>,
    pub activa: bool,
    pub codigo: Option<String>,
    /// Viene de fábrica. El reinicio de datos conserva estas y borra el resto.
    pub es_semilla: bool,
}

impl Categoria {
    pub const COLUMNAS: &'static str = "id, nombre, tipo, color, activa, codigo, es_semilla";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let tipo_txt: String = fila.get(2)?;
        let tipo = TipoCategoria::desde_texto(&tipo_txt).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Categoria {
            id: fila.get(0)?,
            nombre: fila.get(1)?,
            tipo,
            color: fila.get(3)?,
            activa: fila.get::<_, i64>(4)? != 0,
            codigo: fila.get(5)?,
            es_semilla: fila.get::<_, i64>(6)? != 0,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NuevaCategoria {
    pub nombre: String,
    pub tipo: TipoCategoria,
    pub color: Option<String>,
    pub activa: bool,
}
