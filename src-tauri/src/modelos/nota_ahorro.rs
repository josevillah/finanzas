use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;

/// Una nota de propósito dentro de una cuenta de ahorro: cuánto de lo que hay
/// en esa cuenta el usuario tiene mentalmente reservado para algo.
///
/// Es una anotación y nada más. No mueve plata, no entra en el disponible ni en
/// el patrimonio, y su suma puede quedar por debajo o por encima del saldo sin
/// que eso rompa nada: cuando pasa, se avisa y el usuario ajusta cuando quiera.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotaAhorro {
    pub id: i64,
    pub cuenta_id: i64,
    pub nombre: String,
    pub monto: Monto,
    pub orden: i32,
}

impl NotaAhorro {
    pub const COLUMNAS: &'static str = "id, cuenta_id, nombre, monto, orden";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(NotaAhorro {
            id: fila.get(0)?,
            cuenta_id: fila.get(1)?,
            nombre: fila.get(2)?,
            monto: fila.get(3)?,
            orden: fila.get(4)?,
        })
    }
}

/// Payload de creación. El orden lo decide el repositorio: una nota nueva va al
/// final de su cuenta.
#[derive(Debug, Clone, Deserialize)]
pub struct NuevaNota {
    pub cuenta_id: i64,
    pub nombre: String,
    pub monto: Monto,
}
