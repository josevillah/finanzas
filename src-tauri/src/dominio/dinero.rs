//! Manejo de dinero. Regla no negociable del proyecto: los montos son
//! siempre enteros de pesos chilenos. Nunca f64 para almacenar dinero.

/// Un monto en pesos chilenos, sin decimales.
pub type Monto = i64;

/// Reparte `total` en `partes` montos enteros lo más parejos posible.
/// El residuo de la división entera se suma completo a la última parte, de
/// modo que la suma del resultado sea exactamente `total`.
///
/// Devuelve un vector vacío si `partes` es 0.
pub fn repartir(total: Monto, partes: usize) -> Vec<Monto> {
    if partes == 0 {
        return Vec::new();
    }

    let n = partes as i64;
    let base = total / n;
    let residuo = total - base * n;

    let mut cuotas = vec![base; partes];
    // `partes >= 1`, así que el índice siempre existe.
    cuotas[partes - 1] += residuo;
    cuotas
}

/// Redondea un cálculo intermedio en punto flotante al peso más cercano.
/// Se usa solo para intereses derivados de una tasa, nunca para almacenar
/// un monto que ya venía siendo entero.
pub fn redondear_a_peso(valor: f64) -> Monto {
    valor.round() as Monto
}

/// Promedio entero de `total` repartido en `meses`.
///
/// `meses` es la cantidad de meses **con gasto**, no el largo de la ventana:
/// dividir por meses vacíos convierte un gasto de una vez en uno recurrente
/// —"$82.696 en agosto" se ve como "$13.782 al mes"— y castiga a quien lleva
/// menos tiempo que la ventana usando la aplicación.
///
/// Con 0 meses devuelve 0. No hay promedio que calcular, y es la única
/// respuesta que no inventa un número ni revienta.
pub fn promedio_mensual(total: Monto, meses: i64) -> Monto {
    if meses <= 0 {
        return 0;
    }
    total / meses
}

/// Variación porcentual de `antes` a `ahora`. Devuelve `None` cuando no hay
/// base positiva contra la cual comparar: sin eso el porcentaje no significa
/// nada (y dividir por cero da infinito).
pub fn variacion_porcentual(antes: Monto, ahora: Monto) -> Option<f64> {
    if antes <= 0 {
        return None;
    }
    Some(((ahora - antes) as f64 / antes as f64) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variacion_sube_baja_y_se_abstiene() {
        assert_eq!(variacion_porcentual(100_000, 150_000), Some(50.0));
        assert_eq!(variacion_porcentual(100_000, 75_000), Some(-25.0));
        assert_eq!(variacion_porcentual(100_000, 100_000), Some(0.0));
        // Sin base previa no hay porcentaje que reportar.
        assert_eq!(variacion_porcentual(0, 50_000), None);
    }

    #[test]
    fn el_promedio_divide_por_los_meses_con_gasto() {
        // El caso que motivó la función: todo el gasto en un solo mes.
        assert_eq!(promedio_mensual(82_696, 1), 82_696);
        assert_eq!(promedio_mensual(82_696, 6), 13_782, "trunca, no redondea");
    }

    #[test]
    fn sin_meses_no_hay_promedio() {
        assert_eq!(promedio_mensual(500_000, 0), 0);
        assert_eq!(promedio_mensual(500_000, -3), 0);
    }

    #[test]
    fn repartir_suma_exacta_con_residuo() {
        let partes = repartir(100_000, 3);
        assert_eq!(partes, vec![33_333, 33_333, 33_334]);
        assert_eq!(partes.iter().sum::<i64>(), 100_000);
    }

    #[test]
    fn repartir_division_exacta() {
        let partes = repartir(120_000, 12);
        assert!(partes.iter().all(|m| *m == 10_000));
        assert_eq!(partes.iter().sum::<i64>(), 120_000);
    }

    #[test]
    fn repartir_una_parte() {
        assert_eq!(repartir(7, 1), vec![7]);
    }

    #[test]
    fn repartir_cero_partes() {
        assert!(repartir(1_000, 0).is_empty());
    }
}
