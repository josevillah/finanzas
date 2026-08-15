//! Fechas. En la base todo se guarda como TEXT ISO 'YYYY-MM-DD'.

use chrono::{Datelike, Local, NaiveDate};

use crate::error::{AppError, Resultado};

pub const FORMATO_ISO: &str = "%Y-%m-%d";

pub fn hoy() -> NaiveDate {
    Local::now().date_naive()
}

pub fn a_iso(fecha: NaiveDate) -> String {
    fecha.format(FORMATO_ISO).to_string()
}

/// Marca de tiempo para nombres de archivo: 'YYYY-MM-DD-HHMMSS'.
/// Con solo la fecha, dos respaldos del mismo día se pisarían entre sí.
pub fn sello_de_tiempo() -> String {
    Local::now().format("%Y-%m-%d-%H%M%S").to_string()
}

pub fn desde_iso(texto: &str) -> Resultado<NaiveDate> {
    NaiveDate::parse_from_str(texto.trim(), FORMATO_ISO)
        .map_err(|_| AppError::validacion(format!("Fecha inválida: '{texto}'. Se espera YYYY-MM-DD.")))
}

pub fn es_bisiesto(anio: i32) -> bool {
    (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0
}

/// Cantidad de días del mes indicado. `mes` va de 1 a 12.
pub fn dias_del_mes(anio: i32, mes: u32) -> u32 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if es_bisiesto(anio) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Avanza `meses` meses desde `base`, conservando el día del mes cuando existe
/// y recortando al último día del mes de destino cuando no.
///
/// Se calcula siempre desde `base` (no acumulando mes a mes), así el 31-ene
/// produce 28-feb, 31-mar, 30-abr... y no se queda pegado en el día 28.
pub fn avanzar_meses(base: NaiveDate, meses: u32) -> NaiveDate {
    let total = base.month0() as i64 + meses as i64;
    let anio = base.year() + (total / 12) as i32;
    let mes = (total % 12) as u32 + 1;
    let dia = base.day().min(dias_del_mes(anio, mes));

    NaiveDate::from_ymd_opt(anio, mes, dia)
        .expect("mes y día ya fueron acotados a un rango válido")
}

/// Primer día del mes (para rangos `>= x AND <= y` en SQL).
pub fn primer_dia(anio: i32, mes: u32) -> Resultado<NaiveDate> {
    NaiveDate::from_ymd_opt(anio, mes, 1)
        .ok_or_else(|| AppError::validacion(format!("Mes inválido: {mes}/{anio}")))
}

/// Último día del mes.
pub fn ultimo_dia(anio: i32, mes: u32) -> Resultado<NaiveDate> {
    let dia = dias_del_mes(anio, mes);
    NaiveDate::from_ymd_opt(anio, mes, dia)
        .ok_or_else(|| AppError::validacion(format!("Mes inválido: {mes}/{anio}")))
}

/// Índice absoluto del mes, para comparar y recorrer ventanas de tiempo sin
/// pelear con el borde de diciembre. Enero de 2026 y diciembre de 2025 quedan
/// consecutivos.
pub fn mes_absoluto(anio: i32, mes: u32) -> i64 {
    anio as i64 * 12 + mes as i64
}

/// Inversa de [`mes_absoluto`].
pub fn desde_mes_absoluto(absoluto: i64) -> (i32, u32) {
    let mes = ((absoluto - 1).rem_euclid(12) + 1) as u32;
    let anio = ((absoluto - mes as i64) / 12) as i32;
    (anio, mes)
}

/// Clave 'YYYY-MM', la misma que usan las consultas con strftime.
pub fn clave_mes(anio: i32, mes: u32) -> String {
    format!("{anio:04}-{mes:02}")
}

/// Los `cantidad` meses consecutivos que terminan en el mes indicado, en orden
/// cronológico. Es la ventana que usan los reportes.
pub fn ventana_de_meses(anio: i32, mes: u32, cantidad: u32) -> Vec<(i32, u32)> {
    let hasta = mes_absoluto(anio, mes);
    let desde = hasta - (cantidad.max(1) as i64 - 1);

    (desde..=hasta).map(desde_mes_absoluto).collect()
}

/// Cantidad de meses calendario entre dos fechas (desde -> hasta).
/// Negativo si `hasta` es anterior a `desde`.
pub fn meses_entre(desde: NaiveDate, hasta: NaiveDate) -> i32 {
    (hasta.year() - desde.year()) * 12 + (hasta.month() as i32 - desde.month() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn recorta_31_a_fin_de_febrero() {
        assert_eq!(avanzar_meses(f(2026, 1, 31), 1), f(2026, 2, 28));
    }

    #[test]
    fn no_se_queda_pegado_en_el_dia_recortado() {
        let base = f(2026, 1, 31);
        assert_eq!(avanzar_meses(base, 2), f(2026, 3, 31));
        assert_eq!(avanzar_meses(base, 3), f(2026, 4, 30));
    }

    #[test]
    fn febrero_bisiesto() {
        assert_eq!(avanzar_meses(f(2028, 1, 30), 1), f(2028, 2, 29));
        assert!(es_bisiesto(2028) && !es_bisiesto(2027) && !es_bisiesto(2100));
    }

    #[test]
    fn el_mes_absoluto_va_y_vuelve() {
        for (anio, mes) in [(2025, 12), (2026, 1), (2026, 8), (2026, 12), (2027, 1)] {
            let abs = mes_absoluto(anio, mes);
            assert_eq!(desde_mes_absoluto(abs), (anio, mes));
        }

        // Diciembre y enero del año siguiente quedan consecutivos.
        assert_eq!(
            mes_absoluto(2027, 1) - mes_absoluto(2026, 12),
            1,
            "el borde de año no debe saltar"
        );
    }

    #[test]
    fn cruza_el_anio() {
        assert_eq!(avanzar_meses(f(2026, 12, 15), 1), f(2027, 1, 15));
        assert_eq!(avanzar_meses(f(2026, 8, 5), 24), f(2028, 8, 5));
    }
}
