use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoMovimiento {
    Ingreso,
    Gasto,
}

impl TipoMovimiento {
    pub fn como_texto(self) -> &'static str {
        match self {
            TipoMovimiento::Ingreso => "ingreso",
            TipoMovimiento::Gasto => "gasto",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "ingreso" => Ok(TipoMovimiento::Ingreso),
            "gasto" => Ok(TipoMovimiento::Gasto),
            otro => Err(AppError::validacion(format!(
                "Tipo de movimiento desconocido: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MedioPago {
    Efectivo,
    Debito,
    Credito,
    Transferencia,
}

impl MedioPago {
    pub fn como_texto(self) -> &'static str {
        match self {
            MedioPago::Efectivo => "efectivo",
            MedioPago::Debito => "debito",
            MedioPago::Credito => "credito",
            MedioPago::Transferencia => "transferencia",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "efectivo" => Ok(MedioPago::Efectivo),
            "debito" => Ok(MedioPago::Debito),
            "credito" => Ok(MedioPago::Credito),
            "transferencia" => Ok(MedioPago::Transferencia),
            otro => Err(AppError::validacion(format!(
                "Medio de pago desconocido: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movimiento {
    pub id: i64,
    pub periodo_id: i64,
    /// ISO 'YYYY-MM-DD'.
    pub fecha: String,
    pub monto: Monto,
    pub tipo: TipoMovimiento,
    pub categoria_id: Option<i64>,
    pub servicio_id: Option<i64>,
    /// Si viene, este movimiento es el pago de una cuota y no se edita a mano.
    pub cuota_id: Option<i64>,
    pub medio_pago: Option<MedioPago>,
    pub descripcion: Option<String>,
    /// Lo generó el sistema con el monto estimado del servicio y el usuario
    /// todavía no lo confirma. Se apaga al cambiarle el precio o editarlo.
    pub es_estimado: bool,
}

impl Movimiento {
    pub const COLUMNAS: &'static str = "id, periodo_id, fecha, monto, tipo, categoria_id, \
                                        servicio_id, cuota_id, medio_pago, descripcion, \
                                        es_estimado";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let conv = |idx: usize, e: AppError| {
            rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
        };

        let tipo_txt: String = fila.get(4)?;
        let medio_txt: Option<String> = fila.get(8)?;

        let medio_pago = match medio_txt {
            Some(m) => Some(MedioPago::desde_texto(&m).map_err(|e| conv(8, e))?),
            None => None,
        };

        Ok(Movimiento {
            id: fila.get(0)?,
            periodo_id: fila.get(1)?,
            fecha: fila.get(2)?,
            monto: fila.get(3)?,
            tipo: TipoMovimiento::desde_texto(&tipo_txt).map_err(|e| conv(4, e))?,
            categoria_id: fila.get(5)?,
            servicio_id: fila.get(6)?,
            cuota_id: fila.get(7)?,
            medio_pago,
            descripcion: fila.get(9)?,
            es_estimado: fila.get::<_, i64>(10)? != 0,
        })
    }
}

/// Payload de creación/edición. El período se deduce de la fecha.
#[derive(Debug, Clone, Deserialize)]
pub struct NuevoMovimiento {
    /// ISO 'YYYY-MM-DD'.
    pub fecha: String,
    pub monto: Monto,
    pub tipo: TipoMovimiento,
    pub categoria_id: Option<i64>,
    pub servicio_id: Option<i64>,
    pub medio_pago: Option<MedioPago>,
    pub descripcion: Option<String>,
}

/// Movimiento con los nombres ya resueltos, para listar sin joins en el front.
#[derive(Debug, Clone, Serialize)]
pub struct MovimientoDetalle {
    #[serde(flatten)]
    pub movimiento: Movimiento,
    pub categoria_nombre: Option<String>,
    pub categoria_color: Option<String>,
    pub categoria_tipo: Option<String>,
    pub servicio_nombre: Option<String>,
    pub deuda_descripcion: Option<String>,
}

impl MovimientoDetalle {
    /// Los pagos de cuota se generan solos: no se editan ni borran desde la
    /// pantalla de gastos, se deshacen desde la deuda.
    pub fn es_pago_de_cuota(&self) -> bool {
        self.movimiento.cuota_id.is_some()
    }
}

/// Filtros del listado de movimientos de un período.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FiltroMovimientos {
    pub tipo: Option<TipoMovimiento>,
    pub categoria_id: Option<i64>,
    /// Texto libre contra la descripción.
    pub busqueda: Option<String>,
}
