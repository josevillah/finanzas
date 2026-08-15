//! Structs espejo de las tablas + DTOs que consume el frontend.
//! Los nombres de campo se serializan tal cual (snake_case) para que
//! `src/types/dominio.ts` sea una traducción literal.

pub mod categoria;
pub mod configuracion;
pub mod cuota;
pub mod deuda;
pub mod movimiento;
pub mod periodo;
pub mod presupuesto;
pub mod reinicio;
pub mod reporte;
pub mod respaldo;
pub mod servicio;

pub use categoria::{Categoria, NuevaCategoria, TipoCategoria, CODIGO_DEUDAS};
pub use configuracion::{AccionCierre, AjustesApp};
pub use cuota::{Cuota, CuotaConDeuda, EstadoCuota, PagoCuota};
pub use deuda::{
    CargaFinanciera, Deuda, DeudaDetalle, DeudaResumen, EstadoDeuda, FechaLibertad, Liberacion,
    MesCarga, NuevaDeuda, Semaforo, TipoDeuda,
};
pub use movimiento::{
    FiltroMovimientos, MedioPago, Movimiento, MovimientoDetalle, NuevoMovimiento, TipoMovimiento,
};
pub use periodo::{GastoPorCategoria, Periodo, ResumenPeriodo};
pub use presupuesto::{
    AsignacionPresupuesto, EstadoPresupuesto, LineaPresupuesto, Presupuesto, ResumenPresupuesto,
};
pub use reporte::{EvolucionGastos, MesHormiga, PuntoMes, ReporteHormiga, SerieCategoria};
pub use respaldo::{
    ArchivoExportado, EstadoRespaldo, RespaldoJson, ResultadoExportacion, ResultadoRestauracion,
};
pub use servicio::{NuevoServicio, ResumenServicios, Servicio, ServicioConReal, TipoServicio};
