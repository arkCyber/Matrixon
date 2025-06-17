pub async fn analyze_room(&self, room_id: &RoomId) -> Result<RoomAnalysis> {
    pub async fn get_room_analysis(&self, room_id: &RoomId) -> Result<RoomAnalysis> {
    pub async fn update_room_analysis(&self, analysis: RoomAnalysis) -> Result<()> {
    pub async fn delete_room_analysis(&self, room_id: &RoomId) -> Result<()> {
            return Err(Error::BadRequestString(
                ruma::api::client::error::ErrorKind::Unknown,
                "Room analysis is not enabled".to_string(),
            ));
} 
