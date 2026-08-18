use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};
use crate::modelos::cuota::Cuota;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoDeuda {
    CompraCuotas,
    CreditoConsumo,
    Avance,
    Rotativo,
}

impl TipoDeuda {
    pub fn como_texto(self) -> &'static str {
        match self {
            TipoDeuda::CompraCuotas => "compra_cuotas",
            TipoDeuda::CreditoConsumo => "credito_consumo",
            TipoDeuda::Avance => "avance",
            TipoDeuda::Rotativo => "rotativo",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "compra_cuotas" => Ok(TipoDeuda::CompraCuotas),
            "credito_consumo" => Ok(TipoDeuda::CreditoConsumo),
            "avance" => Ok(TipoDeuda::Avance),
            "rotativo" => Ok(TipoDeuda::Rotativo),
            otro => Err(AppError::validacion(format!(
                "Tipo de deuda desconocido: '{otro}'"
            ))),
        }
    }
}

/// Hacia dónde va la deuda: la debo yo, o me la deben.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DireccionDeuda {
    /// La debo yo. Es lo que alimenta la carga financiera y la fecha de libertad.
    Propia,
    /// Me la deben. No cuenta como carga: es plata que va a entrar.
    Tercero,
}

impl DireccionDeuda {
    pub fn como_texto(self) -> &'static str {
        match self {
            DireccionDeuda::Propia => "propia",
            DireccionDeuda::Tercero => "tercero",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "propia" => Ok(DireccionDeuda::Propia),
            "tercero" => Ok(DireccionDeuda::Tercero),
            otro => Err(AppError::validacion(format!(
                "Dirección de deuda desconocida: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoDeuda {
    Vigente,
    Pagada,
    Repactada,
}

impl EstadoDeuda {
    pub fn como_texto(self) -> &'static str {
        match self {
            EstadoDeuda::Vigente => "vigente",
            EstadoDeuda::Pagada => "pagada",
            EstadoDeuda::Repactada => "repactada",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "vigente" => Ok(EstadoDeuda::Vigente),
            "pagada" => Ok(EstadoDeuda::Pagada),
            "repactada" => Ok(EstadoDeuda::Repactada),
            otro => Err(AppError::validacion(format!(
                "Estado de deuda desconocido: '{otro}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deuda {
    pub id: i64,
    pub descripcion: String,
    pub tipo: TipoDeuda,
    pub institucion: Option<String>,
    pub monto_original: Monto,
    pub tasa_mensual: f64,
    pub n_cuotas: i32,
    /// ISO 'YYYY-MM-DD'.
    pub fecha_primera_cuota: String,
    pub estado: EstadoDeuda,
    pub notas: Option<String>,
    pub direccion: DireccionDeuda,
    /// Quién me debe. Solo viene si la dirección es tercero.
    pub deudor: Option<String>,
}

impl Deuda {
    pub const COLUMNAS: &'static str = "id, descripcion, tipo, institucion, monto_original, tasa_mensual, n_cuotas, fecha_primera_cuota, estado, notas, direccion, deudor";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let tipo_txt: String = fila.get(2)?;
        let estado_txt: String = fila.get(8)?;

        let conv = |idx: usize, e: AppError| {
            rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
        };

        Ok(Deuda {
            id: fila.get(0)?,
            descripcion: fila.get(1)?,
            tipo: TipoDeuda::desde_texto(&tipo_txt).map_err(|e| conv(2, e))?,
            institucion: fila.get(3)?,
            monto_original: fila.get(4)?,
            tasa_mensual: fila.get(5)?,
            n_cuotas: fila.get(6)?,
            fecha_primera_cuota: fila.get(7)?,
            estado: EstadoDeuda::desde_texto(&estado_txt).map_err(|e| conv(8, e))?,
            notas: fila.get(9)?,
            direccion: {
                let txt: String = fila.get(10)?;
                DireccionDeuda::desde_texto(&txt).map_err(|e| conv(10, e))?
            },
            deudor: fila.get(11)?,
        })
    }
}

/// Payload de creación/edición enviado desde el formulario.
#[derive(Debug, Clone, Deserialize)]
pub struct NuevaDeuda {
    pub descripcion: String,
    pub tipo: TipoDeuda,
    pub institucion: Option<String>,
    pub monto_original: Monto,
    /// Fracción mensual: 0,025 = 2,5% mensual.
    pub tasa_mensual: f64,
    pub n_cuotas: i32,
    /// ISO 'YYYY-MM-DD'.
    pub fecha_primera_cuota: String,
    pub notas: Option<String>,
    #[serde(default = "direccion_propia")]
    pub direccion: DireccionDeuda,
    /// Obligatorio si la dirección es tercero; se ignora si es propia.
    pub deudor: Option<String>,
}

fn direccion_propia() -> DireccionDeuda {
    DireccionDeuda::Propia
}

// ── DTOs de lectura ──────────────────────────────────────────────────────────

/// Fila del listado de deudas, con el avance ya calculado en Rust.
#[derive(Debug, Clone, Serialize)]
pub struct DeudaResumen {
    #[serde(flatten)]
    pub deuda: Deuda,
    pub total_programado: Monto,
    pub monto_pagado: Monto,
    pub monto_pendiente: Monto,
    pub cuotas_pagadas: i32,
    pub cuotas_totales: i32,
    /// 0.0 a 100.0, calculado sobre el monto (no sobre el número de cuotas).
    pub avance_pct: f64,
    pub cuotas_atrasadas: i32,
    pub proxima_cuota: Option<Cuota>,
}

/// Deuda + tabla de amortización completa.
#[derive(Debug, Clone, Serialize)]
pub struct DeudaDetalle {
    pub resumen: DeudaResumen,
    pub cuotas: Vec<Cuota>,
}

/// Un mes del calendario de carga.
#[derive(Debug, Clone, Serialize)]
pub struct MesCarga {
    pub anio: i32,
    pub mes: u32,
    /// 'YYYY-MM', útil como clave en el frontend.
    pub clave: String,
    /// Total comprometido del mes (todas las cuotas, pagadas o no).
    pub total: Monto,
    /// Parte de `total` que aún no se paga.
    pub total_pendiente: Monto,
    pub n_cuotas: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Semaforo {
    Verde,
    Amarillo,
    Rojo,
    /// No hay sueldo líquido registrado para el período: no se puede calcular.
    SinDatos,
}

/// Cuotas del mes divididas por el sueldo líquido del período.
#[derive(Debug, Clone, Serialize)]
pub struct CargaFinanciera {
    pub anio: i32,
    pub mes: u32,
    pub total_cuotas: Monto,
    pub sueldo_liquido: Monto,
    pub otros_ingresos: Monto,
    /// Porcentaje sobre el sueldo líquido. `None` si el sueldo es 0.
    pub porcentaje: Option<f64>,
    pub semaforo: Semaforo,
    pub n_cuotas: i32,
}

/// Cuánto se libera al terminar de pagar una deuda.
#[derive(Debug, Clone, Serialize)]
pub struct Liberacion {
    pub deuda_id: i64,
    pub descripcion: String,
    /// ISO 'YYYY-MM-DD' del último vencimiento pendiente.
    pub fecha_ultima_cuota: String,
    /// Cuota mensual típica que deja de pagarse (mediana de las pendientes).
    pub monto_mensual_liberado: Monto,
    pub cuotas_restantes: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct FechaLibertad {
    /// ISO 'YYYY-MM-DD'. `None` si no hay cuotas pendientes.
    pub fecha_ultima_cuota: Option<String>,
    pub meses_restantes: Option<i32>,
    pub total_pendiente: Monto,
    /// Ordenadas por fecha de término ascendente.
    pub liberaciones: Vec<Liberacion>,
}

/// Lo que una persona me debe, sumando todas sus deudas.
#[derive(Debug, Clone, Serialize)]
pub struct DeudorResumen {
    pub deudor: String,
    pub n_deudas: i32,
    pub total_pendiente: Monto,
    pub total_cobrado: Monto,
    pub cuotas_pendientes: i32,
    pub cuotas_atrasadas: i32,
    /// ISO del próximo vencimiento por cobrar.
    pub proxima_fecha: Option<String>,
}

/// Vista "Me deben": nunca entra en la carga financiera ni en la fecha de
/// libertad, que son sobre lo que debo yo.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenTerceros {
    pub total_pendiente: Monto,
    pub total_cobrado: Monto,
    pub cuotas_atrasadas: i32,
    /// Ordenados por lo que falta cobrar, de mayor a menor.
    pub deudores: Vec<DeudorResumen>,
}
