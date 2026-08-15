use serde::{Deserialize, Serialize};

/// Qué hacer cuando se cierra la ventana con la X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccionCierre {
    /// Mostrar el diálogo cada vez. Es el comportamiento por omisión.
    Preguntar,
    /// Ocultar la ventana y seguir corriendo en la bandeja.
    Bandeja,
    /// Cerrar la aplicación de verdad.
    Salir,
}

impl AccionCierre {
    pub fn como_texto(self) -> &'static str {
        match self {
            AccionCierre::Preguntar => "preguntar",
            AccionCierre::Bandeja => "bandeja",
            AccionCierre::Salir => "salir",
        }
    }

    /// Convierte desde lo guardado en `configuracion`. Ante un valor ausente o
    /// desconocido cae en `Preguntar`: es la opción que nunca deja al usuario
    /// sin poder cerrar ni sin ventana.
    pub fn desde_texto(texto: Option<&str>) -> Self {
        match texto {
            Some("bandeja") => AccionCierre::Bandeja,
            Some("salir") => AccionCierre::Salir,
            _ => AccionCierre::Preguntar,
        }
    }
}

/// Ajustes de la app que consume la pantalla de configuración.
#[derive(Debug, Clone, Serialize)]
pub struct AjustesApp {
    pub accion_cierre: AccionCierre,
    pub autostart_activo: bool,
    /// Si el atajo global quedó tomado por otra aplicación, esto viene en
    /// false y la pantalla de configuración lo avisa.
    pub atajo_registrado: bool,
    /// Combinación del atajo, para mostrarla sin hardcodearla en el frontend.
    pub atajo: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_de_texto() {
        for accion in [
            AccionCierre::Preguntar,
            AccionCierre::Bandeja,
            AccionCierre::Salir,
        ] {
            assert_eq!(AccionCierre::desde_texto(Some(accion.como_texto())), accion);
        }
    }

    #[test]
    fn sin_valor_o_con_basura_pregunta() {
        assert_eq!(AccionCierre::desde_texto(None), AccionCierre::Preguntar);
        assert_eq!(AccionCierre::desde_texto(Some("")), AccionCierre::Preguntar);
        assert_eq!(
            AccionCierre::desde_texto(Some("minimizar")),
            AccionCierre::Preguntar,
            "un valor desconocido no debe dejar al usuario sin poder cerrar"
        );
    }
}
