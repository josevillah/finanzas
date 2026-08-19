use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::error::{AppError, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoMeta {
    /// En curso: compite por el saldo de su cuenta y suma a los totales.
    Activa,
    /// Alcanzada. Se conserva como historial y deja de reservar saldo.
    Cumplida,
    /// Ya no la persigo, pero no quiero borrar el registro.
    Archivada,
}

impl EstadoMeta {
    pub fn como_texto(self) -> &'static str {
        match self {
            EstadoMeta::Activa => "activa",
            EstadoMeta::Cumplida => "cumplida",
            EstadoMeta::Archivada => "archivada",
        }
    }

    pub fn desde_texto(texto: &str) -> Resultado<Self> {
        match texto {
            "activa" => Ok(EstadoMeta::Activa),
            "cumplida" => Ok(EstadoMeta::Cumplida),
            "archivada" => Ok(EstadoMeta::Archivada),
            otro => Err(AppError::validacion(format!(
                "Estado de meta desconocido: '{otro}'"
            ))),
        }
    }
}

/// Un objetivo de compra o ahorro.
///
/// No mueve plata: describe cuánto hace falta. El avance sale del saldo de la
/// cuenta vinculada, si tiene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub id: i64,
    pub nombre: String,
    pub monto_objetivo: Monto,
    /// Cuenta de ahorro que la financia. Sin ella la meta es solo referencia.
    pub cuenta_id: Option<i64>,
    /// Menor es más prioritaria.
    pub prioridad: i32,
    /// ISO 'YYYY-MM-DD'.
    pub fecha_objetivo: Option<String>,
    pub estado: EstadoMeta,
    pub notas: Option<String>,
    /// ISO 'YYYY-MM-DD'.
    pub creada_en: String,
}

impl Meta {
    pub const COLUMNAS: &'static str = "id, nombre, monto_objetivo, cuenta_id, prioridad, \
         fecha_objetivo, estado, notas, creada_en";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        let estado_txt: String = fila.get(6)?;
        let estado = EstadoMeta::desde_texto(&estado_txt).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Meta {
            id: fila.get(0)?,
            nombre: fila.get(1)?,
            monto_objetivo: fila.get(2)?,
            cuenta_id: fila.get(3)?,
            prioridad: fila.get(4)?,
            fecha_objetivo: fila.get(5)?,
            estado,
            notas: fila.get(7)?,
            creada_en: fila.get(8)?,
        })
    }
}

/// Payload de creación y de edición. La prioridad no viene acá: una meta nueva
/// se agrega al final y el orden se cambia solo reordenando.
#[derive(Debug, Clone, Deserialize)]
pub struct NuevaMeta {
    pub nombre: String,
    pub monto_objetivo: Monto,
    pub cuenta_id: Option<i64>,
    pub fecha_objetivo: Option<String>,
    pub notas: Option<String>,
}

// ── DTOs de lectura ──────────────────────────────────────────────────────────

/// Una meta con todo lo calculado, lista para mostrar.
///
/// Los cálculos van acá y no en el frontend por la regla 3: la pantalla no
/// divide ni proyecta nada, solo formatea.
#[derive(Debug, Clone, Serialize)]
pub struct MetaDetalle {
    #[serde(flatten)]
    pub meta: Meta,
    /// Nombre de la cuenta vinculada, si sigue existiendo.
    pub cuenta_nombre: Option<String>,
    /// Parte del saldo de la cuenta que le toca, según el reparto por
    /// prioridad. Sin cuenta vinculada es 0.
    pub acumulado: Monto,
    /// objetivo - acumulado, nunca negativo.
    pub falta: Monto,
    /// 0-100. Solo tiene sentido si `tiene_progreso`.
    pub progreso_pct: f64,
    /// Hay una cuenta vinculada de la cual leer el avance.
    pub tiene_progreso: bool,
    /// Cuánto habría que apartar por mes para llegar a `fecha_objetivo`.
    /// `None` si no hay fecha, si ya pasó, o si la meta ya está cubierta.
    pub ritmo_mensual: Option<Monto>,
    /// Meses que faltan hasta la fecha objetivo, si está en el futuro.
    pub meses_restantes: Option<i32>,
    /// La fecha objetivo quedó atrás sin haber llegado.
    pub fecha_pasada: bool,
    /// Cuántos meses tomaría cubrir lo que falta al ritmo del balance
    /// promedio. `None` si el promedio no es positivo o no hay historial.
    ///
    /// Supone que **todo** el balance va a esta meta: con varias metas los
    /// números no se suman, y la pantalla lo dice.
    pub meses_al_ritmo: Option<i32>,
}

/// Los totales del conjunto, para responder si es alcanzable.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenMetas {
    pub metas: Vec<MetaDetalle>,
    /// Suma de los objetivos de las metas **activas**.
    pub total_objetivo: Monto,
    /// Suma de lo acumulado por las metas activas.
    pub total_acumulado: Monto,
    pub total_falta: Monto,
    /// Todo lo apartado en cuentas de ahorro, esté o no comprometido con una
    /// meta. Es el contraste que pediste: objetivos contra plata real.
    pub total_ahorrado: Monto,
    /// Ahorros que ninguna meta activa está reclamando.
    pub ahorro_sin_meta: Monto,
    /// Promedio del balance de los últimos meses cerrados con actividad.
    /// `None` si todavía no hay historial del cual sacarlo.
    pub balance_promedio: Option<Monto>,
    /// Cuántos meses de balance promedio tomaría cubrir `total_falta`.
    pub meses_al_ritmo: Option<i32>,
    /// Cuántos meses cerrados con actividad entraron en el promedio.
    pub meses_considerados: i32,
    pub n_activas: i32,
    pub n_cumplidas: i32,
    pub n_archivadas: i32,
}
