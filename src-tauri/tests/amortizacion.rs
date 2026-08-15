//! Tests de la generación de cuotas. Cubren la regla dura del proyecto:
//! la suma de las cuotas debe cuadrar exactamente con el monto original.

use chrono::NaiveDate;
use finanzas_lib::dominio::amortizacion::generar;

fn f(a: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, d).unwrap()
}

// ── sin interés ──────────────────────────────────────────────────────────────

#[test]
fn sin_interes_la_suma_calza_con_el_monto_original() {
    // Casos con residuo, sin residuo y de una sola cuota.
    let casos = [
        (100_000_i64, 3_i32),
        (1_000_000, 12),
        (999_999, 7),
        (450_000, 24),
        (1, 1),
        (7, 5),
        (12_345_678, 36),
    ];

    for (monto, n) in casos {
        let cuotas = generar(monto, 0.0, n, f(2026, 9, 5)).unwrap();

        assert_eq!(cuotas.len(), n as usize, "faltan cuotas para {monto}/{n}");
        assert_eq!(
            cuotas.iter().map(|c| c.monto).sum::<i64>(),
            monto,
            "la suma de las cuotas no calza para {monto} en {n} cuotas"
        );
        assert_eq!(
            cuotas.iter().map(|c| c.capital).sum::<i64>(),
            monto,
            "la suma del capital no calza para {monto} en {n} cuotas"
        );
        assert!(
            cuotas.iter().all(|c| c.interes == 0),
            "sin tasa no puede haber interés"
        );
    }
}

#[test]
fn sin_interes_el_residuo_va_en_la_ultima_cuota() {
    let cuotas = generar(100_000, 0.0, 3, f(2026, 9, 5)).unwrap();

    assert_eq!(cuotas[0].monto, 33_333);
    assert_eq!(cuotas[1].monto, 33_333);
    assert_eq!(cuotas[2].monto, 33_334);
}

#[test]
fn numeracion_correlativa_desde_uno() {
    let cuotas = generar(120_000, 0.0, 12, f(2026, 9, 5)).unwrap();
    let numeros: Vec<i32> = cuotas.iter().map(|c| c.numero).collect();
    assert_eq!(numeros, (1..=12).collect::<Vec<i32>>());
}

// ── crédito de consumo (cuota francesa) ──────────────────────────────────────

#[test]
fn con_interes_el_capital_cierra_en_el_monto_original() {
    let casos = [
        (1_000_000_i64, 0.021_f64, 24_i32),
        (5_000_000, 0.0135, 48),
        (350_000, 0.03, 6),
        (20_000_000, 0.008, 60),
        (100_000, 0.05, 1),
    ];

    for (monto, tasa, n) in casos {
        let cuotas = generar(monto, tasa, n, f(2026, 9, 10)).unwrap();

        assert_eq!(cuotas.len(), n as usize);
        assert_eq!(
            cuotas.iter().map(|c| c.capital).sum::<i64>(),
            monto,
            "el capital no cierra para {monto} al {tasa} en {n} cuotas"
        );
        assert!(
            cuotas.iter().all(|c| c.monto == c.capital + c.interes),
            "cada cuota debe ser capital + interés"
        );
        assert!(
            cuotas.iter().all(|c| c.capital > 0 && c.interes >= 0),
            "no puede haber amortización negativa"
        );
    }
}

#[test]
fn con_interes_la_cuota_es_pareja_salvo_la_ultima() {
    let cuotas = generar(1_000_000, 0.021, 24, f(2026, 9, 10)).unwrap();

    // Todas las cuotas menos la última valen exactamente lo mismo.
    let primera = cuotas[0].monto;
    assert!(
        cuotas[..23].iter().all(|c| c.monto == primera),
        "la cuota francesa debe ser fija hasta la penúltima"
    );

    // La última absorbe el ajuste, pero se mantiene en el mismo orden de magnitud.
    let ultima = cuotas[23].monto;
    assert!(
        (ultima - primera).abs() < primera / 10,
        "la última cuota se desvió demasiado: {ultima} vs {primera}"
    );
}

#[test]
fn con_interes_el_interes_decrece_y_el_capital_crece() {
    let cuotas = generar(5_000_000, 0.0135, 48, f(2026, 9, 10)).unwrap();

    for par in cuotas[..47].windows(2) {
        assert!(
            par[1].interes <= par[0].interes,
            "el interés debe ir bajando"
        );
        assert!(
            par[1].capital >= par[0].capital,
            "el capital debe ir subiendo"
        );
    }
}

// ── fechas de vencimiento ────────────────────────────────────────────────────

#[test]
fn vencimientos_mes_a_mes_recortando_meses_cortos() {
    let cuotas = generar(310_000, 0.0, 5, f(2026, 12, 31)).unwrap();
    let fechas: Vec<&str> = cuotas
        .iter()
        .map(|c| c.fecha_vencimiento.as_str())
        .collect();

    assert_eq!(
        fechas,
        vec![
            "2026-12-31",
            "2027-01-31",
            "2027-02-28", // febrero no bisiesto
            "2027-03-31", // vuelve al 31, no se queda en 28
            "2027-04-30",
        ]
    );
}

#[test]
fn vencimientos_en_febrero_bisiesto() {
    let cuotas = generar(60_000, 0.0, 2, f(2028, 1, 30)).unwrap();
    assert_eq!(cuotas[1].fecha_vencimiento, "2028-02-29");
}

// ── validaciones ─────────────────────────────────────────────────────────────

#[test]
fn rechaza_entradas_invalidas() {
    assert!(generar(0, 0.0, 12, f(2026, 9, 5)).is_err(), "monto 0");
    assert!(generar(-1000, 0.0, 12, f(2026, 9, 5)).is_err(), "monto negativo");
    assert!(generar(100_000, 0.0, 0, f(2026, 9, 5)).is_err(), "cero cuotas");
    assert!(generar(100_000, -0.01, 12, f(2026, 9, 5)).is_err(), "tasa negativa");
    assert!(generar(100_000, 1.5, 12, f(2026, 9, 5)).is_err(), "tasa absurda");
    assert!(generar(100_000, 0.0, 601, f(2026, 9, 5)).is_err(), "demasiadas cuotas");
    assert!(generar(100_000, f64::NAN, 12, f(2026, 9, 5)).is_err(), "tasa NaN");
}
