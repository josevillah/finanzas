use serde::{Serialize, Serializer};

/// Error único de la aplicación. Se serializa como string plano para que el
/// frontend reciba un mensaje legible en español desde `invoke`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error interno de Tauri: {0}")]
    Tauri(#[from] tauri::Error),

    /// Dato inválido enviado por el usuario.
    #[error("{0}")]
    Validacion(String),

    /// La entidad solicitada no existe.
    #[error("No se encontró {0}")]
    NoEncontrado(String),

    /// La operación no se puede realizar en el estado actual.
    #[error("{0}")]
    Conflicto(String),
}

impl AppError {
    pub fn validacion(msg: impl Into<String>) -> Self {
        AppError::Validacion(msg.into())
    }

    pub fn no_encontrado(msg: impl Into<String>) -> Self {
        AppError::NoEncontrado(msg.into())
    }

    pub fn conflicto(msg: impl Into<String>) -> Self {
        AppError::Conflicto(msg.into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Resultado<T> = Result<T, AppError>;
