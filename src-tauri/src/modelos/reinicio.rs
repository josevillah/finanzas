use serde::Serialize;

/// Texto que el usuario debe escribir para confirmar. Se valida también en
/// Rust: la interfaz puede saltarse, el backend no.
pub const CONFIRMACION: &str = "REINICIAR";

/// Cuántos registros hay hoy, para mostrarlos antes de borrar. Ver el número
/// real frena mejor que cualquier texto de advertencia.
#[derive(Debug, Clone, Serialize)]
pub struct ResumenReinicio {
    pub deudas: i64,
    pub cuotas: i64,
    pub movimientos: i64,
    pub presupuestos: i64,
    pub periodos: i64,
    /// Se conservan salvo que se pida borrarlos.
    pub servicios: i64,
    /// Categorías creadas por el usuario; las de fábrica no se tocan.
    pub categorias_propias: i64,
    /// Suma de lo que se borraría sin contar servicios ni categorías.
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultadoReinicio {
    /// Ruta completa del respaldo previo. Es la única vuelta atrás.
    pub ruta_respaldo: String,
    pub registros_borrados: i64,
    pub servicios_borrados: i64,
    pub categorias_borradas: i64,
    pub categorias_reactivadas: i64,
}
