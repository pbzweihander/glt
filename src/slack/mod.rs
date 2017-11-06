pub mod slash_command;

#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub domain: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Response {
    Message(Message),
    AttachedMessage(AttachedMessage),
}

#[derive(Serialize)]
pub struct Message {
    pub response_type: ResponseType,
    pub text: String,
    pub mrkdwn: bool,
}

#[derive(Serialize)]
pub struct AttachedMessage {
    pub response_type: ResponseType,
    pub attachments: Vec<Attachment>,
}

#[derive(Serialize)]
pub enum ResponseType {
    #[serde(rename = "in_channel")] InChannel,
    #[serde(rename = "ephemeral")] Ephemeral,
}

#[derive(Serialize)]
pub struct Attachment {
    pub title: String,
    pub pretext: String,
    pub text: String,
    pub fields: Vec<AttachmentFields>,
    pub mrkdwn_in: Vec<String>,
}

#[derive(Serialize)]
pub struct AttachmentFields {
    pub title: String,
    pub value: String,
}
