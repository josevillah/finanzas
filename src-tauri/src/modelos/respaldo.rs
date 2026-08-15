use serde::Serialize;

use crate::modelos::categoria::Categoria;
use crate::modelos::cuota::Cuota;
use crate::modelos::deuda::Deuda;
use crate::modelos::movimiento::Movimiento;
use crate::modelos::periodo::Periodo;
use crate::modelos::presupuesto::Presupuesto;
use crate::modelos::servicio::Servicio;

/// Cada cuántos días sin respaldar se muestra el recordatorio.
pub const DIAS_RECORDATORIO: i64 = 7;

#[derive(Debug, Clone, Serialize)]
pub struct EstadoRespaldo {
    /// ISO 'YYYY-MM-DD' del último respaldo, si alguna vez se hizo uno.
    pub ultimo_respaldo: Option<String>,
    pub dias_desde_ultimo: Option<i64>,
    /// Nunca respaldaste, o pasaron más de [`DIAS_RECORDATORIO`] días.
    pub requiere_recordatorio: bool,
    pub ruta_db: String,
    pub tamano_bytes: u64,
    pub version_esquema: i32,
    /// Cuántas filas hay en las tablas con datos del usuario.
    pub total_registros: i64,
    /// Copia local automática activada.
    pub respaldo_automatico: bool,
    /// Carpeta donde viven las copias automáticas.
    pub carpeta_respaldos: String,
    /// Fecha ISO de la copia automática más reciente.
    pub ultimo_automatico: Option<String>,
    pub copias_automaticas: i32,
}

/// Estructura del archivo .json de exportación.
#[derive(Debug, Clone, Serialize)]
pub struct RespaldoJson {
    pub app: &'static str,
    pub version_esquema: i32,
    /// ISO 'YYYY-MM-DD'.
    pub exportado_en: String,
    pub periodos: Vec<Periodo>,
    pub categorias: Vec<Categoria>,
    pub servicios: Vec<Servicio>,
    pub deudas: Vec<Deuda>,
    pub cuotas: Vec<Cuota>,
    pub movimientos: Vec<Movimiento>,
    pub presupuestos: Vec<Presupuesto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchivoExportado {
    pub nombre: String,
    pub ruta: String,
    pub filas: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoExportacion {
    pub archivos: Vec<ArchivoExportado>,
    pub total_filas: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoRestauracion {
    pub ruta_respaldo_previo: String,
    pub version_restaurada: i32,
    pub total_registros: i64,
}
