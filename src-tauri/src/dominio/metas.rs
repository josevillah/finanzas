//! Aritmética de las metas: reparto del saldo, ritmo necesario y proyección
//! contra el balance. Sin SQL y sin fechas del sistema, para poder fijarlo
//! todo en tests.

use crate::dominio::dinero::Monto;

/// Reparte el saldo de una cuenta entre las metas que la comparten, en el
/// orden en que vienen (que es el orden de prioridad).
///
/// Cada meta consume lo que le falta para su objetivo antes de dejar pasar
/// algo a la siguiente: con $100.000, una meta de $80.000 y otra de $50.000,
/// la primera queda completa y la segunda con $20.000. El resto de la plata,
/// si sobra, no se asigna a nadie.
///
/// Es entero de punta a punta: no hay proporciones ni porcentajes que puedan
/// perder o inventar un peso.
pub fn repartir_por_prioridad(saldo: Monto, objetivos: &[Monto]) -> Vec<Monto> {
    let mut restante = saldo.max(0);

    objetivos
        .iter()
        .map(|objetivo| {
            let toma = restante.min((*objetivo).max(0));
            restante -= toma;
            toma
        })
        .collect()
}

/// Cuánto habría que apartar por mes para cubrir `falta` en `meses`.
///
/// Redondea hacia arriba: apartar el resultado exacto cada mes tiene que
/// alcanzar, y con división entera hacia abajo quedaría corto por unos pesos.
/// Devuelve `None` cuando no hay nada que juntar o no queda tiempo por
/// delante; el caso "la fecha ya pasó" lo decide quien llama.
pub fn ritmo_necesario(falta: Monto, meses: i32) -> Option<Monto> {
    if falta <= 0 || meses <= 0 {
        return None;
    }

    let meses = meses as Monto;
    Some((falta + meses - 1) / meses)
}

/// Cuántos meses de `balance_promedio` hacen falta para cubrir `falta`.
///
/// Con un promedio negativo o cero no hay proyección posible: al ritmo actual
/// no se llega nunca, y devolver un número enorme sería peor que no devolver
/// ninguno. La pantalla dice eso con palabras.
pub fn meses_al_ritmo(falta: Monto, balance_promedio: Monto) -> Option<i32> {
    if falta <= 0 {
        return Some(0);
    }
    if balance_promedio <= 0 {
        return None;
    }

    Some(((falta + balance_promedio - 1) / balance_promedio) as i32)
}

/// Promedio entero de los balances mensuales. `None` sin muestras.
pub fn promedio(valores: &[Monto]) -> Option<Monto> {
    if valores.is_empty() {
        return None;
    }

    let suma: Monto = valores.iter().sum();
    Some(suma / valores.len() as Monto)
}

/// Avance en porcentaje, acotado a 0-100.
///
/// El tope importa: una cuenta con más plata que el objetivo daría 120% y una
/// barra de progreso más larga que su caja.
pub fn progreso_pct(acumulado: Monto, objetivo: Monto) -> f64 {
    if objetivo <= 0 {
        return 0.0;
    }

    ((acumulado as f64 / objetivo as f64) * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_reparto_sigue_el_orden_de_prioridad() {
        // El ejemplo del brief.
        assert_eq!(repartir_por_prioridad(100_000, &[80_000, 50_000]), vec![80_000, 20_000]);
    }

    #[test]
    fn lo_que_sobra_no_se_asigna_a_nadie() {
        assert_eq!(repartir_por_prioridad(200_000, &[80_000, 50_000]), vec![80_000, 50_000]);
    }

    #[test]
    fn sin_saldo_nadie_avanza() {
        assert_eq!(repartir_por_prioridad(0, &[80_000, 50_000]), vec![0, 0]);
    }

    #[test]
    fn la_primera_meta_puede_dejar_a_las_demas_en_cero() {
        assert_eq!(
            repartir_por_prioridad(50_000, &[80_000, 50_000, 10_000]),
            vec![50_000, 0, 0]
        );
    }

    #[test]
    fn el_reparto_nunca_entrega_mas_que_el_saldo() {
        let objetivos = [33_333, 33_333, 33_334];
        let reparto = repartir_por_prioridad(50_000, &objetivos);
        assert_eq!(reparto.iter().sum::<Monto>(), 50_000);
    }

    #[test]
    fn el_ritmo_redondea_hacia_arriba() {
        // 100.000 en 3 meses: 33.333 no alcanza, 33.334 sí.
        assert_eq!(ritmo_necesario(100_000, 3), Some(33_334));
        assert_eq!(ritmo_necesario(90_000, 3), Some(30_000));
    }

    #[test]
    fn sin_falta_o_sin_meses_no_hay_ritmo() {
        assert_eq!(ritmo_necesario(0, 5), None);
        assert_eq!(ritmo_necesario(100_000, 0), None);
        assert_eq!(ritmo_necesario(100_000, -2), None);
    }

    #[test]
    fn con_balance_negativo_o_cero_no_se_proyecta() {
        assert_eq!(meses_al_ritmo(500_000, -20_000), None);
        assert_eq!(meses_al_ritmo(500_000, 0), None);
    }

    #[test]
    fn la_proyeccion_redondea_hacia_arriba() {
        // Con 200.000 al mes, 500.000 se juntan recién en el tercer mes.
        assert_eq!(meses_al_ritmo(500_000, 200_000), Some(3));
        assert_eq!(meses_al_ritmo(400_000, 200_000), Some(2));
    }

    #[test]
    fn una_meta_ya_cubierta_no_necesita_meses() {
        assert_eq!(meses_al_ritmo(0, -20_000), Some(0));
    }

    #[test]
    fn el_promedio_sin_muestras_no_existe() {
        assert_eq!(promedio(&[]), None);
        assert_eq!(promedio(&[100_000, 200_000, 300_000]), Some(200_000));
        assert_eq!(promedio(&[-100_000, 50_000]), Some(-25_000));
    }

    #[test]
    fn el_progreso_se_acota_a_cien() {
        assert_eq!(progreso_pct(50_000, 100_000), 50.0);
        assert_eq!(progreso_pct(150_000, 100_000), 100.0);
        assert_eq!(progreso_pct(0, 0), 0.0);
    }
}
