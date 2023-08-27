use crate::CONNECTIONS;

pub async fn publish_message_inner(msg: String) -> Result<(), String> {
    for (_id, conn) in CONNECTIONS.lock().await.iter() {
        conn.send_text(msg.clone()).await?;
    }
    Ok(())
}
