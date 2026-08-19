//! Acceso a datos. Todo el SQL vive acá; ninguna otra capa escribe consultas.
//! Las funciones reciben `&Connection`, que también acepta una `&Transaction`
//! vía Deref, así el manejo de transacciones queda en la capa de comandos.

pub mod categorias;
pub mod configuracion;
pub mod cuentas;
pub mod cuotas;
pub mod deudas;
pub mod movimientos;
pub mod notas_ahorro;
pub mod periodos;
pub mod presupuestos;
pub mod servicios;
