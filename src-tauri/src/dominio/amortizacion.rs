//! Generación de la tabla de cuotas de una deuda.
//!
//! Dos casos:
//!  - tasa = 0  -> reparto entero, el residuo va a la última cuota.
//!  - tasa > 0  -> cuota fija francesa, la última cuota cierra el saldo en 0.
//!
//! En ambos casos se garantiza que `suma(capital) == monto_original` exacto.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::dominio::dinero::{self, Monto};
use crate::dominio::fechas;
use crate::error::{AppError, Resultado};

/// Tope de sanidad: 50 años de cuotas mensuales.
const MAX_CUOTAS: i32 = 600;
/// Tope de sanidad para la tasa mensual (100% mensual).
const MAX_TASA_MENSUAL: f64 = 1.0;

/// Una cuota tal como quedará en la tabla `cuotas`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuotaCalculada {
    pub numero: i32,
    /// ISO 'YYYY-MM-DD'.
    pub fecha_vencimiento: String,
    pub monto: Monto,
    pub capital: Monto,
    pub interes: Monto,
}

/// Genera las N cuotas de una deuda.
pub fn generar(
    monto_original: Monto,
    tasa_mensual: f64,
    n_cuotas: i32,
    fecha_primera_cuota: NaiveDate,
) -> Resultado<Vec<CuotaCalculada>> {
    validar(monto_original, tasa_mensual, n_cuotas)?;

    if tasa_mensual == 0.0 {
        Ok(sin_interes(monto_original, n_cuotas, fecha_primera_cuota))
    } else {
        con_interes(monto_original, tasa_mensual, n_cuotas, fecha_primera_cuota)
    }
}

fn validar(monto_original: Monto, tasa_mensual: f64, n_cuotas: i32) -> Resultado<()> {
    if monto_original <= 0 {
        return Err(AppError::validacion("El monto original debe ser mayor a 0."));
    }
    if n_cuotas < 1 {
        return Err(AppError::validacion("La deuda debe tener al menos 1 cuota."));
    }
    if n_cuotas > MAX_CUOTAS {
        return Err(AppError::validacion(format!(
            "El número de cuotas no puede superar {MAX_CUOTAS}."
        )));
    }
    if !tasa_mensual.is_finite() || tasa_mensual < 0.0 {
        return Err(AppError::validacion(
            "La tasa mensual debe ser un número mayor o igual a 0.",
        ));
    }
    if tasa_mensual > MAX_TASA_MENSUAL {
        return Err(AppError::validacion(
            "La tasa mensual parece equivocada: se ingresa como fracción (0,025 = 2,5% mensual).",
        ));
    }
    Ok(())
}

/// Cuotas sin interés: división entera y residuo a la última cuota.
fn sin_interes(monto_original: Monto, n_cuotas: i32, primera: NaiveDate) -> Vec<CuotaCalculada> {
    dinero::repartir(monto_original, n_cuotas as usize)
        .into_iter()
        .enumerate()
        .map(|(idx, monto)| CuotaCalculada {
            numero: idx as i32 + 1,
            fecha_vencimiento: fechas::a_iso(fechas::avanzar_meses(primera, idx as u32)),
            monto,
            capital: monto,
            interes: 0,
        })
        .collect()
}

/// Cuota fija francesa: cuota = P * i / (1 - (1 + i)^(-n)).
/// Luego se desglosa cuota a cuota y la última absorbe el saldo restante.
fn con_interes(
    monto_original: Monto,
    tasa: f64,
    n_cuotas: i32,
    primera: NaiveDate,
) -> Resultado<Vec<CuotaCalculada>> {
    let p = monto_original as f64;
    let n = n_cuotas as f64;

    let cuota_exacta = p * tasa / (1.0 - (1.0 + tasa).powf(-n));
    if !cuota_exacta.is_finite() {
        return Err(AppError::validacion(
            "No se pudo calcular la cuota con los valores entregados.",
        ));
    }
    let cuota_fija = dinero::redondear_a_peso(cuota_exacta);

    let mut cuotas = Vec::with_capacity(n_cuotas as usize);
    let mut saldo = monto_original;

    for numero in 1..=n_cuotas {
        let interes = dinero::redondear_a_peso(saldo as f64 * tasa);
        let es_ultima = numero == n_cuotas;

        let (monto, capital) = if es_ultima {
            // La última cuota paga todo el saldo pendiente: así
            // suma(capital) == monto_original exacto y el saldo cierra en 0.
            (saldo + interes, saldo)
        } else {
            let capital = cuota_fija - interes;
            if capital <= 0 {
                return Err(AppError::validacion(
                    "Con esa tasa y ese número de cuotas la deuda nunca se amortiza. \
                     Revisa la tasa mensual o aumenta el monto de la cuota.",
                ));
            }
            // Blindaje ante redondeos en el tramo final: nunca amortizar más
            // que el saldo vivo.
            let capital = capital.min(saldo);
            (capital + interes, capital)
        };

        cuotas.push(CuotaCalculada {
            numero,
            fecha_vencimiento: fechas::a_iso(fechas::avanzar_meses(primera, (numero - 1) as u32)),
            monto,
            capital,
            interes,
        });

        saldo -= capital;
    }

    debug_assert_eq!(saldo, 0, "el saldo debe cerrar en 0");
    Ok(cuotas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn sin_interes_suma_exacta() {
        let cuotas = generar(100_000, 0.0, 3, f(2026, 9, 5)).unwrap();
        assert_eq!(cuotas.iter().map(|c| c.monto).sum::<i64>(), 100_000);
        assert_eq!(cuotas[2].monto, 33_334);
        assert!(cuotas.iter().all(|c| c.interes == 0 && c.capital == c.monto));
    }

    #[test]
    fn con_interes_cierra_el_capital() {
        let cuotas = generar(1_000_000, 0.021, 24, f(2026, 9, 10)).unwrap();
        assert_eq!(cuotas.len(), 24);
        assert_eq!(cuotas.iter().map(|c| c.capital).sum::<i64>(), 1_000_000);
        assert!(cuotas.iter().all(|c| c.monto == c.capital + c.interes));
    }

    #[test]
    fn rechaza_monto_no_positivo() {
        assert!(generar(0, 0.0, 3, f(2026, 9, 5)).is_err());
    }

    #[test]
    fn rechaza_tasa_absurda() {
        assert!(generar(100_000, 2.5, 12, f(2026, 9, 5)).is_err());
    }
}
