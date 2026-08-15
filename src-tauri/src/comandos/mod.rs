//! Capa de comandos de Tauri: valida la entrada, abre transacciones y delega.
//! Nada de lógica de cálculo acá; eso vive en `dominio`.

pub mod analisis;
pub mod categorias;
pub mod configuracion;
pub mod cuotas;
pub mod deudas;
pub mod movimientos;
pub mod periodos;
pub mod presupuestos;
pub mod reportes;
pub mod respaldo;
pub mod servicios;
