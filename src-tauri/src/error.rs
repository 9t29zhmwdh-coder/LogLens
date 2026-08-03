use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum LlError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Anyhow: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("{0}")]
    Other(String),
}

impl Serialize for LlError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LlError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Haelt die Fehlertexte fest. Sie sind nicht intern: `Serialize` gibt
    /// genau diesen String an das Frontend weiter, wo er im Fenster landet.
    /// Ein Versionssprung von `thiserror` darf die Formatierung deshalb nicht
    /// stillschweigend veraendern.
    #[test]
    fn fehlertexte_bleiben_wie_sie_sind() {
        let io = LlError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "keine solche Datei",
        ));
        assert_eq!(io.to_string(), "IO error: keine solche Datei");

        let db = LlError::Db(sqlx::Error::RowNotFound);
        assert_eq!(
            db.to_string(),
            "Database error: no rows returned by a query that expected to return at least one row"
        );

        let anyhow = LlError::Anyhow(anyhow::anyhow!("kaputt"));
        assert_eq!(anyhow.to_string(), "Anyhow: kaputt");

        let other = LlError::Other("nur der Text".into());
        assert_eq!(other.to_string(), "nur der Text");
    }

    /// Der Weg zum Frontend fuehrt ueber Serialize, nicht ueber Display
    /// direkt. Beide muessen dasselbe liefern.
    #[test]
    fn serialisierung_liefert_denselben_text() {
        let fehler = LlError::Other("etwas ging schief".into());
        let json = serde_json::to_string(&fehler).unwrap();
        assert_eq!(json, "\"etwas ging schief\"");
    }
}
