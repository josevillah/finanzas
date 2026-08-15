//! Política de rotación de los respaldos automáticos.

/// Prefijo de los archivos que genera el respaldo automático.
pub const PREFIJO_AUTOMATICO: &str = "finanzas-auto-";
/// Cuántas copias automáticas se conservan.
pub const COPIAS_A_CONSERVAR: usize = 5;

/// Nombre del respaldo automático de una fecha.
pub fn nombre_para(fecha_iso: &str) -> String {
    format!("{PREFIJO_AUTOMATICO}{fecha_iso}.db")
}

/// ¿Es un archivo generado por el respaldo automático?
pub fn es_automatico(nombre: &str) -> bool {
    nombre.starts_with(PREFIJO_AUTOMATICO) && nombre.ends_with(".db")
}

/// De una lista de nombres de respaldo, cuáles sobran para dejar solo los
/// `conservar` más recientes.
///
/// Los nombres llevan la fecha en ISO, así que el orden alfabético es también
/// el cronológico y no hace falta consultar el sistema de archivos.
pub fn a_eliminar(nombres: &[String], conservar: usize) -> Vec<String> {
    if nombres.len() <= conservar {
        return Vec::new();
    }

    let mut ordenados: Vec<String> = nombres.to_vec();
    ordenados.sort();
    ordenados.truncate(nombres.len() - conservar);
    ordenados
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nombres(fechas: &[&str]) -> Vec<String> {
        fechas.iter().map(|f| nombre_para(f)).collect()
    }

    #[test]
    fn reconoce_sus_propios_archivos() {
        assert!(es_automatico("finanzas-auto-2026-08-15.db"));
        assert!(!es_automatico("finanzas-pre-v4.db"));
        assert!(!es_automatico("finanzas.db"));
        assert!(!es_automatico("finanzas-auto-2026-08-15.db.tmp"));
    }

    #[test]
    fn no_borra_nada_si_no_se_pasa_del_limite() {
        let lista = nombres(&["2026-08-11", "2026-08-12", "2026-08-13"]);
        assert!(a_eliminar(&lista, 5).is_empty());

        let justos = nombres(&[
            "2026-08-11",
            "2026-08-12",
            "2026-08-13",
            "2026-08-14",
            "2026-08-15",
        ]);
        assert!(a_eliminar(&justos, 5).is_empty());
    }

    #[test]
    fn borra_los_mas_viejos_y_conserva_los_recientes() {
        let lista = nombres(&[
            "2026-08-10",
            "2026-08-11",
            "2026-08-12",
            "2026-08-13",
            "2026-08-14",
            "2026-08-15",
            "2026-08-16",
        ]);

        let sobran = a_eliminar(&lista, 5);

        assert_eq!(
            sobran,
            nombres(&["2026-08-10", "2026-08-11"]),
            "deben irse los dos más antiguos"
        );
    }

    #[test]
    fn el_orden_de_entrada_no_importa() {
        let desordenados = nombres(&[
            "2026-08-16",
            "2026-08-10",
            "2026-08-14",
            "2026-08-11",
            "2026-08-15",
            "2026-08-12",
        ]);

        assert_eq!(a_eliminar(&desordenados, 5), nombres(&["2026-08-10"]));
    }

    #[test]
    fn cruza_el_cambio_de_ano() {
        let lista = nombres(&[
            "2025-12-30",
            "2025-12-31",
            "2026-01-01",
            "2026-01-02",
            "2026-01-03",
            "2026-01-04",
        ]);

        assert_eq!(a_eliminar(&lista, 5), nombres(&["2025-12-30"]));
    }
}
