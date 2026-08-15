//! Escritura de CSV. Lo justo para exportar tablas: sin dependencias y con
//! el escapado que exige RFC 4180.

/// Escapa un campo. Se encomilla solo cuando hace falta —coma, comillas o
/// salto de línea—, y las comillas internas se duplican.
pub fn escapar_campo(valor: &str) -> String {
    let necesita_comillas = valor.contains(',')
        || valor.contains('"')
        || valor.contains('\n')
        || valor.contains('\r');

    if !necesita_comillas {
        return valor.to_string();
    }

    format!("\"{}\"", valor.replace('"', "\"\""))
}

/// Arma una línea CSV terminada en CRLF, como pide el RFC (y como espera
/// Excel en Windows).
pub fn linea<S: AsRef<str>>(campos: &[S]) -> String {
    let cuerpo: Vec<String> = campos
        .iter()
        .map(|c| escapar_campo(c.as_ref()))
        .collect();

    format!("{}\r\n", cuerpo.join(","))
}

/// Representación CSV de un valor que puede venir nulo.
pub fn opcional<T: ToString>(valor: &Option<T>) -> String {
    valor.as_ref().map(ToString::to_string).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deja_pasar_lo_simple() {
        assert_eq!(escapar_campo("Supermercado"), "Supermercado");
        assert_eq!(escapar_campo("45000"), "45000");
        assert_eq!(escapar_campo(""), "");
    }

    #[test]
    fn encomilla_cuando_corresponde() {
        assert_eq!(escapar_campo("Pan, queso"), "\"Pan, queso\"");
        assert_eq!(escapar_campo("línea1\nlínea2"), "\"línea1\nlínea2\"");
    }

    #[test]
    fn duplica_las_comillas_internas() {
        assert_eq!(escapar_campo("el \"super\""), "\"el \"\"super\"\"\"");
    }

    #[test]
    fn arma_la_linea_completa() {
        assert_eq!(
            linea(&["1", "Pan, queso", "3500"]),
            "1,\"Pan, queso\",3500\r\n"
        );
    }

    #[test]
    fn los_nulos_quedan_vacios() {
        assert_eq!(opcional(&Some(42)), "42");
        assert_eq!(opcional::<i64>(&None), "");
    }
}
