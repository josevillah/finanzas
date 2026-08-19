use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::Monto;
use crate::modelos::nota_ahorro::NotaAhorro;

/// Una cuenta de ahorro: plata apartada del disponible para no gastarla.
///
/// Es lo único que vive en la tabla. El saldo disponible no es una fila: se
/// calcula desde el saldo inicial y los movimientos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cuenta {
    pub id: i64,
    pub nombre: String,
    pub saldo: Monto,
    pub activa: bool,
    pub orden: i32,
    /// ISO 'YYYY-MM-DD' del último movimiento de plata en esta cuenta.
    pub actualizado_en: Option<String>,
}

impl Cuenta {
    pub const COLUMNAS: &'static str = "id, nombre, saldo, activa, orden, actualizado_en";

    pub fn desde_fila(fila: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Cuenta {
            id: fila.get(0)?,
            nombre: fila.get(1)?,
            saldo: fila.get(2)?,
            activa: fila.get::<_, i64>(3)? != 0,
            orden: fila.get(4)?,
            actualizado_en: fila.get(5)?,
        })
    }
}

/// Payload de creación. Una cuenta nace vacía: la plata entra apartándola.
#[derive(Debug, Clone, Deserialize)]
pub struct NuevaCuenta {
    pub nombre: String,
}

// ── DTOs de lectura ──────────────────────────────────────────────────────────

/// De dónde sale el disponible, término por término.
///
/// Va completo a la pantalla porque un número calculado que el usuario no
/// puede desarmar es un número que no puede verificar: sin el desglose no
/// tiene forma de saber si está bien.
#[derive(Debug, Clone, Serialize)]
pub struct DesgloseSaldo {
    /// Lo que tenía antes de empezar a usar la app. Lo declara él.
    pub saldo_inicial: Monto,
    /// Sueldos y otros ingresos declarados en los períodos.
    pub ingresos_declarados: Monto,
    /// Movimientos de tipo ingreso.
    pub ingresos_registrados: Monto,
    /// Movimientos de tipo gasto, estimados incluidos.
    pub gastos: Monto,
    /// Parte de `gastos` que todavía es una proyección sin confirmar.
    /// Se muestra aparte para explicar por qué el disponible puede verse bajo
    /// a principio de mes.
    pub gastos_estimados: Monto,
    /// Suma de los saldos de ahorro.
    pub apartado: Monto,
}

impl DesgloseSaldo {
    /// Todo lo que entró, de cualquier fuente.
    pub fn ingresos(&self) -> Monto {
        self.ingresos_declarados + self.ingresos_registrados
    }

    /// Saldo inicial más ingresos menos gastos. Incluye lo apartado: la plata
    /// en un ahorro sigue siendo tuya.
    pub fn patrimonio(&self) -> Monto {
        self.saldo_inicial + self.ingresos() - self.gastos
    }

    /// Lo que queda para gastar sin tocar los ahorros.
    ///
    /// Puede ser negativo, y con razón: quiere decir que gastaste más de lo
    /// que entró, o que el saldo inicial todavía no está ajustado.
    pub fn disponible(&self) -> Monto {
        self.patrimonio() - self.apartado
    }
}

/// Una cuenta de ahorro con el desglose informativo de su saldo.
///
/// Las notas no participan de ningún cálculo. `total_notas` y `sin_asignar`
/// existen para que la pantalla pueda avisar cuando no cuadran sin tener que
/// sumarlas ella misma: la aritmética vive en Rust.
#[derive(Debug, Clone, Serialize)]
pub struct CuentaConNotas {
    #[serde(flatten)]
    pub cuenta: Cuenta,
    pub notas: Vec<NotaAhorro>,
    pub total_notas: Monto,
    /// `saldo - total_notas`. Positivo: queda plata sin anotar. Negativo: las
    /// notas se pasaron del saldo, que es un estado válido y solo se avisa.
    pub sin_asignar: Monto,
}

impl CuentaConNotas {
    pub fn nueva(cuenta: Cuenta, notas: Vec<NotaAhorro>) -> Self {
        let total_notas: Monto = notas.iter().map(|n| n.monto).sum();

        CuentaConNotas {
            sin_asignar: cuenta.saldo - total_notas,
            total_notas,
            cuenta,
            notas,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumenCuentas {
    pub disponible: Monto,
    /// Disponible más ahorros. No resta deudas: no es patrimonio neto.
    pub patrimonio: Monto,
    pub total_ahorrado: Monto,
    pub ahorros: Vec<CuentaConNotas>,
    pub desglose: DesgloseSaldo,
}
