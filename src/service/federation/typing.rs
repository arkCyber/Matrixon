use tokio_postgres::types::{ToSql, Type};
use chrono::{DateTime, Utc};

impl ToSql for DateTime<Utc> {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let timestamp = self.timestamp();
        timestamp.to_sql(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        Type::INT8.accepts(ty)
    }
}

impl FederationTyping {
    async fn get_remote_servers(&self, room_id: &RoomId) -> Result<Vec<OwnedServerName>> {
        let rows = self.db.query(
            "SELECT DISTINCT server_name FROM room_servers WHERE room_id = $1",
            &[&room_id.to_string()],
        ).await?;

        let servers: Result<Vec<OwnedServerName>, TypingError> = rows
            .iter()
            .map(|row| {
                let server_name: String = row.get(0);
                OwnedServerName::try_from(server_name.as_str())
                    .map_err(|e| TypingError::InvalidData(format!("Invalid server name: {}", e)))
            })
            .collect();

        servers
    }
} 
